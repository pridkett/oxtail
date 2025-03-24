use std::io::{self, Write};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::thread;
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver as CrossbeamReceiver};
use termion::{
    input::TermRead,
    raw::IntoRawMode,
    event::{Event, Key, MouseEvent, MouseButton},
    cursor,
    clear,
    screen::ToAlternateScreen,
    screen::ToMainScreen,
};
use ratatui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use crate::log_entry::LogEntry;
use crate::log_storage::LogStorage;
use crate::settings::LogSettings;
use crate::commands::{self, CommandResult};
use crate::widgets::{CommandPrompt, CommandInputResult, LogViewer, LogViewerExt};

// Helper function to spawn an input handling thread
fn spawn_input_handler() -> CrossbeamReceiver<Event> {
    let (tx, rx) = unbounded();
    
    thread::spawn(move || {
        let tty = termion::get_tty().expect("Failed to get TTY");
        let events = tty.events();
        
        for event in events {
            if let Ok(evt) = event {
                if tx.send(evt).is_err() {
                    // Channel closed, receiver dropped, exit thread
                    break;
                }
            }
        }
    });
    
    rx
}

pub fn run_ui(rx: Receiver<LogEntry>) -> Result<()> {
    // Set up terminal I/O - direct approach without stacking wrappers
    let mut stdout = io::stdout().into_raw_mode()?;
    
    // Setup terminal features by writing escape sequences directly
    // Use raw escape sequences for mouse capture since termion v2.0 might not export them directly
    write!(stdout, "{}{}[?1000h[?1002h[?1015h[?1006h",
        termion::screen::ToAlternateScreen,
        cursor::Hide
    )?;
    stdout.flush()?;
    
    // Prepare backend and terminal
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create a non-blocking event handler
    let events = spawn_input_handler();
    
    // Log storage - manages all log entries and filtering
    let mut log_storage = LogStorage::new();
    let mut previous_filtered_count = 0;
    let mut command_prompt = CommandPrompt::new();
    let mut log_viewer = LogViewer::new();
    let mut settings = LogSettings::default();
    
    // Initialize log storage filter from settings
    log_storage.update_filter_from_settings(&settings);
    
    // Track time for UI refresh
    let mut last_refresh = std::time::Instant::now();
    let refresh_rate = std::time::Duration::from_millis(100); // 10fps refresh rate

    // Main application loop
    let result: Result<()> = (|| {
        loop {
            // Process log entries
            let mut had_new_entries = false;
            while let Ok(entry) = rx.try_recv() {
                log_storage.add_entry(entry);
                had_new_entries = true;
            }
            
            // Scope for handling log storage operations
            {
                let filtered_logs = log_storage.get_filtered_entries();
                let current_count = filtered_logs.len();
                let new_entries_count = current_count.saturating_sub(previous_filtered_count);
                
                // Update previous count early since we have the current count
                if had_new_entries {
                    previous_filtered_count = current_count;
                }

                let has_visible_entries = if had_new_entries && !log_viewer.is_paused() && new_entries_count > 0 {
                    log_storage.has_new_visible_entries()
                } else {
                    false
                };

                // Now that we're done with filtered_logs, we can perform mutable operations
                if had_new_entries {
                    log_viewer.adjust_for_new_entries(new_entries_count);
                }

                // Check if it's time to refresh the UI (either due to new entries or timer)
                let now = std::time::Instant::now();
                if had_new_entries || now.duration_since(last_refresh) >= refresh_rate {
                    // Draw UI
                    terminal.draw(|f| {
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(1),
                                Constraint::Length(1),
                            ])
                            .split(f.size());
        
                        f.render_log_viewer(log_viewer.clone(), chunks[0], &filtered_logs, &settings);
                        f.render_widget(command_prompt.clone(), chunks[1]);
                    })?;
                    
                    last_refresh = now;
                }
            }
            
            if had_new_entries {
                log_storage.clear_new_entries_flags();
            }

            let visible_count = (terminal.size()?.height as usize).saturating_sub(3);
            
            // Non-blocking event check
            if let Ok(event) = events.try_recv() {
                match event {
                    // Handle keyboard events
                    Event::Key(key) => {
                        if command_prompt.is_active() {
                            let (consumed, result) = command_prompt.handle_key_event(key);
                            if consumed {
                                match result {
                                    CommandInputResult::Command(cmd) => {
                                        match commands::execute_command(&cmd, &mut settings) {
                                            CommandResult::Success(_) => {
                                                log_storage.update_filter_from_settings(&settings);
                                                command_prompt.add_to_history(cmd);
                                                command_prompt.deactivate();
                                            },
                                            CommandResult::Error(err) => {
                                                command_prompt.set_status(Some(format!("Error: {}", err)));
                                            },
                                            CommandResult::Quit => {
                                                return Ok(());
                                            },
                                        }
                                    },
                                    CommandInputResult::Cancelled => {
                                        command_prompt.deactivate();
                                    },
                                    CommandInputResult::Pending => {},
                                    CommandInputResult::LineJump(line) => {
                                        let total_lines = log_storage.get_filtered_entries().len();
                                        log_viewer.jump_to_line(line, total_lines);
                                        command_prompt.deactivate();
                                    }
                                }
                            }
                        } else {
                            match key {
                                Key::Char('q') => return Ok(()),
                                Key::Char(':') => {
                                    command_prompt.activate();
                                },
                                Key::Char('r') => {
                                    settings.show_raw = !settings.show_raw;
                                },
                                Key::Char('p') => {
                                    log_viewer.set_paused(!log_viewer.is_paused());
                                },
                                // Vim-style navigation
                                Key::Char('j') | Key::Down => {
                                    log_viewer.scroll_down(1);
                                },
                                Key::Char('k') | Key::Up => {
                                    log_viewer.scroll_up(1);
                                },
                                // Beginning/end navigation
                                Key::Char('g') | Key::Char('<') => {
                                    let total_lines = log_storage.get_filtered_entries().len();
                                    log_viewer.jump_to_start(total_lines);
                                },
                                Key::Char('G') | Key::Char('>') => {
                                    log_viewer.jump_to_end();
                                },
                                Key::PageUp => {
                                    log_viewer.page_up(visible_count);
                                },
                                Key::PageDown => {
                                    log_viewer.page_down(visible_count);
                                },
                                _ => {},
                            }
                        }
                    },
                    // Handle mouse events
                    Event::Mouse(mouse_event) => {
                        match mouse_event {
                            MouseEvent::Press(MouseButton::WheelUp, _, _) => {
                                log_viewer.scroll_up(3);
                            },
                            MouseEvent::Press(MouseButton::WheelDown, _, _) => {
                                log_viewer.scroll_down(3);
                            },
                            MouseEvent::Press(MouseButton::Left, _x, y) => {
                                // Handle click events
                                // Check if click is in command prompt area
                                let term_height = terminal.size()?.height;
                                if y == term_height {
                                    // Clicked on command prompt
                                    if !command_prompt.is_active() {
                                        command_prompt.activate();
                                    }
                                    // Potentially adjust cursor position based on x
                                }
                            },
                            _ => {},
                        }
                    },
                    _ => {},
                }
            } else {
                // Short sleep to avoid CPU spin when there are no events
                // This is much shorter than before to ensure responsive UI
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    })();

    // Reset terminal state when exiting
    write!(
        terminal.backend_mut(),
        "{}{}{}",
        termion::screen::ToMainScreen,
        cursor::Show,
        termion::clear::All
    )?;
    
    // Return any error that occurred
    result
}
