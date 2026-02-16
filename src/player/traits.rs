use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub name: String,
    pub artist: String,
    pub album: String,
    pub artwork_url: Option<String>,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub state: PlayerState,
    pub source: String, // "Spotify", "Music", "MPD"

    // Audiophile Metadata 🎵
    pub codec: Option<String>,    // "FLAC", "ALAC", "AAC", "MP3"
    pub bitrate: Option<u32>,     // kbps
    pub sample_rate: Option<u32>, // Hz (e.g., 44100, 96000)
    pub bit_depth: Option<u8>,    // 16, 24, 32

    /// File path for embedded artwork extraction (MPD mode)
    pub file_path: Option<String>,

    /// Current Volume (0-100)
    pub volume: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RepeatMode {
    Off,
    Playlist,
    Single,
}

impl Default for RepeatMode {
    fn default() -> Self {
        Self::Off
    }
}

/// The unified interface for any OS Media Player 🎵
pub trait PlayerTrait: Send + Sync {
    fn get_current_track(&self) -> Result<Option<TrackInfo>>;
    fn play_pause(&self) -> Result<bool>;
    fn next(&self) -> Result<()>;
    fn prev(&self) -> Result<()>;
    fn seek(&self, position_secs: f64) -> Result<()>;
    fn volume_up(&self) -> Result<()>;
    fn volume_down(&self) -> Result<()>;
    fn set_volume(&self, volume: u8) -> Result<()>;

    /// Get current queue/playlist (MPD only, returns empty for controller mode)
    /// Returns: (title, artist, duration_ms, is_current, file_path)
    fn get_queue(&self) -> Result<Vec<QueueItem>> {
        Ok(Vec::new())
    }

    // Extended methods for MPD features (defaults for non-MPD players)
    fn shuffle(&self, _enable: bool) -> Result<()> {
        Ok(())
    }
    fn repeat(&self, _mode: RepeatMode) -> Result<()> {
        Ok(())
    }
    fn crossfade(&self, _secs: u32) -> Result<()> {
        Ok(())
    }
    fn delete_queue(&self, _pos: u32) -> Result<()> {
        Ok(())
    }
    fn get_shuffle(&self) -> Result<bool> {
        Ok(false)
    }
    fn get_repeat(&self) -> Result<RepeatMode> {
        Ok(RepeatMode::Off)
    }
}

/// (title, artist, duration_ms, is_current, file_path)
pub type QueueItem = (String, String, u64, bool, String);
