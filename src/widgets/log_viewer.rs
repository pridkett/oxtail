use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use crate::log_entry::LogEntry;
use crate::settings::LogSettings;

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
            scroll_offset: 0,
            is_paused: false,
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
        self.scroll_offset += amount;
        if !self.is_paused {
            self.is_paused = true;
        }
        self
    }
    
    /// Scroll down by the specified amount, bounded by max_scroll
    pub fn scroll_down(&mut self, amount: usize) -> &mut Self {
        if self.scroll_offset >= amount {
            self.scroll_offset -= amount;
        } else {
            self.scroll_offset = 0;
        }
        
        // When we reach the bottom, unpause
        if self.scroll_offset == 0 {
            self.is_paused = false;
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
    
    /// Adjust scroll position for new entries
    pub fn adjust_for_new_entries(&mut self, new_entries_count: usize) -> &mut Self {
        if self.is_paused {
            // When paused, always maintain relative position
            self.scroll_offset += new_entries_count;
        } else if self.scroll_offset > new_entries_count {
            // When not paused but scrolled up, maintain some position but allow gradual scrolling
            self.scroll_offset -= new_entries_count;
        } else {
            // When not paused and near bottom, scroll to bottom
            self.scroll_offset = 0;
        }
        self
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
        let display_lines: Vec<Spans> = filtered_logs[start..end]
            .iter()
            .map(|entry| {
                let formatted = entry.format(settings, None);
                let style = match entry.source.as_str() {
                    "stderr" => Style::default().fg(Color::Red),
                    "stdout" => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::White),
                };
                Spans::from(Span::styled(formatted, style))
            })
            .collect();

        // Create title with pause indicator
        let title = if self.is_paused {
            format!("{} - [PAUSED]", self.title)
        } else {
            self.title.clone()
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
            .wrap(Wrap { trim: true })
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

impl<B: Backend> LogViewerExt for ratatui::Frame<'_, B> {
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
