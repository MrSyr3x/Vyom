use crate::player::traits::{PlayerTrait, TrackInfo, RepeatMode};
use anyhow::Result;

pub struct DummyPlayer;

impl PlayerTrait for DummyPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        Ok(None)
    }
    fn play_pause(&self) -> Result<bool> {
        Ok(false)
    }
    fn next(&self) -> Result<()> {
        Ok(())
    }
    fn prev(&self) -> Result<()> {
        Ok(())
    }
    fn seek(&self, _pos: f64) -> Result<()> {
        Ok(())
    }
    fn volume_up(&self) -> Result<()> {
        Ok(())
    }
    fn volume_down(&self) -> Result<()> {
        Ok(())
    }
    fn set_volume(&self, _volume: u8) -> Result<()> {
        Ok(())
    }
    fn shuffle(&self, _enable: bool) -> Result<()> {
        Ok(())
    }
    fn repeat(&self, _mode: RepeatMode) -> Result<()> {
        Ok(())
    }
    fn get_shuffle(&self) -> Result<bool> {
        Ok(false)
    }
    fn get_repeat(&self) -> Result<RepeatMode> {
        Ok(RepeatMode::Off)
    }
}
