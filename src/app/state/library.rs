/// Library panel sub-mode 📚
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LibraryMode {
    #[default]
    Queue, // Current queue
    Directory, // Neo-tree style music folder browser
    Search,    // Search library
    Playlists, // Saved playlists
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
