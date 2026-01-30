use vyom::app::{App, LibraryMode, QueueItem, TagEditState, ViewMode};
use vyom::config::AppConfig;

/// Helper to create a test app instance
fn create_test_app() -> App {
    let config = AppConfig::default();
    App::new(
        true,         // show_lyrics
        false,        // is_tmux
        true,         // is_mpd
        "Multi-Test", // source_app
        config,
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
    assert_eq!(app.queue[0].is_current, true);
}
