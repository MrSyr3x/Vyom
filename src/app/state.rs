use super::config::{AppConfig, EqPreset};
use super::lyrics::LyricLine;
use crate::audio::device as audio_device;
use crate::audio::dsp::EqGains;
use crate::audio::visualizer::Visualizer;
use crate::player::TrackInfo;
use crate::ui::theme::Theme;
use image::DynamicImage;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum LyricsState {
    Idle,
    Loading,
    Loaded(Vec<LyricLine>),
    Instrumental,
    Failed(String),
    NotFound,
}

pub enum ArtworkState {
    Idle,
    Loading,
    Loaded(DynamicImage),
    Failed,
}

/// View mode for the right panel 🎛️
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Lyrics,
    Visualizer,
    Library, // Renamed from Queue → Library
    EQ,
}

/// Library panel sub-mode 📚
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LibraryMode {
    #[default]
    Queue, // Current queue
    Directory, // Neo-tree style music folder browser
    Search,    // Search library
    Playlists, // Saved playlists
}

/// Tag editing state 🏷️
#[derive(Debug, Clone)]
pub struct TagEditState {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub active_field: usize, // 0=title, 1=artist, 2=album
}

/// Generic Input Popup Mode 📝
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    PlaylistSave,

    EqSave,
    PlaylistRename(String), // Carries old name
}

/// Generic Input Popup State 📝
#[derive(Debug, Clone)]
pub struct InputState {
    pub mode: InputMode,
    pub title: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub start_time: Instant,
    pub deadline: Instant,
}

impl InputState {
    pub fn new(mode: InputMode, title: &str, initial_value: &str) -> Self {
        Self {
            mode,
            title: title.to_string(),
            value: initial_value.to_string(),
        }
    }
}

impl TagEditState {
    pub fn new(path: &str, title: &str, artist: &str, album: &str) -> Self {
        Self {
            file_path: path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            active_field: 0,
        }
    }

    pub fn active_value(&mut self) -> &mut String {
        match self.active_field {
            0 => &mut self.title,
            1 => &mut self.artist,
            _ => &mut self.album,
        }
    }

    pub fn next_field(&mut self) {
        self.active_field = (self.active_field + 1) % 3;
    }

    pub fn prev_field(&mut self) {
        self.active_field = if self.active_field == 0 {
            2
        } else {
            self.active_field - 1
        };
    }
}

/// Library browser item type
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryItemType {
    Artist,
    Album,
    Song,

    Folder,
    Playlist,
}

/// Library browser item
#[derive(Debug, Clone)]
pub struct LibraryItem {
    pub name: String,
    pub item_type: LibraryItemType,
    pub artist: Option<String>,   // For songs/albums
    pub duration_ms: Option<u64>, // For songs
    pub path: Option<String>,     // MPD file path
}

/// Queue item for MPD playlist display 📋
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub is_current: bool,
    pub file_path: String, // For tag editing
}

pub struct App {
    pub theme: Theme,

    pub is_running: bool,
    pub track: Option<TrackInfo>,
    pub lyrics: LyricsState, // changed from Option<Vec<LyricLine>>
    pub artwork: ArtworkState,
    // Manual Scroll State (None = Auto-sync)
    pub lyrics_offset: Option<usize>,
    pub lyrics_selected: Option<usize>, // Manual selection for j/k navigation
    pub lyrics_cache: HashMap<String, Vec<LyricLine>>,
    pub last_scroll_time: Option<Instant>,

    // Seek Accumulation State ⏩
    pub seek_accumulator: f64,
    pub last_seek_time: Option<Instant>,
    pub seek_initial_pos: Option<f64>,

    // Animation State 🌊
    pub smooth_scroll_accum: f64,

    // Playback Timing State ⏱️
    pub last_track_update: Option<std::time::Instant>,

    pub app_show_lyrics: bool,
    pub is_tmux: bool,      // Layout logic
    pub is_mpd: bool,       // MPD backend mode
    pub source_app: String, // "MPD", "Spotify", "Apple Music"

    /// Current panel view mode (Lyrics/Cava/Queue/EQ) 🎛️
    pub view_mode: ViewMode,

    /// MPD Queue (playlist) 📋
    pub queue: Vec<QueueItem>,

