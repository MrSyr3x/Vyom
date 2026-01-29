use serde::{Deserialize, Serialize};
use ratatui::style::Color;
use std::fs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Theme {
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,
    pub text: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
}

impl Theme {
    pub fn default() -> Self {
        Self {
            base: Color::Rgb(30, 30, 46),
            surface: Color::Rgb(49, 50, 68),
            overlay: Color::Rgb(108, 112, 134),
            text: Color::Rgb(205, 214, 244),
            red: Color::Rgb(243, 139, 168),
            green: Color::Rgb(166, 227, 161),
            yellow: Color::Rgb(249, 226, 175),
            blue: Color::Rgb(137, 180, 250),
            magenta: Color::Rgb(203, 166, 247),
            cyan: Color::Rgb(148, 226, 213),
        }
    }
}

// Helper for serialization/deserialization
#[derive(Serialize, Deserialize)]
struct ThemeFile {
    theme: Theme,
}

pub fn get_theme_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(&home).join(".config/vyom/theme.toml")
}

pub fn load_current_theme() -> Theme {
    let path = get_theme_path();

    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            // Try parsing as nested [theme] first (Theme Selector format)
            if let Ok(wrapper) = toml::from_str::<ThemeFile>(&content) {
                return wrapper.theme;
            }
            // Fallback: Try parsing as flat file (Manual/Legacy format)
            if let Ok(theme) = toml::from_str::<Theme>(&content) {
                return theme;
            }
        }
    } else {
        // Auto-create default theme file if it doesn't exist
        let default_theme = Theme::default();
        let wrapper = ThemeFile { theme: default_theme.clone() };
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        // Write to file
        if let Ok(toml_str) = toml::to_string_pretty(&wrapper) {
            let _ = fs::write(&path, toml_str);
        }
        
        return default_theme;
    }
    
    Theme::default()
}
