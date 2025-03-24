use clap::Parser;
use clap::CommandFactory;
use std::sync::mpsc;
use anyhow::{Result, Context};
use std::path::PathBuf;
mod process_handler;
mod ui;
mod settings;
mod commands;
mod log_entry;
mod log_storage;
mod widgets;
mod file_watcher;
mod stdin_reader;

#[derive(Parser, Debug)]
#[command(
    author, 
    version, 
    about,
    long_about = "Monitor files and process output in a neon-themed terminal UI.
    
Key Bindings:
  - q: Quit the application
  - : (colon): Enter command mode
  - Up Arrow: Scroll up one line
  - Down Arrow: Scroll down one line
  - PageUp: Scroll up one page
  - PageDown: Scroll down one page
    
Commands:
  - :show_source stdout/stderr/file/<filename>/stdin
  - :hide_source stdout/stderr/file/<filename>/stdin
  - :show_meta time/source/lines
  - :hide_meta time/source/lines
    
Usage:
  oxtail [FILES]... [-- COMMAND [ARGS]...]
For example:
  oxtail a.log b.log -- ./server
  oxtail error.log -- npm start
  oxtail app.log test.log
  cat log.txt | oxtail"
)]
struct Args {
    /// Files to monitor
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    files: Vec<PathBuf>,

    /// The command to run followed by its arguments (after --)
    #[arg(last = true)]
    cmd: Vec<String>,
}

use std::io::{self, BufRead};
use std::fs::File;
use chrono::Local;

fn main() -> Result<()> {
    let args = Args::parse();

    // Only show help if we have no inputs at all (no files, no command, and no stdin)
    if args.files.is_empty() && args.cmd.is_empty() && atty::is(atty::Stream::Stdin) {
        Args::command().print_help().context("Failed to print help")?;
        println!();
        return Ok(());
    }

    // Check if we can access /dev/tty for keyboard input
    // This is required for the TUI to work with stdin piping
    let has_tty_access = File::open("/dev/tty").is_ok();
    let has_stdin_pipe = !atty::is(atty::Stream::Stdin);
    let stdin_only = has_stdin_pipe && args.files.is_empty() && args.cmd.is_empty();
    
    // Use non-interactive mode when:
    // 1. We have piped stdin and no other inputs (files or commands)
    // 2. AND we can't access /dev/tty for keyboard input
    let use_non_interactive = stdin_only && !has_tty_access;

    if use_non_interactive {
        // SIMPLE MODE: Process stdin and format output without a TUI
        // This mode works even when stdin is piped
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut buffer = String::new();
        let mut line_number = 0;
        
        while reader.read_line(&mut buffer)? > 0 {
            let content = buffer.trim_end().to_string();
            if !content.is_empty() {
                // Format the output similar to how the TUI would
                let timestamp = Local::now();
                println!("[{}] [stdin:{}] {}", 
                    timestamp.format("%H:%M:%S%.3f"),
                    line_number,
                    content);
                line_number += 1;
            }
            buffer.clear();
        }
        
        return Ok(());
    } else {
        // INTERACTIVE MODE: Full terminal UI with all sources
        let (tx, rx) = mpsc::channel::<log_entry::LogEntry>();

        // Start file watchers if files are specified
        if !args.files.is_empty() {
            file_watcher::start_watching(args.files.clone(), tx.clone())
                .context("Failed to start file watcher")?;
        }

        // Spawn the specified process if a command was given
        if !args.cmd.is_empty() {
            let cmd = &args.cmd[0];
            let cmd_args: Vec<&str> = args.cmd.iter().skip(1).map(|s| s.as_str()).collect();
            process_handler::start_process(cmd, &cmd_args, tx.clone())
                .context("Failed to start process")?;
        }

        // In interactive mode, we can safely enable stdin reading if stdin is not a terminal
        // but only if other sources are also present
        if !atty::is(atty::Stream::Stdin) {
            stdin_reader::start_reading_stdin(tx.clone()).context("Failed to initialize input reader")?;
        }

        // Run the neon-styled UI to display output
        ui::run_ui(rx)
            .context("UI error")?;
    }

    Ok(())
}
