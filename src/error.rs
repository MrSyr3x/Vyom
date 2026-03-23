//! Domain-specific error types for Vyom 🎯
//!
//! Internal modules use these typed errors for programmatic matching.
//! At the `main()` boundary, these are converted to `anyhow::Error` for reporting.

/// Core error types for the Vyom application.
#[derive(thiserror::Error, Debug)]
pub enum VyomError {
    #[error("MPD connection failed: {0}")]
    MpdConnection(String),

    #[error("Config parse error: {0}")]
    ConfigParse(String),

    #[error("Audio pipeline error: {0}")]
    AudioPipeline(String),

    #[error("Lyrics fetch failed: {0}")]
    LyricsFetch(String),

    #[error("Artwork fetch failed: {0}")]
    ArtworkFetch(String),

    #[error("Player command failed: {0}")]
    PlayerCommand(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = VyomError::MpdConnection("timeout".to_string());
        assert_eq!(format!("{}", err), "MPD connection failed: timeout");
    }

    #[test]
    fn test_config_error() {
        let err = VyomError::ConfigParse("invalid TOML".to_string());
        assert_eq!(format!("{}", err), "Config parse error: invalid TOML");
    }

    #[test]
    fn test_error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(VyomError::AudioPipeline("no device".to_string()));
        assert_eq!(format!("{}", err), "Audio pipeline error: no device");
    }
}
