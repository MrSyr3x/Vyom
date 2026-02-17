use serde::{Deserialize, Serialize};

/// User-editable configuration (ReadOnly by App after load)
/// stored in `config.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub keys: crate::app::keys::KeyConfig,
    #[serde(default = "default_music_dir")]
    pub music_directory: String,
}

fn default_music_dir() -> String {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    format!("{}/Music", home)
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            keys: crate::app::keys::KeyConfig::default(),
            music_directory: default_music_dir(),
        }
    }
}
