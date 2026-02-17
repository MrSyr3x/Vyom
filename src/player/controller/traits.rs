use crate::player::{RepeatMode, TrackInfo};
use anyhow::Result;

/// Internal trait for platform-specific player implementations (Spotify, Music, etc.)
pub trait PlatformController {
    fn is_running(&self) -> bool;
    fn get_current_track(&self) -> Result<Option<TrackInfo>>;
    fn play_pause(&self) -> Result<bool>;
    fn next(&self) -> Result<()>;
    fn prev(&self) -> Result<()>;
    fn seek(&self, position_secs: f64) -> Result<()>;
    fn volume_up(&self) -> Result<()>;
    fn volume_down(&self) -> Result<()>;
    fn set_volume(&self, volume: u8) -> Result<()>;
    fn shuffle(&self, enable: bool) -> Result<()>;
    fn repeat(&self, mode: RepeatMode) -> Result<()>;
    fn get_shuffle(&self) -> Result<bool>;
    fn get_repeat(&self) -> Result<RepeatMode>;
}
