use crate::spotify::{TrackInfo};
use crate::lyrics::{LyricLine};
use crate::artwork::ArtworkData;
use ratatui::layout::Rect;

use crate::theme::Theme;

pub struct App {
    pub theme: Theme,
    pub is_running: bool,
    pub track: Option<TrackInfo>,
    pub lyrics: Option<Vec<LyricLine>>,
    pub artwork: Option<ArtworkData>,
    // Button Hit Areas
    pub prev_btn: Rect,
    pub play_btn: Rect,
    pub next_btn: Rect,
    pub progress_rect: Rect,
    // (Rect, Timestamp in ms)
    pub lyrics_hitboxes: Vec<(Rect, u64)>,
    
    // Manual Scroll State (None = Auto-sync)
    pub lyrics_offset: Option<usize>,
    
    // Display Mode
    pub show_lyrics: bool,

}



impl App {
    pub fn new(show_lyrics: bool) -> Self {
        let theme = crate::theme::load_current_theme();
        
        Self {
            theme,
            is_running: true,
            track: None,
            lyrics: None,
            artwork: None,
            prev_btn: Rect::default(),
            play_btn: Rect::default(),
            next_btn: Rect::default(),
            progress_rect: Rect::default(),
            lyrics_hitboxes: Vec::new(),
            lyrics_offset: None,
            show_lyrics,
        }
    }

    pub fn handle_click(&mut self, x: u16, y: u16) {
        if self.prev_btn.contains((x, y).into()) {
            let _ = crate::spotify::Spotify::prev();
        } else if self.play_btn.contains((x, y).into()) {
            let _ = crate::spotify::Spotify::play_pause();
        } else if self.next_btn.contains((x, y).into()) {
             let _ = crate::spotify::Spotify::next();
        } else if self.progress_rect.contains((x, y).into()) {
            if let Some(track) = &self.track {
                if track.duration_ms > 0 {
                     let relative_x = x.saturating_sub(self.progress_rect.x);
                     let width = self.progress_rect.width.max(1);
                     let percent = relative_x as f64 / width as f64;
                     let target_sec = (track.duration_ms as f64 / 1000.0) * percent;
                     let _ = crate::spotify::Spotify::seek(target_sec);
                }
            }
        }
    }
}
