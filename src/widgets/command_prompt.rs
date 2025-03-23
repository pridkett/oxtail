use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
};

/// Result returned after command input is complete
#[derive(Debug, Clone)]
pub enum CommandInputResult {
    /// Command was accepted and should be processed
    Command(String),
    /// User cancelled the command input
    Cancelled,
    /// Still in input mode, no command to process yet
    Pending,
}

/// Manages command history for the command prompt
#[derive(Debug, Clone, Default)]
pub struct CommandHistory {
    commands: Vec<String>,
    position: Option<usize>,
}

impl CommandHistory {
    /// Create a new empty command history
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            position: None,
        }
    }
    
    /// Add a command to the history
    pub fn add(&mut self, command: String) {
        if !command.trim().is_empty() {
            self.commands.push(command);
        }
        self.position = None;
    }
    
    /// Navigate up in command history
    pub fn up(&mut self) -> Option<String> {
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
    
    /// Navigate down in command history
    pub fn down(&mut self) -> Option<String> {
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
    
    /// Search command history for a given query
    pub fn search(&mut self, query: &str) -> Option<String> {
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

/// A command prompt widget for ratatui
#[derive(Debug, Clone)]
pub struct CommandPrompt {
    /// The current input buffer
    buffer: String,
    /// Current cursor position within the buffer
    cursor_position: usize,
    /// Command history
    history: CommandHistory,
    /// Status message (shown after command execution)
    status: Option<String>,
    /// Whether we're in search mode
    search_mode: bool,
    /// Current search query
    search_query: String,
    /// Whether the prompt is active
    active: bool,
}

impl Default for CommandPrompt {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPrompt {
    /// Create a new command prompt widget
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_position: 0,
            history: CommandHistory::new(),
            status: None,
            search_mode: false,
            search_query: String::new(),
            active: false,
        }
    }
    
    /// Activate the command prompt
    pub fn activate(&mut self) {
        self.active = true;
        self.buffer.clear();
        self.cursor_position = 0;
        self.status = None;
        self.search_mode = false;
        self.search_query.clear();
    }
    
    /// Deactivate the command prompt
    pub fn deactivate(&mut self) {
        self.active = false;
        self.buffer.clear();
        self.cursor_position = 0;
        self.status = None;
        self.search_mode = false;
        self.search_query.clear();
    }
    
    /// Check if the prompt is currently active
    pub fn is_active(&self) -> bool {
        self.active
    }
    
    /// Set a status message
    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }
    
    /// Add a command to history
    pub fn add_to_history(&mut self, command: String) {
        self.history.add(command);
    }
    
    /// Handle keyboard input, returning whether the input was consumed
    /// and any completed command
    pub fn handle_key_event(&mut self, key: KeyEvent) -> (bool, CommandInputResult) {
        if !self.active {
            return (false, CommandInputResult::Pending);
        }
        
        // Handle search mode separately
        if self.search_mode {
            match key.code {
                KeyCode::Esc => {
                    // Exit search mode but stay in command mode
                    self.search_mode = false;
                    self.search_query.clear();
                    return (true, CommandInputResult::Pending);
                },
                KeyCode::Enter => {
                    // Accept the search result and exit search mode
                    self.search_mode = false;
                    self.search_query.clear();
                    self.cursor_position = self.buffer.len();
                    return (true, CommandInputResult::Pending);
                },
                KeyCode::Char(c) => {
                    // Add character to search query and search
                    self.search_query.push(c);
                    if let Some(result) = self.history.search(&self.search_query) {
                        self.buffer = result;
                        self.cursor_position = self.buffer.len();
                    }
                    return (true, CommandInputResult::Pending);
                },
                KeyCode::Backspace => {
                    // Remove character from search query and search again
                    if !self.search_query.is_empty() {
                        self.search_query.pop();
                        if let Some(result) = self.history.search(&self.search_query) {
                            self.buffer = result;
                            self.cursor_position = self.buffer.len();
                        }
                    }
                    return (true, CommandInputResult::Pending);
                },
                _ => return (true, CommandInputResult::Pending),
            }
        } else {
            // Regular command mode
            match key.code {
                KeyCode::Esc => {
                    return (true, CommandInputResult::Cancelled);
                },
                KeyCode::Enter => {
                    // Return the command for execution
                    let cmd = self.buffer.clone();
                    return (true, CommandInputResult::Command(cmd));
                },
                KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Handle special control key combinations
                    match c {
                        'a' => {
                            // Ctrl+A: Move to beginning of line
                            self.cursor_position = 0;
                        },
                        'e' => {
                            // Ctrl+E: Move to end of line
                            self.cursor_position = self.buffer.len();
                        },
                        'k' => {
                            // Ctrl+K: Kill to end of line
                            if self.cursor_position < self.buffer.len() {
                                self.buffer.truncate(self.cursor_position);
                            }
                        },
                        'u' => {
                            // Ctrl+U: Kill to beginning of line
                            if self.cursor_position > 0 {
                                self.buffer = self.buffer[self.cursor_position..].to_string();
                                self.cursor_position = 0;
                            }
                        },
                        'w' => {
                            // Ctrl+W: Delete word backward
                            let mut new_pos = self.cursor_position;
                            // Skip spaces
                            while new_pos > 0 && self.buffer.chars().nth(new_pos - 1).unwrap_or(' ').is_whitespace() {
                                new_pos -= 1;
                            }
                            // Skip non-spaces
                            while new_pos > 0 && !self.buffer.chars().nth(new_pos - 1).unwrap_or(' ').is_whitespace() {
                                new_pos -= 1;
                            }
                            
                            if new_pos < self.cursor_position {
                                self.buffer.replace_range(new_pos..self.cursor_position, "");
                                self.cursor_position = new_pos;
                            }
                        },
                        'r' => {
                            // Ctrl+R: Reverse search
                            self.search_mode = true;
                            self.search_query.clear();
                        },
                        _ => {} // Ignore other control characters
                    }
                },
                KeyCode::Char(c) => {
                    self.status = None; // Clear status when editing
                    
                    // Insert character at cursor position
                    if self.cursor_position >= self.buffer.len() {
                        self.buffer.push(c);
                    } else {
                        self.buffer.insert(self.cursor_position, c);
                    }
                    self.cursor_position += 1;
                },
                KeyCode::Backspace => {
                    self.status = None; // Clear status when editing
                    
                    // Handle backspace at cursor position
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        if self.cursor_position < self.buffer.len() {
                            self.buffer.remove(self.cursor_position);
                        }
                    }
                },
                KeyCode::Delete => {
                    self.status = None;
                    // Delete character under cursor
                    if self.cursor_position < self.buffer.len() {
                        self.buffer.remove(self.cursor_position);
                    }
                },
                KeyCode::Left => {
                    // Move cursor left
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                },
                KeyCode::Right => {
                    // Move cursor right
                    if self.cursor_position < self.buffer.len() {
                        self.cursor_position += 1;
                    }
                },
                KeyCode::Home => {
                    // Move to beginning of line
                    self.cursor_position = 0;
                },
                KeyCode::End => {
                    // Move to end of line
                    self.cursor_position = self.buffer.len();
                },
                KeyCode::Up => {
                    // Navigate command history backward
                    if let Some(cmd) = self.history.up() {
                        self.buffer = cmd;
                        self.cursor_position = self.buffer.len();
                    }
                },
                KeyCode::Down => {
                    // Navigate command history forward
                    if let Some(cmd) = self.history.down() {
                        self.buffer = cmd;
                        self.cursor_position = self.buffer.len();
                    } else {
                        self.buffer = String::new();
                        self.cursor_position = 0;
                    }
                },
                _ => {},
            }
            return (true, CommandInputResult::Pending);
        }
    }
}

impl Widget for CommandPrompt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.active && self.status.is_none() {
            // In normal mode, just show a helpful message
            let normal_text = "Press ':' to enter command mode";
            let span = Span::styled(normal_text, Style::default().fg(Color::Gray));
            buf.set_span(area.x, area.y, &span, area.width);
            return;
        }
        
        let display_text = if self.search_mode {
            format!("(reverse-i-search)`{}': {}", self.search_query, self.buffer)
        } else if let Some(ref msg) = self.status {
            format!(":{} | {}", self.buffer, msg)
        } else {
            // Format with cursor position indicator
            let cursor_indicator = if self.cursor_position < self.buffer.len() {
                // Cursor within the text
                format!("{}\u{2588}{}", 
                    &self.buffer[..self.cursor_position], 
                    &self.buffer[self.cursor_position..])
            } else {
                // Cursor at the end
                format!("{}\u{2588}", self.buffer)
            };
            format!(":{}", cursor_indicator)
        };
        
        let style = if self.search_mode {
            Style::default().fg(Color::Blue)
        } else if self.status.is_some() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };
        
        let span = Span::styled(display_text, style);
        buf.set_span(area.x, area.y, &span, area.width);
    }
}
