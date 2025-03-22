use chrono::{DateTime, Local};
use crate::settings::LogSettings;

pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub source: String,      // e.g., "stdout", "stderr", "file.log"
    pub content: String,     // The actual log message
}

impl LogEntry {
    pub fn new(source: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            timestamp: Local::now(),
            source: source.into(),
            content: content.into(),
        }
    }
    
    // Format the entry according to settings
    pub fn format(&self, settings: &LogSettings, line_number: Option<usize>) -> String {
        let mut parts = Vec::new();
        
        // Add line number if enabled
        if settings.show_line_numbers {
            if let Some(num) = line_number {
                parts.push(format!("[{:>6}]", num));
            }
        }
        
        // Add timestamp if enabled
        if settings.show_time {
            parts.push(format!("[{}]", self.timestamp.format("%Y-%m-%d %H:%M:%S")));
        }
        
        // Add source label if enabled
        if settings.show_source_labels {
            parts.push(format!("[{}]", self.source.to_uppercase()));
        }
        
        // Add the content
        parts.push(self.content.clone());
        
        parts.join(" ")
    }
}
