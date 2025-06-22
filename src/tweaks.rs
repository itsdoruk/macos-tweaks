use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tweak {
    pub name: String,
    pub description: String,
    pub enable_command: String,
    pub disable_command: String,
    pub is_enabled: bool,
}

impl Tweak {
    pub fn new(
        name: &str,
        description: &str,
        enable_command: &str,
        disable_command: &str,
        is_enabled: bool,
    ) -> Self {
        Tweak {
            name: name.to_string(),
            description: description.to_string(),
            enable_command: enable_command.to_string(),
            disable_command: disable_command.to_string(),
            is_enabled,
        }
    }
} 