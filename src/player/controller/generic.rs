use crate::player::traits::{PlayerState, PlayerTrait, QueueItem, RepeatMode, TrackInfo};
use anyhow::{bail, Result};

pub struct DummyPlayer;

impl PlayerTrait for DummyPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        Ok(None)
    }

    fn play_pause(&self) -> Result<bool> {
        bail!("Controller mode not supported on this OS")
    }

    fn next(&self) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn prev(&self) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn seek(&self, _position: f64) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn volume_up(&self) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn volume_down(&self) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn set_volume(&self, _volume: u8) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn get_queue(&self) -> Result<Vec<QueueItem>> {
        Ok(Vec::new())
    }

    fn shuffle(&self, _enable: bool) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn repeat(&self, _mode: RepeatMode) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn crossfade(&self, _secs: u32) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn delete_queue(&self, _pos: u32) -> Result<()> {
        bail!("Controller mode not supported on this OS")
    }

    fn get_shuffle(&self) -> Result<bool> {
        Ok(false)
    }

    fn get_repeat(&self) -> Result<RepeatMode> {
        Ok(RepeatMode::Off)
    }
}
