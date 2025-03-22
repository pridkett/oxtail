use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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

pub struct CommandHistory {
    commands: Vec<String>,
    position: Option<usize>,
}

impl CommandHistory {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            position: None,
        }
    }
    
    fn add(&mut self, command: String) {
        if !command.trim().is_empty() {
            self.commands.push(command);
        }
        self.position = None;
    }
    
    fn up(&mut self) -> Option<String> {
        if self.commands.is_empty() {
            return None;
        }
        
        match self.position {
            None => {
                // First time pressing up, start at the most recent command
                self.position = Some(self.commands.len() - 1);
            }
            Some(pos) if pos > 0 => {
                // Move up in history
                self.position = Some(pos - 1);
            }
            _ => {}
        }
        
        self.position.map(|pos| self.commands[pos].clone())
    }
    
    fn down(&mut self) -> Option<String> {
        match self.position {
            Some(pos) if pos < self.commands.len() - 1 => {
                // Move down in history
                self.position = Some(pos + 1);
                Some(self.commands[pos + 1].clone())
            }
            Some(_) => {
                // At the end of history, return to empty
                self.position = None;
                Some(String::new())
            }
            None => None,
        }
    }
    
    fn search(&mut self, query: &str) -> Option<String> {
        if query.is_empty() || self.commands.is_empty() {
            return None;
        }
        
        // Search backwards from current position or end
        let start = self.position.unwrap_or(self.commands.len());
        for i in (0..start).rev() {
            if self.commands[i].contains(query) {
                self.position = Some(i);
                return Some(self.commands[i].clone());
            }
        }
        None
    }
}

pub enum UiMode {
    Normal,
    Command {
        buffer: String,
        status: Option<String>,
        history: CommandHistory,
        cursor_position: usize,
        search_mode: bool,
        search_query: String,
    },
}

