use std::fs;
use std::path::PathBuf;

pub mod persistence;
pub mod presets;
pub mod user;

// Fix circular dependency: create a wrapper module or re-export to allow `persistence.rs` to find `AppConfig`.
// Actually, declaring `pub mod mod_container` logic here.

use persistence::LegacyConfigMixin;
pub use persistence::PersistentState;
pub use presets::{get_default_presets, EqPreset};
pub use user::UserConfig;

pub struct AppConfig;

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
