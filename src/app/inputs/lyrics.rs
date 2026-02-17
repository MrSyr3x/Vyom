use crossterm::event::KeyEvent;
use crate::app::{self, App, LyricsState};
use crate::player::PlayerTrait;
use std::sync::Arc;

pub async fn handle_lyrics_events(
    key: KeyEvent,
    app: &mut App,
    player: &Arc<dyn PlayerTrait>,
) -> bool {
    let keys = &app.keys;

    if app.view_mode != app::ViewMode::Lyrics {
        return false;
    }

    if keys.matches(key, &keys.nav_down) || keys.matches(key, &keys.nav_down_alt) {
        if let LyricsState::Loaded(ref lines, _) = &app.lyrics {
            let max = lines.len().saturating_sub(1);
                let current_playing = {
                let track_ms = app.track.as_ref().map(|t| t.position_ms).unwrap_or(0);
                lines.iter()
                    .position(|l| l.timestamp_ms > track_ms)
                    .map(|i| if i > 0 { i - 1 } else { 0 })
                    .unwrap_or(max)
            };
            let current = app.lyrics_selected.unwrap_or(current_playing);
            let new_sel = (current + 1).min(max);
            app.lyrics_selected = Some(new_sel);
            app.lyrics_offset = Some(new_sel);
            app.last_scroll_time = Some(std::time::Instant::now());
        }
        return true;
    }

    if keys.matches(key, &keys.nav_up) || keys.matches(key, &keys.nav_up_alt) {
        if let LyricsState::Loaded(ref lines, _) = &app.lyrics {
            let max = lines.len().saturating_sub(1);
                let current_playing = {
                let track_ms = app.track.as_ref().map(|t| t.position_ms).unwrap_or(0);
                lines.iter()
                    .position(|l| l.timestamp_ms > track_ms)
                    .map(|i| if i > 0 { i - 1 } else { 0 })
                    .unwrap_or(max)
            };
            let current = app.lyrics_selected.unwrap_or(current_playing);
            let new_sel = current.saturating_sub(1);
            app.lyrics_selected = Some(new_sel);
            app.lyrics_offset = Some(new_sel);
            app.last_scroll_time = Some(std::time::Instant::now());
        }
        return true;
    }

    if keys.matches(key, &keys.seek_to_line) {
        if let LyricsState::Loaded(ref lines, _) = &app.lyrics {
            if let Some(idx) = app.lyrics_selected {
                if idx < lines.len() {
                    let target_ms = lines[idx].timestamp_ms;
                    let target_secs = target_ms as f64 / 1000.0;
                    let player_bg = player.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = player_bg.seek(target_secs);
                    });
                    let mins = target_ms / 60000;
                    let secs = (target_ms % 60000) / 1000;
                    app.show_toast(&format!("🎤 Jump to {}:{:02}", mins, secs));
                    app.lyrics_selected = None;
                    app.lyrics_offset = None;
                    app.last_scroll_time = None;
                }
            }
        }
        return true;
    }

    false
}