    /// Smart Library Panel 📚
    pub library_mode: LibraryMode,
    pub previous_library_mode: Option<LibraryMode>, // Track previous mode for search exit
    pub library_items: Vec<LibraryItem>,
    pub library_selected: usize,
    pub browse_path: Vec<String>, // Breadcrumb navigation
    pub search_query: String,
    pub search_active: bool,    // Is search input active
    pub playlists: Vec<String>, // Available playlists

    /// Visualizer bars (0.0-1.0 heights) 📊
    pub visualizer_bars: Vec<f32>,
    pub visualizer: Visualizer,

    /// EQ State 🎛️
    /// 10-band EQ: 32Hz, 64Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz
    /// Values: 0.0 = -12dB, 0.5 = 0dB, 1.0 = +12dB
    pub eq_bands: [f32; 10],
    pub eq_selected: usize,
    pub eq_enabled: bool,
    pub eq_preset: usize, // Index into EQ_PRESETS

    /// Audiophile Controls 🎚️
    /// Internal Volume State (0-100)
    pub app_volume: u8,
    pub preamp_db: f32,       // -12 to +12 dB
    pub balance: f32,         // -1.0 (L) to +1.0 (R)
    pub crossfade_secs: u32,  // MPD crossfade in seconds
    pub replay_gain_mode: u8, // 0=Off, 1=Track, 2=Album, 3=Auto

    /// UI State
    pub show_keyhints: bool, // WhichKey popup visible
    pub show_audio_info: bool, // Audio Info popup visible (like Poweramp)
    pub tag_edit: Option<TagEditState>,
    pub input_state: Option<InputState>,
    pub toast: Option<Toast>,
    pub gapless_mode: bool, // True when current+next song are from same album
    pub last_album: String, // Track album changes
    pub shuffle: bool,      // MPD random mode
    pub repeat: bool,       // MPD repeat mode

    /// Audio output devices 🔊
    pub output_device: String,
    pub audio_devices: Vec<String>,
    pub selected_device_idx: usize,

    /// Shared EQ gains for DSP engine
    pub eq_gains: EqGains,
    /// DSP EQ is always available (built-in)
    pub dsp_available: bool,
    // Persistence & Custom Presets
    pub presets: Vec<EqPreset>,
    pub eq_preset_name: String, // Tracks current preset name for logic

    /// Music directory for local file operations 📂
    pub music_directory: String,
}

impl App {
    pub fn new(
        app_show_lyrics: bool,
        is_tmux: bool,
        is_mpd: bool,
        source_app: &str,
        config: AppConfig,
    ) -> Self {
        // Merge defaults with user presets
        // If config.presets is empty, it means we don't have custom ones yet, but we should always have defaults available.
        // Actually, let's keep user custom presets separate or append them?
        // Let's create a combined list: Defaults + User Config Presets
        let mut presets = AppConfig::get_default_presets();
        presets.extend(config.presets.clone());

        // Find index of last used preset
        let eq_preset_idx = presets
            .iter()
            .position(|p| p.name == config.last_preset_name)
            .unwrap_or(0); // Default to first (Custom or Flat)

        let app = Self {
            theme: crate::ui::theme::load_current_theme(),
            is_running: true,
            track: None,
            lyrics: LyricsState::Idle,
            artwork: ArtworkState::Idle,
            lyrics_offset: None,
            lyrics_selected: None,
            lyrics_cache: HashMap::new(),
            last_scroll_time: None,
            seek_accumulator: 0.0,
            last_seek_time: None,
            seek_initial_pos: None,
            smooth_scroll_accum: 0.0,
            last_track_update: None,
            app_show_lyrics,
            is_tmux,
            is_mpd,
            source_app: source_app.to_string(),
            view_mode: ViewMode::default(),
            queue: Vec::new(),

            library_mode: LibraryMode::default(),
            previous_library_mode: None,
            library_items: Vec::new(),
            library_selected: 0,
            browse_path: Vec::new(),
            search_query: String::new(),
            search_active: false,
            playlists: Vec::new(),
            visualizer_bars: vec![0.0; 60],
            visualizer: Visualizer::new(44100), // Default 44.1k, will adapt? Or fixed for vis?

            // Persistence loading
            eq_bands: config.eq_bands,
            eq_selected: 0,
            eq_enabled: config.eq_enabled,
            eq_preset: eq_preset_idx,
            app_volume: 100,
            preamp_db: config.preamp_db,
            balance: config.balance,
            crossfade_secs: config.crossfade,
            replay_gain_mode: config.replay_gain_mode,

            show_keyhints: false,   // Hidden by default
            show_audio_info: false, // Hidden by default
            tag_edit: None,
            input_state: None,   // No input popup active
            toast: None,         // No toast notification
            gapless_mode: false, // No gapless detected initially
            last_album: String::new(),
            shuffle: false, // Will be updated from MPD
            repeat: false,  // Will be updated from MPD
            output_device: audio_device::get_output_device_name(),
            audio_devices: {
                let sys_devices = audio_device::get_devices_from_system();
                if !sys_devices.is_empty() {
                    sys_devices
                } else {
                    audio_device::get_output_devices()
                        .into_iter()
                        .map(|d| d.name)
                        .collect()
                }
            },
            selected_device_idx: 0,

            // Initialization for Persistence fields
            eq_gains: EqGains::default(),

            dsp_available: true, // Built-in DSP is always available

            // Persistence & Custom Presets
            presets,
            eq_preset_name: config.last_preset_name,

            music_directory: config.music_directory,
        };

        // CRITICAL FIX: Sync loaded EQ state to DSP engine immediately! 🔊
        // Otherwise, we launch with flat EQ despite UI showing "Bass Boost".
        app.sync_eq_to_dsp();

        app
    }

