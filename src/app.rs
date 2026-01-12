use crate::player::{TrackInfo, PlayerTrait};
use crate::lyrics::{LyricLine};
use std::collections::HashMap;
use std::time::Instant;

use image::DynamicImage;
use ratatui::layout::Rect;

use crate::theme::Theme;
use crate::audio_device;
use crate::dsp_eq::EqGains;



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
    Cava,
    Library,  // Renamed from Queue → Library
    EQ,
}

/// Library panel sub-mode 📚
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LibraryMode {
    #[default]
    Queue,      // Current queue
    Browse,     // Artist/Album/Genre browser
    Search,     // Search library
    Playlists,  // Saved playlists
}

/// Tag editing state 🏷️
#[derive(Debug, Clone)]
pub struct TagEditState {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub active_field: usize,  // 0=title, 1=artist, 2=album
}

/// Generic Input Popup Mode 📝
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    PlaylistSave,
    PlaylistRename,
}

/// Generic Input Popup State 📝
#[derive(Debug, Clone)]
pub struct InputState {
    pub mode: InputMode,
    pub title: String,
    pub value: String,
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
        self.active_field = if self.active_field == 0 { 2 } else { self.active_field - 1 };
    }
}

/// Library browser item type
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryItemType {
    Artist,
    Album,
    Song,
    Genre,
    Folder,
    Playlist,
}

/// Library browser item
#[derive(Debug, Clone)]
pub struct LibraryItem {
    pub name: String,
    pub item_type: LibraryItemType,
    pub artist: Option<String>,    // For songs/albums
    pub duration_ms: Option<u64>,  // For songs
    pub path: Option<String>,      // MPD file path
}

/// Queue item for MPD playlist display 📋
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub is_current: bool,
    pub file_path: String,  // For tag editing
}

pub struct App {
    pub theme: Theme,

    pub is_running: bool,
    pub track: Option<TrackInfo>,
    pub lyrics: LyricsState,       // changed from Option<Vec<LyricLine>>
    pub artwork: ArtworkState,
    // Manual Scroll State (None = Auto-sync)
    pub lyrics_offset: Option<usize>,
    pub lyrics_cache: HashMap<String, Vec<LyricLine>>,
    pub last_scroll_time: Option<Instant>,
    
    // Button Hit Areas
    // Mouse fields removed

    
    // Display Mode
    pub app_show_lyrics: bool,
    pub is_tmux: bool, // New field for layout logic
    
    /// Current panel view mode (Lyrics/Cava/Queue/EQ) 🎛️
    pub view_mode: ViewMode,
    
    /// MPD Queue (playlist) 📋
    pub queue: Vec<QueueItem>,
    pub _queue_scroll: usize,
    
    /// Smart Library Panel 📚
    pub library_mode: LibraryMode,
    pub library_items: Vec<LibraryItem>,
    pub library_selected: usize,
    pub browse_path: Vec<String>,    // Breadcrumb navigation
    pub search_query: String,
    pub search_active: bool,         // Is search input active
    pub playlists: Vec<String>,      // Available playlists
    
    /// Visualizer bars (0.0-1.0 heights) 📊
    pub visualizer_bars: Vec<f32>,
    
    /// EQ State 🎛️
    /// 10-band EQ: 32Hz, 64Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz
    /// Values: 0.0 = -12dB, 0.5 = 0dB, 1.0 = +12dB
    pub eq_bands: [f32; 10],
    pub eq_selected: usize,
    pub eq_enabled: bool,
    pub eq_preset: usize, // Index into EQ_PRESETS
    
    /// Audiophile Controls 🎚️
    pub preamp_db: f32,         // -12 to +12 dB
    pub balance: f32,           // -1.0 (L) to +1.0 (R)
    pub crossfade_secs: u32,    // MPD crossfade in seconds
    pub replay_gain_mode: u8,   // 0=Off, 1=Track, 2=Album, 3=Auto
    
    /// UI State
    pub show_keyhints: bool,    // WhichKey popup visible
    pub show_audio_info: bool,  // Audio Info popup visible (like Poweramp)
    pub tag_edit: Option<TagEditState>,
    pub input_state: Option<InputState>,  // Generic input popup
    pub toast: Option<(String, std::time::Instant)>,  // Toast notification (message, shown_at)
    pub gapless_mode: bool,     // True when current+next song are from same album
    pub last_album: String,     // Track album changes
    pub shuffle: bool,          // MPD random mode
    pub repeat: bool,           // MPD repeat mode
    
    /// Audio output devices 🔊
    pub output_device: String,
    pub audio_devices: Vec<String>,
    pub selected_device_idx: usize,
    
