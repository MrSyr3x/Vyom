use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqPreset {
    pub name: String,
    pub bands: [f32; 10],
}

impl EqPreset {
    pub fn new(name: &str, bands: [f32; 10]) -> Self {
        Self {
            name: name.to_string(),
            bands,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub presets: Vec<EqPreset>,
    #[serde(default)]
    pub last_preset_name: String,
    #[serde(default)] // Default false
    pub eq_enabled: bool,
    #[serde(default = "default_bands")] // Default flat
    pub eq_bands: [f32; 10],
    #[serde(default)] // Default 0.0
    pub preamp_db: f32, 
    #[serde(default)]
    pub balance: f32,
    #[serde(default)]
    pub crossfade: u32,
    #[serde(default)]
    pub replay_gain_mode: u8,
}

fn default_bands() -> [f32; 10] {
    [0.5; 10]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            presets: Vec::new(),
            last_preset_name: "Flat".to_string(),
            eq_enabled: false,
            eq_bands: [0.5; 10],
            preamp_db: 0.0,
            balance: 0.0,
            crossfade: 0,
            replay_gain_mode: 0, // Off
        }
    }
}

impl AppConfig {
    pub fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("vyom");
        std::fs::create_dir_all(&path).ok();
        path.push("state.toml");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::get_config_path();
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
    
    pub fn get_default_presets() -> Vec<EqPreset> {
        vec![
            EqPreset::new("Flat",           [0.50; 10]),
            EqPreset::new("Acoustic",       [0.583, 0.542, 0.50, 0.542, 0.542, 0.50, 0.50, 0.542, 0.583, 0.542]),
            EqPreset::new("Bass Booster",   [0.667, 0.708, 0.667, 0.583, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50]),
            EqPreset::new("Bass Reducer",   [0.333, 0.292, 0.333, 0.417, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50]),
            EqPreset::new("Classical",      [0.50, 0.50, 0.50, 0.50, 0.50, 0.458, 0.458, 0.458, 0.542, 0.583]),
            EqPreset::new("Dance",          [0.708, 0.667, 0.583, 0.50, 0.417, 0.417, 0.50, 0.583, 0.625, 0.625]),
            EqPreset::new("Deep",           [0.708, 0.667, 0.583, 0.50, 0.50, 0.50, 0.458, 0.417, 0.375, 0.333]),
            EqPreset::new("Electronic",     [0.708, 0.667, 0.50, 0.458, 0.458, 0.50, 0.50, 0.583, 0.667, 0.708]),
            EqPreset::new("Hip-Hop",        [0.708, 0.667, 0.583, 0.50, 0.50, 0.542, 0.542, 0.50, 0.542, 0.583]),
            EqPreset::new("Jazz",           [0.583, 0.542, 0.50, 0.542, 0.583, 0.583, 0.50, 0.542, 0.583, 0.542]),
            EqPreset::new("Late Night",     [0.458, 0.50, 0.542, 0.583, 0.583, 0.583, 0.542, 0.50, 0.458, 0.458]),
            EqPreset::new("Latin",          [0.583, 0.542, 0.542, 0.542, 0.50, 0.50, 0.50, 0.542, 0.625, 0.625]),
            EqPreset::new("Loudness",       [0.667, 0.583, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.583, 0.667]),
            EqPreset::new("Lounge",         [0.417, 0.50, 0.542, 0.583, 0.583, 0.542, 0.50, 0.458, 0.50, 0.50]),
            EqPreset::new("Piano",          [0.542, 0.50, 0.50, 0.542, 0.583, 0.583, 0.542, 0.542, 0.583, 0.542]),
            EqPreset::new("Pop",            [0.417, 0.50, 0.542, 0.625, 0.667, 0.625, 0.542, 0.50, 0.542, 0.542]),
            EqPreset::new("R&B",            [0.625, 0.583, 0.542, 0.50, 0.542, 0.583, 0.583, 0.542, 0.542, 0.583]),
            EqPreset::new("Rock",           [0.625, 0.583, 0.50, 0.458, 0.50, 0.583, 0.625, 0.625, 0.625, 0.583]),
            EqPreset::new("Small Speakers", [0.708, 0.667, 0.625, 0.542, 0.50, 0.50, 0.542, 0.625, 0.667, 0.708]),
            EqPreset::new("Spoken Word",    [0.375, 0.458, 0.542, 0.667, 0.708, 0.667, 0.583, 0.50, 0.417, 0.375]),
        ]
    }
}
