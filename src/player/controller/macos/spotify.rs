use super::common::{is_app_running, run_script};
use crate::player::controller::traits::PlatformController;
use crate::player::{PlayerState, TrackInfo, RepeatMode};
use anyhow::Result;

pub struct SpotifyController;

impl PlatformController for SpotifyController {
    fn is_running(&self) -> bool {
        is_app_running("Spotify")
    }

    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        if !self.is_running() {
            return Ok(None);
        }

        let script = r#"
            tell application "Spotify"
                if player state is stopped then
                    return "STOPPED"
                end if
                
                set tName to name of current track
                set tArtist to artist of current track
                set tAlbum to album of current track
                set tDuration to duration of current track
                set tPosition to player position
                set tState to player state as string
                set tArtwork to artwork url of current track
                set tVol to sound volume
                
                return tName & "|||" & tArtist & "|||" & tAlbum & "|||" & tDuration & "|||" & tPosition & "|||" & tState & "|||" & tArtwork & "|||" & tVol
            end tell
        "#;

        match run_script(script) {
            Ok(output) => Ok(parse_spotify_output(&output)),
            Err(_) => Ok(None),
        }
    }

    fn play_pause(&self) -> Result<bool> {
        let script = r#"
            tell application "Spotify"
                playpause
                delay 0.1
                return player state as string
            end tell
        "#;
        let output = run_script(script)?;
        Ok(output == "playing")
    }

    fn next(&self) -> Result<()> {
        run_script("tell application \"Spotify\" to next track")?;
        Ok(())
    }

    fn prev(&self) -> Result<()> {
        run_script("tell application \"Spotify\" to previous track")?;
        Ok(())
    }

    fn seek(&self, position_secs: f64) -> Result<()> {
        run_script(&format!("tell application \"Spotify\" to set player position to {}", position_secs))?;
        Ok(())
    }

    fn volume_up(&self) -> Result<()> {
        run_script("tell application \"Spotify\" to set sound volume to (sound volume + 5)")?;
        Ok(())
    }

    fn volume_down(&self) -> Result<()> {
        run_script("tell application \"Spotify\" to set sound volume to (sound volume - 5)")?;
        Ok(())
    }

    fn set_volume(&self, volume: u8) -> Result<()> {
        let vol = volume.min(100);
        run_script(&format!("tell application \"Spotify\" to set sound volume to {}", vol))?;
        Ok(())
    }

    fn shuffle(&self, enable: bool) -> Result<()> {
        run_script(&format!("tell application \"Spotify\" to set shuffling to {}", enable))?;
        Ok(())
    }

    fn repeat(&self, mode: RepeatMode) -> Result<()> {
        if mode == RepeatMode::Single {
            // Best effort: just enable repeat
            run_script("tell application \"Spotify\" to set repeating to true")?;
        } else {
            let enabled = mode != RepeatMode::Off;
            run_script(&format!("tell application \"Spotify\" to set repeating to {}", enabled))?;
        }
        Ok(())
    }

    fn get_shuffle(&self) -> Result<bool> {
        let output = run_script("tell application \"Spotify\" to return shuffling")?;
        Ok(output == "true")
    }

    fn get_repeat(&self) -> Result<RepeatMode> {
        let output = run_script("tell application \"Spotify\" to return repeating")?;
        if output == "true" {
            Ok(RepeatMode::Playlist)
        } else {
            Ok(RepeatMode::Off)
        }
    }
}

fn parse_spotify_output(output: &str) -> Option<TrackInfo> {
    if output.trim() == "STOPPED" {
        return None;
    }

    let parts: Vec<&str> = output.split("|||").collect();
    if parts.len() < 7 {
        return None;
    }

    let position_secs: f64 = parts[4].replace(',', ".").parse().unwrap_or(0.0);

    let state = match parts[5] {
        "playing" => PlayerState::Playing,
        "paused" => PlayerState::Paused,
        _ => PlayerState::Stopped,
    };

    let duration_ms: u64 = parts[3].parse::<f64>().unwrap_or(0.0) as u64;
    let volume: u32 = if parts.len() >= 8 {
        parts[7].parse().unwrap_or(0)
    } else {
        0
    };

    Some(TrackInfo {
        name: parts[0].to_string(),
        artist: parts[1].to_string(),
        album: parts[2].to_string(),
        duration_ms,
        position_ms: (position_secs * 1000.0) as u64,
        state,
        artwork_url: Some(parts[6].to_string()).filter(|s| !s.is_empty() && s != "NONE"),
        source: "Spotify".to_string(),
        codec: None,
        bitrate: None,
        sample_rate: None,
        bit_depth: None,
        file_path: None,
        volume: Some(volume),
    })
}
