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
    widgets::{Block, Borders, Paragraph, Wrap},
    text::{Span, Spans},
    style::{Color, Modifier, Style},
};
use crate::log_entry::LogEntry;
use crate::settings::LogSettings;
use crate::commands::{self, CommandResult};
use crate::widgets::CommandPrompt;
use crate::widgets::CommandInputResult;

pub fn run_ui(rx: Receiver<LogEntry>) {
    // Setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Vector to store log entries received from process_handler
    let mut log_entries: Vec<LogEntry> = Vec::new();
    // Scroll offset: number of lines from the bottom.
    let mut scroll_offset: usize = 0;
    // Track whether output is paused
    let mut is_paused = false;
    // Keep track of previous filtered entry count for pause adjustment
    let mut previous_filtered_count = 0;
    // Command prompt widget - this will manage the UI mode state
    let mut command_prompt = CommandPrompt::new();
    // Settings
    let mut settings = LogSettings::default();

    // Application loop
    loop {
        // Non-blocking check for new messages
        let mut had_new_entries = false;
        while let Ok(entry) = rx.try_recv() {
            log_entries.push(entry);
            had_new_entries = true;
        }

        // Filter logs based on settings (do this before scroll adjustment)
        let filtered_logs: Vec<&LogEntry> = log_entries.iter()
            .filter(|entry| settings.is_source_visible(&entry.source))
            .collect();
        
        // Update scroll position based on new entries and pause state
        if had_new_entries {
            let new_filtered_entries = filtered_logs.len().saturating_sub(previous_filtered_count);
            if is_paused {
                // When paused, always maintain relative position
                scroll_offset += new_filtered_entries;
            } else if scroll_offset > new_filtered_entries {
                // When not paused but scrolled up, maintain some position but allow gradual scrolling
                scroll_offset -= new_filtered_entries;
            } else {
                // When not paused and near bottom, scroll to bottom
                scroll_offset = 0;
            }
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

            // Calculate visible lines
            let total_filtered_lines = filtered_logs.len();
            let log_area_height = chunks[0].height as usize - 2; // Subtract 2 for the borders
            
            // Ensure scroll_offset doesn't exceed the available lines
            scroll_offset = scroll_offset.min(total_filtered_lines.saturating_sub(log_area_height));
            
            let start = if total_filtered_lines > log_area_height + scroll_offset {
                total_filtered_lines - log_area_height - scroll_offset
            } else {
                0
            };
            let end = total_filtered_lines.saturating_sub(scroll_offset);

            // Format the visible lines based on settings
            let display_lines: Vec<Spans> = filtered_logs[start..end]
                .iter()
                .map(|entry| {
                    let formatted = entry.format(&settings, None);
                    let style = match entry.source.as_str() {
                        "stderr" => Style::default().fg(Color::Red),
                        "stdout" => Style::default().fg(Color::Yellow),
                        _ => Style::default().fg(Color::White),
                    };
                    Spans::from(Span::styled(formatted, style))
                })
                .collect();

            // Create title with pause indicator
            let title = if is_paused {
                "Oxtail - [PAUSED] - Neon Terminal UI"
            } else {
                "Oxtail - Neon Terminal UI"
            };

            // Render the log area
            let log_block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    title,
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                ));
            
            let logs_paragraph = Paragraph::new(display_lines)
                .block(log_block)
                .wrap(Wrap { trim: true });
            
            f.render_widget(logs_paragraph, chunks[0]);
            
            // Render the command prompt widget
            f.render_widget(command_prompt.clone(), chunks[1]);
        }).unwrap();

        if event::poll(Duration::from_millis(200)).unwrap() {
            // Calculate visible lines based on current terminal height
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
                                is_paused = !is_paused;
                            },
                            KeyCode::Up => {
                                if scroll_offset + 1 <= filtered_logs.len().saturating_sub(visible_count) {
                                    scroll_offset += 1;
                                    if !is_paused {
                                        is_paused = true;
                                    }
                                }
                            },
                            KeyCode::Down => {
                                if scroll_offset > 0 {
                                    scroll_offset -= 1;
                                    // When we reach the bottom, unpause
                                    if scroll_offset == 0 {
                                        is_paused = false;
                                    }
                                }
                            },
                            KeyCode::PageUp => {
                                let increment = visible_count;
                                if scroll_offset + increment <= filtered_logs.len().saturating_sub(visible_count) {
                                    scroll_offset += increment;
                                    if !is_paused {
                                        is_paused = true;
                                    }
                                } else {
                                    scroll_offset = filtered_logs.len().saturating_sub(visible_count);
                                }
                            },
                            KeyCode::PageDown => {
                                if scroll_offset >= visible_count {
                                    scroll_offset -= visible_count;
                                } else {
                                    scroll_offset = 0;
                                }
                                // When we reach the bottom, unpause
                                if scroll_offset == 0 {
                                    is_paused = false;
                                }
                            },
                            _ => {},
                        }
                    }
                },
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        crossterm::event::MouseEventKind::ScrollUp => {
                            if scroll_offset + 1 <= filtered_logs.len().saturating_sub(visible_count) {
                                scroll_offset += 1;
                                if !is_paused {
                                    is_paused = true;
                                }
                            }
                        },
                        crossterm::event::MouseEventKind::ScrollDown => {
                            if scroll_offset > 0 {
                                scroll_offset -= 1;
                                // When we reach the bottom, unpause
                                if scroll_offset == 0 {
                                    is_paused = false;
                                }
                            }
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
