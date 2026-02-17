use std::time::Instant;

/// View mode for the right panel 🎛️
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Lyrics,
    Visualizer,
    Library, // Renamed from Queue → Library
    EQ,
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

impl InputState {
    pub fn new(mode: InputMode, title: &str, initial_value: &str) -> Self {
        Self {
            mode,
            title: title.to_string(),
            value: initial_value.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub start_time: Instant,
    pub deadline: Instant,
}
