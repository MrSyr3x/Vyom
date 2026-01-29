use std::process::Command;
use anyhow::{Result, Context};
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
    pub codec: Option<String>,      // "FLAC", "ALAC", "AAC", "MP3"
    pub bitrate: Option<u32>,        // kbps
    pub sample_rate: Option<u32>,    // Hz (e.g., 44100, 96000)
    pub bit_depth: Option<u8>,       // 16, 24, 32
    
    /// File path for embedded artwork extraction (MPD mode)
    pub file_path: Option<String>,
    
    /// Current Volume (0-100)
    pub volume: Option<u32>,
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
    
    /// Get current queue/playlist (MPD only, returns empty for controller mode)
    /// Returns: (title, artist, duration_ms, is_current, file_path)
    fn get_queue(&self) -> Result<Vec<(String, String, u64, bool, String)>> {
        Ok(Vec::new())
    }
    
    // Extended methods for MPD features (defaults for non-MPD players)
    fn shuffle(&self, _enable: bool) -> Result<()> { Ok(()) }
    fn repeat(&self, _enable: bool) -> Result<()> { Ok(()) }
    fn crossfade(&self, _secs: u32) -> Result<()> { Ok(()) }
    fn delete_queue(&self, _pos: u32) -> Result<()> { Ok(()) }
    fn get_shuffle(&self) -> Result<bool> { Ok(false) }
    fn get_repeat(&self) -> Result<bool> { Ok(false) }
}

/// Factory to get the correct player for the current OS
pub fn get_player() -> Box<dyn PlayerTrait> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOsPlayer)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Placeholder for Linux/Windows
        Box::new(DummyPlayer)
    }
}

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
        let output = Command::new("pgrep")
            .arg("-x")
            .arg(app_name)
            .output();
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
        } else { 0 };

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

        let script = format!(r#"
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
        "#, app_name, app_name);


        match Self::run_script(&script) {
            Ok(output) => Ok(Self::parse_player_output(&output, app_name)),
            Err(_) => Ok(None)
        }
    }

    fn play_pause(&self) -> Result<bool> {
        if let Some(app) = self.detect_active_player() {
            // Toggle and then check logic
            let script = format!(r#"
                tell application "{}"
                    playpause
                    delay 0.1
                    return player state as string
                end tell
            "#, app);
            
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
            Self::run_script(&format!("tell application \"{}\" to set player position to {}", app, position_secs))?;
        }
        Ok(())
    }

    fn volume_up(&self) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
             Self::run_script(&format!("tell application \"{}\" to set sound volume to (sound volume + 10)", app))?;
        }
        Ok(())
    }

    fn volume_down(&self) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            Self::run_script(&format!("tell application \"{}\" to set sound volume to (sound volume - 10)", app))?;
        }
        Ok(())
    }

    fn shuffle(&self, enable: bool) -> Result<()> {
        if let Some(app) = self.detect_active_player() {
            if app == "Spotify" {
                Self::run_script(&format!("tell application \"Spotify\" to set shuffling to {}", enable))?;
            } else if app == "Music" {
                 Self::run_script(&format!("tell application \"Music\" to set shuffle enabled to {}", enable))?;
            }
        }
        Ok(())
    }

    fn repeat(&self, enable: bool) -> Result<()> {
         if let Some(app) = self.detect_active_player() {
            if app == "Spotify" {
                Self::run_script(&format!("tell application \"Spotify\" to set repeating to {}", enable))?;
            } else if app == "Music" {
                let val = if enable { "all" } else { "off" };
                Self::run_script(&format!("tell application \"Music\" to set song repeat to {}", val))?;
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

    fn get_repeat(&self) -> Result<bool> {
         if let Some(app) = self.detect_active_player() {
            if app == "Spotify" {
                let output = Self::run_script("tell application \"Spotify\" to return repeating")?;
                Ok(output == "true")
            } else {
                let output = Self::run_script("tell application \"Music\" to return song repeat")?;
                // Music returns: off, one, all
                Ok(output == "all" || output == "one")
            }
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spotify_output() {
        // "Name|||Artist|||Album|||Duration|||Position|||State|||Artwork|||Volume"
        let output = "Song Name|||Artist Name|||Album Name|||180000|||12.5|||playing|||http://art.url|||80";
        let info = MacOsPlayer::parse_player_output(output, "Spotify").unwrap();
        
        assert_eq!(info.name, "Song Name");
        assert_eq!(info.artist, "Artist Name");
        assert_eq!(info.state, PlayerState::Playing);
        assert_eq!(info.duration_ms, 180000);
        assert_eq!(info.position_ms, 12500); // 12.5s * 1000
        assert_eq!(info.volume, Some(80));
    }

    #[test]
    fn test_parse_music_output() {
        // Music app often returns duration in seconds, but we handle that in AppleScript? 
        // Logic says: "tDuration to tDurSec * 1000" in AppleScript. So parsing expects ms.
        let output = "My Song|||The Artist|||The Album|||240000|||60.0|||paused|||NONE|||100";
        let info = MacOsPlayer::parse_player_output(output, "Music").unwrap();
        
        assert_eq!(info.name, "My Song");
        assert_eq!(info.source, "Music");
        assert_eq!(info.state, PlayerState::Paused);
        assert_eq!(info.artwork_url, None);
    }
}

// --- Dummy Implementation (Linux/Windows Placeholder) ---
#[cfg(not(target_os = "macos"))]
pub struct DummyPlayer;

#[cfg(not(target_os = "macos"))]
impl PlayerTrait for DummyPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> { Ok(None) }
    fn play_pause(&self) -> Result<bool> { Ok(false) }
    fn next(&self) -> Result<()> { Ok(()) }
    fn prev(&self) -> Result<()> { Ok(()) }
    fn seek(&self, _pos: f64) -> Result<()> { Ok(()) }
    fn volume_up(&self) -> Result<()> { Ok(()) }
    fn volume_down(&self) -> Result<()> { Ok(()) }
}
