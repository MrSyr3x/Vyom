//! MPD Player Backend for Vyom Pro 🎵
//! 
//! Implements `PlayerTrait` using the Music Player Daemon protocol.

#[cfg(feature = "mpd")]
use mpd::{Client, Song, State};
use anyhow::{Result, Context};
use crate::player::{PlayerTrait, TrackInfo, PlayerState};

/// MPD Player implementation
#[cfg(feature = "mpd")]
pub struct MpdPlayer {
    host: String,
    port: u16,
    music_directory: String,
}

#[cfg(feature = "mpd")]
impl MpdPlayer {
    pub fn new(host: &str, port: u16) -> Self {
        // Get music directory from MPD config or use default
        let music_dir = std::env::var("HOME")
            .map(|h| format!("{}/Music", h))
            .unwrap_or_else(|_| "/Users/syr3x/Music".to_string());
        
        Self {
            host: host.to_string(),
            port,
            music_directory: music_dir,
        }
    }

    #[allow(dead_code)]
    pub fn default() -> Self {
        Self::new("localhost", 6600)
    }

    fn connect(&self) -> Result<Client> {
        let addr = format!("{}:{}", self.host, self.port);
        Client::connect(&addr).context("Failed to connect to MPD")
    }

    fn extract_audio_info(&self, song: &Song) -> (Option<String>, Option<u32>, Option<u32>, Option<u8>) {
        // MPD provides audio format in status, but we can infer from file extension
        let path = &song.file;
        let codec = if path.ends_with(".flac") {
            Some("FLAC".to_string())
        } else if path.ends_with(".mp3") {
            Some("MP3".to_string())
        } else if path.ends_with(".m4a") || path.ends_with(".aac") {
            Some("AAC".to_string())
        } else if path.ends_with(".wav") {
            Some("WAV".to_string())
        } else if path.ends_with(".ogg") {
            Some("OGG".to_string())
        } else if path.ends_with(".opus") {
            Some("OPUS".to_string())
        } else {
            None
        };

        // Bitrate, Sample Rate, Bit Depth come from status.audio
        // We'll populate these from status in get_current_track
        (codec, None, None, None)
    }
    
    /// Get current audio format from MPD status
    /// Returns (sample_rate, bit_depth, channels)
    pub fn get_audio_format(&self) -> Option<(u32, u16, u16)> {
        let mut conn = self.connect().ok()?;
        let status = conn.status().ok()?;
        
        status.audio.map(|audio| {
            (audio.rate, audio.bits as u16, audio.chans as u16)
        })
    }
}

#[cfg(feature = "mpd")]
impl PlayerTrait for MpdPlayer {
    fn get_current_track(&self) -> Result<Option<TrackInfo>> {
        let mut conn = self.connect()?;
        
        let status = conn.status()?;
        let song = match conn.currentsong()? {
            Some(s) => s,
            None => return Ok(None),
        };

        let state = match status.state {
            State::Play => PlayerState::Playing,
            State::Pause => PlayerState::Paused,
            State::Stop => PlayerState::Stopped,
        };

        let (codec, _, _, _) = self.extract_audio_info(&song);
        
        // Extract audio format from status
        let (sample_rate, bit_depth, bitrate) = if let Some(audio) = status.audio {
            (Some(audio.rate), Some(audio.bits as u8), None)
        } else {
            (None, None, status.bitrate.map(|b| b as u32))
        };

        let duration_ms = song.duration
            .map(|d| d.as_secs() * 1000 + d.subsec_millis() as u64)
            .unwrap_or(0);

        let position_ms = status.elapsed
            .map(|d| d.as_secs() * 1000 + d.subsec_millis() as u64)
            .unwrap_or(0);

        // Helper closure to find tag value (case-insensitive)
        let find_tag = |tags: &[(String, String)], key: &str| -> Option<String> {
            let key_lower = key.to_lowercase();
            tags.iter()
                .find(|(k, _)| k.to_lowercase() == key_lower)
                .map(|(_, v)| v.clone())
        };

        // Build absolute file path for embedded artwork extraction
        let file_path = format!("{}/{}", self.music_directory, song.file);
        
        Ok(Some(TrackInfo {
            name: song.title.as_deref().or_else(|| find_tag(&song.tags, "Title").as_deref()).unwrap_or(&song.file).to_string(),
            artist: song.artist.as_deref()
                .or_else(|| find_tag(&song.tags, "Artist").as_deref())
                .or_else(|| find_tag(&song.tags, "AlbumArtist").as_deref())
                .or_else(|| find_tag(&song.tags, "Composer").as_deref())
                .unwrap_or("Unknown Artist").to_string(),
            album: song.album.as_deref()
                .or_else(|| find_tag(&song.tags, "Album").as_deref())
                .unwrap_or("Unknown Album").to_string(),
            artwork_url: None, // Will be extracted from embedded art
            duration_ms,
            position_ms,
            state,
            source: "MPD".to_string(),
            codec,
            bitrate,
            sample_rate,
            bit_depth,
            file_path: Some(file_path),
            volume: if status.volume >= 0 { Some(status.volume as u32) } else { None },
        }))
    }

