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

/// User-editable configuration (ReadOnly by App after load)
/// stored in `config.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub keys: crate::app::keys::KeyConfig,
    #[serde(default = "default_music_dir")]
    pub music_directory: String,
}

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

fn default_music_dir() -> String {
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
    format!("{}/Music", home)
}

fn default_bands() -> [f32; 10] {
    [0.5; 10]
}

fn default_volume() -> u8 {
    50
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            keys: crate::app::keys::KeyConfig::default(),
            music_directory: default_music_dir(),
        }
    }
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
struct LegacyConfigMixin {
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

impl AppConfig {
    pub fn get_config_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let xdg_dir = home.join(".config").join("vyom");
        
        // Ensure it exists
        if !xdg_dir.exists() {
             let _ = std::fs::create_dir_all(&xdg_dir);
        }
        
        xdg_dir
    }

    pub fn get_config_path() -> PathBuf {
        Self::get_config_dir().join("config.toml")
    }

    pub fn get_state_path() -> PathBuf {
        Self::get_config_dir().join("state.toml")
    }

    /// Load both (with migration)
    pub fn load() -> (UserConfig, PersistentState) {
        let config_path = Self::get_config_path();
        let state_path = Self::get_state_path();

        // 1. Load User Config
        let user_config = if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                toml::from_str(&content).unwrap_or_else(|_| UserConfig::default())
            } else {
                UserConfig::default()
            }
        } else {
            // Create default config.toml if missing
            let c = UserConfig::default();
            if let Ok(content) = toml::to_string_pretty(&c) {
                let _ = fs::write(&config_path, content);
            }
            c
        };

        // 2. Load State
        let state = if state_path.exists() {
             if let Ok(content) = fs::read_to_string(&state_path) {
                toml::from_str(&content).unwrap_or_else(|_| PersistentState::default())
            } else {
                PersistentState::default()
            }
        } else {
            // MIGRATION: If state.toml is missing, try to scrape from config.toml
            if config_path.exists() {
                 if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(legacy) = toml::from_str::<LegacyConfigMixin>(&content) {
                        // Found legacy state in config! Use it.
                        let s = PersistentState {
                            last_preset_name: legacy.last_preset_name,
                            eq_enabled: legacy.eq_enabled,
                            eq_bands: legacy.eq_bands,
                            preamp_db: legacy.preamp_db,
                            balance: legacy.balance,
                            crossfade: legacy.crossfade,
                            replay_gain_mode: legacy.replay_gain_mode,
                            volume: legacy.volume,
                            presets: legacy.presets, // Migrate presets too
                        };
                        s.save(); // Save to new state.toml immediately
                        s
                    } else {
                        PersistentState::default()
                    }
                 } else {
                     PersistentState::default()
                 }
            } else {
                PersistentState::default()
            }
        };

        (user_config, state)
    }
}

pub struct AppConfig; // Namespace only

// NOTE: UserConfig no longer needs save() because presets are now in PersistentState!
// config.toml is effectively read-only.

impl PersistentState {
    pub fn save(&self) {
        let path = AppConfig::get_state_path();
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}

impl AppConfig {
    pub fn get_default_presets() -> Vec<EqPreset> {
        vec![
            EqPreset::new("Flat", [0.50; 10]),
            EqPreset::new(
                "Acoustic",
                [
                    0.583, 0.542, 0.50, 0.542, 0.542, 0.50, 0.50, 0.542, 0.583, 0.542,
                ],
            ),
            EqPreset::new(
                "Bass Booster",
                [
                    0.667, 0.708, 0.667, 0.583, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50,
                ],
            ),
            EqPreset::new(
                "Bass Reducer",
                [
                    0.333, 0.292, 0.333, 0.417, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50,
                ],
            ),
            EqPreset::new(
                "Classical",
                [
                    0.50, 0.50, 0.50, 0.50, 0.50, 0.458, 0.458, 0.458, 0.542, 0.583,
                ],
            ),
            EqPreset::new(
                "Dance",
                [
                    0.708, 0.667, 0.583, 0.50, 0.417, 0.417, 0.50, 0.583, 0.625, 0.625,
                ],
            ),
            EqPreset::new(
                "Deep",
                [
                    0.708, 0.667, 0.583, 0.50, 0.50, 0.50, 0.458, 0.417, 0.375, 0.333,
                ],
            ),
            EqPreset::new(
                "Electronic",
                [
                    0.708, 0.667, 0.50, 0.458, 0.458, 0.50, 0.50, 0.583, 0.667, 0.708,
                ],
            ),
            EqPreset::new(
                "Hip-Hop",
                [
                    0.708, 0.667, 0.583, 0.50, 0.50, 0.542, 0.542, 0.50, 0.542, 0.583,
                ],
            ),
            EqPreset::new(
                "Jazz",
                [
                    0.583, 0.542, 0.50, 0.542, 0.583, 0.583, 0.50, 0.542, 0.583, 0.542,
                ],
            ),
            EqPreset::new(
                "Late Night",
                [
                    0.458, 0.50, 0.542, 0.583, 0.583, 0.583, 0.542, 0.50, 0.458, 0.458,
                ],
            ),
            EqPreset::new(
                "Latin",
                [
                    0.583, 0.542, 0.542, 0.542, 0.50, 0.50, 0.50, 0.542, 0.625, 0.625,
                ],
            ),
            EqPreset::new(
                "Loudness",
                [
                    0.667, 0.583, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.583, 0.667,
                ],
            ),
            EqPreset::new(
                "Lounge",
                [
                    0.417, 0.50, 0.542, 0.583, 0.583, 0.542, 0.50, 0.458, 0.50, 0.50,
                ],
            ),
            EqPreset::new(
                "Piano",
                [
                    0.542, 0.50, 0.50, 0.542, 0.583, 0.583, 0.542, 0.542, 0.583, 0.542,
                ],
            ),
            EqPreset::new(
                "Pop",
                [
                    0.417, 0.50, 0.542, 0.625, 0.667, 0.625, 0.542, 0.50, 0.542, 0.542,
                ],
            ),
            EqPreset::new(
                "R&B",
                [
                    0.625, 0.583, 0.542, 0.50, 0.542, 0.583, 0.583, 0.542, 0.542, 0.583,
                ],
            ),
            EqPreset::new(
                "Rock",
                [
                    0.625, 0.583, 0.50, 0.458, 0.50, 0.583, 0.625, 0.625, 0.625, 0.583,
                ],
            ),
            EqPreset::new(
                "Small Speakers",
                [
                    0.708, 0.667, 0.625, 0.542, 0.50, 0.50, 0.542, 0.625, 0.667, 0.708,
                ],
            ),
            EqPreset::new(
                "Spoken Word",
                [
                    0.375, 0.458, 0.542, 0.667, 0.708, 0.667, 0.583, 0.50, 0.417, 0.375,
                ],
            ),
        ]
    }
}