    /// Shared EQ gains for DSP engine
    pub eq_gains: EqGains,
    /// DSP EQ is always available (built-in)
    pub dsp_available: bool,
}

/// EQ Presets: (name, [10 band values 0.0-1.0])
/// Bands: 32Hz, 64Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz
/// Values: 0.0 = -12dB, 0.5 = 0dB (flat), 1.0 = +12dB
/// Formula: value = (dB / 24) + 0.5
/// Presets extracted from Apple Music Equalizer screenshots
pub const EQ_PRESETS: &[(&str, [f32; 10])] = &[
    // Custom - user adjustable
    ("Custom",         [0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50]),
    // Flat - neutral
    ("Flat",           [0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50]),
    // Acoustic: slight warm bass, forward mids, airy highs
    //           32:+2, 64:+1, 125:0, 250:+1, 500:+1, 1k:0, 2k:0, 4k:+1, 8k:+2, 16k:+1
    ("Acoustic",       [0.583, 0.542, 0.50, 0.542, 0.542, 0.50, 0.50, 0.542, 0.583, 0.542]),
    // Bass Booster: heavy low end boost
    //           32:+4, 64:+5, 125:+4, 250:+2, 500:0, 1k:0, 2k:0, 4k:0, 8k:0, 16k:0
    ("Bass Booster",   [0.667, 0.708, 0.667, 0.583, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50]),
    // Bass Reducer: cut low frequencies
    //           32:-4, 64:-5, 125:-4, 250:-2, 500:0, rest:0
    ("Bass Reducer",   [0.333, 0.292, 0.333, 0.417, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50]),
    // Classical: subtle, clear, natural
    //           32:0, 64:0, 125:0, 250:0, 500:0, 1k:-1, 2k:-1, 4k:-1, 8k:+1, 16k:+2
    ("Classical",      [0.50, 0.50, 0.50, 0.50, 0.50, 0.458, 0.458, 0.458, 0.542, 0.583]),
    // Dance: heavy bass, cut mids, bright highs
    //           32:+5, 64:+4, 125:+2, 250:0, 500:-2, 1k:-2, 2k:0, 4k:+2, 8k:+3, 16k:+3
    ("Dance",          [0.708, 0.667, 0.583, 0.50, 0.417, 0.417, 0.50, 0.583, 0.625, 0.625]),
    // Deep: sub-bass emphasis, very dark
    //           32:+5, 64:+4, 125:+2, 250:0, 500:0, 1k:0, 2k:-1, 4k:-2, 8k:-3, 16k:-4
    ("Deep",           [0.708, 0.667, 0.583, 0.50, 0.50, 0.50, 0.458, 0.417, 0.375, 0.333]),
    // Electronic: V-curve, bass + treble
    //           32:+5, 64:+4, 125:0, 250:-1, 500:-1, 1k:0, 2k:0, 4k:+2, 8k:+4, 16k:+5
    ("Electronic",     [0.708, 0.667, 0.50, 0.458, 0.458, 0.50, 0.50, 0.583, 0.667, 0.708]),
    // Hip-Hop: heavy bass, clear mids, crisp highs
    //           32:+5, 64:+4, 125:+2, 250:0, 500:0, 1k:+1, 2k:+1, 4k:0, 8k:+1, 16k:+2
    ("Hip-Hop",        [0.708, 0.667, 0.583, 0.50, 0.50, 0.542, 0.542, 0.50, 0.542, 0.583]),
    // Jazz: warm, smooth, detailed
    //           32:+2, 64:+1, 125:0, 250:+1, 500:+2, 1k:+2, 2k:0, 4k:+1, 8k:+2, 16k:+1
    ("Jazz",           [0.583, 0.542, 0.50, 0.542, 0.583, 0.583, 0.50, 0.542, 0.583, 0.542]),
    // Late Night: compressed, quieter dynamics
    //           32:-1, 64:0, 125:+1, 250:+2, 500:+2, 1k:+2, 2k:+1, 4k:0, 8k:-1, 16k:-1
    ("Late Night",     [0.458, 0.50, 0.542, 0.583, 0.583, 0.583, 0.542, 0.50, 0.458, 0.458]),
    // Latin: punchy, rhythmic, bright
    //           32:+2, 64:+1, 125:+1, 250:+1, 500:0, 1k:0, 2k:0, 4k:+1, 8k:+3, 16k:+3
    ("Latin",          [0.583, 0.542, 0.542, 0.542, 0.50, 0.50, 0.50, 0.542, 0.625, 0.625]),
    // Loudness: classic loudness curve (bass + treble boost)
    //           32:+4, 64:+2, 125:0, 250:0, 500:0, 1k:0, 2k:0, 4k:0, 8k:+2, 16k:+4
    ("Loudness",       [0.667, 0.583, 0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.583, 0.667]),
    // Lounge: mellow, relaxed
    //           32:-2, 64:0, 125:+1, 250:+2, 500:+2, 1k:+1, 2k:0, 4k:-1, 8k:0, 16k:0
    ("Lounge",         [0.417, 0.50, 0.542, 0.583, 0.583, 0.542, 0.50, 0.458, 0.50, 0.50]),
    // Piano: clear mids, natural bass
    //           32:+1, 64:0, 125:0, 250:+1, 500:+2, 1k:+2, 2k:+1, 4k:+1, 8k:+2, 16k:+1
    ("Piano",          [0.542, 0.50, 0.50, 0.542, 0.583, 0.583, 0.542, 0.542, 0.583, 0.542]),
    // Pop: vocals forward, punchy
    //           32:-2, 64:0, 125:+1, 250:+3, 500:+4, 1k:+3, 2k:+1, 4k:0, 8k:+1, 16k:+1
    ("Pop",            [0.417, 0.50, 0.542, 0.625, 0.667, 0.625, 0.542, 0.50, 0.542, 0.542]),
    // R&B: warm bass, silky vocals
    //           32:+3, 64:+2, 125:+1, 250:0, 500:+1, 1k:+2, 2k:+2, 4k:+1, 8k:+1, 16k:+2
    ("R&B",            [0.625, 0.583, 0.542, 0.50, 0.542, 0.583, 0.583, 0.542, 0.542, 0.583]),
    // Rock: guitar-focused, powerful
    //           32:+3, 64:+2, 125:0, 250:-1, 500:0, 1k:+2, 2k:+3, 4k:+3, 8k:+3, 16k:+2
    ("Rock",           [0.625, 0.583, 0.50, 0.458, 0.50, 0.583, 0.625, 0.625, 0.625, 0.583]),
    // Small Speakers: bass compensation
    //           32:+5, 64:+4, 125:+3, 250:+1, 500:0, 1k:0, 2k:+1, 4k:+3, 8k:+4, 16k:+5
    ("Small Speakers", [0.708, 0.667, 0.625, 0.542, 0.50, 0.50, 0.542, 0.625, 0.667, 0.708]),
    // Spoken Word: voice clarity
    //           32:-3, 64:-1, 125:+1, 250:+4, 500:+5, 1k:+4, 2k:+2, 4k:0, 8k:-2, 16k:-3
    ("Spoken Word",    [0.375, 0.458, 0.542, 0.667, 0.708, 0.667, 0.583, 0.50, 0.417, 0.375]),
    // Treble Booster: high frequency emphasis
    //           32:0, 64:0, 125:0, 250:0, 500:0, 1k:0, 2k:+1, 4k:+3, 8k:+4, 16k:+4
    ("Treble Booster", [0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.542, 0.625, 0.667, 0.667]),
    // Treble Reducer: high frequency cut
    //           32:0, 64:0, 125:0, 250:0, 500:0, 1k:0, 2k:-1, 4k:-3, 8k:-4, 16k:-4
    ("Treble Reducer", [0.50, 0.50, 0.50, 0.50, 0.50, 0.50, 0.458, 0.375, 0.333, 0.333]),
    // Vocal Booster: mid boost for voice clarity
    //           32:-3, 64:-2, 125:0, 250:+3, 500:+5, 1k:+5, 2k:+3, 4k:+1, 8k:-1, 16k:-3
    ("Vocal Booster",  [0.375, 0.417, 0.50, 0.625, 0.708, 0.708, 0.625, 0.542, 0.458, 0.375]),
];