    pub fn get_current_position_ms(&self) -> u64 {
        if let Some(track) = &self.track {
            if track.state == crate::player::PlayerState::Playing {
                if let Some(last_update) = self.last_track_update {
                    let elapsed = last_update.elapsed().as_millis() as u64;
                    // Clamp to duration to prevent overshooting
                    return (track.position_ms + elapsed).min(track.duration_ms);
                }
            }
            track.position_ms
        } else {
            0
        }
    }

    /// Sync EQ bands to DSP engine
    pub fn sync_eq_to_dsp(&self) {
        self.eq_gains.set_all_from_values(&self.eq_bands);
        self.eq_gains.set_enabled(self.eq_enabled);
    }

    /// Sync a single EQ band to DSP engine
    pub fn sync_band_to_dsp(&self, band_index: usize) {
        if band_index < 10 {
            self.eq_gains
                .set_gain_from_value(band_index, self.eq_bands[band_index]);
        }
    }

    /// Reset EQ to flat and clear Custom preset
    pub fn reset_eq(&mut self) {
        self.eq_bands = [0.5; 10];

        // Reset Custom preset if it exists so previous tweaks are discarded
        if let Some(custom_pos) = self.presets.iter().position(|p| p.name == "Custom") {
            self.presets[custom_pos].bands = [0.5; 10];
        }

        // Switch back to Flat preset
        if let Some(pos) = self.presets.iter().position(|p| p.name == "Flat") {
            self.eq_preset = pos;
            self.eq_preset_name = "Flat".to_string();
        }
        self.eq_gains.reset();
    }

    /// Toggle EQ enabled state
    pub fn toggle_eq(&mut self) {
        self.eq_enabled = !self.eq_enabled;
        self.eq_gains.set_enabled(self.eq_enabled);
    }

    pub fn show_toast(&mut self, message: &str) {
        let now = Instant::now();
        let duration = std::time::Duration::from_millis(2000); // 2s display time
        let deadline = now + duration;

        if let Some(ref mut current) = self.toast {
            // Intelligent Update:
            // If previous toast is recent (less than 500ms old) OR still active,
            // update message and extend deadline, but keep start_time to preserve "Entrance" state.
            // This prevents the "flashing" animation on rapid updates.
            current.message = message.to_string();
            current.deadline = deadline;
            // start_time is INTENTIONALLY left alone!
        } else {
            // New Toast
            self.toast = Some(Toast {
                message: message.to_string(),
                start_time: now,
                deadline,
            });
        }
    }

