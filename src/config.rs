use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub text: String,
    pub text_dim: String,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            primary: "#fe640b".to_string(),    // Orange
            secondary: "#ffffff".to_string(),  // White
            accent: "#00ff00".to_string(),     // Green
            success: "#00ff00".to_string(),    // Green
            warning: "#ffa500".to_string(),    // Orange
            error: "#ff0000".to_string(),      // Red
            text: "#ffffff".to_string(),       // White
            text_dim: "#808080".to_string(),   // Gray
        }
    }
}

impl ColorScheme {
    pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        
        Some((r, g, b))
    }
    
    pub fn get_color(&self, color_name: &str) -> Color {
        let hex = match color_name {
            "primary" => &self.primary,
            "secondary" => &self.secondary,
            "accent" => &self.accent,
            "success" => &self.success,
            "warning" => &self.warning,
            "error" => &self.error,
            "text" => &self.text,
            "text_dim" => &self.text_dim,
            _ => &self.primary, // fallback
        };
        
        if let Some((r, g, b)) = Self::hex_to_rgb(hex) {
            Color::Rgb(r, g, b)
        } else {
            Color::White // fallback color
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub color_scheme: ColorScheme,
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            theme: "default".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str(&contents) {
                return config;
            }
        }
        
        // If loading fails, create default config
        let default_config = Config::default();
        default_config.save();
        default_config
    }
    
    pub fn save(&self) {
        let config_path = Self::get_config_path();
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&config_path, json);
        }
    }
    
    fn get_config_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".config");
        path.push("macos-tweaks");
        path.push("config.json");
        path
    }
    
    pub fn get_color_scheme(&self) -> &ColorScheme {
        &self.color_scheme
    }
} 