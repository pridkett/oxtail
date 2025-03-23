use crate::settings::LogSettings;

pub enum CommandResult {
    Success(String),
    Error(String),
    Quit,
}

pub fn execute_command(cmd: &str, settings: &mut LogSettings) -> CommandResult {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    
    if parts.is_empty() {
        return CommandResult::Error("Empty command".to_string());
    }
    
    match parts[0] {
        // Quit command
        "q" | "quit" => {
            CommandResult::Quit
        },
        
        // Source visibility commands - accept both full and shortened forms
        "show_source" | "show" => {
            if parts.len() < 2 {
                return CommandResult::Error("Source name required".to_string());
            }
            
            let source_name = parts[1];
            if source_name == "all" {
                settings.set_all_sources_visibility(true);
                CommandResult::Success("All sources are now visible".to_string())
            } else {
                settings.get_source_config(source_name).visible = true;
                CommandResult::Success(format!("Source '{}' is now visible", source_name))
            }
        },
        
        "hide_source" | "hide" => {
            if parts.len() < 2 {
                return CommandResult::Error("Source name required".to_string());
            }
            
            let source_name = parts[1];
            if source_name == "all" {
                settings.set_all_sources_visibility(false);
                CommandResult::Success("All sources are now hidden".to_string())
            } else {
                settings.get_source_config(source_name).visible = false;
                CommandResult::Success(format!("Source '{}' is now hidden", source_name))
            }
        },
        
        "show_meta" | "hide_meta" => {
            if parts.len() < 2 {
                return CommandResult::Error("Metadata type required".to_string());
            }
            
            let show = parts[0].starts_with("show");
            
            match parts[1] {
                "time" => {
                    settings.show_time = show;
                    CommandResult::Success(format!("Timestamps are now {}", if show { "visible" } else { "hidden" }))
                },
                "source" => {
                    settings.show_source_labels = show;
                    CommandResult::Success(format!("Source labels are now {}", if show { "visible" } else { "hidden" }))
                },
                "lines" => {
                    settings.show_line_numbers = show;
                    CommandResult::Success(format!("Line numbers are now {}", if show { "visible" } else { "hidden" }))
                },
                _ => CommandResult::Error(format!("Unknown metadata type: {}", parts[1]))
            }
        },
        
        _ => CommandResult::Error(format!("Unknown command: {}", parts[0]))
    }
}