    /// Called every tick to update state
    pub fn on_tick(&mut self) {
        // Handle Toast Expiry
        if let Some(ref toast) = self.toast {
            if Instant::now() > toast.deadline {
                self.toast = None;
            }
        }
    }

    /// Apply current preset to EQ bands
    pub fn apply_preset(&mut self) {
        if self.eq_preset < self.presets.len() {
            self.eq_bands = self.presets[self.eq_preset].bands;
            self.eq_preset_name = self.presets[self.eq_preset].name.clone();
            self.sync_eq_to_dsp();
        }
    }

    /// Cycle to next preset
    pub fn next_preset(&mut self) {
        self.eq_preset = (self.eq_preset + 1) % self.presets.len();
        self.apply_preset();
    }

    /// Cycle to previous preset
    pub fn prev_preset(&mut self) {
        self.eq_preset = if self.eq_preset == 0 {
            self.presets.len() - 1
        } else {
            self.eq_preset - 1
        };
        self.apply_preset();
    }

    /// Get current preset name
    pub fn get_preset_name(&self) -> String {
        if self.eq_preset < self.presets.len() {
            self.presets[self.eq_preset].name.clone()
        } else {
            "Unknown".to_string()
        }
    }

    /// Mark as custom preset when user manually adjusts bands
    /// Mark as custom preset when user manually adjusts bands
    pub fn mark_custom(&mut self) {
        // Find "Custom" preset or create it
        if let Some(pos) = self.presets.iter().position(|p| p.name == "Custom") {
            self.eq_preset = pos;
            self.eq_preset_name = "Custom".to_string();
            // Critical: Sync the Custom preset's storage with current live bands
            // so if we switch away and back, we keep the tweaks.
            self.presets[pos].bands = self.eq_bands;
        } else {
            // "Custom" doesn't exist, create it at index 0
            let custom = EqPreset::new("Custom", self.eq_bands);
            self.presets.insert(0, custom);
            self.eq_preset = 0;
            self.eq_preset_name = "Custom".to_string();
        }
    }

    /// Save current EQ bands as a new preset
    pub fn save_preset(&mut self, name: String) {
        let preset = EqPreset::new(&name, self.eq_bands);
        self.presets.push(preset);
        self.eq_preset = self.presets.len() - 1; // Switch to new preset
        self.eq_preset_name = name;
    }

    /// Delete current preset (if not a builtin)
    pub fn delete_preset(&mut self) -> Result<(), String> {
        if self.eq_preset < self.presets.len() {
            let name = self.presets[self.eq_preset].name.as_str();

            // Prevent deleting defaults
            let defaults = AppConfig::get_default_presets();
            if defaults.iter().any(|d| d.name == name) {
                return Err("Cannot delete built-in preset".to_string());
            }

            self.presets.remove(self.eq_preset);
            if self.eq_preset >= self.presets.len() {
                self.eq_preset = self.presets.len().saturating_sub(1);
            }
            // re-apply
            self.apply_preset();
            return Ok(());
        }
        Err("No preset selected".to_string())
    }

    pub fn save_state(&self) {
        // Collect only user presets (exclude defaults)
        let defaults = AppConfig::get_default_presets();
        let default_names: Vec<&String> = defaults.iter().map(|p| &p.name).collect();

        let _user_presets: Vec<EqPreset> = self
            .presets
            .iter()
            .filter(|p| !default_names.contains(&&p.name) || p.name == "Custom") // Keep Custom if modified? Actually Custom is in default list.
            .cloned()
            .collect();

        // Actually, we should probably ONLY save presets that are NOT in the default list?
        // But if user modified "Custom", we might want to save it if we allow it.
        // For now, simple logic: Filter out any preset where Name + Bands matches a default?
        // Simpler: Just filter by Name. If user names it "Bass Booster", tough luck, it's treated as default.
        // Just saving unique names.

        let config = AppConfig {
            presets: self
                .presets
                .iter()
                .filter(|p| !defaults.iter().any(|d| d.name == p.name))
                .cloned()
                .collect(),
            last_preset_name: self.get_preset_name(),
            eq_enabled: self.eq_enabled,
            eq_bands: self.eq_bands,
            preamp_db: self.preamp_db,
            balance: self.balance,
            crossfade: self.crossfade_secs,
            replay_gain_mode: self.replay_gain_mode,
            music_directory: self.music_directory.clone(),
        };
        config.save();
    }

