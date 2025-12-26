use ratatui::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct Theme {
    pub name: String,
    #[serde(deserialize_with = "deserialize_color")]
    pub base: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub text: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub red: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub green: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub yellow: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub blue: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub magenta: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub cyan: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub surface: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub overlay: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub progress_fg: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub progress_bg: Color,
}

impl Theme {
    pub fn default() -> Self {
        // Fallback: Catppuccin Mocha
        Self {
            name: "Catppuccin Mocha".to_string(),
            base: Color::Rgb(30, 30, 46),      // #1e1e2e
            text: Color::Rgb(205, 214, 244),   // #cdd6f4
            red: Color::Rgb(243, 139, 168),    // #f38ba8
            green: Color::Rgb(166, 227, 161),  // #a6e3a1
            yellow: Color::Rgb(249, 226, 175), // #f9e2af
            blue: Color::Rgb(137, 180, 250),   // #89b4fa
            magenta: Color::Rgb(203, 166, 247),// #cba6f7
            cyan: Color::Rgb(148, 226, 213),   // #94e2d5
            surface: Color::Rgb(49, 50, 68),   // #313244 (Surface0)
            overlay: Color::Rgb(108, 112, 134),// #6c7086 (Overlay0)
            progress_fg: Color::Rgb(166, 227, 161), // Green
            progress_bg: Color::Rgb(49, 50, 68), // Surface
        }
    }
}

#[derive(Deserialize)]
struct ThemeConfig {
    theme: Theme,
}

pub fn load_current_theme() -> Theme {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = PathBuf::from(&home).join(".config/vyom/theme.toml");
    
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(config) = toml::from_str::<ThemeConfig>(&content) {
            return config.theme;
        }
    }

    // Try parsing just the raw theme if the struct is flat in file? 
    // Usually it is better to have [theme] table.
    // Let's assume the user template writes:
    // [theme]
    // name = "..."
    // ...
    
    Theme::default()
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    parse_hex(&s).map_err(serde::de::Error::custom)
}

fn parse_hex(hex: &str) -> Result<Color, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
        Ok(Color::Rgb(r, g, b))
    } else {
        Ok(Color::Reset) 
    }
}
