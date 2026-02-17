use crossterm::event::Event;
use crate::app::{ArtworkState, LyricsState};
use crate::player::TrackInfo;
use crate::ui::theme::Theme;


pub enum AppEvent {
    Input(Event),
    TrackUpdate(Option<TrackInfo>),
    LyricsUpdate(String, LyricsState),
    ArtworkUpdate(ArtworkState),
    ThemeUpdate(Theme),
    KeyConfigUpdate(Box<crate::app::keys::KeyConfig>),
    QueueUpdate(Vec<(String, String, u64, bool, String)>),
    StatusUpdate(bool, crate::player::RepeatMode),
    Tick,
}
