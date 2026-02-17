use super::traits::{PlayerState, PlayerTrait, QueueItem, RepeatMode, TrackInfo};
use anyhow::{Context, Result};
#[cfg(feature = "mpd")]
use mpd::{Client, Song, State};
use std::sync::Mutex;

/// MPD Player implementation
#[cfg(feature = "mpd")]
pub struct MpdPlayer {
    host: String,
    port: u16,
    music_directory: String,
    client: Mutex<Option<Client>>,
}

#[cfg(feature = "mpd")]
impl MpdPlayer {
    pub fn new(host: String, port: u16, music_directory: String) -> Self {
        Self {
            host,
            port,
            music_directory,
            client: Mutex::new(None),
        }
    }

    /// Get a mutable reference to the MPD client, reconnecting if necessary.
    fn with_client<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut mpd::Client) -> Result<T>,
    {
        // 1. Lock the mutex 🔒
        let mut client_guard = self
            .client
            .lock()
            .map_err(|_| anyhow::anyhow!("MPD client mutex poisoned"))?;

        // 2. Check connection status
        let needs_connect = if let Some(client) = client_guard.as_mut() {
            client.status().is_err()
        } else {
            true
        };

        // 3. Reconnect if needed
        if needs_connect {
            let addr = format!("{}:{}", self.host, self.port);
            match mpd::Client::connect(&addr) {
                Ok(c) => {
                    *client_guard = Some(c);
                }
                Err(e) => {
                    *client_guard = None;
                    return Err(anyhow::anyhow!(
                        "Failed to connect to MPD at {}: {}",
                        addr,
                        e
                    ));
                }
            }
        }

        // 4. Use the client
        if let Some(client) = client_guard.as_mut() {
            f(client)
        } else {
            // Should be unreachable if connect logic works
            Err(anyhow::anyhow!("No MPD connection"))
        }
    }

    /// Get current audio format from MPD status
    /// Returns (sample_rate, bit_depth, channels)
    pub fn get_audio_format(&self) -> Option<(u32, u16, u16)> {
        self.with_client(|client| {
            let status = client.status()?;
            Ok(status
                .audio
                .map(|audio| (audio.rate, audio.bits as u16, audio.chans as u16)))
        })
        .ok()
        .flatten()
    }
}

#[cfg(feature = "mpd")]
impl Default for MpdPlayer {
    fn default() -> Self {
        let music_dir = dirs::audio_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Music")))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        Self::new("localhost".to_string(), 6600, music_dir)
    }
}

