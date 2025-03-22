use clap::Parser;
use clap::CommandFactory;
use std::sync::mpsc;
use anyhow::Result;
mod process_handler;
mod ui;
mod settings;
mod commands;
mod log_entry;

#[derive(Parser, Debug)]
#[command(
    author, 
    version, 
    about,
    long_about = "The command to run and capture its output in a neon-themed terminal UI.
    
Key Bindings:
  - q: Quit the application.
  - : (colon): Enter command mode.
  - Up Arrow: Scroll up one line.
  - Down Arrow: Scroll down one line.
  - PageUp: Scroll up one page.
  - PageDown: Scroll down one page.
    
Commands:
  - :show_source stdout/stderr/all
  - :hide_source stdout/stderr/all
  - :show_meta time/source/lines
  - :hide_meta time/source/lines
    
Usage:
  highlog <COMMAND> [ARGS]
For example:
  highlog ls -lR"
)]
struct Args {
    /// The command to run followed by its arguments. Use '--' to separate any flags.
    #[arg(required = true, trailing_var_arg = true)]
    cmd: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.cmd.is_empty() {
        Args::command().print_help().unwrap();
        println!();
        return Ok(());
    }

    let cmd = &args.cmd[0];
    let cmd_args: Vec<&str> = args.cmd.iter().skip(1).map(|s| s.as_str()).collect();

    let (tx, rx) = mpsc::channel::<log_entry::LogEntry>();

    // Spawn the specified process
    process_handler::start_process(cmd, &cmd_args, tx)?;

    // Run the neon-styled UI to display process output
    ui::run_ui(rx);

    Ok(())
}
