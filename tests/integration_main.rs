use vyom::app::config::UserConfig;
use vyom::app::{App, LibraryMode, QueueItem, TagEditState, ViewMode};

/// Helper to create a test app instance
fn create_test_app() -> App {
    let config = UserConfig::default();
    App::new(
        true,         // show_lyrics
        false,        // is_tmux
        true,         // is_mpd
        "Multi-Test", // source_app
        true,         // is_test
        config,
        vyom::app::config::PersistentState::default(),
    )
}

#[test]
fn test_app_initialization() {
    let app = create_test_app();
    assert!(app.is_running);
    assert_eq!(app.view_mode, ViewMode::default());
    assert_eq!(app.library_mode, LibraryMode::default());
    assert!(app.queue.is_empty());
}

#[test]
fn test_navigation_state() {
    let mut app = create_test_app();

    // Simulate navigating into "Rock" folder
    app.browse_path.push("Rock".to_string());
    assert_eq!(app.browse_path.len(), 1);
    assert_eq!(app.browse_path[0], "Rock");

    // Navigate deeper
    app.browse_path.push("Classic".to_string());
    assert_eq!(app.browse_path.len(), 2);

    // Simulate "Back" action
    app.browse_path.pop();
    assert_eq!(app.browse_path.len(), 1);
    assert_eq!(app.browse_path[0], "Rock");
}

#[test]
fn test_tag_edit_logic() {
    // Test the logic extracted in TagEditState
    let mut tag_state = TagEditState::new("/tmp/song.mp3", "Title", "Artist", "Album");

    // Initial state: Field 0 (Title)
    assert_eq!(tag_state.active_field, 0);

    // Next field -> Artist
    tag_state.next_field();
    assert_eq!(tag_state.active_field, 1);

    // Next field -> Album
    tag_state.next_field();
    assert_eq!(tag_state.active_field, 2);

    // Loop back -> Title
    tag_state.next_field();
    assert_eq!(tag_state.active_field, 0);

    // Prev field -> Album (Loop back)
    tag_state.prev_field();
    assert_eq!(tag_state.active_field, 2);
}

#[test]
fn test_search_query_state() {
    let mut app = create_test_app();

    app.library_mode = LibraryMode::Search;
    app.search_query = "Pink Floyd".to_string();
    app.search_active = true;

    assert_eq!(app.library_mode, LibraryMode::Search);
    assert_eq!(app.search_query, "Pink Floyd");
    assert!(app.search_active);

    // Simulate clearing search
    app.search_query.clear();
    app.search_active = false;
    assert!(app.search_query.is_empty());
}

#[test]
fn test_queue_manipulation() {
    let mut app = create_test_app();

    let item = QueueItem {
        title: "Song A".to_string(),
        artist: "Artist A".to_string(),
        duration_ms: 1000,
        is_current: true,
        file_path: "song_a.mp3".to_string(),
    };

    app.queue.push(item);

    assert_eq!(app.queue.len(), 1);
    assert_eq!(app.queue[0].title, "Song A");
    assert!(app.queue[0].is_current);
}

#[test]
fn test_toast_creation() {
    let mut app = create_test_app();
    assert!(app.toast.is_none());

    app.show_toast("Hello!");
    assert!(app.toast.is_some());
    assert_eq!(app.toast.as_ref().unwrap().message, "Hello!");
}

#[test]
fn test_toast_stacking_updates_message() {
    let mut app = create_test_app();
    app.show_toast("First");
    let start_time = app.toast.as_ref().unwrap().start_time;

    app.show_toast("Second");
    // Message should be updated
    assert_eq!(app.toast.as_ref().unwrap().message, "Second");
    // start_time should be preserved (no re-animation)
    assert_eq!(app.toast.as_ref().unwrap().start_time, start_time);
}

#[test]
fn test_toast_expiry_on_tick() {
    let mut app = create_test_app();
    app.show_toast("Expiring");
    assert!(app.toast.is_some());

    // Manually set deadline to the past to simulate expiry
    if let Some(ref mut toast) = app.toast {
        toast.deadline = std::time::Instant::now() - std::time::Duration::from_millis(1);
    }

    app.on_tick();
    assert!(
        app.toast.is_none(),
        "Toast should be cleared after deadline"
    );
}

#[test]
fn test_toast_not_expired_during_active() {
    let mut app = create_test_app();
    app.show_toast("Still alive");

    // Don't modify deadline — it should be 2s in the future
    app.on_tick();
    assert!(app.toast.is_some(), "Toast should still be visible");
}