    fn play_pause(&self) -> Result<bool> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        match status.state {
            State::Play => {
                conn.pause(true)?;
                Ok(false) // Now Paused
            },
            State::Pause => {
                conn.pause(false)?;
                Ok(true) // Now Playing
            },
            State::Stop => {
                conn.play()?;
                Ok(true) // Now Playing
            }
        }
    }

    fn next(&self) -> Result<()> {
        let mut conn = self.connect()?;
        conn.next()?;
        Ok(())
    }

    fn prev(&self) -> Result<()> {
        let mut conn = self.connect()?;
        conn.prev()?;
        Ok(())
    }

    fn seek(&self, position_secs: f64) -> Result<()> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        if let Some(song_pos) = status.song {
            conn.seek(song_pos.pos, std::time::Duration::from_secs_f64(position_secs))?;
        }
        Ok(())
    }

    fn volume_up(&self) -> Result<()> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        let new_vol = (status.volume + 5).min(100);
        conn.volume(new_vol)?;
        Ok(())
    }

    fn volume_down(&self) -> Result<()> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        let new_vol = status.volume.saturating_sub(5);
        conn.volume(new_vol)?;
        Ok(())
    }
    
    fn get_queue(&self) -> Result<Vec<(String, String, u64, bool, String)>> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        let queue = conn.queue()?;
        
        let current_pos = status.song.map(|s| s.pos);
        
        // Helper closure to find tag value (case-insensitive)
        let find_tag = |tags: &[(String, String)], key: &str| -> Option<String> {
            let key_lower = key.to_lowercase();
            tags.iter()
                .find(|(k, _)| k.to_lowercase() == key_lower)
                .map(|(_, v)| v.clone())
        };
        
        let items: Vec<(String, String, u64, bool, String)> = queue.iter().enumerate().map(|(i, song)| {
            let title = song.title.clone()
                .unwrap_or_else(|| song.file.clone());
            let artist = song.artist.clone()
                .or_else(|| find_tag(&song.tags, "Artist"))
                .or_else(|| find_tag(&song.tags, "AlbumArtist"))
                .or_else(|| find_tag(&song.tags, "Composer"))
                .unwrap_or_else(|| "Unknown Artist".to_string());
            let duration_ms = song.duration
                .map(|d| d.as_secs() * 1000 + d.subsec_millis() as u64)
                .unwrap_or(0);
            let is_current = Some(i as u32) == current_pos;
            let file_path = song.file.clone();
            
            (title, artist, duration_ms, is_current, file_path)
        }).collect();
        
        Ok(items)
    }
    
    fn shuffle(&self, enable: bool) -> Result<()> {
        let mut conn = self.connect()?;
        conn.random(enable)?;
        Ok(())
    }
    
    fn repeat(&self, enable: bool) -> Result<()> {
        let mut conn = self.connect()?;
        conn.repeat(enable)?;
        Ok(())
    }
    
    fn crossfade(&self, secs: u32) -> Result<()> {
        let mut conn = self.connect()?;
        conn.crossfade(secs as i64)?;
        Ok(())
    }
    
    fn delete_queue(&self, pos: u32) -> Result<()> {
        let mut conn = self.connect()?;
        conn.delete(pos)?;
        Ok(())
    }
    
    fn get_shuffle(&self) -> Result<bool> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        Ok(status.random)
    }
    
    fn get_repeat(&self) -> Result<bool> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        Ok(status.repeat)
    }
}

