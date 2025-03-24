use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use crate::log_entry::LogEntry;
use crate::settings::LogSettings;
use ansi_parser::{Output, AnsiParser};
use unicode_width::UnicodeWidthChar;

/// A widget for displaying log entries
#[derive(Debug, Clone)]
pub struct LogViewer {
    /// Scroll offset: number of lines from the bottom
    scroll_offset: usize,
    /// Whether output is paused
    is_paused: bool,
    /// Widget title
    title: String,
}

impl Default for LogViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl LogViewer {
    /// Create a new log viewer widget
    pub fn new() -> Self {
        Self {
            scroll_offset: 0, // offset is the number of lines up from the bottom
            is_paused: false, // if true it should now scroll
            title: "Oxtail - Neon Terminal UI".to_string(),
        }
    }
    
    /// Set whether the log viewer is paused
    pub fn set_paused(&mut self, paused: bool) -> &mut Self {
        self.is_paused = paused;
        self
    }
    
    /// Get whether the log viewer is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }
    
    /// Set the title of the log viewer
    #[allow(dead_code)]
    pub fn set_title<S: Into<String>>(&mut self, title: S) -> &mut Self {
        self.title = title.into();
        self
    }
    
    /// Get the current scroll offset
    #[allow(dead_code)]
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
    
    /// Set the scroll offset
    #[allow(dead_code)]
    pub fn set_scroll_offset(&mut self, offset: usize) -> &mut Self {
        self.scroll_offset = offset;
        self
    }
    
    /// Scroll up by the specified amount
    pub fn scroll_up(&mut self, amount: usize) -> &mut Self {
        // FIXME: this should be capped at the number of lines in the log
        self.scroll_offset += amount;
        self.set_paused(true);
        self
    }
    
    /// Scroll down by the specified amount, bounded by max_scroll
    pub fn scroll_down(&mut self, amount: usize) -> &mut Self {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        
        // When we reach the bottom, unpause
        if self.scroll_offset == 0 {
            self.set_paused(false);
        }
        self
    }
    
    /// Page up (scroll up by a page)
    pub fn page_up(&mut self, page_size: usize) -> &mut Self {
        self.scroll_up(page_size)
    }
    
    /// Page down (scroll down by a page)
    pub fn page_down(&mut self, page_size: usize) -> &mut Self {
        self.scroll_down(page_size)
    }

    /// Jump to a specific line number (1-based)
    pub fn jump_to_line(&mut self, line_number: usize, total_lines: usize) -> &mut Self {
        if line_number == 0 || total_lines == 0 {
            return self;
        }

        // Convert 1-based line number to 0-based index
        let target_line = line_number.saturating_sub(1);
        
        // Calculate scroll offset to show the target line
        // We want the target line to appear at the top of the view if possible
        if target_line >= total_lines {
            self.scroll_offset = 0; // If target is beyond total lines, go to bottom
        } else {
            self.scroll_offset = total_lines.saturating_sub(target_line).saturating_sub(1);
        }
        
        self.set_paused(true);
        self
    }

    /// Jump to the start of the log
    pub fn jump_to_start(&mut self, total_lines: usize) -> &mut Self {
        self.scroll_offset = total_lines.saturating_sub(1);
        self.set_paused(true);
        self
    }

    /// Jump to the end of the log
    pub fn jump_to_end(&mut self) -> &mut Self {
        self.scroll_offset = 0;
        self.set_paused(false);
        self
    }
    
    /// Adjust scroll position for new entries
    pub fn adjust_for_new_entries(&mut self, new_entries_count: usize) -> &mut Self {
        // When paused, we should maintain the exact position in the log
        // by adjusting the scroll offset by the number of new entries
        if self.is_paused {
            self.scroll_offset += new_entries_count;
        // } else if self.scroll_offset > new_entries_count {
        //     // When not paused but scrolled up, maintain some position but allow gradual scrolling
        //     self.scroll_offset -= new_entries_count;
        } else {
            // When not paused, we want to stay at the bottom
            self.scroll_offset = 0;
        }
        self
    }

