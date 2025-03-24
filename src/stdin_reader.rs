use std::io::{self, BufRead};
use std::sync::mpsc::Sender;
use std::thread;
use chrono::Local;
use crate::log_entry::LogEntry;
use anyhow::{Result, Context};

/// Starts reading from stdin in a separate thread
/// This is kept simple - just a thread that reads from stdin and sends log entries
/// The UI will read keyboard events from /dev/tty instead to avoid conflicts
pub fn start_reading_stdin(tx: Sender<LogEntry>) -> Result<()> {
    // Skip if stdin is a terminal
    if atty::is(atty::Stream::Stdin) {
        return Ok(());
    }

    // Spawn a thread to read from stdin
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut line_number = 0;
        
        // Process each line from stdin
        for line in stdin.lock().lines() {
            match line {
                Ok(content) if !content.is_empty() => {
                    // Create a log entry for this line
                    let entry = LogEntry {
                        timestamp: Local::now(),
                        source: "stdin".to_string(),
                        content: content.clone(),
                        content_plain: content,
                        line_number,
                        is_json: false,  // Let LogEntry handle JSON detection
                    };
                    line_number += 1;
                    
                    // Send to the main thread
                    if tx.send(entry).is_err() {
                        break; // Channel closed, stop reading
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    break;
                }
                _ => continue,
            }
        }
    });

    Ok(())
}
