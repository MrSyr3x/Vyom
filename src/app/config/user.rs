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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_toml_produces_defaults() {
        let config: UserConfig = toml::from_str("").unwrap();
        assert_eq!(config.keys.quit, "q");
        assert_eq!(config.keys.play_pause, "Space");
        assert!(config.music_directory.ends_with("/Music"));
    }

    #[test]
    fn test_partial_toml_fills_missing_fields() {
        let toml_str = r#"
[keys]
quit = "Q"
"#;
        let config: UserConfig = toml::from_str(toml_str).unwrap();
        // Overridden field
        assert_eq!(config.keys.quit, "Q");
        // Default-filled fields
        assert_eq!(config.keys.play_pause, "Space");
        assert_eq!(config.keys.next_track, "n");
        assert_eq!(config.keys.cycle_art, "A");
    }

    #[test]
    fn test_full_config_roundtrip() {
        let original = UserConfig::default();
        let serialized = toml::to_string_pretty(&original).unwrap();
        let deserialized: UserConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(original.keys.quit, deserialized.keys.quit);
        assert_eq!(original.keys.play_pause, deserialized.keys.play_pause);
        assert_eq!(original.music_directory, deserialized.music_directory);
    }

    #[test]
    fn test_malformed_toml_returns_error() {
        let bad_toml = "this is not valid [[[toml";
        let result: Result<UserConfig, _> = toml::from_str(bad_toml);
        assert!(result.is_err());
    }
}
