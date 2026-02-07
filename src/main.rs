// mod config; // using vyom::config

use anyhow::Result;
use crossterm::{
    event::{Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use vyom::app::config::AppConfig;


// Modules now in lib.rs
use vyom::app;
use vyom::artwork;
use vyom::audio::pipeline as audio_pipeline;

use vyom::player;
use vyom::ui::theme;
use vyom::ui;
use vyom::inputs::handle_input;


#[cfg(feature = "mpd")]
use vyom::mpd_player;



use clap::Parser;
use app::cli::Args;
use app::events::AppEvent;

use app::{ArtworkState, LyricsState};
use artwork::ArtworkRenderer;
use vyom::app::lyrics::LyricsFetcher;








#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let is_standalone = args.standalone;
    let is_tmux = std::env::var("TMUX").is_ok();

    // Smart Window Logic
    // Default is Full UI (!mini).
    let want_lyrics = !args.mini;

    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.to_str().unwrap();

    // 1. WINDOW TITLE (For Yabai/Amethyst) 🏷️
    print!("\x1b]2;Vyom\x07");

    // 2. TMUX LOGIC
    if app::tmux::handle_tmux_split(&args, exe_path, is_tmux, is_standalone, want_lyrics)? {
        return Ok(());
    }
    // No else block for Standalone Resize - User manages window size manually.

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // In Tmux, we assume full split/window, so show lyrics by default.
    // In Standalone, strict mode applies.
    let app_show_lyrics = want_lyrics || is_tmux;

    // Determine backend mode and source app name
    #[cfg(feature = "mpd")]
    let (is_mpd_mode, source_app) = if args.controller {
        // Auto-Pause MPD to prevent concurrent audio ⏸️
        if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
            let _ = mpd.pause(true);
        }
        (false, "Spotify / Apple Music")
    } else {
        // Default mode - MPD
        (true, "MPD")
    };
    #[cfg(not(feature = "mpd"))]
    let (is_mpd_mode, source_app) = (false, "Spotify / Apple Music");

    // 1. Initial State
    // Start Audio Pipeline 🔊 (FIFO → DSP EQ → Speakers)
    // SINGLETON CHECK: Only start audio if we acquire the lock
    let audio_lock = app::lock::try_acquire_audio_lock();
    let is_audio_master = audio_lock.is_some();

    // Load persisted state (Split into UserConfig and PersistentState)
    let (user_config, persistent_state) = AppConfig::load();
    
    if persistent_state.eq_enabled && !is_audio_master {
        // Maybe log that EQ is visual only?
    }

    let mut app = app::App::new(app_show_lyrics, is_tmux, is_mpd_mode, source_app, user_config, persistent_state);

    let mut audio_pipeline = audio_pipeline::AudioPipeline::new(app.eq_gains.clone());

    // Attach Visualizer 📊
    audio_pipeline.attach_visualizer(app.visualizer.get_audio_buffer());

    if is_audio_master {
        if let Err(e) = audio_pipeline.start() {
            let msg = format!("Audio Error: {} (Visuals Only)", e);
            eprintln!("{}", msg); // Keep log for stderr
            app.show_toast(&msg);
        }
    } else {
        // We are secondary. No audio output.
        // Visuals might still work if we tap into the same FIFO?
        // For now, secondary = no audio processing = no visualizer (unless we share data, which is complex).
        app.show_toast("🔇 Shared Audio Mode (UI Only)");
    }

    // Player Backend Selection 🎛️
    #[cfg(feature = "mpd")]
    let player: std::sync::Arc<dyn player::PlayerTrait> = if !args.controller {
        std::sync::Arc::new(mpd_player::MpdPlayer::new(&args.mpd_host, args.mpd_port))
    } else {
        std::sync::Arc::from(player::get_player())
    };

    #[cfg(not(feature = "mpd"))]
    let player: std::sync::Arc<dyn player::PlayerTrait> =
        std::sync::Arc::from(player::get_player());

    let (tx, mut rx) = mpsc::channel(100);

    // Performance Optimization: Global HTTP Client (Reused)
    let client = reqwest::Client::builder()
        .user_agent("vyom-rs/1.0.1")
        .build()
        .unwrap_or_default();

    // 1. Input Event Task
    let tx_input = tx.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if tx_input.send(AppEvent::Input(event)).await.is_err() {
                break;
            }
        }
    });

    // 2. Track Polling Task
    let tx_spotify = tx.clone();
    let player_poll = player.clone();
    tokio::spawn(async move {
        loop {
            // Use shared player reference
            let player_ref = player_poll.clone();
            let track_result = tokio::task::spawn_blocking(move || {
                let track = player_ref.get_current_track();
                let queue = player_ref.get_queue();
                (track, queue)
            })
            .await;

            if let Ok((Ok(info), Ok(queue))) = track_result {
                let _ = tx_spotify.send(AppEvent::TrackUpdate(info)).await;
                let _ = tx_spotify.send(AppEvent::QueueUpdate(queue)).await;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });

    // 3. Theme Watcher Task 🎨
    let tx_theme = tx.clone();
    tokio::spawn(async move {
        // Use file modification time for efficient hot reloading 🌶️
        let theme_path = theme::get_theme_path();
        let path_clone = theme_path.clone();
        
        let mut last_modified = tokio::task::spawn_blocking(move || {
            std::fs::metadata(&path_clone)
                .and_then(|m| m.modified())
                .ok()
        }).await.unwrap_or(None);

        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;

            // Check file modification time (blocking I/O wrapped)
            let check_path = theme_path.clone(); // Clone for closure
            let metadata_result = tokio::task::spawn_blocking(move || {
                std::fs::metadata(&check_path).and_then(|m| m.modified())
            }).await;

            if let Ok(Ok(modified)) = metadata_result {
                // If file modified or we never saw it before (and it exists)
                if last_modified.is_none_or(|last| modified > last) {
                    last_modified = Some(modified);

                    // Load and broadcast new theme
                    let new_theme = theme::load_current_theme();
                    if tx_theme
                        .send(AppEvent::ThemeUpdate(new_theme))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    // 4. Config Watcher Task (Hot Reloading) 🔧
    let tx_config = tx.clone();
    tokio::spawn(async move {
        let config_path = AppConfig::get_config_path();
        let path_clone = config_path.clone();
        
        let mut last_modified = tokio::task::spawn_blocking(move || {
            std::fs::metadata(&path_clone)
                .and_then(|m| m.modified())
                .ok()
        }).await.unwrap_or(None);

        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;

            let check_path = config_path.clone();
            let metadata_result = tokio::task::spawn_blocking(move || {
                std::fs::metadata(&check_path).and_then(|m| m.modified())
            }).await;

            if let Ok(Ok(modified)) = metadata_result {
                if last_modified.is_none_or(|last| modified > last) {
                    last_modified = Some(modified);
                    
                    // Reload config to get new keys
                    // Hot Reload: Reload config, verify keys changed
                    let (new_user_config, _) = AppConfig::load(); 
                    if tx_config
                        .send(AppEvent::KeyConfigUpdate(Box::new(new_user_config.keys)))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    // 5. Animation Tick Task ⚡
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        // 60 FPS Update Rate (approx 16ms) - Back to smooth fluidity
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        loop {
            interval.tick().await;
            if tx_tick.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    let mut last_track_id = String::new();
    let mut last_artwork_url = None;

    loop {
        // Auto-Reset Lyrics Scroll Logic
        if let Some(t) = app.last_scroll_time {
            if t.elapsed().as_secs() >= 3 {
                // Time up! removing "manual mode" flag to let Tick animation take over
                app.last_scroll_time = None;
            }
        }

        // Update visualizer bars 60fps (called before draw)
        if app.view_mode == app::ViewMode::Visualizer {
            // We request 64 bars which is the max rendering width we limited to
            app.visualizer_bars = app.visualizer.get_bars(64);
        }

        terminal.draw(|f| ui::ui(f, &mut app))?;

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                app.is_running = false;
            }
            Some(event) = rx.recv() => {
                match event {
                // ... (Input handling omitted)
                // Mouse Interaction Removed as per User Request
                AppEvent::Input(Event::Mouse(_)) => {},
                AppEvent::Input(Event::Key(key)) => {
                    handle_input(key, &mut app, &player, &mut audio_pipeline, &args, &tx, &client).await;
                },
                AppEvent::Input(_) => {},

                AppEvent::TrackUpdate(info) => {
                    app.track = info.clone();
                    app.last_track_update = Some(std::time::Instant::now());
                    if let Some(track) = info {
                        let id = format!("{}{}", track.name, track.artist);

                        // Gapless album detection: check if same album as previous track
                        if !track.album.is_empty() && !app.last_album.is_empty() {
                            app.gapless_mode = track.album == app.last_album;
                        } else {
                            app.gapless_mode = false;
                        }
                        app.last_album = track.album.clone();

                        if id != last_track_id {
                            last_track_id = id.clone();
                            // Critical: Set Loading state immediately
                            app.lyrics = LyricsState::Loading;
                            // Critical Fix: Reset manual scroll state on song change
                            app.lyrics_offset = None;
                            app.last_scroll_time = None;
                            // Critical Fix: Reset seek state on song change to prevent seeks carrying over
                            app.seek_accumulator = 0.0;
                            app.seek_initial_pos = None;
                            app.last_seek_time = None;

                            // 1. Check Cache
                            if let Some(cached) = app.lyrics_cache.get(&id) {
                                app.lyrics = LyricsState::Loaded(cached.clone());
                            } else {
                                // 2. If not in cache, fetch
                                let tx_lyrics = tx.clone();
                                let (artist, name, dur) = (track.artist.clone(), track.name.clone(), track.duration_ms);
                                let fetch_id = id.clone();
                                let file_path = track.file_path.clone();

                                let client = client.clone();
                                tokio::spawn(async move {
                                    let fetcher = LyricsFetcher::new(client);
                                    use vyom::app::lyrics::LyricsFetchResult;
                                    match fetcher.fetch(&artist, &name, dur, file_path.as_ref()).await {
                                        Ok(LyricsFetchResult::Found(lyrics)) => {
                                            let _ = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::Loaded(lyrics))).await;
                                        },
                                        Ok(LyricsFetchResult::Instrumental) => {
                                             let _ = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::Instrumental)).await;
                                        },
                                        Ok(LyricsFetchResult::None) => {
                                             let _ = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::NotFound)).await;
                                        }
                                        Err(e) => {
                                            // Send Error State
                                            let _ = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::Failed(e.to_string()))).await;
                                        }
                                    }
                                });
                            }

                            // 2. Artwork Logic (Once per song checks - Apple Music Fallback)
                            if track.source == "Music" && track.artwork_url.is_none() {
                                app.artwork = ArtworkState::Loading;
                                let tx_art = tx.clone();
                                let (artist, album) = (track.artist.clone(), track.album.clone());
                                let client = client.clone();
                                tokio::spawn(async move {
                                    let renderer = ArtworkRenderer::new(client);
                                    match renderer.fetch_itunes_artwork(&artist, &album).await {
                                        Ok(url) => {
                                             match renderer.fetch_image(&url).await {
                                                 Ok(img) => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Loaded(img))).await; },
                                                 Err(_) => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Failed)).await; }
                                             }
                                        },
                                        Err(_) => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Failed)).await; }
                                    }
                                });
                            }

                            // 3. MPD Artwork: Extract embedded art from audio file 🎵
                            #[cfg(feature = "mpd")]
                            if track.source == "MPD" {
                                if let Some(file_path) = &track.file_path {
                                    app.artwork = ArtworkState::Loading;
                                    let tx_art = tx.clone();
                                    let fp = file_path.clone();
                                    tokio::spawn(async move {
                                        let result = tokio::task::spawn_blocking(move || {
                                            ArtworkRenderer::extract_embedded_art(&fp)
                                        }).await;

                                        match result {
                                            Ok(Ok(img)) => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Loaded(img))).await; },
                                            _ => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Failed)).await; }
                                        }
                                    });
                                }
                            }
                        }

                        // Standard URL-based update (Spotify or when Music eventually resolves one)
                        if let Some(url) = track.artwork_url.clone() {
                            if Some(url.clone()) != last_artwork_url {
                                last_artwork_url = Some(url.clone());
                                app.artwork = ArtworkState::Loading;
                                let tx_art = tx.clone();
                                let client = client.clone();
                                tokio::spawn(async move {
                                    let renderer = ArtworkRenderer::new(client);
                                    match renderer.fetch_image(&url).await {
                                         Ok(img) => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Loaded(img))).await; },
                                         Err(_) => { let _ = tx_art.send(AppEvent::ArtworkUpdate(ArtworkState::Failed)).await; }
                                    }
                                });
                            }
                        }
                    } else {
                        last_track_id.clear();
                        last_artwork_url = None;
                        app.artwork = ArtworkState::Idle;
                    }
                },
                AppEvent::LyricsUpdate(id, state) => {
                    // Update cache if loaded
                    if let LyricsState::Loaded(ref l) = state {
                         if app.lyrics_cache.len() > 50 {
                             app.lyrics_cache.clear();
                         }
                         app.lyrics_cache.insert(id.clone(), l.clone());
                    }

                    // Only update UI if we are still on the same song
                    if id == last_track_id {
                         app.lyrics = state;
                    }
                },
                AppEvent::ArtworkUpdate(data) => app.artwork = data,
                AppEvent::ThemeUpdate(new_theme) => app.theme = new_theme,
                AppEvent::KeyConfigUpdate(new_keys) => {
                    app.keys = *new_keys;
                    app.show_toast("🔧 Config Reloaded");
                },
                AppEvent::QueueUpdate(queue_data) => {
                    app.queue = queue_data.into_iter().map(|(title, artist, duration_ms, is_current, file_path)| {
                        app::QueueItem { title, artist, duration_ms, is_current, file_path }
                    }).collect();
                },

                AppEvent::Tick => {
                    app.on_tick();

                    // Poll Shuffle/Repeat status every ~2 seconds (120 ticks at 16ms/tick, roughly)
                    // Actually tick rate is 100ms? View main loop setup.
                    // Assuming ~10Hz or similar.
                    // Poll Shuffle/Repeat status every ~1 second
                    // This is now enabled for BOTH Controller AND MPD modes
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    
                    static mut LAST_SYNC: u64 = 0;
                    unsafe {
                        if now > LAST_SYNC {
                            LAST_SYNC = now;
                            if let Ok(s) = player.get_shuffle() { app.shuffle = s; }
                            if let Ok(r) = player.get_repeat() { app.repeat = r; }
                        }
                    }

                    if app.last_scroll_time.is_none() && (app.lyrics_offset.is_some() || app.lyrics_selected.is_some()) {
                        if let (LyricsState::Loaded(lyrics), Some(_track)) = (&app.lyrics, &app.track) {
                            // 1. Calculate Target
                            // Find target line based on interpolated time
                            let target_idx = lyrics.iter()
                               .position(|l| l.timestamp_ms > app.get_current_position_ms())
                               .map(|i| i.saturating_sub(1))
                               .unwrap_or(lyrics.len().saturating_sub(1));

                            // 2. Smooth Scroll Animation 🌊
                            // Update accumulator (16ms per tick)
                            app.smooth_scroll_accum += 0.016;

                            // Threshold: 0.05s (approx 20 lines/sec max speed)
                            // This prevents "teleporting" and ensures visible motion
                            if app.smooth_scroll_accum >= 0.05 {
                                let mut done_offset = false;
                                let mut done_selected = false;

                                // Animate Offset (Viewport)
                                if let Some(curr) = &mut app.lyrics_offset {
                                    if *curr < target_idx {
                                        *curr += 1;
                                    } else if *curr > target_idx {
                                        *curr -= 1;
                                    } else {
                                        done_offset = true;
                                    }
                                } else {
                                    done_offset = true;
                                }

                                // Animate Selection (Highlight) - The Fix!
                                if let Some(curr_sel) = &mut app.lyrics_selected {
                                    if *curr_sel < target_idx {
                                        *curr_sel += 1;
                                    } else if *curr_sel > target_idx {
                                        *curr_sel -= 1;
                                    } else {
                                        done_selected = true;
                                    }
                                } else {
                                    // If no selection, we don't need to animate it (or set it to target? no leave it)
                                    done_selected = true;
                                }
                                
                                // Clean up if both reached target
                                if done_offset && done_selected {
                                    app.lyrics_offset = None;
                                    app.lyrics_selected = None;
                                }

                                app.smooth_scroll_accum = 0.0;
                            }
                        }
                    }
                }
            }
        }

            }

        if !app.is_running {
            break;
        }
    }

    // Stop Audio Pipeline 🛑
    audio_pipeline.stop();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Save state on exit
    app.save_state();

    // Cleanup Lock File (if we own it)
    if is_audio_master {
        app::lock::release_audio_lock();
    }

    Ok(())
}
