use anyhow::Result;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use crate::log_entry::LogEntry;

pub fn start_process(cmd: &str, args: &[&str], tx: Sender<LogEntry>) -> Result<()> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Capture stdout in a separate thread
    let tx_stdout = tx.clone();
    thread::spawn(move || {
        let mut line_number = 1;
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let mut entry = LogEntry::new("stdout", l);
                entry.line_number = line_number;
                let _ = tx_stdout.send(entry);
                line_number += 1;
            }
        }
    });

    // Capture stderr in a separate thread
    thread::spawn(move || {
        let mut line_number = 1;
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                let mut entry = LogEntry::new("stderr", l);
                entry.line_number = line_number;
                let _ = tx.send(entry);
                line_number += 1;
            }
        }
    });

    Ok(())
}
