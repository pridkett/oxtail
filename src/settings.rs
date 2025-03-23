use std::collections::HashMap;

// Source configuration - uses string identifiers for flexibility
pub struct SourceConfig {
    pub visible: bool,
}

// Global settings
pub struct LogSettings {
    // Per-source configurations
    pub sources: HashMap<String, SourceConfig>,
    
    // Global metadata settings
    pub show_time: bool,
    pub show_source_labels: bool,
    pub show_line_numbers: bool,
    pub show_file_type: bool,
    pub show_raw: bool,  // When true, shows content with ANSI codes, otherwise shows plain content
}

impl Default for LogSettings {
    fn default() -> Self {
        let mut sources = HashMap::new();
        sources.insert(
            "stdout".to_string(), 
            SourceConfig { 
                visible: true 
            }
        );
        sources.insert(
            "stderr".to_string(), 
            SourceConfig { 
                visible: true 
            }
        );

        Self {
            sources,
            show_time: true,
            show_source_labels: true,
            show_line_numbers: false,
            show_file_type: false,
            show_raw: false,  // Default to plain text (no ANSI codes)
        }
    }
}

// Helper methods for settings
impl LogSettings {
    pub fn get_source_config(&mut self, name: &str) -> &mut SourceConfig {
        let normalized_name = name.to_lowercase();
        if !self.sources.contains_key(&normalized_name) {
            // Add a new source config if it doesn't exist
            self.sources.insert(
                normalized_name.clone(),
                SourceConfig {
                    visible: true,
                }
            );
        }
        self.sources.get_mut(&normalized_name).unwrap()
    }
    
    pub fn is_source_visible(&self, name: &str) -> bool {
        let normalized_name = name.to_lowercase();
        self.sources.get(&normalized_name)
            .map_or(true, |s| s.visible)
    }
    
    pub fn set_all_sources_visibility(&mut self, visible: bool) {
        for (_, source) in self.sources.iter_mut() {
            source.visible = visible;
        }
    }
}
