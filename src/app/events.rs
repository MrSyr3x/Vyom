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
    QueueUpdate(Vec<(String, String, u64, bool, String)>),
    Tick,
}
