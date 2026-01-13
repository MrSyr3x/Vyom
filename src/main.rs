use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode, EventStream, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use futures::{StreamExt};
#[cfg(feature = "mpd")]
use lofty::{file::TaggedFileExt, tag::{Accessor, TagExt}};
#[cfg(feature = "mpd")]
use mpd::{Query, Term};



mod app;
mod artwork;
mod theme; 
mod lyrics;
mod player; 
mod ui;
mod audio_device;
mod dsp_eq;
mod audio_pipeline;

#[cfg(feature = "mpd")]
mod mpd_player;


use clap::Parser;

use app::{App, ArtworkState, LyricsState};
use player::{TrackInfo}; 
use crate::lyrics::{LyricsFetcher}; 
use artwork::{ArtworkRenderer}; 


use theme::{Theme};


enum AppEvent {
    Input(Event),
    TrackUpdate(Option<TrackInfo>),
    LyricsUpdate(String, LyricsState),
    ArtworkUpdate(ArtworkState),
    ThemeUpdate(Theme),
    QueueUpdate(Vec<(String, String, u64, bool, String)>),
    CavaUpdate(Vec<f32>),
    Tick,
}

/// Vyom - A beautiful music companion for your terminal 🎵
#[derive(Parser, Debug)]
#[command(name = "vyom", version, about)]
struct Args {
    /// Run inside tmux split (internal)
    #[arg(long)]
    standalone: bool,
    
    /// Enable lyrics panel
    #[arg(long)]
    lyrics: bool,
    
    /// Use MPD backend instead of Spotify/Music.app
    #[cfg(feature = "mpd")]
    #[arg(long)]
    mpd: bool,
    
    /// MPD host (default: localhost)
    #[cfg(feature = "mpd")]
    #[arg(long, default_value = "localhost")]
    mpd_host: String,
    
    /// MPD port (default: 6600)
    #[cfg(feature = "mpd")]
    #[arg(long, default_value_t = 6600)]
    mpd_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let is_standalone = args.standalone;
    let is_tmux = std::env::var("TMUX").is_ok();

    // Smart Window Logic
    let want_lyrics = args.lyrics;
    
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.to_str().unwrap();

    // 1. WINDOW TITLE (For Yabai/Amethyst) 🏷️
    print!("\x1b]2;Vyom\x07");

    // 2. TMUX LOGIC
    if is_tmux && !is_standalone {
        // Build child command with all necessary flags
        let mut child_args = vec!["--standalone".to_string()];
        
        if want_lyrics {
            child_args.push("--lyrics".to_string());
        }
        
        #[cfg(feature = "mpd")]
        if args.mpd {
            child_args.push("--mpd".to_string());
            child_args.push("--mpd-host".to_string());
            child_args.push(args.mpd_host.clone());
            child_args.push("--mpd-port".to_string());
            child_args.push(args.mpd_port.to_string());
        }
        
        let child_cmd = format!("{} {}", exe_path, child_args.join(" "));
        
        // Auto-split logic (Tmux)
        let status = std::process::Command::new("tmux")
            .arg("split-window")
            .arg("-h")
            .arg("-p")
            .arg("22")
            .arg(child_cmd)
            .status();

        match status {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("Failed to create tmux split: {}", e);
            }
        }
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

    // 1. Initial State
    let mut app = App::new(app_show_lyrics, is_tmux);
    
    // Start Audio Pipeline 🔊 (FIFO → DSP EQ → Speakers)
    let mut audio_pipeline = audio_pipeline::AudioPipeline::new(app.eq_gains.clone());
    if let Err(e) = audio_pipeline.start() {
        // Not fatal - just log and continue (EQ will be visual only)
        eprintln!("Audio pipeline: {} (EQ will be visual only)", e);
    }
    
    // Player Backend Selection 🎛️
    #[cfg(feature = "mpd")]
    let player: std::sync::Arc<dyn player::PlayerTrait> = if args.mpd {
        std::sync::Arc::new(mpd_player::MpdPlayer::new(&args.mpd_host, args.mpd_port))
    } else {
        std::sync::Arc::from(player::get_player())
    };
    