    /// Cycle to next audio device and actually switch output
    /// Cycle to next audio device and actually switch output
    pub fn next_device(&mut self) {
        // ALWAYS refresh first to catch hotplugged devices (Earphones etc)
        self.refresh_devices();

        if !self.audio_devices.is_empty() {
            self.selected_device_idx = (self.selected_device_idx + 1) % self.audio_devices.len();
            let device_name = self.audio_devices[self.selected_device_idx].clone();
            // Actually switch the system audio output
            if audio_device::switch_audio_device(&device_name) {
                self.output_device = device_name.clone();
                self.show_toast(&format!("🎧 Device: {}", device_name));
            }
        }
    }

    /// Cycle to previous audio device and actually switch output
    pub fn prev_device(&mut self) {
        // ALWAYS refresh first to catch hotplugged devices
        self.refresh_devices();

        if !self.audio_devices.is_empty() {
            self.selected_device_idx = if self.selected_device_idx == 0 {
                self.audio_devices.len() - 1
            } else {
                self.selected_device_idx - 1
            };
            let device_name = self.audio_devices[self.selected_device_idx].clone();
            // Actually switch the system audio output
            if audio_device::switch_audio_device(&device_name) {
                self.output_device = device_name.clone();
                self.show_toast(&format!("🎧 Device: {}", device_name));
            }
        }
    }

    /// Refresh device list from system
    pub fn refresh_devices(&mut self) {
        // Use SwitchAudioSource for reliable device names
        let system_devices = audio_device::get_devices_from_system();
        let new_list = if !system_devices.is_empty() {
            system_devices
        } else {
            // Fall back to cpal
            audio_device::get_output_devices()
                .into_iter()
                .map(|d| d.name)
                .collect()
        };

        if new_list.is_empty() {
            return;
        }

        // Smart Selection Preservation 🧠
        // Find where our current output device is in the NEW list
        // This handles list reordering or insertions/deletions
        let current_name = &self.output_device;
        if let Some(idx) = new_list.iter().position(|name| name == current_name) {
            self.selected_device_idx = idx;
        } else {
            // Check if "current" is actually valid via system (maybe it was renamed or we just started)
            // But usually we just default to 0 if our current device vanished
            if self.selected_device_idx >= new_list.len() {
                self.selected_device_idx = 0;
            }
        }

        self.audio_devices = new_list;

        // Sync output_device field to match reality if we defaulted
        if self.selected_device_idx < self.audio_devices.len() {
            // Don't overwrite output_device here unless we want to "snap" to a valid one,
            // but for UI consistency it's better to show what we *think* we have configured
            // until user explicitly switches.
            // Actually, if the device DISAPPEARED, we should probably update output_device.
            if !self.audio_devices.contains(&self.output_device) {
                self.output_device = self.audio_devices[self.selected_device_idx].clone();
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Audiophile Controls
    // ═══════════════════════════════════════════════════════════════

    /// Adjust preamp (+/- 1dB)
    pub fn adjust_preamp(&mut self, delta: f32) {
        self.preamp_db = (self.preamp_db + delta).clamp(-12.0, 12.0);
        self.eq_gains.set_preamp_db(self.preamp_db);
    }

    /// Reset preamp to 0dB
    pub fn reset_preamp(&mut self) {
        self.preamp_db = 0.0;
        self.eq_gains.set_preamp_db(0.0);
    }

    /// Adjust stereo balance (+/- 0.1)
    pub fn adjust_balance(&mut self, delta: f32) {
        self.balance = (self.balance + delta).clamp(-1.0, 1.0);
        self.eq_gains.set_balance(self.balance);
    }

    /// Reset balance to center
    pub fn reset_balance(&mut self) {
        self.balance = 0.0;
        self.eq_gains.set_balance(0.0);
    }

    /// Toggle crossfade (0, 2, 4, 6 seconds)
    pub fn toggle_crossfade(&mut self) {
        self.crossfade_secs = match self.crossfade_secs {
            0 => 2,
            2 => 4,
            4 => 6,
            _ => 0,
        };
    }
}
