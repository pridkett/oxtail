use anyhow::Result;
use chrono::Local;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

pub fn start_process(cmd: &str, args: &[&str], tx: Sender<String>) -> Result<()> {
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
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let output = format!(
                    "[{}][STDOUT] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    l
                );
                let _ = tx_stdout.send(output);
            }
        }
    });

    // Capture stderr in a separate thread
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                let output = format!(
                    "[{}][STDERR] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    l
                );
                let _ = tx.send(output);
            }
        }
    });

    Ok(())
}