impl App {
    pub fn new(app_show_lyrics: bool, is_tmux: bool) -> Self {
        let theme = crate::theme::load_current_theme();
        let eq_gains = EqGains::new();
        
        Self {
            theme,
            is_running: true,
            track: None,
            lyrics: LyricsState::Idle, // changed
            artwork: ArtworkState::Idle,

            lyrics_offset: None,
            lyrics_cache: HashMap::new(),
            last_scroll_time: None,
            app_show_lyrics,
            is_tmux,
            view_mode: ViewMode::default(),
            queue: Vec::new(),
            _queue_scroll: 0,
            library_mode: LibraryMode::default(),
            library_items: Vec::new(),
            library_selected: 0,
            browse_path: Vec::new(),
            search_query: String::new(),
            search_active: false,
            playlists: Vec::new(),
            visualizer_bars: vec![0.3; 32], // Start with 32 bars at 30% height
            eq_bands: [0.5; 10], // All bands at 0dB (centered)
            eq_selected: 0,     // First band selected
            eq_enabled: true,   // EQ enabled by default
            eq_preset: 0,       // Start with "Custom" preset
            preamp_db: 0.0,         // No preamp adjustment
            balance: 0.0,           // Center
            crossfade_secs: 0,      // No crossfade
            replay_gain_mode: 0,    // Off by default
            show_keyhints: false,   // Hidden by default
            show_audio_info: false, // Hidden by default
            tag_edit: None,
            input_state: None,         // No input popup active
            toast: None,               // No toast notification
            gapless_mode: false,    // No gapless detected initially
            last_album: String::new(),
            shuffle: false,         // Will be updated from MPD
            repeat: false,          // Will be updated from MPD
            output_device: audio_device::get_output_device_name(),
            audio_devices: {
                let sys_devices = audio_device::get_devices_from_system();
                if !sys_devices.is_empty() {
                    sys_devices
                } else {
                    audio_device::get_output_devices().into_iter().map(|d| d.name).collect()
                }
            },
            selected_device_idx: 0,
            eq_gains,
            dsp_available: true, // Built-in DSP is always available
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
            self.eq_gains.set_gain_from_value(band_index, self.eq_bands[band_index]);
        }
    }
    
