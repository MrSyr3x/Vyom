use crate::app::cli::Args;
use crate::app::events::AppEvent;
use crate::app::lyrics::LyricsFetcher;
use crate::app::{App, ArtworkState, LyricsState};
use crate::artwork::ArtworkRenderer;
use crate::audio::pipeline::AudioPipeline;
use crate::player::PlayerTrait;
use crate::ui;

use crossterm::event::Event;
use ratatui::{backend::Backend, Terminal};
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn run_app<B: Backend>(
    app: &mut App,
    terminal: &mut Terminal<B>,
    player: &Arc<dyn PlayerTrait>,
    audio_pipeline: &mut AudioPipeline,
    args: &Args,
    tx: mpsc::Sender<AppEvent>,
    mut rx: mpsc::Receiver<AppEvent>,
    client: reqwest::Client,
) -> anyhow::Result<()>
where
    <B as Backend>::Error: std::error::Error + Send + Sync + 'static,
{
    let mut last_track_id = String::new();
    let mut last_artwork_url = None;
    let mut last_view_mode = app.view_mode.clone();

    loop {
        // Auto-Reset Lyrics Scroll Logic
        if let Some(t) = app.last_scroll_time {
            if t.elapsed().as_secs() >= 3 {
                app.last_scroll_time = None;
            }
        }

        // Update visualizer bars 60fps (called before draw)
        if app.view_mode == crate::app::ViewMode::Visualizer {
            app.visualizer_bars = app.visualizer.get_bars(64);
        }

        // --- SEAMLESS POPUP OVERLAY FIX ---
        let has_popup = app.show_keyhints
            || app.show_audio_info
            || app.input_state.is_some()
            || app.tag_edit.is_some();

        let popup_closed = !has_popup && app.had_popup_last_frame;
        let view_changed = app.view_mode != last_view_mode;

        if popup_closed || view_changed {
            terminal.clear()?;
        }

        last_view_mode = app.view_mode.clone();
        app.had_popup_last_frame = has_popup;

        // Reactive Rendering: Only draw if state was actually mutated
        if app.needs_redraw {
            terminal.draw(|f| ui::ui(f, app))?;
            app.needs_redraw = false; // Reset flag after a successful draw
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                app.is_running = false;
            }
            Some(event) = rx.recv() => {
                match event {
                AppEvent::Input(Event::Mouse(_)) => {},
                AppEvent::Input(Event::Resize(_, _)) => {
                    terminal.clear()?;
                    app.image_protocol = None;
                    app.needs_redraw = true;
                },
                AppEvent::Input(Event::Key(key)) => {
                    if matches!(key.code, crossterm::event::KeyCode::Modifier(_)) {
                        continue;
                    }
                    crate::app::inputs::handle_event(key, app, player, audio_pipeline, args, &tx, &client).await;
                    app.needs_redraw = true;
                },
                AppEvent::Input(_) => {},

                AppEvent::TrackUpdate(info) => {
                    app.track = info.clone();
                    app.last_track_update = Some(std::time::Instant::now());
                    if let Some(track) = info {
                        let mpd_volume_bug = track.source == "MPD" && track.volume.unwrap_or(0) == 0 && app.app_volume > 0;

                        let ignore_sync = app.last_volume_action
                            .map(|t| t.elapsed() < std::time::Duration::from_millis(1000))
                            .unwrap_or(false)
                            || mpd_volume_bug;

                        if !ignore_sync {
                            if let Some(vol) = track.volume {
                                let new_vol = (vol as u8).min(100);
                                if (app.app_volume as i16 - new_vol as i16).abs() > 1 {
                                    app.app_volume = new_vol;
                                    audio_pipeline.set_volume(new_vol);
                                }
                            }
                        }

                        let id = format!("{}{}", track.name, track.artist);

                        if !track.album.is_empty() && !app.last_album.is_empty() {
                            app.gapless_mode = track.album == app.last_album;
                        } else {
                            app.gapless_mode = false;
                        }
                        app.last_album = track.album.clone();

                        if id != last_track_id {
                            last_track_id = id.clone();
                            app.lyrics = LyricsState::Loading;

                            app.lyrics_offset = None;
                            app.last_scroll_time = None;
                            app.seek_accumulator = 0.0;
                            app.seek_initial_pos = None;
                            app.last_seek_time = None;
                            app.needs_redraw = true;

                            if let Some(cached) = app.lyrics_cache.get(&id) {
                                app.lyrics = LyricsState::Loaded(cached.clone(), "Memory Cache".to_string());
                            } else {
                                let tx_lyrics = tx.clone();
                                let (artist, name, dur) = (track.artist.clone(), track.name.clone(), track.duration_ms);
                                let fetch_id = id.clone();
                                let file_path = track.file_path.clone();

                                let client = client.clone();
                                tokio::spawn(async move {
                                    let fetcher = LyricsFetcher::new(client);
                                    use crate::app::lyrics::LyricsFetchResult;
                                    match fetcher.fetch(&artist, &name, dur, file_path.as_ref()).await {
                                        Ok(LyricsFetchResult::Found(lyrics, source)) => {
                                            if let Err(e) = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::Loaded(lyrics, source))).await { tracing::debug!("Channel closed: {}", e); }
                                        },
                                        Ok(LyricsFetchResult::Instrumental) => {
                                             if let Err(e) = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::Instrumental)).await { tracing::debug!("Channel closed: {}", e); }
                                        },
                                        Ok(LyricsFetchResult::None) => {
                                             if let Err(e) = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::NotFound)).await { tracing::debug!("Channel closed: {}", e); }
                                        }
                                        Err(e) => {
                                            if let Err(err) = tx_lyrics.send(AppEvent::LyricsUpdate(fetch_id, LyricsState::Failed(e.to_string()))).await { tracing::debug!("Channel closed: {}", err); }
                                        }
                                    }
                                });
                            }

                            app.needs_redraw = true;

                            if track.source == "Music" && track.artwork_url.is_none() {
                                app.artwork = ArtworkState::Loading;
                                let tx_art = tx.clone();
                                let (artist, album) = (track.artist.clone(), track.album.clone());
                                let client = client.clone();
                                let fetch_id = id.clone();
                                tokio::spawn(async move {
                                    let renderer = ArtworkRenderer::new(client);
                                    match renderer.fetch_itunes_artwork(&artist, &album).await {
                                        Ok(url) => {
                                             match renderer.fetch_image(&url).await {
                                                 Ok(img) => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id, ArtworkState::Loaded(img))).await { tracing::debug!("Channel closed: {}", e); } },
                                                 Err(_) => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id.clone(), ArtworkState::Failed)).await { tracing::debug!("Channel closed: {}", e); } }
                                             }
                                        },
                                        Err(_) => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id, ArtworkState::Failed)).await { tracing::debug!("Channel closed: {}", e); } }
                                    }
                                });
                            }

                            #[cfg(feature = "mpd")]
                            if track.source == "MPD" {
                                if let Some(file_path) = &track.file_path {
                                    app.artwork = ArtworkState::Loading;
                                    let tx_art = tx.clone();
                                    let fp = file_path.clone();
                                    let fetch_id = id.clone();
                                    tokio::spawn(async move {
                                        let result = tokio::task::spawn_blocking(move || {
                                            ArtworkRenderer::extract_embedded_art(&fp)
                                        }).await;

                                        match result {
                                            Ok(Ok(img)) => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id, ArtworkState::Loaded(img))).await { tracing::debug!("Channel closed: {}", e); } },
                                            _ => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id, ArtworkState::Failed)).await { tracing::debug!("Channel closed: {}", e); } }
                                        }
                                    });
                                }
                            }
                        }

                        if let Some(url) = track.artwork_url.clone() {
                            if Some(url.clone()) != last_artwork_url {
                                last_artwork_url = Some(url.clone());
                                app.artwork = ArtworkState::Loading;
                                let tx_art = tx.clone();
                                let client = client.clone();
                                let fetch_id = id.clone();
                                tokio::spawn(async move {
                                    let renderer = ArtworkRenderer::new(client);
                                    match renderer.fetch_image(&url).await {
                                         Ok(img) => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id, ArtworkState::Loaded(img))).await { tracing::debug!("Channel closed: {}", e); } },
                                         Err(_) => { if let Err(e) = tx_art.send(AppEvent::ArtworkUpdate(fetch_id, ArtworkState::Failed)).await { tracing::debug!("Channel closed: {}", e); } }
                                    }
                                });
                            }
                        }
                    } else {
                        last_track_id.clear();
                        last_artwork_url = None;
                        app.artwork = ArtworkState::Idle;
                    }
                    app.needs_redraw = true;
                },
                AppEvent::LyricsUpdate(id, state) => {
                    if let LyricsState::Loaded(ref l, _) = state {
                         if app.lyrics_cache.len() > 50 {
                             if let Some(oldest_key) = app.lyrics_cache.keys().next().cloned() {
                                 app.lyrics_cache.remove(&oldest_key);
                             }
                         }
                         app.lyrics_cache.insert(id.clone(), l.clone());
                    }

                    if id == last_track_id {
                         app.lyrics = state;
                         app.needs_redraw = true;
                    }
                },
                AppEvent::ArtworkUpdate(id, data) => {
                    if id == last_track_id {
                        app.artwork = data;
                        app.image_protocol = None;
                        app.needs_redraw = true;
                    }
                },
                AppEvent::ThemeUpdate(new_theme) => {
                    app.theme = new_theme;
                    app.needs_redraw = true;
                },
                AppEvent::KeyConfigUpdate(new_keys) => {
                    app.keys = *new_keys;
                    app.show_toast("🔧 Config Reloaded");
                },
                AppEvent::QueueUpdate(queue_data) => {
                    app.queue = queue_data.into_iter().map(|(title, artist, duration_ms, is_current, file_path)| {
                        crate::app::QueueItem { title, artist, duration_ms, is_current, file_path }
                    }).collect();
                    app.needs_redraw = true;
                },

                AppEvent::StatusUpdate(shuffle, repeat) => {
                    app.shuffle = shuffle;
                    app.repeat = repeat;
                    app.needs_redraw = true;
                },

                AppEvent::ToastUpdate(msg) => {
                    app.show_toast(&msg);
                    app.needs_redraw = true;
                },

                AppEvent::Tick => {
                    app.on_tick();
                    app.tick_count = app.tick_count.wrapping_add(1);

                    let mut is_playing = false;
                    if app.track.is_some() {
                        is_playing = true;
                    }

                    let is_animating_lyrics = app.last_scroll_time.is_none() && (app.lyrics_offset.is_some() || app.lyrics_selected.is_some());
                    let has_active_toast = app.toast.is_some();
                    let needs_high_fps = app.view_mode == crate::app::ViewMode::Visualizer || is_animating_lyrics || has_active_toast;

                    if needs_high_fps {
                        app.needs_redraw = true;
                    } else if is_playing && app.tick_count % 30 == 0 {
                        app.needs_redraw = true;
                    }

                    if is_animating_lyrics {
                        if let (LyricsState::Loaded(lyrics, _), Some(_track)) = (&app.lyrics, &app.track) {
                            let target_idx = lyrics.iter()
                               .position(|l| l.timestamp_ms > app.get_current_position_ms())
                               .map(|i| i.saturating_sub(1))
                               .unwrap_or(lyrics.len().saturating_sub(1));

                            app.smooth_scroll_accum += 0.016;

                            if app.smooth_scroll_accum >= 0.05 {
                                let mut done_offset = false;
                                let mut done_selected = false;

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

                                if let Some(curr_sel) = &mut app.lyrics_selected {
                                    if *curr_sel < target_idx {
                                        *curr_sel += 1;
                                    } else if *curr_sel > target_idx {
                                        *curr_sel -= 1;
                                    } else {
                                        done_selected = true;
                                    }
                                } else {
                                    done_selected = true;
                                }

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

    Ok(())
}