#[cfg(feature = "mpd")]
impl MpdPlayer {
    /// Set crossfade duration in seconds (0 to disable)
    pub fn set_crossfade(&self, seconds: u32) -> Result<()> {
        let mut conn = self.connect()?;
        conn.crossfade(seconds as i64)?;
        Ok(())
    }
    
    /// Get current crossfade setting
    pub fn get_crossfade(&self) -> Result<u32> {
        let mut conn = self.connect()?;
        let status = conn.status()?;
        Ok(status.crossfade.map(|d| d.as_secs() as u32).unwrap_or(0))
    }
    
    // ═══════════════════════════════════════════════════════════════
    // Library Browsing Methods 📚
    // ═══════════════════════════════════════════════════════════════
    
    /// List all artists from the database
    pub fn list_artists(&self) -> Result<Vec<String>> {
        let mut conn = self.connect()?;
        // Use simple search and extract unique artists
        let songs = conn.listall()?;
        let mut artists: Vec<String> = songs.iter()
            .filter_map(|s| {
                s.tags.iter()
                    .find(|(k, _)| k == "Artist")
                    .map(|(_, v)| v.clone())
            })
            .collect();
        artists.sort();
        artists.dedup();
        Ok(artists)
    }
    
    /// List all albums (or albums by artist)
    pub fn list_albums(&self, _artist: Option<&str>) -> Result<Vec<String>> {
        let mut conn = self.connect()?;
        let songs = conn.listall()?;
        let mut albums: Vec<String> = songs.iter()
            .filter_map(|s| {
                s.tags.iter()
                    .find(|(k, _)| k == "Album")
                    .map(|(_, v)| v.clone())
            })
            .collect();
        albums.sort();
        albums.dedup();
        Ok(albums)
    }
    
    /// List genres
    pub fn list_genres(&self) -> Result<Vec<String>> {
        let mut conn = self.connect()?;
        let songs = conn.listall()?;
        let mut genres: Vec<String> = songs.iter()
            .filter_map(|s| {
                s.tags.iter()
                    .find(|(k, _)| k == "Genre")
                    .map(|(_, v)| v.clone())
            })
            .collect();
        genres.sort();
        genres.dedup();
        Ok(genres)
    }
    
    /// Search library by any field
    pub fn search_library(&self, query: &str) -> Result<Vec<Song>> {
        let mut conn = self.connect()?;
        // Get all songs and filter manually
        let songs = conn.listall()?;
        let query_lower = query.to_lowercase();
        let results: Vec<Song> = songs.into_iter()
            .filter(|s| {
                s.file.to_lowercase().contains(&query_lower) ||
                s.title.as_ref().map(|t| t.to_lowercase().contains(&query_lower)).unwrap_or(false) ||
                s.tags.iter().any(|(_, v)| v.to_lowercase().contains(&query_lower))
            })
            .take(50) // Limit results
            .collect();
        Ok(results)
    }
    
    /// List saved playlists
    pub fn list_playlists(&self) -> Result<Vec<String>> {
        let mut conn = self.connect()?;
        let playlists = conn.playlists()?;
        Ok(playlists.iter().map(|p| p.name.clone()).collect())
    }
    
    /// Load a playlist
    pub fn load_playlist(&self, name: &str) -> Result<()> {
        let mut conn = self.connect()?;
        conn.load(name, ..)?;
        Ok(())
    }
    
    /// Save current queue as playlist
    pub fn save_playlist(&self, name: &str) -> Result<()> {
        let mut conn = self.connect()?;
        conn.save(name)?;
        Ok(())
    }
    
    /// Add song to queue by file path
    pub fn add_to_queue(&self, path: &str) -> Result<()> {
        let mut conn = self.connect()?;
        let song = Song { file: path.to_string(), ..Default::default() };
        conn.push(&song)?;
        Ok(())
    }
    
    /// Play song at position in queue
    pub fn play_pos(&self, pos: u32) -> Result<()> {
        let mut conn = self.connect()?;
        conn.switch(pos)?;
        Ok(())
    }
}

