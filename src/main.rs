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

mod app;
mod artwork;
mod lyrics;
mod spotify;
mod ui;

use app::{App};
use spotify::{Spotify, TrackInfo};
use lyrics::{LyricsFetcher}; 
use artwork::{ArtworkRenderer, ArtworkData};

enum AppEvent {
    Input(Event),
    TrackUpdate(Option<TrackInfo>),
    LyricsUpdate(Option<Vec<lyrics::LyricLine>>),
    ArtworkUpdate(Option<ArtworkData>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let is_standalone = args.iter().any(|a| a == "--standalone");
    let is_tmux = std::env::var("TMUX").is_ok();

    // Smart Window Logic
    let want_lyrics = args.iter().any(|a| a == "--lyrics");
    
    if is_tmux && !is_standalone {
        // Auto-split logic (Tmux)
        let current_exe = std::env::current_exe()?;
        let exe_path = current_exe.to_str().unwrap();
        
        let status = std::process::Command::new("tmux")
            .arg("split-window")
            .arg("-h")
            .arg("-p")
            .arg("35")
            .arg(format!("{} --standalone {}", exe_path, if want_lyrics { "--lyrics" } else { "" }))
            .status();

        match status {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("Failed to create tmux split: {}", e);
            }
        }
    } else if !is_tmux {
        // Standalone Resize Logic (e.g. iTerm/Terminal/Ghostty)
        // \x1b[8;ROWS;COLSt
        if want_lyrics {
            print!("\x1b[8;60;80t"); // Full Size: 60 rows, 80 cols
        } else {
            print!("\x1b[8;33;51t"); // Mini Size: 33 rows, 51 cols
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
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
    
    let mut last_track_id = String::new();
    let mut last_artwork_url = None;

    loop {
        terminal.draw(|f| ui::ui(f, &mut app))?;

        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::Input(Event::Mouse(mouse)) => {
                    use crossterm::event::{MouseEventKind, MouseButton};
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                             let (col, row) = (mouse.column, mouse.row);
                            // Checks for lyrics hitboxes (Jump -> Auto Sync)
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
                                    // Reset manual scroll on click!!
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
                                // Initialize offset if None
                                if app.lyrics_offset.is_none() {
                                    // Calculate current auto position to start scrolling from it
                                    // Logic duplicated from ui.rs (approximate) or simple:
                                    // We need to know where we ARE to scroll down from it.
                                    // UI logic: current_idx - half_height.
                                    // We don't have height here easily.
                                    // Let's assume a default height or just start from current_idx?
                                    // Better: UI should persist "visual_start_index" back to App?
                                    // Or we just default to current_idx.
                                    let current_idx = lyrics.iter()
                                       .position(|l| l.timestamp_ms > track.position_ms)
                                       .map(|i| if i > 0 { i - 1 } else { 0 })
                                       .unwrap_or(0);
                                     // Roughly center it? 
                                     // Let's just set it to current_idx. 
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
                                       
                                     // If we scroll up from auto-mode, we probably want to start a bit higher than current line?
                                     // Or just current line.
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
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Char(' ') => { let _ = Spotify::play_pause(); },
                        KeyCode::Char('n') => { let _ = Spotify::next(); },
                        KeyCode::Char('p') => { let _ = Spotify::prev(); },
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
                                    // Maximize resolution for Length(24) constraint.
                                    // 24 rows * 2 subpixels = 48 vertical pixels.
                                    // Square aspect ratio = 48 width roughly.
                                    if let Ok(data) = renderer.render_from_url(&url, 48, 24).await {
                                        let _ = tx_art.send(AppEvent::ArtworkUpdate(Some(data))).await;
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
            }
        }
        
        if app.should_quit { break; }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