pub fn run_ui(rx: Receiver<LogEntry>) {
    // Setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Vector to store log entries received from process_handler
    let mut log_entries: Vec<LogEntry> = Vec::new();
    // Scroll offset: number of lines from the bottom.
    let mut scroll_offset: usize = 0;
    // UI mode
    let mut mode = UiMode::Normal;
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
                .enumerate()
                .map(|(idx, entry)| {
                    let line_number = Some(start + idx + 1);
                    let formatted = entry.format(&settings, line_number);
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

            // Render the status/command line
            let status_text = match &mode {
                UiMode::Normal => {
                    "Press ':' to enter command mode".to_string()
                },
                UiMode::Command { 
                    buffer, 
                    status, 
                    cursor_position,
                    search_mode,
                    search_query,
                    ..
                } => {
                    if *search_mode {
                        format!("(reverse-i-search)`{}': {}", search_query, buffer)
                    } else if let Some(msg) = status {
                        format!(": {} | {}", buffer, msg)
                    } else {
                        // Format with cursor position indicator
                        let cursor_indicator = if *cursor_position < buffer.len() {
                            // Cursor within the text
                            format!("{}\u{2588}{}", 
                                &buffer[..*cursor_position], 
                                &buffer[*cursor_position..])
                        } else {
                            // Cursor at the end
                            format!("{}\u{2588}", buffer)
                        };
                        format!(": {}", cursor_indicator)
                    }
                },
            };
            
            let status_style = match &mode {
                UiMode::Normal => Style::default().fg(Color::Gray),
                UiMode::Command { status, search_mode, .. } => {
                    if *search_mode {
                        Style::default().fg(Color::Blue)
                    } else if status.is_some() {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    }
                },
            };
            
            let status_bar = Paragraph::new(Spans::from(Span::styled(
                status_text,
                status_style,
            )));
            
            f.render_widget(status_bar, chunks[1]);
        }).unwrap();

        if event::poll(Duration::from_millis(200)).unwrap() {
            // Calculate visible lines based on current terminal height
            let visible_count = (terminal.size().unwrap().height as usize).saturating_sub(3); // -2 for log borders, -1 for status
            
            match event::read().unwrap() {
                Event::Key(key) => match &mut mode {
                    UiMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(':') => {
                            mode = UiMode::Command {
                                buffer: String::new(),
                                status: None,
                                history: CommandHistory::new(),
                                cursor_position: 0,
                                search_mode: false,
                                search_query: String::new(),
                            };
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
                    },
                    UiMode::Command { 
                        buffer, 
                        status, 
                        history, 
                        cursor_position,
                        search_mode,
                        search_query,
                    } => {
                        // Handle search mode separately
                        if *search_mode {
                            match key.code {
                                KeyCode::Esc => {
                                    // Exit search mode but stay in command mode
                                    *search_mode = false;
                                    *search_query = String::new();
                                },
                                KeyCode::Enter => {
                                    // Accept the search result and exit search mode
                                    *search_mode = false;
                                    *search_query = String::new();
                                    *cursor_position = buffer.len();
                                },
                                KeyCode::Char(c) => {
                                    // Add character to search query and search
                                    search_query.push(c);
                                    if let Some(result) = history.search(search_query) {
                                        *buffer = result;
                                        *cursor_position = buffer.len();
                                    }
                                },
                                KeyCode::Backspace => {
                                    // Remove character from search query and search again
                                    if !search_query.is_empty() {
                                        search_query.pop();
                                        if let Some(result) = history.search(search_query) {
                                            *buffer = result;
                                            *cursor_position = buffer.len();
                                        }
                                    }
                                },
                                _ => {},
                            }
                        } else {
                            // Regular command mode
                            match key.code {
                                KeyCode::Esc => {
                                    mode = UiMode::Normal;
                                },
                                KeyCode::Enter => {
                                    // Execute the command
                                    let cmd = buffer.clone();
                                    match commands::execute_command(buffer, &mut settings) {
                                        CommandResult::Success(_msg) => {
                                            // Add command to history
                                            history.add(cmd);
                                            // Return to normal mode
                                            mode = UiMode::Normal;
                                        },
                                        CommandResult::Error(err) => {
                                            *status = Some(format!("Error: {}", err));
                                        },
                                    }
                                },
                                KeyCode::Char(c) if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    // Handle special control key combinations
                                    match c {
                                        'a' => {
                                            // Ctrl+A: Move to beginning of line
                                            *cursor_position = 0;
                                        },
                                        'e' => {
                                            // Ctrl+E: Move to end of line
                                            *cursor_position = buffer.len();
                                        },
                                        'k' => {
                                            // Ctrl+K: Kill to end of line
                                            if *cursor_position < buffer.len() {
                                                buffer.truncate(*cursor_position);
                                            }
                                        },
                                        'u' => {
                                            // Ctrl+U: Kill to beginning of line
                                            if *cursor_position > 0 {
                                                *buffer = buffer[*cursor_position..].to_string();
                                                *cursor_position = 0;
                                            }
                                        },
                                        'w' => {
                                            // Ctrl+W: Delete word backward
                                            let mut new_pos = *cursor_position;
                                            // Skip spaces
                                            while new_pos > 0 && buffer.chars().nth(new_pos - 1).unwrap_or(' ').is_whitespace() {
                                                new_pos -= 1;
                                            }
                                            // Skip non-spaces
                                            while new_pos > 0 && !buffer.chars().nth(new_pos - 1).unwrap_or(' ').is_whitespace() {
                                                new_pos -= 1;
                                            }
                                            
                                            if new_pos < *cursor_position {
                                                buffer.replace_range(new_pos..*cursor_position, "");
                                                *cursor_position = new_pos;
                                            }
                                        },
                                        'r' => {
                                            // Ctrl+R: Reverse search
                                            *search_mode = true;
                                            *search_query = String::new();
                                        },
                                        _ => {} // Ignore other control characters
                                    }
                                },
                                KeyCode::Char(c) => {
                                    *status = None; // Clear status when editing
                                    
                                    // Insert character at cursor position
                                    if *cursor_position >= buffer.len() {
                                        buffer.push(c);
                                    } else {
                                        buffer.insert(*cursor_position, c);
                                    }
                                    *cursor_position += 1;
                                },
                                KeyCode::Backspace => {
                                    *status = None; // Clear status when editing
                                    
                                    // Handle backspace at cursor position
                                    if *cursor_position > 0 {
                                        *cursor_position -= 1;
                                        if *cursor_position < buffer.len() {
                                            buffer.remove(*cursor_position);
                                        }
                                    }
                                },
                                KeyCode::Delete => {
                                    *status = None;
                                    // Delete character under cursor
                                    if *cursor_position < buffer.len() {
                                        buffer.remove(*cursor_position);
                                    }
                                },
                                KeyCode::Left => {
                                    // Move cursor left
                                    if *cursor_position > 0 {
                                        *cursor_position -= 1;
                                    }
                                },
                                KeyCode::Right => {
                                    // Move cursor right
                                    if *cursor_position < buffer.len() {
                                        *cursor_position += 1;
                                    }
                                },
                                KeyCode::Home => {
                                    // Move to beginning of line
                                    *cursor_position = 0;
                                },
                                KeyCode::End => {
                                    // Move to end of line
                                    *cursor_position = buffer.len();
                                },
                                KeyCode::Up => {
                                    // Navigate command history backward
                                    if let Some(cmd) = history.up() {
                                        *buffer = cmd;
                                        *cursor_position = buffer.len();
                                    }
                                },
                                KeyCode::Down => {
                                    // Navigate command history forward
                                    if let Some(cmd) = history.down() {
                                        *buffer = cmd;
                                        *cursor_position = buffer.len();
                                    } else {
                                        *buffer = String::new();
                                        *cursor_position = 0;
                                    }
                                },
                                _ => {},
                            }
                        }
                    },
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