    /// Truncate ANSI strings to fit within a specified width
    /// This is non-trivial because of ANSI escape codes
    /// it also doesn't always clear at the end
    fn truncate_ansi(&self, input: &str, max_width: usize) -> String {
        let mut result = String::new();
        let mut current_width = 0;
    
        // Parse the input string into ANSI pieces.
        for piece in input.ansi_parse() {
            match piece {
                Output::TextBlock(text) => {
                    let mut remaining = text;
                    while !remaining.is_empty() && current_width < max_width {
                        // Get the next character.
                        let ch = remaining.chars().next().unwrap();
                        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                        
                        // Check if adding this character would exceed max_width.
                        if current_width + ch_width > max_width {
                            break;
                        }
                        result.push(ch);
                        current_width += ch_width;
                        remaining = &remaining[ch.len_utf8()..];
                    }
                }
                // For escape sequences, convert them to string properly
                Output::Escape(seq) => {
                    result.push_str(&format!("\x1b{}", seq));
                }
            }
            if current_width >= max_width {
                break;
            }
        }
    
        // Pad with spaces if the visible width is less than max_width.
        // if current_width < max_width {
        //     result.push_str(&"X".repeat(max_width - current_width));
        // }

        // Strip any trailing whitespace
        result.trim_end().to_string()
    }

    /// Handle rendering the log entries to the screen
    fn render_logs<'a>(
        &self,
        filtered_logs: &[&'a LogEntry],
        settings: &LogSettings,
        area: Rect,
    ) -> Paragraph<'a> {
        // Calculate visible lines
        let total_filtered_lines = filtered_logs.len();
        let log_area_height = area.height.saturating_sub(2) as usize; // Subtract 2 for the borders
        let log_area_width = area.width.saturating_sub(2) as usize; // Subtract 2 for the borders

        // Calculate valid scroll range
        let max_scroll = total_filtered_lines.saturating_sub(log_area_height);
        let effective_scroll = self.scroll_offset.min(max_scroll);
        
        // Calculate the range of logs to display
        let start = if total_filtered_lines > log_area_height + effective_scroll {
            total_filtered_lines - log_area_height - effective_scroll
        } else {
            0
        };
        let end = total_filtered_lines.saturating_sub(effective_scroll);
        
        // Format the visible lines based on settings
        let display_lines: Vec<Line> = filtered_logs[start..end]
            .iter()
            .map(|entry| {
                let formatted = entry.format(settings, None);
                let style = match entry.source.as_str() {
                    "stderr" => Style::default().fg(Color::Red),
                    "stdout" => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::White),
                };
                // pad the formatted string to fit the log area width
                let formatted = if settings.show_raw {
                    // raw mode -- need to figure out some better way to pad this
                    let plain_len = entry.content_plain.len();
                    let padding = log_area_width.saturating_sub(plain_len);
                    let truncated = self.truncate_ansi(&formatted, log_area_width);
                    
                    // create extra spaces based on the padding
                    let extra_spaces = " ".repeat(padding);
                    format!("{truncated}{extra_spaces}")
                } else {
                    // if not raw mode, we should be okay
                    let padding = log_area_width.saturating_sub(formatted.len());
                    let extra_spaces = " ".repeat(padding);
                    format!("{:<width$}{}", formatted, extra_spaces, width = log_area_width)
                };
                Line::from(Span::styled(formatted, style))
            })
            .collect();
        
        // Get the title with pause indicator
        let title = if self.is_paused {
            format!("{} offset: {} - [PAUSED]", self.title, self.scroll_offset)
        } else {
            format!("{} offset: {}", self.title, self.scroll_offset)
        };
        
        // Create the block with title
        let log_block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                title,
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ));
        
        // Create and return the paragraph widget
        Paragraph::new(display_lines)
            .block(log_block)
            // .wrap(Wrap { trim: true })
    }
}

impl Widget for LogViewer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // This is a placeholder implementation
        // In practice, we need to pass the filtered logs and settings externally
        // since this widget doesn't store logs itself
        let block = Block::default()
            .title(
                Span::styled(
                    if self.is_paused { 
                        format!("{} - [PAUSED]", self.title) 
                    } else { 
                        self.title 
                    },
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                )
            )
            .borders(Borders::ALL);
            
        block.render(area, buf);
    }
}

/// Extension trait to enable rendering LogViewer with log entries
pub trait LogViewerExt {
    fn render_log_viewer(
        &mut self,
        widget: LogViewer,
        area: Rect,
        filtered_logs: &[&LogEntry],
        settings: &LogSettings,
    );
}

impl LogViewerExt for ratatui::Frame<'_> {
    fn render_log_viewer(
        &mut self,
        widget: LogViewer,
        area: Rect,
        filtered_logs: &[&LogEntry],
        settings: &LogSettings,
    ) {
        let paragraph = widget.render_logs(filtered_logs, settings, area);
        self.render_widget(paragraph, area);
    }
}