#[cfg(feature = "mpd")]
impl PlayerTrait for MpdPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        self.with_client(|client| {
            let current_song = client.currentsong().ok().flatten();
            let status = client.status()?;

            if let Some(song) = current_song {
                let position_ms = status
                    .elapsed
                    .map(|t| t.as_secs() * 1000 + t.subsec_millis() as u64)
                    .unwrap_or(0);
                let duration_ms = status
                    .duration
                    .map(|t| t.as_secs() * 1000 + t.subsec_millis() as u64)
                    .unwrap_or(0);

                let file_path = if !song.file.starts_with('/') {
                    format!("{}/{}", self.music_directory, song.file)
                } else {
                    song.file.clone()
                };

                let artwork_url = None; // Placeholder

                let audio_format = status.audio.map(|a| (a.rate, a.bits, a.chans));

                // Helper to find tag
                let find_tag = |tags: &[(String, String)], key: &str| -> Option<String> {
                    let key_lower = key.to_lowercase();
                    tags.iter()
                        .find(|(k, _)| k.to_lowercase() == key_lower)
                        .map(|(_, v)| v.clone())
                };

                let album = find_tag(&song.tags, "Album").unwrap_or_else(|| "Unknown".to_string());

                Ok(Some(TrackInfo {
                    name: song.title.unwrap_or_else(|| "Unknown".to_string()),
                    artist: song.artist.unwrap_or_else(|| "Unknown".to_string()),
                    album,
                    duration_ms,
                    position_ms,
                    state: match status.state {
                        State::Play => PlayerState::Playing,
                        State::Pause => PlayerState::Paused,
                        State::Stop => PlayerState::Stopped,
                    },
                    artwork_url,
                    source: "MPD".to_string(),
                    codec: std::path::Path::new(&song.file)
                        .extension()
                        .map(|os| os.to_string_lossy().to_string())
                        .or_else(|| find_tag(&song.tags, "Format"))
                        .or_else(|| find_tag(&song.tags, "Codec")),
                    bitrate: status.bitrate,
                    sample_rate: audio_format.map(|(r, _, _)| r),
                    bit_depth: audio_format.map(|(_, b, _)| b),
                    file_path: Some(file_path),
                    volume: Some(status.volume.unsigned_abs() as u32),
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn play_pause(&self) -> Result<bool> {
        self.with_client(|client| {
            let status = client.status()?;
            match status.state {
                State::Play => client.pause(true)?,
                State::Pause | State::Stop => client.play()?,
            };
            Ok(status.state != State::Play)
        })
    }

    fn next(&self) -> Result<()> {
        self.with_client(|client| client.next().context("Failed to skip to next track"))
    }

    fn prev(&self) -> Result<()> {
        self.with_client(|client| client.prev().context("Failed to skip to previous track"))
    }

    fn seek(&self, position_secs: f64) -> Result<()> {
        self.with_client(|client| {
            let song = client.currentsong()?.context("No song playing")?;
            let place = song.place.context("No song place")?;
            client
                .seek(place.id, position_secs)
                .context("Failed to seek")
        })
    }

    fn volume_up(&self) -> Result<()> {
        self.with_client(|client| {
            let status = client.status()?;
            let vol = status.volume;
            if vol >= 0 {
                let new_vol = (vol + 5).min(100);
                client.volume(new_vol)?;
            }
            Ok(())
        })
    }

    fn volume_down(&self) -> Result<()> {
        self.with_client(|client| {
            let status = client.status()?;
            let vol = status.volume;
            if vol >= 0 {
                let new_vol = (vol - 5).max(0);
                client.volume(new_vol)?;
            }
            Ok(())
        })
    }

    fn set_volume(&self, volume: u8) -> Result<()> {
        self.with_client(|client| {
            client
                .volume(volume.min(100) as i8)
                .context("Failed to set volume")
        })
    }

    fn get_queue(&self) -> Result<Vec<QueueItem>> {
        self.with_client(|client| {
            let queue = client.queue()?;
            let current_song = client.currentsong().ok().flatten();
            let current_id = current_song.and_then(|s| s.place).map(|p| p.id);

            // Helper closure to find tag value (case-insensitive)
            let find_tag = |tags: &[(String, String)], key: &str| -> Option<String> {
                let key_lower = key.to_lowercase();
                tags.iter()
                    .find(|(k, _)| k.to_lowercase() == key_lower)
                    .map(|(_, v)| v.clone())
            };

            Ok(queue
                .into_iter()
                .map(|song| {
                    let id = song.place.map(|p| p.id).unwrap_or_default();
                    let title = song.title.clone().unwrap_or_else(|| song.file.clone());
                    let artist = song
                        .artist
                        .clone()
                        .or_else(|| find_tag(&song.tags, "Artist"))
                        .or_else(|| find_tag(&song.tags, "AlbumArtist"))
                        .or_else(|| find_tag(&song.tags, "Composer"))
                        .unwrap_or_else(|| "Unknown Artist".to_string());
                    let duration_ms = song
                        .duration
                        .map(|d| d.as_secs() * 1000 + d.subsec_millis() as u64)
                        .unwrap_or(0);

                    (
                        title,
                        artist,
                        duration_ms,
                        Some(id.0) == current_id.map(|i| i.0),
                        song.file.clone(),
                    )
                })
                .collect())
        })
    }

    fn shuffle(&self, enable: bool) -> Result<()> {
        self.with_client(|client| client.random(enable).context("Failed to toggle shuffle"))
    }

    fn repeat(&self, mode: RepeatMode) -> Result<()> {
        self.with_client(|client| {
            match mode {
                RepeatMode::Off => {
                    client.repeat(false)?;
                    client.single(false)?;
                }
                RepeatMode::Playlist => {
                    client.repeat(true)?;
                    client.single(false)?;
                }
                RepeatMode::Single => {
                    client.repeat(true)?;
                    client.single(true)?;
                }
            }
            Ok(())
        })
    }

    fn crossfade(&self, secs: u32) -> Result<()> {
        self.with_client(|client| {
            client
                .crossfade(secs as i64)
                .context("Failed to set crossfade")
        })
    }

    fn delete_queue(&self, pos: u32) -> Result<()> {
        self.with_client(|client| client.delete(pos).context("Failed to delete from queue"))
    }

    fn get_shuffle(&self) -> Result<bool> {
        self.with_client(|client| Ok(client.status()?.random))
    }

    fn get_repeat(&self) -> Result<RepeatMode> {
        self.with_client(|client| {
            let status = client.status()?;
            if status.single && status.repeat {
                Ok(RepeatMode::Single)
            } else if status.repeat {
                Ok(RepeatMode::Playlist)
            } else {
                Ok(RepeatMode::Off)
            }
        })
    }
}

#[cfg(feature = "mpd")]
impl MpdPlayer {
    /// Set crossfade duration in seconds (0 to disable)
    pub fn set_crossfade(&self, seconds: u32) -> Result<()> {
        self.with_client(|client| {
            client
                .crossfade(seconds as i64)
                .context("Failed to set crossfade")
        })
    }

    /// Get current crossfade setting
    pub fn get_crossfade(&self) -> Result<u32> {
        self.with_client(|client| {
            let status = client.status()?;
            Ok(status.crossfade.map(|d| d.as_secs() as u32).unwrap_or(0))
        })
    }

    // ═══════════════════════════════════════════════════════════════
    // Library Browsing Methods 📚
    // ═══════════════════════════════════════════════════════════════

    /// List all artists from the database
    pub fn list_artists(&self) -> Result<Vec<String>> {
        self.with_client(|client| {
            let songs = client.listall()?;
            let mut artists: Vec<String> = songs
                .iter()
                .filter_map(|s| {
                    s.tags
                        .iter()
                        .find(|(k, _)| k == "Artist")
                        .map(|(_, v)| v.clone())
                })
                .collect();
            artists.sort();
            artists.dedup();
            Ok(artists)
        })
    }

    /// List all albums (or albums by artist)
    pub fn list_albums(&self, _artist: Option<&str>) -> Result<Vec<String>> {
        self.with_client(|client| {
            let songs = client.listall()?;
            let mut albums: Vec<String> = songs
                .iter()
                .filter_map(|s| {
                    s.tags
                        .iter()
                        .find(|(k, _)| k == "Album")
                        .map(|(_, v)| v.clone())
                })
                .collect();
            albums.sort();
            albums.dedup();
            Ok(albums)
        })
    }

    /// List genres
    pub fn list_genres(&self) -> Result<Vec<String>> {
        self.with_client(|client| {
            let songs = client.listall()?;
            let mut genres: Vec<String> = songs
                .iter()
                .filter_map(|s| {
                    s.tags
                        .iter()
                        .find(|(k, _)| k == "Genre")
                        .map(|(_, v)| v.clone())
                })
                .collect();
            genres.sort();
            genres.dedup();
            Ok(genres)
        })
    }

    /// Search library by any field
    pub fn search_library(&self, query: &str) -> Result<Vec<Song>> {
        self.with_client(|client| {
            // Get all songs and filter manually
            let songs = client.listall()?;
            let query_lower = query.to_lowercase();
            let results: Vec<Song> = songs
                .into_iter()
                .filter(|s| {
                    s.file.to_lowercase().contains(&query_lower)
                        || s.title
                            .as_ref()
                            .map(|t| t.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                        || s.tags
                            .iter()
                            .any(|(_, v)| v.to_lowercase().contains(&query_lower))
                })
                .take(50) // Limit results
                .collect();
            Ok(results)
        })
    }

    /// List saved playlists
    pub fn list_playlists(&self) -> Result<Vec<String>> {
        self.with_client(|client| {
            let playlists = client.playlists()?;
            Ok(playlists.iter().map(|p| p.name.clone()).collect())
        })
    }

    /// Load a playlist
    pub fn load_playlist(&self, name: &str) -> Result<()> {
        self.with_client(|client| client.load(name, ..).context("Failed to load playlist"))
    }

    /// Save current queue as playlist
    pub fn save_playlist(&self, name: &str) -> Result<()> {
        self.with_client(|client| client.save(name).context("Failed to save playlist"))
    }

    /// Rename a playlist
    pub fn rename_playlist(&self, old_name: &str, new_name: &str) -> Result<()> {
        self.with_client(|client| {
            client
                .pl_rename(old_name, new_name)
                .context("Failed to rename playlist")
        })
    }

    /// Add song to queue by file path
    pub fn add_to_queue(&self, path: &str) -> Result<()> {
        self.with_client(|client| {
            let song = Song {
                file: path.to_string(),
                ..Default::default()
            };
            let _ = client.push(&song); // Discard result (Id)
            Ok(())
        })
    }

    /// Play song at position in queue
    pub fn play_pos(&self, pos: u32) -> Result<()> {
        self.with_client(|client| client.switch(pos).context("Failed to switch to position"))
    }
}