    #[cfg(not(feature = "mpd"))]
    let player: std::sync::Arc<dyn player::PlayerTrait> = std::sync::Arc::from(player::get_player());
    
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
            if tx_input.send(AppEvent::Input(event)).await.is_err() { break; }
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
            }).await;
            
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
        // We act like a dumb poller for now. 
        let mut last_theme_debug = format!("{:?}", theme::load_current_theme());

        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            
            // Reload & Check difference based on Debug impl (hacky but cheap)
            let new_theme = theme::load_current_theme();
            let new_debug = format!("{:?}", new_theme);
            
            if new_debug != last_theme_debug {
                last_theme_debug = new_debug;
                 if tx_theme.send(AppEvent::ThemeUpdate(new_theme)).await.is_err() { break; }
            }
        }
    });

    // 4a. Cava Integration Task 📊 (real audio visualization)
    let tx_cava = tx.clone();
    tokio::spawn(async move {
        use tokio::process::Command;
        use tokio::io::{AsyncBufReadExt, BufReader};
        
        // Spawn cava with our custom config
        let cava_config = format!("{}/.config/cava/vyom_config", std::env::var("HOME").unwrap_or_default());
        let child = Command::new("cava")
            .arg("-p")
            .arg(&cava_config)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();
        
        if let Ok(mut child) = child {
            if let Some(stdout) = child.stdout.take() {
                let mut reader = BufReader::new(stdout).lines();
                
                while let Ok(Some(line)) = reader.next_line().await {
                    // Parse ASCII values (semicolon-separated)
                    let bars: Vec<f32> = line.split(';')
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| s.trim().parse::<f32>().ok())
                        .map(|v| v / 1000.0) // Normalize to 0.0-1.0
                        .collect();
                    
                    if !bars.is_empty() {
                        if tx_cava.send(AppEvent::CavaUpdate(bars)).await.is_err() { break; }
                    }
                }
            }
            let _ = child.kill().await;
        }
    });

    // 4. Animation Tick Task ⚡
    let tx_tick = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        loop {
            interval.tick().await;
            if tx_tick.send(AppEvent::Tick).await.is_err() { break; }
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

        terminal.draw(|f| ui::ui(f, &mut app))?;

        if let Some(event) = rx.recv().await {
            match event {
                // ... (Input handling omitted)
                // Mouse Interaction Removed as per User Request
                AppEvent::Input(Event::Mouse(_)) => {},
                AppEvent::Input(Event::Key(key)) => {
                    // Generic Input Popup Handling (Highest Priority) 📝
                    if let Some(ref mut input) = app.input_state {
                        match key.code {
                            KeyCode::Esc => {
                                app.input_state = None;
                            },
                            KeyCode::Enter => {
                                // Execute Action based on Mode
                                match input.mode {
                                    app::InputMode::PlaylistSave => {
                                        let name = input.value.trim().to_string();
                                        if !name.is_empty() {
                                            #[cfg(feature = "mpd")]
                                            if args.mpd {
                                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                                    let _ = mpd.save(&name);
                                                    // Refresh playlists
                                                    if let Ok(pls) = mpd.playlists() {
                                                        app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                                                    }
                                                    app.toast = Some((format!("💾 Saved: {}", name), std::time::Instant::now()));
                                                }
                                            }
                                        }
                                    },
                                    app::InputMode::PlaylistRename => {
                                        // TODO: Implement Rename logic when 'r' key is added
                                    }
                                }
                                app.input_state = None;
                            },
                            KeyCode::Backspace => {
                                input.value.pop();
                            },
                            KeyCode::Char(c) => {
                                input.value.push(c);
                            },
                            _ => {}
                        }
                        // Consume event, don't propagate
                        continue;
                    }

                    // Tag editor input handling (takes priority)
                    if app.tag_edit.is_some() {
                        match key.code {
                            KeyCode::Esc => {
                                app.tag_edit = None;  // Cancel
                            },
                            KeyCode::Tab => {
                                if let Some(ref mut tag) = app.tag_edit {
                                    tag.next_field();
                                }
                            },
                            KeyCode::BackTab => {
                                if let Some(ref mut tag) = app.tag_edit {
                                    tag.prev_field();
                                }
                            },
                            KeyCode::Backspace => {
                                if let Some(ref mut tag) = app.tag_edit {
                                    tag.active_value().pop();
                                }
                            },
                            KeyCode::Enter => {
                                // Save tags using lofty
                                if let Some(ref tag_state) = app.tag_edit {
                                    #[cfg(feature = "mpd")]
                                    if !tag_state.file_path.is_empty() {
                                        // MPD music directory from config or env
                                        let music_dir = std::env::var("MPD_MUSIC_DIR")
                                            .unwrap_or_else(|_| "/Users/syr3x/Music".to_string());
                                        let full_path = format!("{}/{}", music_dir, tag_state.file_path);
                                        
                                        // Write tags using lofty
                                        if let Ok(mut tagged_file) = lofty::read_from_path(&full_path) {
                                            let mut modified = false;
                                            if let Some(tag) = tagged_file.primary_tag_mut() {
                                                tag.set_title(tag_state.title.clone());
                                                tag.set_artist(tag_state.artist.clone());
                                                if !tag_state.album.is_empty() {
                                                    tag.set_album(tag_state.album.clone());
                                                }
                                                modified = true;
                                            }
                                            
                                            if !modified {
                                                if let Some(tag) = tagged_file.first_tag_mut() {
                                                    tag.set_title(tag_state.title.clone());
                                                    tag.set_artist(tag_state.artist.clone());
                                                    if !tag_state.album.is_empty() {
                                                        tag.set_album(tag_state.album.clone());
                                                    }
                                                }
                                            }
                                            
                                            // Save to file
                                            if let Ok(mut file) = std::fs::OpenOptions::new()
                                                .read(true).write(true).open(&full_path) 
                                            {
                                                use lofty::file::AudioFile;
                                                let _ = tagged_file.save_to(&mut file, lofty::config::WriteOptions::default());
                                            }
                                        }
                                    }
                                }
                                app.tag_edit = None;
                            },
                            KeyCode::Char(c) => {
                                if let Some(ref mut tag) = app.tag_edit {
                                    tag.active_value().push(c);
                                }
                            },
                            _ => {}
                        }
                        continue; // Don't process other key handlers while tag editor is open
                    }
                    // When search is active, capture ALL character input (except special keys)
                    else if app.search_active {
                        match key.code {
                            KeyCode::Esc => app.search_active = false,
                            KeyCode::Backspace => { app.search_query.pop(); },
                            KeyCode::Enter => {
                                app.search_active = false;
                                // Perform MPD search
                                #[cfg(feature = "mpd")]
                                if args.mpd && !app.search_query.is_empty() {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        if let Ok(songs) = mpd.listall() {
                                            let query_lower = app.search_query.to_lowercase();
                                            app.library_items = songs.into_iter()
                                                .filter(|s| {
                                                    s.file.to_lowercase().contains(&query_lower) ||
                                                    s.title.as_ref().map(|t| t.to_lowercase().contains(&query_lower)).unwrap_or(false) ||
                                                    s.tags.iter().any(|(_, v)| v.to_lowercase().contains(&query_lower))
                                                })
                                                .take(50)
                                                .map(|s| app::LibraryItem {
                                                    name: s.title.unwrap_or_else(|| s.file.clone()),
                                                    item_type: app::LibraryItemType::Song,
                                                    artist: s.tags.iter().find(|(k,_)| k == "Artist").map(|(_,v)| v.clone()),
                                                    duration_ms: s.duration.map(|d| d.as_millis() as u64),
                                                    path: Some(s.file),
                                                })
                                                .collect();
                                            app.library_selected = 0;
                                        }
                                    }
                                }
                            },
                            KeyCode::Up => {
                                app.library_selected = app.library_selected.saturating_sub(1);
                            },
                            KeyCode::Down => {
                                let max = app.library_items.len().max(1);
                                if app.library_selected < max - 1 {
                                    app.library_selected += 1;
                                }
                            },
                            KeyCode::Char(c) => app.search_query.push(c),
                            _ => {}
                        }
                    } else {
                        // Normal key handling when NOT typing in search
                        match key.code {
                            KeyCode::Char('q') => app.is_running = false,
                            KeyCode::Char(' ') => { 
                                let _ = player.play_pause();
                                app.toast = Some(("⏯ Play/Pause".to_string(), std::time::Instant::now()));
                            },
                            KeyCode::Char('n') => { 
                                let _ = player.next();
                                app.toast = Some(("⏭ Next Track".to_string(), std::time::Instant::now()));
                            },
                            KeyCode::Char('p') => { 
                                let _ = player.prev();
                                app.toast = Some(("⏮ Previous Track".to_string(), std::time::Instant::now()));
                            },
                            KeyCode::Char('+') | KeyCode::Char('=') => { 
                            // Note: Volume is hardware/DAC controlled in bit-perfect mode
                                let _ = player.volume_down();
                            },
                            // Seek Controls (cumulative & safe) ⏩
                            KeyCode::Char('h') => {
                                let now = std::time::Instant::now();
                                let is_new_sequence = if let Some(last) = app.last_seek_time {
                                    now.duration_since(last).as_millis() >= 500
                                } else { true };
                                
                                if is_new_sequence {
                                    if let Some(track) = &app.track {
                                        app.seek_initial_pos = Some(track.position_ms as f64 / 1000.0);
                                    } else {
                                        app.seek_initial_pos = Some(0.0);
                                    }
                                    app.seek_accumulator = -5.0;
                                } else {
                                    app.seek_accumulator -= 5.0;
                                }
                                app.last_seek_time = Some(now);
                                
                                if let Some(start_pos) = app.seek_initial_pos {
                                    let mut target = start_pos + app.seek_accumulator;
                                    
                                    // Clamp to safe range (0.0 to Duration) to prevent panic
                                    if let Some(track) = &app.track {
                                        let duration = track.duration_ms as f64 / 1000.0;
                                        // Ensure positive and within bounds
                                        target = target.max(0.0).min(duration);
                                    } else {
                                        target = target.max(0.0);
                                    }
                                    
                                    // Non-blocking seek with track verification! 🚀
                                    let player_bg = player.clone();
                                    // Use name+artist as unique track identifier
                                    let original_track_key = app.track.as_ref().map(|t| (t.name.clone(), t.artist.clone()));
                                    tokio::task::spawn_blocking(move || {
                                        // Verify we're still on the same track before seeking
                                        if let Ok(Some(current_track)) = player_bg.get_current_track() {
                                            let current_key = (current_track.name.clone(), current_track.artist.clone());
                                            if original_track_key.as_ref() == Some(&current_key) {
                                                let _ = player_bg.seek(target);
                                            }
                                            // If track changed, skip the seek silently
                                        }
                                    });
                                    app.toast = Some((format!("⏪ Seek: {:+.0}s", app.seek_accumulator), now));
                                }
                            },
                            KeyCode::Char('l') => {
                                let now = std::time::Instant::now();
                                let is_new_sequence = if let Some(last) = app.last_seek_time {
                                    now.duration_since(last).as_millis() >= 500
                                } else { true };
                                
                                if is_new_sequence {
                                    if let Some(track) = &app.track {
                                        app.seek_initial_pos = Some(track.position_ms as f64 / 1000.0);
                                    } else {
                                        app.seek_initial_pos = Some(0.0);
                                    }
                                    app.seek_accumulator = 5.0;
                                } else {
                                    app.seek_accumulator += 5.0;
                                }
                                app.last_seek_time = Some(now);
                                
                                if let Some(start_pos) = app.seek_initial_pos {
                                    let mut target = start_pos + app.seek_accumulator;
                                    
                                    // Clamp to safe range (0.0 to Duration)
                                    if let Some(track) = &app.track {
                                        let duration = track.duration_ms as f64 / 1000.0;
                                        target = target.max(0.0).min(duration);
                                    } else {
                                        target = target.max(0.0);
                                    }
                                    
                                    // Non-blocking seek with track verification! 🚀
                                    let player_bg = player.clone();
                                    // Use name+artist as unique track identifier
                                    let original_track_key = app.track.as_ref().map(|t| (t.name.clone(), t.artist.clone()));
                                    tokio::task::spawn_blocking(move || {
                                        // Verify we're still on the same track before seeking
                                        if let Ok(Some(current_track)) = player_bg.get_current_track() {
                                            let current_key = (current_track.name.clone(), current_track.artist.clone());
                                            if original_track_key.as_ref() == Some(&current_key) {
                                                let _ = player_bg.seek(target);
                                            }
                                            // If track changed, skip the seek silently
                                        }
                                    });
                                    app.toast = Some((format!("⏩ Seek: {:+.0}s", app.seek_accumulator), now));
                                }
                            },
                            
                            // Queue Reordering with J/K (Shift+j/k) 🔄
                            KeyCode::Char('J') if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                                if app.library_selected < app.queue.len().saturating_sub(1) {
                                    #[cfg(feature = "mpd")]
                                    if args.mpd {
                                        if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                            let current_pos = app.library_selected as u32;
                                            let new_pos = current_pos + 1;
                                            if let Ok(_) = mpd.shift(current_pos, new_pos as usize) {
                                                 app.library_selected = new_pos as usize;
                                            }
                                        }
                                    }
                                }
                            },
                            KeyCode::Char('K') if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                                if app.library_selected > 0 {
                                    #[cfg(feature = "mpd")]
                                    if args.mpd {
                                        if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                            let current_pos = app.library_selected as u32;
                                            let new_pos = current_pos - 1;
                                            if let Ok(_) = mpd.shift(current_pos, new_pos as usize) {
                                                 app.library_selected = new_pos as usize;
                                            }
                                        }
                                    }
                                }
                            },
                            // View Mode Switching 🎛️
                            KeyCode::Char('1') => app.view_mode = app::ViewMode::Lyrics,
                            KeyCode::Char('2') => app.view_mode = app::ViewMode::Cava,
                            KeyCode::Char('3') => app.view_mode = app::ViewMode::Library,
                            KeyCode::Char('4') => app.view_mode = app::ViewMode::EQ,
                        
                        // Lyrics Navigation (j/k scroll, Enter to seek) 📜
                        KeyCode::Char('j') if app.view_mode == app::ViewMode::Lyrics => {
                            if let LyricsState::Loaded(ref lines) = &app.lyrics {
                                let max = lines.len().saturating_sub(1);
                                
                                // If no selection yet, start from current playing line
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
                                
                                // CRITICAL: Mark scroll time to prevent auto-recenter!
                                app.last_scroll_time = Some(std::time::Instant::now());
                            }
                        },
                        KeyCode::Char('k') if app.view_mode == app::ViewMode::Lyrics => {
                            if let LyricsState::Loaded(ref lines) = &app.lyrics {
                                let max = lines.len().saturating_sub(1);
                                
                                // If no selection yet, start from current playing line
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
                                
                                // CRITICAL: Mark scroll time to prevent auto-recenter!
                                app.last_scroll_time = Some(std::time::Instant::now());
                            }
                        },
                        KeyCode::Enter if app.view_mode == app::ViewMode::Lyrics => {
                            if let LyricsState::Loaded(ref lines) = &app.lyrics {
                                if let Some(idx) = app.lyrics_selected {
                                    if idx < lines.len() {
                                        let target_ms = lines[idx].timestamp_ms;
                                        let target_secs = target_ms as f64 / 1000.0;
                                        
                                        // Non-blocking seek! 🚀
                                        let player_bg = player.clone();
                                        tokio::task::spawn_blocking(move || {
                                            let _ = player_bg.seek(target_secs);
                                        });
                                        
                                        let mins = target_ms / 60000;
                                        let secs = (target_ms % 60000) / 1000;
                                        app.toast = Some((format!("🎤 Jump to {}:{:02}", mins, secs), std::time::Instant::now()));
                                        app.lyrics_selected = None; // Exit selection mode
                                        app.lyrics_offset = None; // Return to auto-sync
                                        app.last_scroll_time = None; // Allow immediate auto-follow
                                    }
                                }
                            }
                        },
                        // EQ Controls (only when in EQ view) 🎚️
                        KeyCode::Left if app.view_mode == app::ViewMode::EQ => {
                            app.eq_selected = app.eq_selected.saturating_sub(1);
                        },
                        KeyCode::Right if app.view_mode == app::ViewMode::EQ => {
                            if app.eq_selected < 9 { app.eq_selected += 1; }
                        },
                        KeyCode::Up if app.view_mode == app::ViewMode::EQ => {
                            let band = &mut app.eq_bands[app.eq_selected];
                            *band = (*band + 0.05).min(1.0); // +1.2dB
                            app.mark_custom();
                            app.sync_band_to_dsp(app.eq_selected);
                            let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
                            app.toast = Some((format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db), std::time::Instant::now()));
                        },
                        KeyCode::Down if app.view_mode == app::ViewMode::EQ => {
                            let band = &mut app.eq_bands[app.eq_selected];
                            *band = (*band - 0.05).max(0.0); // -1.2dB
                            app.mark_custom();
                            app.sync_band_to_dsp(app.eq_selected);
                            let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
                            app.toast = Some((format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db), std::time::Instant::now()));
                        },
                        KeyCode::Char('e') if app.view_mode == app::ViewMode::EQ => {
                            app.toggle_eq();
                            app.toast = Some((format!("🎛 EQ: {}", if app.eq_enabled { "ON" } else { "OFF" }), std::time::Instant::now()));
                        },
                        KeyCode::Char('r') if app.view_mode == app::ViewMode::EQ => {
                            app.reset_eq();
                            app.toast = Some(("🔄 EQ Reset".to_string(), std::time::Instant::now()));
                        },
                        // Preset cycling: P (shift) or Tab for next, Shift+Tab for previous
                        KeyCode::Char('P') | KeyCode::Tab if app.view_mode == app::ViewMode::EQ => {
                            app.next_preset();
                            app.toast = Some((format!("🎵 Preset: {}", app.get_preset_name()), std::time::Instant::now()));
                        },
                        KeyCode::BackTab if app.view_mode == app::ViewMode::EQ => {
                            app.prev_preset();
                            app.toast = Some((format!("🎵 Preset: {}", app.get_preset_name()), std::time::Instant::now()));
                        },
                        // Audio device switching: d for next, D for previous
                        KeyCode::Char('d') if app.view_mode == app::ViewMode::EQ => {
                            app.next_device();
                        },
                        KeyCode::Char('D') if app.view_mode == app::ViewMode::EQ => {
                            app.prev_device();
                        },
                        // Audiophile Controls 🎚️
                        // Preamp: g/G for +/- 1dB
                        KeyCode::Char('g') if app.view_mode == app::ViewMode::EQ => {
                            app.adjust_preamp(1.0);
                        },
                        KeyCode::Char('G') if app.view_mode == app::ViewMode::EQ => {
                            app.adjust_preamp(-1.0);
                        },
                        // Balance: b/B for +/- 0.1 (right/left)
                        KeyCode::Char('b') if app.view_mode == app::ViewMode::EQ => {
                            app.adjust_balance(0.1);
                        },
                        KeyCode::Char('B') if app.view_mode == app::ViewMode::EQ => {
                            app.adjust_balance(-0.1);
                        },
                        // Crossfade: c to toggle (sends to MPD)
                        KeyCode::Char('c') if app.view_mode == app::ViewMode::EQ => {
                            app.toggle_crossfade();
                            // Send crossfade command to MPD
                            #[cfg(feature = "mpd")]
                            if args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    let _ = mpd.crossfade(app.crossfade_secs as i64);
                                }
                            }
                        },
                        // ReplayGain: R (Shift+R) to cycle modes (Off → Track → Album → Auto)
                        KeyCode::Char('R') if app.view_mode == app::ViewMode::EQ => {
                            app.replay_gain_mode = (app.replay_gain_mode + 1) % 4;
                            #[cfg(feature = "mpd")]
                            if args.mpd {
                                let mode = match app.replay_gain_mode {
                                    1 => mpd::status::ReplayGain::Track,
                                    2 => mpd::status::ReplayGain::Album,
                                    3 => mpd::status::ReplayGain::Auto,
                                    _ => mpd::status::ReplayGain::Off,
                                };
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    let _ = mpd.replaygain(mode);
                                }
                            }
                        },
                        // WhichKey popup: ? to toggle, ESC to close
                        KeyCode::Char('?') => {
                            app.show_keyhints = !app.show_keyhints;
                        },
                        // Audio Info popup: i to toggle (like Poweramp)
                        KeyCode::Char('i') => {
                            app.show_audio_info = !app.show_audio_info;
                        },
                        KeyCode::Esc => {
                            if app.show_keyhints {
                                app.show_keyhints = false;
                            }
                            if app.show_audio_info {
                                app.show_audio_info = false;
                            }
                        },
                        
                        // Queue Reordering (Mature Feature) 🔄
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) && app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                            if app.library_selected > 0 {
                                #[cfg(feature = "mpd")]
                                if args.mpd {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        let current_pos = app.library_selected as u32;
                                        let new_pos = current_pos - 1;
                                        if let Ok(_) = mpd.shift(current_pos, new_pos as usize) {
                                             app.library_selected = new_pos as usize;
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) && app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                             if app.library_selected < app.queue.len().saturating_sub(1) {
                                #[cfg(feature = "mpd")]
                                if args.mpd {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        let current_pos = app.library_selected as u32;
                                        let new_pos = current_pos + 1;
                                        if let Ok(_) = mpd.shift(current_pos, new_pos as usize) {
                                             app.library_selected = new_pos as usize;
                                        }
                                    }
                                }
                            }
                        },

                        // Library Panel Controls (when in Library view) 📚
                        KeyCode::Tab if app.view_mode == app::ViewMode::Library => {
                            // Cycle library modes
                            app.library_mode = match app.library_mode {
                                app::LibraryMode::Queue => app::LibraryMode::Browse,
                                app::LibraryMode::Browse => app::LibraryMode::Search,
                                app::LibraryMode::Search => app::LibraryMode::Playlists,
                                app::LibraryMode::Playlists => app::LibraryMode::Queue,
                            };
                            // Clear state when switching modes
                            app.library_selected = 0;
                            app.library_items.clear();
                            app.browse_path.clear();
                            app.search_query.clear();
                            
                            // Load playlists when entering Playlists mode
                            #[cfg(feature = "mpd")]
                            if app.library_mode == app::LibraryMode::Playlists && args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(pls) = mpd.playlists() {
                                        app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                                    }
                                }
                            }
                        },
                        KeyCode::BackTab if app.view_mode == app::ViewMode::Library => {
                            // Reverse cycle
                            app.library_mode = match app.library_mode {
                                app::LibraryMode::Queue => app::LibraryMode::Playlists,
                                app::LibraryMode::Browse => app::LibraryMode::Queue,
                                app::LibraryMode::Search => app::LibraryMode::Browse,
                                app::LibraryMode::Playlists => app::LibraryMode::Search,
                            };
                            // Clear state when switching modes
                            app.library_selected = 0;
                            app.library_items.clear();
                            app.browse_path.clear();
                            app.search_query.clear();
                            
                            // Load playlists when entering Playlists mode
                            #[cfg(feature = "mpd")]
                            if app.library_mode == app::LibraryMode::Playlists && args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(pls) = mpd.playlists() {
                                        app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                                    }
                                }
                            }
                        },
                        KeyCode::Char('/') if app.view_mode == app::ViewMode::Library => {
                            // Jump to search mode and activate search input
                            app.library_mode = app::LibraryMode::Search;
                            app.search_active = true;
                        },
                        // Save queue as playlist (Library view, Playlists mode)
                        KeyCode::Char('s') if app.view_mode == app::ViewMode::Library => {
                            // Open Input Popup for Playlist Name 📝
                            app.input_state = Some(app::InputState::new(
                                app::InputMode::PlaylistSave,
                                "Save Playlist As:",
                                ""
                            ));
                        },
                        // Tag editing: t to edit selected song's tags
                        KeyCode::Char('t') if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                            // Get selected song from queue
                            if let Some(item) = app.queue.get(app.library_selected) {
                                // Open tag editor with current values
                                app.tag_edit = Some(app::TagEditState::new(
                                    &item.file_path,
                                    &item.title,
                                    &item.artist,
                                    "",  // Album not in QueueItem, will be loaded from file
                                ));
                            }
                        },
                        // Delete: d to delete playlist or remove song from queue
                        KeyCode::Char('d') if app.view_mode == app::ViewMode::Library => {
                            #[cfg(feature = "mpd")]
                            if args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    match app.library_mode {
                                        app::LibraryMode::Queue => {
                                            // Remove song from queue
                                            let _ = mpd.delete(app.library_selected as u32);
                                        },
                                        app::LibraryMode::Playlists => {
                                            // Delete playlist
                                            if let Some(name) = app.playlists.get(app.library_selected) {
                                                let _ = mpd.pl_remove(name);
                                                // Refresh playlists
                                                if let Ok(pls) = mpd.playlists() {
                                                    app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                                                }
                                                if app.library_selected > 0 {
                                                    app.library_selected -= 1;
                                                }
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                            }
                        },
                        // Shuffle toggle: z
                        KeyCode::Char('z') => {
                            #[cfg(feature = "mpd")]
                            if args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(status) = mpd.status() {
                                        let new_state = !status.random;
                                        let _ = mpd.random(new_state);
                                        app.shuffle = new_state;
                                        app.toast = Some((format!("🔀 Shuffle: {}", if new_state { "ON" } else { "OFF" }), std::time::Instant::now()));
                                    }
                                }
                            }
                        },
                        // Repeat toggle: x
                        KeyCode::Char('x') => {
                            #[cfg(feature = "mpd")]
                            if args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(status) = mpd.status() {
                                        let new_state = !status.repeat;
                                        let _ = mpd.repeat(new_state);
                                        app.repeat = new_state;
                                        app.toast = Some((format!("🔁 Repeat: {}", if new_state { "ON" } else { "OFF" }), std::time::Instant::now()));
                                    }
                                }
                            }
                        },
                        // Add to Queue: 'a' key ➕
                        KeyCode::Char('a') if app.view_mode == app::ViewMode::Library && (app.library_mode == app::LibraryMode::Browse || app.library_mode == app::LibraryMode::Search) => {
                             #[cfg(feature = "mpd")]
                             if args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if !app.library_items.is_empty() {
                                         if let Some(item) = app.library_items.get(app.library_selected) {
                                            let added_name = item.name.clone();
                                            let added = match item.item_type {
                                                app::LibraryItemType::Song => {
                                                    if let Some(path) = &item.path {
                                                        let song = mpd::Song {
                                                            file: path.clone(),
                                                            ..Default::default()
                                                        };
                                                        mpd.push(song).is_ok()
                                                    } else { false }
                                                },
                                                app::LibraryItemType::Album => {
                                                     mpd.findadd(&mpd::Query::new().and(mpd::Term::Tag("Album".into()), &item.name)).is_ok()
                                                },
                                                app::LibraryItemType::Artist => {
                                                     mpd.findadd(&mpd::Query::new().and(mpd::Term::Tag("Artist".into()), &item.name)).is_ok()
                                                },
                                                app::LibraryItemType::Playlist => {
                                                     mpd.load(&item.name, ..).is_ok()
                                                },
                                                _ => false
                                            };
                                            if added {
                                                app.toast = Some((format!("Added: {}", added_name), std::time::Instant::now()));
                                            }
                                         }
                                    }
                                }
                             }
                        },
                        // Enter key for Library actions
                        KeyCode::Enter if app.view_mode == app::ViewMode::Library => {
                            #[cfg(feature = "mpd")]
                            if args.mpd {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    match app.library_mode {
                                        app::LibraryMode::Queue => {
                                            // Play selected song in queue
                                            let _ = mpd.switch(app.library_selected as u32);
                                        }
                                        app::LibraryMode::Browse => {
                                            if app.browse_path.is_empty() {
                                                // Enter root category
                                                let categories = ["Artists", "Albums", "Genres", "Folders"];
                                                if let Some(cat) = categories.get(app.library_selected) {
                                                    app.browse_path.push(cat.to_string());
                                                    app.library_selected = 0;
                                                    
                                                    // Load items for the selected category
                                                    // Use generic MPD commands instead of inefficient listall
                                                    let items = match *cat {
                                                        "Artists" => {
                                                            mpd.list(&Term::Tag("Artist".into()), &Query::new()).ok()
                                                                .map(|v| v.into_iter().map(|name| app::LibraryItem {
                                                                    name, item_type: app::LibraryItemType::Artist, 
                                                                    artist: None, duration_ms: None, path: None 
                                                                }).collect::<Vec<_>>())
                                                        },
                                                        "Albums" => {
                                                            mpd.list(&Term::Tag("Album".into()), &Query::new()).ok()
                                                                .map(|v| v.into_iter().map(|name| app::LibraryItem {
                                                                    name, item_type: app::LibraryItemType::Album, 
                                                                    artist: None, duration_ms: None, path: None 
                                                                }).collect::<Vec<_>>())
                                                        },
                                                        "Genres" => {
                                                            mpd.list(&Term::Tag("Genre".into()), &Query::new()).ok()
                                                                .map(|v| v.into_iter().map(|name| app::LibraryItem {
                                                                    name, item_type: app::LibraryItemType::Genre, 
                                                                    artist: None, duration_ms: None, path: None 
                                                                }).collect::<Vec<_>>())
                                                        },
                                                        "Folders" => {
                                                            mpd.listfiles("/").ok().map(|pairs| {
                                                                pairs.into_iter().map(|(kind, name)| {
                                                                    match kind.as_str() {
                                                                        "directory" => app::LibraryItem {
                                                                            name: name.clone(),
                                                                            item_type: app::LibraryItemType::Folder,
                                                                            artist: None, duration_ms: None, path: Some(name)
                                                                        },
                                                                        "file" => app::LibraryItem {
                                                                            name: name.clone(),
                                                                            item_type: app::LibraryItemType::Song,
                                                                            artist: None, duration_ms: None, path: Some(name)
                                                                        },
                                                                        _ => app::LibraryItem {
                                                                            name,
                                                                            item_type: app::LibraryItemType::Song,
                                                                            artist: None, duration_ms: None, path: None
                                                                        }
                                                                    }
                                                                }).collect::<Vec<_>>()
                                                            })
                                                        },
                                                        _ => None
                                                    };
                                                    
                                                    if let Some(mut i) = items {
                                                        i.sort_by(|a, b| a.name.cmp(&b.name));
                                                        app.library_items = i;
                                                    }
                                                }
                                            } else {
                                                // DRILL DOWN LOGIC
                                                let root = app.browse_path[0].as_str();
                                                if let Some(item) = app.library_items.get(app.library_selected) {
                                                    match (root, &item.item_type) {
                                                        // Entering a Folder
                                                        ("Folders", app::LibraryItemType::Folder) => {
                                                            if let Some(path) = &item.path {
                                                                let path_str = path.clone();
                                                                app.browse_path.push(item.name.clone()); 
                                                                app.library_selected = 0;
                                                                
                                                                if let Ok(pairs) = mpd.listfiles(&path_str) {
                                                                    let mut new_items: Vec<app::LibraryItem> = pairs.into_iter().map(|(kind, name)| {
                                                                        let full_path = if path_str == "/" { name.clone() } else { format!("{}/{}", path_str, name) };
                                                                        match kind.as_str() {
                                                                            "directory" => app::LibraryItem {
                                                                                name,
                                                                                item_type: app::LibraryItemType::Folder,
                                                                                artist: None, duration_ms: None, path: Some(full_path)
                                                                            },
                                                                            "file" => app::LibraryItem {
                                                                                name,
                                                                                item_type: app::LibraryItemType::Song,
                                                                                artist: None, duration_ms: None, path: Some(full_path)
                                                                            },
                                                                            _ => app::LibraryItem { name, item_type: app::LibraryItemType::Song, artist: None, duration_ms: None, path: None }
                                                                        }
                                                                    }).collect();
                                                                    new_items.sort_by(|a, b| a.name.cmp(&b.name));
                                                                    app.library_items = new_items;
                                                                }
                                                            }
                                                        },
                                                        
                                                        // Artist -> Album List
                                                        ("Artists", app::LibraryItemType::Artist) => {
                                                            let artist = item.name.clone();
                                                            app.browse_path.push(artist.clone());
                                                            app.library_selected = 0;
                                                            
                                                            let mut q = Query::new();
                                                            q.and(Term::Tag("Artist".into()), artist);
                                                            if let Ok(albums) = mpd.list(&Term::Tag("Album".into()), &q) {
                                                                 let mut i: Vec<app::LibraryItem> = albums.into_iter().map(|name| app::LibraryItem {
                                                                     name, item_type: app::LibraryItemType::Album,
                                                                     artist: None, duration_ms: None, path: None
                                                                 }).collect();
                                                                 i.sort_by(|a, b| a.name.cmp(&b.name));
                                                                 app.library_items = i;
                                                            }
                                                        },

                                                        // Album -> Song List
                                                        (_, app::LibraryItemType::Album) => {
                                                            let album = item.name.clone();
                                                            app.browse_path.push(album.clone());
                                                            app.library_selected = 0;
                                                            
                                                            let mut q = Query::new();
                                                            q.and(Term::Tag("Album".into()), album);
                                                            // Use None for window range (all)
                                                            if let Ok(songs) = mpd.find(&q, Option::<(u32, u32)>::None) {
                                                                 let mut i: Vec<app::LibraryItem> = songs.into_iter().map(|s| app::LibraryItem {
                                                                     name: s.title.clone().unwrap_or(s.file.clone()),
                                                                     item_type: app::LibraryItemType::Song,
                                                                     artist: Some(s.tags.iter().find(|(k, _)| k == "Artist").map(|(_, v)| v.clone()).unwrap_or_default()),
                                                                     duration_ms: s.duration.map(|d| d.as_secs() * 1000),
                                                                     path: Some(s.file)
                                                                 }).collect();
                                                                 app.library_items = i;
                                                            }
                                                        },

                                                        // Playing a Song (from Browse view) - Add and play immediately
                                                        (_, app::LibraryItemType::Song) => {
                                                            if let Some(path) = &item.path {
                                                                let song = mpd::Song { file: path.clone(), ..Default::default() };
                                                                // Add to queue and play immediately
                                                                if let Ok(id) = mpd.push(&song) {
                                                                    let _ = mpd.switch(id);
                                                                }
                                                            }
                                                        },
                                                        
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                        app::LibraryMode::Search => {
                                            // Add selected search result to queue and PLAY
                                            if let Some(item) = app.library_items.get(app.library_selected) {
                                                if let Some(path) = &item.path {
                                                    let song = mpd::Song { file: path.clone(), ..Default::default() };
                                                    if let Ok(id) = mpd.push(&song) {
                                                        let _ = mpd.switch(id);
                                                    }
                                                }
                                            }
                                        }
                                        app::LibraryMode::Playlists => {
                                            // Load selected playlist
                                            if let Some(pl) = app.playlists.get(app.library_selected) {
                                                let _ = mpd.load(pl, ..);
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        // Backspace to go back in Browse
                        KeyCode::Backspace | KeyCode::Esc if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Browse => {
                            app.browse_path.pop();
                            app.library_items.clear();
                            app.library_selected = 0;
                            
                            // Re-fetch items for the parent level if not root
                            if !app.browse_path.is_empty() {
                                #[cfg(feature = "mpd")]
                                if args.mpd {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        let root_cat = app.browse_path[0].as_str();
                                        
                                        // Level 1: Listing Categories (Artists, Albums, etc.)
                                        if app.browse_path.len() == 1 {
                                            let items = match root_cat {
                                                "Artists" => {
                                                    mpd.list(&Term::Tag("Artist".into()), &Query::new()).ok()
                                                        .map(|v| v.into_iter().map(|name| app::LibraryItem {
                                                            name, item_type: app::LibraryItemType::Artist, 
                                                            artist: None, duration_ms: None, path: None 
                                                        }).collect::<Vec<_>>())
                                                },
                                                "Albums" => {
                                                    mpd.list(&Term::Tag("Album".into()), &Query::new()).ok()
                                                        .map(|v| v.into_iter().map(|name| app::LibraryItem {
                                                            name, item_type: app::LibraryItemType::Album, 
                                                            artist: None, duration_ms: None, path: None 
                                                        }).collect::<Vec<_>>())
                                                },
                                                "Genres" => {
                                                    mpd.list(&Term::Tag("Genre".into()), &Query::new()).ok()
                                                        .map(|v| v.into_iter().map(|name| app::LibraryItem {
                                                            name, item_type: app::LibraryItemType::Genre, 
                                                            artist: None, duration_ms: None, path: None 
                                                        }).collect::<Vec<_>>())
                                                },
                                                "Folders" => {
                                                    mpd.listfiles("").ok().map(|files| {
                                                        files.into_iter().filter_map(|f| {
                                                            if f.0 == "file" || f.0 == "directory" {
                                                                let name = f.1;
                                                                Some(app::LibraryItem {
                                                                    name: name.clone(),
                                                                    item_type: app::LibraryItemType::Folder,
                                                                    artist: None, duration_ms: None, path: Some(name)
                                                                })
                                                            } else {
                                                                None
                                                            }
                                                        }).collect::<Vec<_>>()
                                                    })
                                                },
                                                _ => None,
                                            };
                                            
                                            if let Some(mut i) = items {
                                                i.sort_by(|a, b| a.name.cmp(&b.name));
                                                i.dedup_by(|a, b| a.name == b.name);
                                                app.library_items = i;
                                            }
                                        } 
                                        // Level 2: Inside a Category (Songs of Artist, etc.)
                                        else if app.browse_path.len() == 2 {
                                            let parent_name = app.browse_path[1].as_str();
                                            match root_cat {
                                                "Artists" => {
                                                    let mut q = Query::new();
                                                    q.and(Term::Tag("Artist".into()), parent_name);
                                                    if let Ok(albums) = mpd.list(&Term::Tag("Album".into()), &q) {
                                                         let mut i: Vec<app::LibraryItem> = albums.into_iter().map(|name| app::LibraryItem {
                                                             name, item_type: app::LibraryItemType::Album,
                                                             artist: None, duration_ms: None, path: None
                                                         }).collect();
                                                         i.sort_by(|a, b| a.name.cmp(&b.name));
                                                         app.library_items = i;
                                                    }
                                                },
                                                "Genres" => {
                                                     let mut q = Query::new();
                                                     q.and(Term::Tag("Genre".into()), parent_name);
                                                     if let Ok(artists) = mpd.list(&Term::Tag("Artist".into()), &q) {
                                                         let mut i: Vec<app::LibraryItem> = artists.into_iter().map(|name| app::LibraryItem {
                                                             name, item_type: app::LibraryItemType::Artist,
                                                             artist: None, duration_ms: None, path: None
                                                         }).collect();
                                                         i.sort_by(|a, b| a.name.cmp(&b.name));
                                                         app.library_items = i;
                                                     }
                                                },
                                                "Folders" => {
                                                    if let Ok(files) = mpd.listfiles(parent_name) {
                                                        let mut i: Vec<app::LibraryItem> = files.into_iter().filter_map(|f| {
                                                            if f.0 == "file" || f.0 == "directory" {
                                                                let name = f.1;
                                                                Some(app::LibraryItem {
                                                                    name: name.split('/').last().unwrap_or(&name).to_string(),
                                                                    item_type: app::LibraryItemType::Folder,
                                                                    artist: None, duration_ms: None, path: Some(name)
                                                                })
                                                            } else {
                                                                None
                                                            }
                                                        }).collect();
                                                        i.sort_by(|a, b| a.name.cmp(&b.name));
                                                        app.library_items = i;
                                                    }
                                                },
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        // Navigation keys for Library view (use Up/Down arrows)
                        KeyCode::Up if app.view_mode == app::ViewMode::Library => {
                            app.library_selected = app.library_selected.saturating_sub(1);
                        },
                        KeyCode::Down if app.view_mode == app::ViewMode::Library => {
                            let max_items = match app.library_mode {
                                app::LibraryMode::Queue => app.queue.len().max(1),
                                app::LibraryMode::Browse if app.browse_path.is_empty() => 4,
                                app::LibraryMode::Playlists => app.playlists.len().max(1),
                                _ => app.library_items.len().max(1),
                            };
                            if app.library_selected < max_items.saturating_sub(1) {
                                app.library_selected += 1;
                            }
                        },
                        _ => {}
                        }
                    }
                },
                AppEvent::Input(_) => {},
                
                AppEvent::TrackUpdate(info) => {
                    app.track = info.clone();
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
                                
                                let client = client.clone();
                                tokio::spawn(async move {
                                    let fetcher = LyricsFetcher::new(client);
                                    use crate::lyrics::LyricsFetchResult;
                                    match fetcher.fetch(&artist, &name, dur).await {
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
                         app.lyrics_cache.insert(id.clone(), l.clone());
                    }
                    
                    // Only update UI if we are still on the same song
                    if id == last_track_id {
                         app.lyrics = state;
                    }
                },
                AppEvent::ArtworkUpdate(data) => app.artwork = data,
                AppEvent::ThemeUpdate(new_theme) => app.theme = new_theme,
                AppEvent::QueueUpdate(queue_data) => {
                    app.queue = queue_data.into_iter().map(|(title, artist, duration_ms, is_current, file_path)| {
                        app::QueueItem { title, artist, duration_ms, is_current, file_path }
                    }).collect();
                },
                AppEvent::CavaUpdate(bars) => {
                    // Update visualizer with real cava data
                    app.visualizer_bars = bars;
                },
                AppEvent::Tick => {
                    if app.last_scroll_time.is_none() && app.lyrics_offset.is_some() {
                        if let (LyricsState::Loaded(lyrics), Some(track)) = (&app.lyrics, &app.track) {
                            // 1. Calculate Target
                            let target_idx = lyrics.iter()
                               .position(|l| l.timestamp_ms > track.position_ms)
                               .map(|i| if i > 0 { i - 1 } else { 0 })
                               .unwrap_or(0);
                            
                            // 2. Animate Offset
                            if let Some(curr) = &mut app.lyrics_offset {
                                if *curr < target_idx {
                                    *curr += 1;
                                } else if *curr > target_idx {
                                    *curr -= 1;
                                } else {
                                    // Reached target - reset selection too!
                                    app.lyrics_offset = None;
                                    app.lyrics_selected = None; // Clear selection for fresh start
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if !app.is_running { break; }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
