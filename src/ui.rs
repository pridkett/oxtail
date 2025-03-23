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
    // Command prompt widget - this will manage the UI mode state
    let mut command_prompt = CommandPrompt::new();
    // Settings
    let mut settings = LogSettings::default();

    // Application loop
    loop {
        // Non-blocking check for new messages
        while let Ok(entry) = rx.try_recv() {
            log_entries.push(entry);
        }

        terminal.draw(|f| {
            // Create the main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Log area
                    Constraint::Length(1), // Status/command line
                ])
                .split(f.size());

            // Filter logs based on settings
            let filtered_logs: Vec<&LogEntry> = log_entries.iter()
                .filter(|entry| settings.is_source_visible(&entry.source))
                .collect();

            // Calculate visible lines
            let total_filtered_lines = filtered_logs.len();
            let log_area_height = chunks[0].height as usize - 2; // Subtract 2 for the borders
            let adjusted_scroll_offset = scroll_offset.min(total_filtered_lines.saturating_sub(log_area_height));
            
            let start = if total_filtered_lines > log_area_height + adjusted_scroll_offset {
                total_filtered_lines - log_area_height - adjusted_scroll_offset
            } else {
                0
            };
            let end = total_filtered_lines.saturating_sub(adjusted_scroll_offset);
            
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

            // Render the log area
            let log_block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    "Oxtail - Neon Terminal UI",
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
            let visible_count = (terminal.size().unwrap().height as usize).saturating_sub(3); // -2 for log borders, -1 for status
            
            match event::read().unwrap() {
                Event::Key(key) => {
                    // Check if command prompt is active and should handle the event
                    if command_prompt.is_active() {
                        let (consumed, result) = command_prompt.handle_key_event(key);
                        if consumed {
                            match result {
                                CommandInputResult::Command(cmd) => {
                                    // Process the command
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
                                CommandInputResult::Pending => {
                                    // Continue input
                                },
                            }
                        }
                    } else {
                        // Handle normal mode keys
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char(':') => {
                                command_prompt.activate();
                            },
                            KeyCode::Char('r') => {
                                // Toggle show_raw setting
                                settings.show_raw = !settings.show_raw;
                            },
                            KeyCode::Up => {
                                if scroll_offset + 1 <= log_entries.len().saturating_sub(visible_count) {
                                    scroll_offset += 1;
                                }
                            },
                            KeyCode::Down => {
                                if scroll_offset > 0 {
                                    scroll_offset -= 1;
                                }
                            },
                            KeyCode::PageUp => {
                                let increment = visible_count;
                                if scroll_offset + increment <= log_entries.len().saturating_sub(visible_count) {
                                    scroll_offset += increment;
                                } else {
                                    scroll_offset = log_entries.len().saturating_sub(visible_count);
                                }
                            },
                            KeyCode::PageDown => {
                                if scroll_offset >= visible_count {
                                    scroll_offset -= visible_count;
                                } else {
                                    scroll_offset = 0;
                                }
                            },
                            _ => {},
                        }
                    }
                },
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        crossterm::event::MouseEventKind::ScrollUp => {
                            if scroll_offset + 1 <= log_entries.len().saturating_sub(visible_count) {
                                scroll_offset += 1;
                            }
                        },
                        crossterm::event::MouseEventKind::ScrollDown => {
                            if scroll_offset > 0 {
                                scroll_offset -= 1;
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
