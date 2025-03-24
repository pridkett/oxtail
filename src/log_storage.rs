use std::collections::HashMap;
use regex::Regex;
use crate::log_entry::LogEntry;
use crate::settings::LogSettings;

/// Manages log entries from a single source
pub struct LogSource {
    name: String,
    entries: Vec<LogEntry>,
    next_line_number: usize,
    has_new_entries: bool,
    visible: bool,
}

impl LogSource {
    pub fn new(name: String) -> Self {
        Self {
            name,
            entries: Vec::new(),
            next_line_number: 1, // Start from 1 for human readability
            has_new_entries: false,
            visible: true, // Default to visible
        }
    }
    
    pub fn add_entry(&mut self, mut entry: LogEntry) -> &LogEntry {
        entry.line_number = self.next_line_number;
        self.next_line_number += 1;
        self.entries.push(entry);
        self.has_new_entries = true;
        self.entries.last().unwrap()
    }
    
    pub fn get_entries(&self, filter: &Filter) -> Vec<&LogEntry> {
        self.entries.iter()
            .filter(|e| filter.check(e))
            .collect()
    }
    
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    pub fn has_new_entries(&self) -> bool {
        self.has_new_entries
    }
    
    pub fn clear_new_entries_flag(&mut self) {
        self.has_new_entries = false;
    }
}

/// Encapsulates filtering logic for log entries
pub struct Filter {
    pub source_visibility: HashMap<String, bool>,
    pub filter_in: Option<Regex>,
    pub filter_out: Option<Regex>,
}

impl Filter {
    pub fn new() -> Self {
        let mut source_visibility = HashMap::new();
        // Default visibility for stdout and stderr
        source_visibility.insert("stdout".to_string(), true);
        source_visibility.insert("stderr".to_string(), true);
        
        Self {
            source_visibility,
            filter_in: None,
            filter_out: None,
        }
    }
    
    /// Check if an entry passes all filter criteria
    pub fn check(&self, entry: &LogEntry) -> bool {
        // Check source visibility
        if !self.source_visibility.get(&entry.source).copied().unwrap_or(true) {
            return false;
        }
        
        // Check filter_in (entry must match)
        if let Some(regex) = &self.filter_in {
            if !regex.is_match(&entry.content_plain) {
                return false;
            }
        }
        
        // Check filter_out (entry must NOT match)
        if let Some(regex) = &self.filter_out {
            if regex.is_match(&entry.content_plain) {
                return false;
            }
        }
        
        true
    }
    
    /// Update filter from LogSettings
    pub fn update_from_settings(&mut self, settings: &LogSettings) {
        // Update source visibility from settings
        for (source, source_config) in &settings.sources {
            self.source_visibility.insert(source.clone(), source_config.visible);
        }
    }
}

/// Main component that aggregates log sources and handles filtering
pub struct LogStorage {
    sources: HashMap<String, LogSource>,
    filter: Filter,
    active_source: Option<String>,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            filter: Filter::new(),
            active_source: None,
        }
    }
    
    pub fn add_source(&mut self, name: String) -> &mut LogSource {
        self.sources.entry(name.clone()).or_insert_with(|| LogSource::new(name.clone()));
        self.sources.get_mut(&name).unwrap()
    }
    
    pub fn get_source(&self, name: &str) -> Option<&LogSource> {
        self.sources.get(name)
    }
    
    pub fn set_active_source(&mut self, name: Option<String>) {
        self.active_source = name;
    }
    
    pub fn get_active_source(&self) -> &Option<String> {
        &self.active_source
    }
    
    pub fn add_entry(&mut self, entry: LogEntry) {
        let source_name = entry.source.clone();
        let source = self.add_source(source_name);
        source.add_entry(entry);
    }
    
    pub fn get_filtered_entries(&self) -> Vec<&LogEntry> {
        let mut result = Vec::new();
        
        // Get entries from each source that pass the filter
        for source in self.sources.values() {
            result.extend(source.get_entries(&self.filter));
        }
        
        // Sort by timestamp for a unified view
        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        result
    }
    
    pub fn update_filter_from_settings(&mut self, settings: &LogSettings) {
        self.filter.update_from_settings(settings);
    }
    
    pub fn total_entries(&self) -> usize {
        self.sources.values().map(|s| s.len()).sum()
    }
    
    // Check if there are new entries in any visible source that are currently filtered in
    pub fn has_new_visible_entries(&self) -> bool {
        if let Some(active) = &self.active_source {
            if let Some(source) = self.sources.get(active) {
                return source.has_new_entries() && source.is_visible();
            }
        }
        
        // If no active source, check all visible sources
        self.sources.values()
            .any(|source| source.has_new_entries() && source.is_visible())
    }
    
    // Clear the new entries flags on all sources
    pub fn clear_new_entries_flags(&mut self) {
        for source in self.sources.values_mut() {
            source.clear_new_entries_flag();
        }
    }
}