    /// Reset EQ to flat
    pub fn reset_eq(&mut self) {
        self.eq_bands = [0.5; 10];
        self.eq_preset = 1; // Set to "Flat" preset
        self.eq_gains.reset();
    }
    
    /// Toggle EQ enabled state
    pub fn toggle_eq(&mut self) {
        self.eq_enabled = !self.eq_enabled;
        self.eq_gains.set_enabled(self.eq_enabled);
    }
    
    /// Apply current preset to EQ bands
    pub fn apply_preset(&mut self) {
        if self.eq_preset < EQ_PRESETS.len() {
            self.eq_bands = EQ_PRESETS[self.eq_preset].1;
            self.sync_eq_to_dsp();
        }
    }
    
    /// Cycle to next preset
    pub fn next_preset(&mut self) {
        self.eq_preset = (self.eq_preset + 1) % EQ_PRESETS.len();
        self.apply_preset();
    }
    
    /// Cycle to previous preset
    pub fn prev_preset(&mut self) {
        self.eq_preset = if self.eq_preset == 0 { 
            EQ_PRESETS.len() - 1 
        } else { 
            self.eq_preset - 1 
        };
        self.apply_preset();
    }
    
    /// Get current preset name
    pub fn get_preset_name(&self) -> &'static str {
        if self.eq_preset < EQ_PRESETS.len() {
            EQ_PRESETS[self.eq_preset].0
        } else {
            "Custom"
        }
    }
    
    /// Mark as custom preset when user manually adjusts bands
    pub fn mark_custom(&mut self) {
        self.eq_preset = 0; // "Custom"
    }
    
    /// Cycle to next audio device and actually switch output
    pub fn next_device(&mut self) {
        if !self.audio_devices.is_empty() {
            self.selected_device_idx = (self.selected_device_idx + 1) % self.audio_devices.len();
            let device_name = self.audio_devices[self.selected_device_idx].clone();
            // Actually switch the system audio output
            if audio_device::switch_audio_device(&device_name) {
                self.output_device = device_name;
            }
        }
    }
    
    /// Cycle to previous audio device and actually switch output
    pub fn prev_device(&mut self) {
        if !self.audio_devices.is_empty() {
            self.selected_device_idx = if self.selected_device_idx == 0 {
                self.audio_devices.len() - 1
            } else {
                self.selected_device_idx - 1
            };
            let device_name = self.audio_devices[self.selected_device_idx].clone();
            // Actually switch the system audio output
            if audio_device::switch_audio_device(&device_name) {
                self.output_device = device_name;
            }
        }
    }
    
    /// Refresh device list from system
    pub fn refresh_devices(&mut self) {
        // Use SwitchAudioSource for reliable device names
        let system_devices = audio_device::get_devices_from_system();
        if !system_devices.is_empty() {
            self.audio_devices = system_devices;
        } else {
            // Fall back to cpal
            self.audio_devices = audio_device::get_output_devices().into_iter().map(|d| d.name).collect();
        }
        // Keep current selection if still valid
        if self.selected_device_idx >= self.audio_devices.len() {
            self.selected_device_idx = 0;
        }
        if !self.audio_devices.is_empty() {
            self.output_device = self.audio_devices[self.selected_device_idx].clone();
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
