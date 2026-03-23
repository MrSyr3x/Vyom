use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::EventStream,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use vyom::app::config::AppConfig;

use vyom::app;
use vyom::artwork;
use vyom::audio::pipeline as audio_pipeline;

use vyom::player;
use vyom::ui::theme;

use app::cli::Args;
use app::events::AppEvent;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    // 0. Set up beautiful panic handler to intercept unrecoverable crashes 🚨
    human_panic::setup_panic!();

    let args = Args::parse();

    if args.generate_config {
        let default_config = app::config::UserConfig::default();
        println!("{}", toml::to_string_pretty(&default_config).unwrap());
        return Ok(());
    }

    let is_standalone = args.standalone;
    let is_tmux = std::env::var("TMUX").is_ok();

    // Smart Window Logic
    // Default is Full UI (!mini).
    let want_lyrics = !args.mini;

    let current_exe = std::env::current_exe()?;
    let exe_path_cow = current_exe.to_string_lossy();
    let exe_path = exe_path_cow.as_ref();

    // 1. WINDOW TITLE (For Yabai/Amethyst) 🏷️
    print!("\x1b]2;Vyom\x07");

    // 3. LOGGER INITIALIZATION 📝
    let log_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("vyom");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Warning: failed to create log directory: {}", e);
    }
    // Use a daily rolling log file to prevent infinite growth
    let file_appender = tracing_appender::rolling::daily(log_dir, "vyom.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // We only set this up if we are not generating config
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    // 4. TMUX LOGIC
    if app::tmux::handle_tmux_split(&args, exe_path, is_tmux, is_standalone, want_lyrics)? {
        return Ok(());
    }
    // No else block for Standalone Resize - User manages window size manually.

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Enable Kitty Keyboard Protocol (DisambiguateEscapeCodes | ReportAllKeysAsEscapeCodes)
    // This often stops terminals from "peeking" at modifiers for local shortcuts
    use crossterm::event::{
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    };
    execute!(
        stdout,
        EnterAlternateScreen,
        Hide,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // In Tmux, we assume full split/window, so show lyrics by default UNLESS mini mode is requested.
    let app_show_lyrics = want_lyrics; // This is already !args.mini

    // Determine backend mode and source app name
    #[cfg(feature = "mpd")]
    let (is_mpd_mode, source_app) = if args.controller {
        // Auto-Pause MPD to prevent concurrent audio ⏸️
        if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
            if let Err(e) = mpd.pause(true) {
                tracing::debug!("Failed to pause MPD on controller init: {}", e);
            }
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
    let (user_config, persistent_state, config_err) = AppConfig::load();

    if persistent_state.eq_enabled && !is_audio_master {
        // Maybe log that EQ is visual only?
    }

    let mut app = app::App::new(
        app_show_lyrics,
        is_tmux,
        is_mpd_mode,
        source_app,
        user_config.clone(),
        persistent_state,
    );

    if let Some(msg) = config_err {
        app.show_toast(&msg);
    }

    let mut audio_pipeline = audio_pipeline::AudioPipeline::new(app.eq_gains.clone());

    // Attach Visualizer 📊
    audio_pipeline.attach_visualizer(app.visualizer.get_audio_buffer());

    if is_audio_master {
        if let Err(e) = audio_pipeline.start() {
            let msg = format!("Audio Error: {} (Visuals Only)", e);
            tracing::error!("{}", msg);
            app.show_toast(&msg);
        }
        // CRITICAL: Apply persisted volume immediately 🔊
        audio_pipeline.set_volume(app.app_volume);
    } else {
        // We are secondary. No audio output.
        // Visuals might still work if we tap into the same FIFO?
        // For now, secondary = no audio processing = no visualizer (unless we share data, which is complex).
        app.show_toast("🔇 Shared Audio Mode (UI Only)");
    }

    // Player Backend Selection 🎛️
    let player: std::sync::Arc<dyn player::PlayerTrait> =
        player::PlayerFactory::create(&args, &user_config);

    let (tx, rx) = mpsc::channel(100);

    // Performance Optimization: Global HTTP Client (Reused)
    let client = reqwest::Client::builder()
        .user_agent(format!("vyom-rs/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
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
        // Track last status poll time to run it less frequently (e.g., 1s)
        let mut last_status_poll = std::time::Instant::now();
        let mut last_queue_version: Option<u64> = None;

        loop {
            // Use shared player reference
            let player_ref = player_poll.clone();

            // Poll track & queue frequently (250ms)
            // Poll status (shuffle/repeat) less frequently (1s) to save AppleScript calls
            let should_poll_status = last_status_poll.elapsed() >= Duration::from_secs(1);
            if should_poll_status {
                last_status_poll = std::time::Instant::now();
            }

            let last_q_vers_clone = last_queue_version;

            let result = tokio::task::spawn_blocking(move || {
                let track = player_ref.get_current_track();
                let q_vers = player_ref.get_queue_version();

                let queue = if q_vers.is_some() && q_vers == last_q_vers_clone {
                    None // Queue hasn't changed, skip heavy allocation
                } else {
                    Some(player_ref.get_queue())
                };

                let (shuffle, repeat) = if should_poll_status {
                    (player_ref.get_shuffle().ok(), player_ref.get_repeat().ok())
                } else {
                    (None, None)
                };

                (track, queue, q_vers, shuffle, repeat)
            })
            .await;

            if let Ok((track_res, queue_opt, new_q_vers, shuffle_opt, repeat_opt)) = result {
                if new_q_vers != last_queue_version {
                    last_queue_version = new_q_vers;
                }

                if let Ok(info) = track_res {
                    if let Err(e) = tx_spotify.send(AppEvent::TrackUpdate(info)).await {
                        tracing::debug!("Channel closed during track update: {}", e);
                    }
                }
                if let Some(Ok(queue)) = queue_opt {
                    if let Err(e) = tx_spotify.send(AppEvent::QueueUpdate(queue)).await {
                        tracing::debug!("Channel closed during queue update: {}", e);
                    }
                }
                // Send status update if we polled it successfully
                if let (Some(s), Some(r)) = (shuffle_opt, repeat_opt) {
                    if let Err(e) = tx_spotify.send(AppEvent::StatusUpdate(s, r)).await {
                        tracing::debug!("Channel closed during status update: {}", e);
                    }
                }
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
        })
        .await
        .unwrap_or(None);

        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;

            // Check file modification time (blocking I/O wrapped)
            let check_path = theme_path.clone(); // Clone for closure
            let metadata_result = tokio::task::spawn_blocking(move || {
                std::fs::metadata(&check_path).and_then(|m| m.modified())
            })
            .await;

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
        })
        .await
        .unwrap_or(None);

        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;

            let check_path = config_path.clone();
            let metadata_result = tokio::task::spawn_blocking(move || {
                std::fs::metadata(&check_path).and_then(|m| m.modified())
            })
            .await;

            if let Ok(Ok(modified)) = metadata_result {
                if last_modified.is_none_or(|last| modified > last) {
                    last_modified = Some(modified);

                    // Reload config to get new keys
                    // Hot Reload: Reload config, verify keys changed
                    let (new_user_config, _, reload_err) = AppConfig::load();
                    if let Some(err_msg) = reload_err {
                        if let Err(e) = tx_config.send(AppEvent::ToastUpdate(err_msg)).await {
                            tracing::debug!("Channel closed during config toast: {}", e);
                        }
                    } else if tx_config
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

    // 5. Animation / Status Tick Task ⚡
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        // Fast animations (like Seeking and Cava Visualizer) need 16ms to render correctly.
        // We handle conditional drawing inside the event loop so this doesn't burn CPU unnecessarily.
        let mut interval = tokio::time::interval(Duration::from_millis(16));
        loop {
            interval.tick().await;
            if tx_tick.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    // 6. Launch Runner Core Orchestrator 🚀
    if let Err(e) = vyom::app::runner::run_app(
        &mut app,
        &mut terminal,
        &player,
        &mut audio_pipeline,
        &args,
        tx,
        rx,
        client,
    )
    .await
    {
        tracing::error!("Core event runner failed: {}", e);
    }

    // Stop Audio Pipeline 🛑
    audio_pipeline.stop();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        Show,
        PopKeyboardEnhancementFlags
    )?;
    terminal.show_cursor()?;

    // Save state on exit
    app.save_state();

    // Cleanup Lock File (if we own it)
    if is_audio_master {
        app::lock::release_audio_lock();
    }

    // Force Exit to bypass slow Tokio unwind of blocking tasks (AppleScript/MPD) 🚀
    std::process::exit(0);
}
