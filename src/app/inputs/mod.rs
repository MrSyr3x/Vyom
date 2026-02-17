use crossterm::event::KeyEvent;
use crate::app::{self, App};
use crate::app::events::AppEvent;
use crate::player::PlayerTrait;
use crate::audio::pipeline::AudioPipeline;
use crate::app::cli::Args;
use std::sync::Arc;
use tokio::sync::mpsc;
use reqwest::Client;

pub mod common;
pub mod player;
pub mod library;
pub mod eq;
pub mod lyrics;
pub mod input_box;

pub async fn handle_event(
    key: KeyEvent,
    app: &mut App,
    player: &Arc<dyn PlayerTrait>,
    audio_pipeline: &mut AudioPipeline,
    args: &Args,
    tx: &mpsc::Sender<AppEvent>,
    client: &Client,
) {
    // 1. Priority: Input Box / Tag Edit
    // These capture keys aggressively, so we check them first and return if consumed
    if input_box::handle_input_box(key, app, args, tx, client).await {
        return;
    }

    // 2. Common/Global Keys (Quit, Help, etc.)
    if common::handle_common_events(key, app, args) {
        return;
    }

    // 3. View Switchers
    // Check global view switch keys before context specific logic
    let keys = app.keys.clone(); // Clone keys to avoid borrowing app
    if keys.matches(key, &keys.view_lyrics) { app.view_mode = app::ViewMode::Lyrics; return; }
    #[cfg(feature = "mpd")]
    if keys.matches(key, &keys.view_visualizer) && !args.controller { app.view_mode = app::ViewMode::Visualizer; return; }
    #[cfg(feature = "mpd")]
    if keys.matches(key, &keys.view_library) && !args.controller { app.view_mode = app::ViewMode::Library; return; }
    #[cfg(feature = "mpd")]
    if keys.matches(key, &keys.view_eq) && !args.controller { app.view_mode = app::ViewMode::EQ; return; }

    // 4. Context Specific Handlers
    // We try specific handlers based on view mode. If they return true (consumed), we stop.
    // If not, we fall through to "Global Player Controls".
    
    let consumed = match app.view_mode {
        app::ViewMode::Library => library::handle_library_events(key, app, args),

        app::ViewMode::Lyrics => lyrics::handle_lyrics_events(key, app, player).await,
        app::ViewMode::Visualizer => false, // Visualizer has no specific controls other than global player/device
        app::ViewMode::EQ => eq::handle_eq_events(key, app, args),
    };

    if consumed {
        return;
    }

    // 5. Global Player Controls
    // These apply anywhere IF not consumed by specific view logic
    // (e.g. Space to Pause should work in Library, unless Library uses Space for selection)
    if player::handle_player_events(key, app, player, audio_pipeline, args).await {
        return;
    }
}
