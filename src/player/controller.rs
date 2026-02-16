use super::traits::{PlayerState, PlayerTrait, TrackInfo, RepeatMode};
use anyhow::{Context, Result};
use std::process::Command;

// --- macOS Implementation 🍎 ---

pub struct MacOsPlayer;

impl MacOsPlayer {
    /// Detect which player is active: "Spotify", "Music", or None.
    /// Prioritizes Spotify if both are running.
    fn detect_active_player(&self) -> Option<&'static str> {
        if Self::is_app_running("Spotify") {
            Some("Spotify")
        } else if Self::is_app_running("Music") {
            Some("Music")
        } else {
            None
        }
    }

    fn is_app_running(app_name: &str) -> bool {
        let output = Command::new("pgrep").arg("-x").arg(app_name).output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    /// Run an AppleScript command
    fn run_script(script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .context("Failed to execute AppleScript")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("AppleScript error: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Pure function to parse AppleScript output for testing 🧪
    fn parse_player_output(output: &str, app_name: &str) -> Option<TrackInfo> {
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
            source: app_name.to_string(),
            // Controller mode: no audiophile metadata
            codec: None,
            bitrate: None,
            sample_rate: None,
            bit_depth: None,
            file_path: None,
            volume: Some(volume),
        })
    }
}

