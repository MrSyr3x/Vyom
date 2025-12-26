use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use futures::{StreamExt};

use image::DynamicImage;

mod app;
mod artwork;
mod lyrics;
mod spotify;
mod theme;
mod ui;

use app::{App};
use spotify::{Spotify, TrackInfo};
use lyrics::{LyricsFetcher}; 
use artwork::{ArtworkRenderer}; 
use theme::{Theme}; // Import Theme struct

enum AppEvent {
    Input(Event),
    TrackUpdate(Option<TrackInfo>),
    LyricsUpdate(Option<Vec<lyrics::LyricLine>>),
    ArtworkUpdate(Option<DynamicImage>),
    ThemeUpdate(Theme),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let is_standalone = args.iter().any(|a| a == "--standalone");
    let is_tmux = std::env::var("TMUX").is_ok();

    // Smart Window Logic
    let want_lyrics = args.iter().any(|a| a == "--lyrics");
    
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.to_str().unwrap();

    // 1. WINDOW TITLE (For Yabai/Amethyst) 🏷️
    print!("\x1b]2;Vyom\x07");

    // 2. TMUX LOGIC
    if is_tmux && !is_standalone {
        // Auto-split logic (Tmux)
        let status = std::process::Command::new("tmux")
            .arg("split-window")
            .arg("-h")
            .arg("-p")
            .arg("29") // Changed from "33" to "29"
            .arg(format!("{} --standalone {}", exe_path, if want_lyrics { "--lyrics" } else { "" }))
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // In Tmux, we assume full split/window, so show lyrics by default.
    // In Standalone, strict mode applies (Mini unless --lyrics).
    let app_show_lyrics = want_lyrics || is_tmux;
    let mut app = App::new(app_show_lyrics);
    let (tx, mut rx) = mpsc::channel(100);

    // 1. Input Event Task
    let tx_input = tx.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if tx_input.send(AppEvent::Input(event)).await.is_err() { break; }
        }
    });

    // 2. Spotify Polling Task
    let tx_spotify = tx.clone();
    tokio::spawn(async move {
        loop {
            let track_result = tokio::task::spawn_blocking(Spotify::get_current_track).await;
            if let Ok(Ok(info)) = track_result {
                 if tx_spotify.send(AppEvent::TrackUpdate(info)).await.is_err() { break; }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // 3. Theme Watcher Task 🎨
    let tx_theme = tx.clone();
    tokio::spawn(async move {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        // Watch the actual config file now
        let config_path = std::path::PathBuf::from(&home).join(".config/vyom/theme.toml");
        let mut last_content = String::new();
        
        // Initial read
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            last_content = content;
        }

        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if content != last_content {
                    last_content = content;
                    // Reload theme fully
                    let new_theme = theme::load_current_theme();
                    if tx_theme.send(AppEvent::ThemeUpdate(new_theme)).await.is_err() { break; }
                }
            }
        }
    });

    let mut last_track_id = String::new();
    let mut last_artwork_url = None;

    loop {
        terminal.draw(|f| ui::ui(f, &mut app))?;

        if let Some(event) = rx.recv().await {
            match event {
                // ... (Input handling omitted)
                AppEvent::Input(Event::Mouse(mouse)) => {
                     // ... same as before
                     use crossterm::event::{MouseEventKind, MouseButton};
                     match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                             let (col, row) = (mouse.column, mouse.row);
                            // ...
                            let mut hit_lyrics = false;
                            for (rect, timestamp) in &app.lyrics_hitboxes {
                                if rect.contains((col, row).into()) {
                                    let seconds = *timestamp as f64 / 1000.0;
                                    let _ = tokio::task::block_in_place(|| {
                                         crate::spotify::Spotify::seek(seconds)
                                    });
                                    if let Some(track) = &mut app.track {
                                        track.position_ms = *timestamp;
                                    }
                                    hit_lyrics = true;
                                    app.lyrics_offset = None; 
                                    break;
                                }
                            }
                            
                            if !hit_lyrics {
                                app.handle_click(col, row);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if let (Some(lyrics), Some(track)) = (&app.lyrics, &app.track) {
                                if app.lyrics_offset.is_none() {
                                    let current_idx = lyrics.iter()
                                       .position(|l| l.timestamp_ms > track.position_ms)
                                       .map(|i| if i > 0 { i - 1 } else { 0 })
                                       .unwrap_or(0);
                                     app.lyrics_offset = Some(current_idx);
                                }
                                
                                if let Some(off) = &mut app.lyrics_offset {
                                    *off = off.saturating_add(1).min(lyrics.len().saturating_sub(1));
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                             if let (Some(lyrics), Some(track)) = (&app.lyrics, &app.track) {
                                if app.lyrics_offset.is_none() {
                                     let current_idx = lyrics.iter()
                                       .position(|l| l.timestamp_ms > track.position_ms)
                                       .map(|i| if i > 0 { i - 1 } else { 0 })
                                       .unwrap_or(0);
                                     app.lyrics_offset = Some(current_idx);
                                }
                                
                                if let Some(off) = &mut app.lyrics_offset {
                                    *off = off.saturating_sub(1);
                                }
                             }
                        }
                        _ => {}
                    }
                },
                AppEvent::Input(Event::Key(key)) => {
                    match key.code {
                        KeyCode::Char('q') => app.is_running = false,
                        KeyCode::Char(' ') => { let _ = Spotify::play_pause(); },
                        KeyCode::Char('n') => { let _ = Spotify::next(); },
                        KeyCode::Char('p') => { let _ = Spotify::prev(); },
                        KeyCode::Char('+') | KeyCode::Char('=') => { let _ = Spotify::volume_up(); },
                        KeyCode::Char('-') | KeyCode::Char('_') => { let _ = Spotify::volume_down(); },
                        _ => {}
                    }
                },
                AppEvent::Input(_) => {},
                
                AppEvent::TrackUpdate(info) => {
                    app.track = info.clone();
                    if let Some(track) = info {
                        let id = format!("{}{}", track.name, track.artist);
                        if id != last_track_id {
                            last_track_id = id.clone();
                            app.lyrics = None;
                            let tx_lyrics = tx.clone();
                            let (artist, name, dur) = (track.artist.clone(), track.name.clone(), track.duration_ms);
                            tokio::spawn(async move {
                                let fetcher = LyricsFetcher::new();
                                if let Ok(lyrics) = fetcher.fetch(&artist, &name, dur).await {
                                    let _ = tx_lyrics.send(AppEvent::LyricsUpdate(lyrics)).await;
                                }
                            });
                        }
                        if track.artwork_url != last_artwork_url {
                            last_artwork_url = track.artwork_url.clone();
                            app.artwork = None;
                            if let Some(url) = last_artwork_url.clone() {
                                let tx_art = tx.clone();
                                tokio::spawn(async move {
                                    let renderer = ArtworkRenderer::new();
                                    // Fetch RAW image. Resize happens in UI.
                                    if let Ok(img) = renderer.fetch_image(&url).await {
                                        let _ = tx_art.send(AppEvent::ArtworkUpdate(Some(img))).await;
                                    }
                                });
                            }
                        }
                    } else {
                        last_track_id.clear();
                        last_artwork_url = None;
                    }
                },
                AppEvent::LyricsUpdate(lyrics) => app.lyrics = lyrics,
                AppEvent::ArtworkUpdate(data) => app.artwork = data,
                AppEvent::ThemeUpdate(new_theme) => app.theme = new_theme,
            }
        }
        
        if !app.is_running { break; }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
