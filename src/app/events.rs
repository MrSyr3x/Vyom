use crate::app::{ArtworkState, LyricsState};
use crate::player::TrackInfo;
use crate::ui::theme::Theme;
use crossterm::event::Event;

pub enum AppEvent {
    Input(Event),
    TrackUpdate(Option<TrackInfo>),
    LyricsUpdate(String, LyricsState),
    ArtworkUpdate(String, ArtworkState),
    ThemeUpdate(Theme),
    KeyConfigUpdate(Box<crate::app::keys::KeyConfig>),
    QueueUpdate(Vec<(String, String, u64, bool, String)>),
    StatusUpdate(bool, crate::player::RepeatMode),
    Tick,
}
