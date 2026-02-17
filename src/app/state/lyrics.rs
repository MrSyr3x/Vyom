use crate::app::lyrics::LyricLine;

#[derive(Debug, Clone, PartialEq)]
pub enum LyricsState {
    Idle,
    Loading,
    Loaded(Vec<LyricLine>, String),
    Instrumental,
    Failed(String),
    NotFound,
}
