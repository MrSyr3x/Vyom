use crate::player::controller::traits::PlatformController;
use crate::player::{TrackInfo, RepeatMode, PlayerTrait};
use anyhow::Result;

pub mod common;
pub mod spotify;
pub mod music;

use spotify::SpotifyController;
use music::MusicController;

pub struct MacOsPlayer {
    spotify: SpotifyController,
    music: MusicController,
}

impl MacOsPlayer {
    pub fn new() -> Self {
        Self {
            spotify: SpotifyController,
            music: MusicController,
        }
    }

    /// Helper to get active controller
    fn active_controller(&self) -> Option<&dyn PlatformController> {
        if self.spotify.is_running() {
            Some(&self.spotify)
        } else if self.music.is_running() {
            Some(&self.music)
        } else {
            None
        }
    }
}

// Implement PlayerTrait based on delegation
impl PlayerTrait for MacOsPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        match self.active_controller() {
            Some(c) => c.get_current_track(),
            None => Ok(None),
        }
    }

    fn play_pause(&self) -> Result<bool> {
        match self.active_controller() {
            Some(c) => c.play_pause(),
            None => Ok(false),
        }
    }

    fn next(&self) -> Result<()> {
        if let Some(c) = self.active_controller() {
             c.next()?;
        }
        Ok(())
    }

    fn prev(&self) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.prev()?;
        }
        Ok(())
    }

    fn seek(&self, position_secs: f64) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.seek(position_secs)?;
        }
        Ok(())
    }

    fn volume_up(&self) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.volume_up()?;
        }
        Ok(())
    }

    fn volume_down(&self) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.volume_down()?;
        }
        Ok(())
    }

    fn set_volume(&self, volume: u8) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.set_volume(volume)?;
        }
        Ok(())
    }

    fn shuffle(&self, enable: bool) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.shuffle(enable)?;
        }
        Ok(())
    }

    fn repeat(&self, mode: RepeatMode) -> Result<()> {
         if let Some(c) = self.active_controller() {
             c.repeat(mode)?;
        }
        Ok(())
    }

    fn get_shuffle(&self) -> Result<bool> {
         match self.active_controller() {
            Some(c) => c.get_shuffle(),
            None => Ok(false),
        }
    }

    fn get_repeat(&self) -> Result<RepeatMode> {
         match self.active_controller() {
            Some(c) => c.get_repeat(),
            None => Ok(RepeatMode::Off),
        }
    }
}
