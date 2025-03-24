use chrono::{DateTime, Local};
use crate::settings::LogSettings;
use serde_json::Value;
use strip_ansi_escapes::strip;

pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub source: String,      // e.g., "stdout", "stderr", "file.log"
    pub content: String,     // The actual log message
    pub content_plain: String, // content with ANSI codes stripped out
    pub is_json: bool,       // true if the content is JSON
    pub line_number: usize,  // The line number within this stream
}

impl LogEntry {
    pub fn new(source: impl Into<String>, content: impl Into<String>) -> Self {
        let content_str = content.into();
        
        // Strip ANSI escape codes to get plain text content
        let stripped_bytes = strip(content_str.as_bytes());
        let content_plain = String::from_utf8_lossy(&stripped_bytes).to_string();
        
        // Check if content is valid JSON
        let is_json = serde_json::from_str::<Value>(&content_plain)
            .map(|_| true)
            .unwrap_or(false);
            
        Self {
            timestamp: Local::now(),
            source: source.into(),
            content: content_str,
            content_plain,
            is_json,
            line_number: 0, // Default value, should be set later
        }
    }
    
    // Format the entry according to settings
    pub fn format(&self, settings: &LogSettings, _line_number: Option<usize>) -> String {
        let mut parts = Vec::new();

        // Add line number if enabled
        if settings.show_line_numbers {
            parts.push(format!("[{:>6}]", self.line_number));
        }
        
        // Add timestamp if enabled
        if settings.show_time {
            parts.push(format!("[{}]", self.timestamp.format("%Y-%m-%d %H:%M:%S")));
        }
        
        // Add source label if enabled
        if settings.show_source_labels {
            parts.push(format!("[{}]", self.source.to_uppercase()));
        }
        
        // Choose between raw content (with ANSI codes) or plain content
        let display_content = if settings.show_raw {
            &self.content
        } else {
            &self.content_plain
        };
        
        // Add the content with file type indicator if enabled
        let content_with_type = if settings.show_file_type {
            if self.is_json {
                format!("\u{e60b} {}", display_content)
            } else {
                format!("  {}", display_content)
            }
        } else {
            display_content.clone()
        };
        
        parts.push(content_with_type);
        
        parts.join(" ")
    }

    pub fn get_content_plain_len(&self) -> usize {
        self.content_plain.len()
    }
}
