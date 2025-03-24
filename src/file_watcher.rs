use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::SystemTime;
use anyhow::{Result, Context};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::io::{self, BufReader, BufRead, Seek, SeekFrom};
use std::fs::File;
use chrono::Local;
use crate::log_entry::LogEntry;

struct FileState {
    last_modified: SystemTime,
    last_size: u64,
    last_position: u64,
}

pub fn start_watching(files: Vec<PathBuf>, tx: Sender<LogEntry>) -> Result<()> {
    // First, read the current contents of all files
    for file in &files {
        read_file_contents(file, &tx)?;
    }

    // Create a channel for notify events
    let (watcher_tx, watcher_rx) = std::sync::mpsc::channel();

    // Create a watcher
    let mut watcher = notify::recommended_watcher(watcher_tx)?;

    // Start watching each file
    for file in &files {
        watcher.watch(file, RecursiveMode::NonRecursive)?;
    }

    // Spawn a thread to handle file changes
    std::thread::spawn(move || {
        let mut file_states: std::collections::HashMap<PathBuf, FileState> = std::collections::HashMap::new();

        for res in watcher_rx {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        for path in event.paths {
                            // Check if the file was modified since our last read
                            let metadata = match std::fs::metadata(&path) {
                                Ok(m) => m,
                                Err(_) => continue,
                            };

                            let modified = metadata.modified().unwrap_or(SystemTime::now());
                            let current_size = metadata.len();

                            let state = file_states.entry(path.clone()).or_insert(FileState {
                                last_modified: modified,
                                last_size: current_size,
                                last_position: 0,
                            });

                            // Skip if modification time hasn't changed
                            if state.last_modified >= modified {
                                continue;
                            }

                            // Update the state
                            state.last_modified = modified;

                            // Handle file truncation
                            if current_size < state.last_size {
                                state.last_position = 0;
                            }
                            state.last_size = current_size;

                            // Read new content
                            if let Err(e) = read_new_content(&path, &tx, state) {
                                eprintln!("Error reading file {}: {}", path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Watch error: {}", e),
            }
        }
    });

    Ok(())
}

fn read_file_contents(path: &Path, tx: &Sender<LogEntry>) -> Result<()> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let source = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    for line in reader.lines() {
        let content = line.with_context(|| format!("Failed to read line from {}", path.display()))?;
        if !content.is_empty() {
            tx.send(LogEntry {
                timestamp: Local::now(),
                source: source.clone(),
                content: content.clone(),
                content_plain: content,
                line_number: 0,  // Will be set by LogSource
                is_json: false,  // Let LogEntry handle JSON detection
            })?;
        }
    }

    Ok(())
}

fn read_new_content(path: &Path, tx: &Sender<LogEntry>, state: &mut FileState) -> Result<()> {
    let mut file = File::open(path)?;
    
    // First seek to the last position
    file.seek(SeekFrom::Start(state.last_position))?;
    
    // Create reader after getting current position
    let reader = BufReader::new(&file);
    let source = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    for line in reader.lines() {
        let content = line?;
        if !content.is_empty() {
            tx.send(LogEntry {
                timestamp: Local::now(),
                source: source.clone(),
                content: content.clone(),
                content_plain: content,
                line_number: 0,  // Will be set by LogSource
                is_json: false,  // Let LogEntry handle JSON detection
            })?;
        }
    }

    // Get the current position after reading
    state.last_position = file.stream_position()?;

    Ok(())
}