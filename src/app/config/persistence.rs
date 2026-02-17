use serde::{Deserialize, Serialize};
use std::fs;

use super::presets::EqPreset;

/// Automatically saved session state
/// stored in `state.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentState {
    #[serde(default)]
    pub last_preset_name: String,
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default = "default_bands")]
    pub eq_bands: [f32; 10],
    #[serde(default)]
    pub preamp_db: f32,
    #[serde(default)]
    pub balance: f32,
    #[serde(default)]
    pub crossfade: u32,
    #[serde(default)]
    pub replay_gain_mode: u8,
    #[serde(default = "default_volume")]
    pub volume: u8,
    // Moved from UserConfig:
    #[serde(default)]
    pub presets: Vec<EqPreset>,
}

fn default_bands() -> [f32; 10] {
    [0.5; 10]
}

fn default_volume() -> u8 {
    50
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            last_preset_name: "Flat".to_string(),
            eq_enabled: false,
            eq_bands: [0.5; 10],
            preamp_db: 0.0,
            balance: 0.0,
            crossfade: 0,
            replay_gain_mode: 0,
            volume: 50,
            presets: Vec::new(),
        }
    }
}

// Temporary struct for migration only
#[derive(Deserialize)]
pub(crate) struct LegacyConfigMixin {
    #[serde(default)]
    pub last_preset_name: String,
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default = "default_bands")]
    pub eq_bands: [f32; 10],
    #[serde(default)]
    pub preamp_db: f32,
    #[serde(default)]
    pub balance: f32,
    #[serde(default)]
    pub crossfade: u32,
    #[serde(default)]
    pub replay_gain_mode: u8,
    #[serde(default = "default_volume")]
    pub volume: u8,
    #[serde(default)]
    pub presets: Vec<EqPreset>,
}

impl PersistentState {
    pub fn save(&self) {
        let path = super::AppConfig::get_state_path();
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}
