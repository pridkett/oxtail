use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    cursor::MoveTo,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use crate::log_entry::LogEntry;
use crate::log_storage::LogStorage;
use crate::settings::LogSettings;
use crate::commands::{self, CommandResult};
use crate::widgets::{CommandPrompt, CommandInputResult, LogViewer, LogViewerExt};

pub fn run_ui(rx: Receiver<LogEntry>) {
    // Setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Log storage - manages all log entries and filtering
    let mut log_storage = LogStorage::new();
    // Keep track of previous filtered entry count for pause adjustment
    let mut previous_filtered_count = 0;
    // Command prompt widget - this will manage the UI mode state
    let mut command_prompt = CommandPrompt::new();
    // Log viewer widget - this will manage the log display and scroll state
    let mut log_viewer = LogViewer::new();
    // Settings
    let mut settings = LogSettings::default();
    
    // Initialize log storage filter from settings
    log_storage.update_filter_from_settings(&settings);

    // Application loop
    loop {
        // Non-blocking check for new messages
        let mut had_new_entries = false;
        while let Ok(entry) = rx.try_recv() {
            log_storage.add_entry(entry);
            had_new_entries = true;
        }

        // Get filtered logs from storage
        let filtered_logs = log_storage.get_filtered_entries();
        
        // Update scroll position based on new entries and pause state
        if had_new_entries {
            let new_filtered_entries = filtered_logs.len().saturating_sub(previous_filtered_count);
            log_viewer.adjust_for_new_entries(new_filtered_entries);
        }
        
        // Store the current filtered count for next iteration
        previous_filtered_count = filtered_logs.len();

        terminal.draw(|f| {
            // Create the main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Log area
                    Constraint::Length(1), // Status/command line
                ])
                .split(f.size());

            // Render the log viewer widget with filtered logs
            f.render_log_viewer(log_viewer.clone(), chunks[0], &filtered_logs, &settings);
            
            // Render the command prompt widget
            f.render_widget(command_prompt.clone(), chunks[1]);
        }).unwrap();

        if event::poll(Duration::from_millis(200)).unwrap() {
            // Calculate visible lines based on current terminal height for page scrolling
            let visible_count = (terminal.size().unwrap().height as usize).saturating_sub(3);
            
            match event::read().unwrap() {
                Event::Key(key) => {
                    if command_prompt.is_active() {
                        let (consumed, result) = command_prompt.handle_key_event(key);
                        if consumed {
                            match result {
                                CommandInputResult::Command(cmd) => {
                                    match commands::execute_command(&cmd, &mut settings) {
                                        CommandResult::Success(_) => {
                                            // Update log storage filter after settings change
                                            log_storage.update_filter_from_settings(&settings);
                                            command_prompt.add_to_history(cmd);
                                            command_prompt.deactivate();
                                        },
                                        CommandResult::Error(err) => {
                                            command_prompt.set_status(Some(format!("Error: {}", err)));
                                        },
                                        CommandResult::Quit => {
                                            break;
                                        },
                                    }
                                },
                                CommandInputResult::Cancelled => {
                                    command_prompt.deactivate();
                                },
                                CommandInputResult::Pending => {},
                            }
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char(':') => {
                                command_prompt.activate();
                            },
                            KeyCode::Char('r') => {
                                settings.show_raw = !settings.show_raw;
                            },
                            KeyCode::Char('p') | KeyCode::Pause | KeyCode::ScrollLock => {
                                log_viewer.set_paused(!log_viewer.is_paused());
                            },
                            KeyCode::Up => {
                                log_viewer.scroll_up(1);
                            },
                            KeyCode::Down => {
                                log_viewer.scroll_down(1);
                            },
                            KeyCode::PageUp => {
                                log_viewer.page_up(visible_count);
                            },
                            KeyCode::PageDown => {
                                log_viewer.page_down(visible_count);
                            },
                            _ => {},
                        }
                    }
                },
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        crossterm::event::MouseEventKind::ScrollUp => {
                            log_viewer.scroll_up(1);
                        },
                        crossterm::event::MouseEventKind::ScrollDown => {
                            log_viewer.scroll_down(1);
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    terminal.show_cursor().unwrap();
}