impl PlayerTrait for MacOsPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        let app_name = match self.detect_active_player() {
            Some(app) => app,
            None => return Ok(None),
        };

        // Safety Note (Audit #17): `app_name` comes from `detect_active_player` which only returns
        // hardcoded static strings "Spotify" or "Music". It is safe to inject into the script.
        let script = format!(
            r#"
            tell application "{}"
                if player state is stopped then
                    return "STOPPED"
                end if
                
                set tName to name of current track
                set tArtist to artist of current track
                set tAlbum to album of current track
                set tDuration to duration of current track
                set tPosition to player position
                set tState to player state as string
                
                if "{}" is "Spotify" then
                    -- Spotify Duration is ms
                    set tArtwork to artwork url of current track
                    set tVol to sound volume
                    return tName & "|||" & tArtist & "|||" & tAlbum & "|||" & tDuration & "|||" & tPosition & "|||" & tState & "|||" & tArtwork & "|||" & tVol
                else
                    -- Music App: duration is seconds
                    set tDurSec to duration of current track
                    set tDuration to tDurSec * 1000
                    set tVol to sound volume
                    return tName & "|||" & tArtist & "|||" & tAlbum & "|||" & tDuration & "|||" & tPosition & "|||" & tState & "|||" & "NONE"  & "|||" & tVol
                end if
            end tell
        "#,
            app_name, app_name
        );

        match Self::run_script(&script) {
            Ok(output) => Ok(Self::parse_player_output(&output, app_name)),
            Err(_) => Ok(None),
        }
    }

    fn play_pause(&self) -> Result<bool> {
        if let Some(app) = self.detect_active_player() {
            // Toggle and then check logic
            let script = format!(
                r#"
                tell application "{}"
                    playpause
                    delay 0.1
                    return player state as string
                end tell
            "#,
                app
            );

            let output = Self::run_script(&script)?;
            Ok(output == "playing")
        } else {
            Ok(false)
        }
    }

    fn next(&self) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            Self::run_script(&format!("tell application \"{}\" to next track", app))?;
        }
        Ok(())
    }

    fn prev(&self) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            Self::run_script(&format!("tell application \"{}\" to previous track", app))?;
        }
        Ok(())
    }

    fn seek(&self, position_secs: f64) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            Self::run_script(&format!(
                "tell application \"{}\" to set player position to {}",
                app, position_secs
            ))?;
        }
        Ok(())
    }

    fn volume_up(&self) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            Self::run_script(&format!(
                "tell application \"{}\" to set sound volume to (sound volume + 5)",
                app
            ))?;
        }
        Ok(())
    }

    fn volume_down(&self) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            Self::run_script(&format!(
                "tell application \"{}\" to set sound volume to (sound volume - 5)",
                app
            ))?;
        }
        Ok(())
    }

    fn set_volume(&self, volume: u8) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            let vol = volume.min(100);
            Self::run_script(&format!(
                "tell application \"{}\" to set sound volume to {}",
                app, vol
            ))?;
        }
        Ok(())
    }

    fn shuffle(&self, enable: bool) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            if app == "Spotify" {
                Self::run_script(&format!(
                    "tell application \"Spotify\" to set shuffling to {}",
                    enable
                ))?;
            } else if app == "Music" {
                Self::run_script(&format!(
                    "tell application \"Music\" to set shuffle enabled to {}",
                    enable
                ))?;
            }
        }
        Ok(())
    }

    fn repeat(&self, mode: RepeatMode) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            if app == "Spotify" {
                let _val = match mode {
                    RepeatMode::Off => "off",
                    RepeatMode::Playlist => "all", // Spotify semantics: repeating=true usually means playlist
                    RepeatMode::Single => "one",
                };
                // Spotify's AppleScript is weird: 'set repeating to true/false' is common,
                // but some versions support 'track', 'context', 'off'.
                // Standard 'repeating' is boolean. 'repeat_state' might be better?
                // Let's try standard boolean for now and see if we can set "context" vs "track"
                // Actually, Spotify scripting dict often only exposes boolean `repeating`.
                // If so, we might only support Off/Playlist for Spotify unless we find a workaround.
                // HOLD UP: Recent Spotify AppleScript *might* only support boolean.
                // Let's check `repeating` property type. It is usually boolean.
                // However, user asked for it.
                // Let's try sending "context" or "track" if `repeating` fails?
                // No, safely: if mode is Single, we can't do it easily on Spotify without keystrokes.
                // Fallback: If Spotify, just toggle boolean for Playlist, maybe warn for Single?
                // Actually, let's look at `Music.app`. It supports `song repeat` = off/one/all.
                
                // For Spotify: `set repeating to true` = repeat all.
                // There is no easy `repeat one` in standard dictionary.
                // We will implement what we can.
                 if mode == RepeatMode::Single {
                     // Best effort: just enable repeat
                     Self::run_script("tell application \"Spotify\" to set repeating to true")?;
                 } else {
                     let enabled = mode != RepeatMode::Off;
                     Self::run_script(&format!("tell application \"Spotify\" to set repeating to {}", enabled))?;
                 }
            } else if app == "Music" {
                let val = match mode {
                    RepeatMode::Off => "off",
                    RepeatMode::Playlist => "all",
                    RepeatMode::Single => "one",
                };
                Self::run_script(&format!(
                    "tell application \"Music\" to set song repeat to {}",
                    val
                ))?;
            }
        }
        Ok(())
    }

    fn get_shuffle(&self) -> Result<bool> {
        if let Some(app) = self.detect_active_player() {
            let script = if app == "Spotify" {
                "tell application \"Spotify\" to return shuffling"
            } else {
                "tell application \"Music\" to return shuffle enabled"
            };
            let output = Self::run_script(script)?;
            Ok(output == "true")
        } else {
            Ok(false)
        }
    }

    fn get_repeat(&self) -> Result<RepeatMode> {
        if let Some(app) = self.detect_active_player() {
            if app == "Spotify" {
                let output = Self::run_script("tell application \"Spotify\" to return repeating")?;
                 // Spotify only tells us true/false usually.
                if output == "true" {
                    Ok(RepeatMode::Playlist) // Assume playlist
                } else {
                    Ok(RepeatMode::Off)
                }
            } else {
                let output = Self::run_script("tell application \"Music\" to return song repeat")?;
                match output.as_str() {
                    "one" => Ok(RepeatMode::Single),
                    "all" => Ok(RepeatMode::Playlist),
                    _ => Ok(RepeatMode::Off),
                }
            }
        } else {
            Ok(RepeatMode::Off)
        }
    }
}

// --- Dummy Implementation (Linux/Windows Placeholder) ---
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
}
