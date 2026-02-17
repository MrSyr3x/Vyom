use crossterm::event::{KeyCode, KeyEvent};
use crate::app::App;

pub fn handle_common_events(
    key: KeyEvent,
    app: &mut App,
    args: &crate::app::cli::Args,
) -> bool {
    let keys = &app.keys;

    // Quit ('q')
    if keys.matches(key, &keys.quit) {
        // Close popups first, then quit (Neovim-style)
        if app.show_keyhints {
            app.show_keyhints = false;
        } else if app.show_audio_info {
            app.show_audio_info = false;
        } else {
            app.is_running = false;
        }
        return true;
    }

    if keys.matches(key, &keys.toggle_keyhints) {
        app.show_keyhints = !app.show_keyhints;
        return true;
    }
    
    if keys.matches(key, &keys.toggle_audio_info) {
        app.show_audio_info = !app.show_audio_info;
        return true;
    }
    
    // Global Popup Close (Esc)
    if (keys.matches(key, &keys.back_dir_alt) || key.code == KeyCode::Esc) && (app.show_keyhints || app.show_audio_info) {
        if app.show_keyhints { app.show_keyhints = false; }
        if app.show_audio_info { app.show_audio_info = false; }
        return true;
    }

    // Toggle Search (/) - Global Context -> Switch to Library and Focus Search
    #[cfg(feature = "mpd")]
    if key.code == KeyCode::Char('/') && !args.controller {
        app.view_mode = crate::app::ViewMode::Library;
        // Save current mode only if we are NOT already in Search mode
        if app.library_mode != crate::app::LibraryMode::Search {
            app.previous_library_mode = Some(app.library_mode);
        }
        app.library_mode = crate::app::LibraryMode::Search;
        app.search_active = true;
        // Critical: Clear items so we don't see previous Directory contents
        app.library_items.clear();
        app.library_selected = 0;
        return true;
    }

    false
}
