use crossterm::event::{KeyCode, KeyModifiers};
use crate::app::{self, App, LyricsState};
use crate::player::PlayerTrait;
use crate::audio::pipeline::AudioPipeline;
use crate::app::cli::Args;
use crate::app::events::AppEvent;
#[cfg(feature = "mpd")]
use crate::app::library_helpers::fetch_directory_items;
#[cfg(feature = "mpd")]
use crate::mpd_player;
#[cfg(feature = "mpd")]
use lofty::file::TaggedFileExt;
#[cfg(feature = "mpd")]
use lofty::tag::Accessor;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use tokio::sync::mpsc;
use std::sync::Arc;
use reqwest::Client;

pub async fn handle_input(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    player: &Arc<dyn PlayerTrait>,
    audio_pipeline: &mut AudioPipeline,
    args: &Args,
    _tx: &mpsc::Sender<AppEvent>,
    _client: &Client,
) {
                    if app.input_state.is_some() {
                        match key.code {
                            KeyCode::Esc => {
                                app.input_state = None;
                            },
                            KeyCode::Enter => {
                                // Consume input state (Take ownership, releasing app borrow)
                                if let Some(input) = app.input_state.take() {
                                    match input.mode {
                                        app::InputMode::PlaylistSave => {
                                            if !input.value.is_empty() {
                                                #[cfg(feature = "mpd")]
                                                {
                                                    let player = mpd_player::MpdPlayer::new(&args.mpd_host, args.mpd_port);
                                                    if let Err(e) = player.save_playlist(&input.value) {
                                                        app.show_toast(&format!("❌ Error: {}", e));
                                                    } else {
                                                        app.show_toast(&format!("💾 Saved: {}", input.value));
                                                        app.playlists.push(input.value.clone());
                                                    }
                                                }
                                            }
                                        },

                                        app::InputMode::EqSave => {
                                            if !input.value.is_empty() {
                                                app.save_preset(input.value.clone());
                                                app.show_toast(&format!("💾 Preset Saved: {}", input.value));
                                            }
                                        },

                                        app::InputMode::PlaylistRename(old_name) => {
                                            if !input.value.is_empty() {
                                                #[cfg(feature = "mpd")]
                                                {
                                                     let player = mpd_player::MpdPlayer::new(&args.mpd_host, args.mpd_port);
                                                     if let Err(e) = player.rename_playlist(&old_name, &input.value) {
                                                         app.show_toast(&format!("❌ Error: {}", e));
                                                     } else {
                                                         app.show_toast(&format!("✏️ Renamed: {} -> {}", old_name, input.value));
                                                         // Refresh playlists
                                                         if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                                             if let Ok(playlists) = mpd.playlists() {
                                                                 app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                                                                 // Update library items if in Playlist mode
                                                                 if app.library_mode == app::LibraryMode::Playlists {
                                                                     app.library_items = app.playlists.iter().map(|p| app::LibraryItem {
                                                                         name: p.clone(),
                                                                         item_type: app::LibraryItemType::Playlist,
                                                                         artist: None, duration_ms: None, path: None
                                                                     }).collect();
                                                                 }
                                                             }
                                                         }
                                                     }
                                                }
                                            }
                                        },


                                    }
                                }
                            },
                            KeyCode::Backspace => {
                                if let Some(input) = app.input_state.as_mut() {
                                    input.value.pop();
                                }
                            },
                            KeyCode::Char(c) => {
                                if let Some(input) = app.input_state.as_mut() {
                                    input.value.push(c);
                                }
                            },
                            _ => {}
                        }
                        // Consume event, don't propagate
                        return;
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
                                        // MPD music directory from config
                                        let music_dir = &app.music_directory;
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
                        return; // Don't process other key handlers while tag editor is open
                    }
                    // When search is active, capture ALL character input (except special keys)
                    else if app.search_active {
                        match key.code {
                            KeyCode::Esc => {
                                app.search_active = false;
                                // Restore previous mode or default to Directory
                                let target_mode = app.previous_library_mode.take().unwrap_or(app::LibraryMode::Directory);
                                app.library_mode = target_mode;
                                app.search_query.clear();

                                // Reset selection
                                app.library_selected = 0;

                                #[cfg(feature = "mpd")]
                                if !args.controller {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        match target_mode {
                                            app::LibraryMode::Directory => {
                                                // Restore Directory view (refresh from current path)
                                                let current_path = app.browse_path.join("/");
                                                if let Ok(items) = fetch_directory_items(&mut mpd, &current_path) {
                                                    app.library_items = items;
                                                }
                                            },
                                            app::LibraryMode::Playlists => {
                                                // Refresh playlists
                                                if let Ok(playlists) = mpd.playlists() {
                                                    app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                                                    app.library_items = app.playlists.iter().map(|p| app::LibraryItem {
                                                        name: p.clone(),
                                                        item_type: app::LibraryItemType::Playlist,
                                                        artist: None, duration_ms: None, path: None
                                                    }).collect();
                                                }
                                            },
                                            app::LibraryMode::Queue => {
                                                // Queue is updated via idle/status logic, but we can ensure items match queue
                                                // Repopulating library_items from queue is optional if UI uses app.queue directly
                                                // But usually Queue view uses app.queue.
                                                // ensure library items is cleared so we dont show search results in queue mode
                                                app.library_items.clear();
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                            },


                            KeyCode::Backspace => { app.search_query.pop(); },
                            KeyCode::Enter => {
                                app.search_active = false;
                                // Perform MPD search
                                #[cfg(feature = "mpd")]
                                if !args.controller && !app.search_query.is_empty() {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        if let Ok(songs) = mpd.listall() {
                                            let matcher = SkimMatcherV2::default();
                                            // Fuzzy Match 🔍
                                            let mut matched_items: Vec<(i64, mpd::Song)> = songs.into_iter()
                                                .filter_map(|s| {
                                                    let search_text = format!("{} {} {}",
                                                        s.title.as_deref().unwrap_or(""),
                                                        s.artist.as_deref().unwrap_or(""),
                                                        s.file
                                                    );
                                                    matcher.fuzzy_match(&search_text, &app.search_query).map(|score| (score, s))
                                                })
                                                .collect();

                                            // Sort by score (descending)
                                            matched_items.sort_by(|a, b| b.0.cmp(&a.0));

                                            app.library_items = matched_items.into_iter()
                                                .take(50)
                                                .map(|(_, s)| app::LibraryItem {
                                                    name: s.title.clone().unwrap_or_else(|| s.file.clone()),
                                                    item_type: app::LibraryItemType::Song,
                                                    artist: s.artist.clone().or_else(|| s.tags.iter().find(|(k,_)| k == "Artist").map(|(_,v)| v.clone())),
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
                            // Global search: / to jump to search from anywhere (MPD only)
                            #[cfg(feature = "mpd")]
                            KeyCode::Char('/') if !args.controller => {
                                app.view_mode = app::ViewMode::Library;
                                // Save current mode before switching to Search, unless we are already searching
                                if app.library_mode != app::LibraryMode::Search {
                                    app.previous_library_mode = Some(app.library_mode);
                                }
                                app.library_mode = app::LibraryMode::Search;
                                app.search_active = true;
                            },
                            KeyCode::Char('q') => {
                                // Close popups first, then quit (Neovim-style)
                                if app.show_keyhints {
                                    app.show_keyhints = false;
                                } else if app.show_audio_info {
                                    app.show_audio_info = false;
                                } else {
                                    app.is_running = false;
                                }
                            },
                            KeyCode::Char(' ') => {
                                if let Ok(is_playing) = player.play_pause() {
                                    app.show_toast(if is_playing { "▶ Play" } else { "⏸ Pause" });
                                }
                            },
                            KeyCode::Char('n') => {
                                let _ = player.next();
                                app.show_toast("⏭ Next Track");
                            },
                            KeyCode::Char('p') => {
                                let _ = player.prev();
                                app.show_toast("⏮ Previous Track");
                            },
                            KeyCode::Char('+') => {
                                // Hardware/MPD Volume
                                let _ = player.volume_up();

                                // Software Gain (Hi-Res Pipeline) 🎚️
                                app.app_volume = (app.app_volume.saturating_add(5)).min(100);
                                audio_pipeline.set_volume(app.app_volume);

                                app.show_toast(&format!("🔊 Volume: {}%", app.app_volume));
                            },
                            KeyCode::Char('-') => {
                                // Hardware/MPD Volume
                                let _ = player.volume_down();

                                // Software Gain (Hi-Res Pipeline) 🎚️
                                app.app_volume = app.app_volume.saturating_sub(5);
                                audio_pipeline.set_volume(app.app_volume);

                                app.show_toast(&format!("🔉 Volume: {}%", app.app_volume));
                            },
                            // Seek Controls (cumulative & safe) ⏩
                            // Enable in Library view too (user request)
                            // Only blocked in EQ View (uses h/l for nav)
                            KeyCode::Char('h') if app.view_mode != app::ViewMode::EQ => {
                                let now = std::time::Instant::now();
                                let is_new_sequence = if let Some(last) = app.last_seek_time {
                                    now.duration_since(last).as_millis() >= 500
                                } else { true };

                                if is_new_sequence {
                                    if let Some(_track) = &app.track {
                                        // Start seek from CURRENT interpolated position
                                        app.seek_initial_pos = Some(app.get_current_position_ms() as f64 / 1000.0);
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
                                    app.show_toast(&format!("⏪ Seek: {:+.0}s", app.seek_accumulator));
                                }
                            },
                            KeyCode::Char('l') if app.view_mode != app::ViewMode::EQ => {
                                let now = std::time::Instant::now();
                                let is_new_sequence = if let Some(last) = app.last_seek_time {
                                    now.duration_since(last).as_millis() >= 500
                                } else { true };

                                if is_new_sequence {
                                    if let Some(_track) = &app.track {
                                        // Start seek from CURRENT interpolated position
                                        app.seek_initial_pos = Some(app.get_current_position_ms() as f64 / 1000.0);
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
                                    app.show_toast(&format!("⏩ Seek: {:+.0}s", app.seek_accumulator));
                                }
                            },

                            // Queue Reordering with J/K (Shift+j/k) 🔄
                            KeyCode::Char('J') if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                                if app.library_selected < app.queue.len().saturating_sub(1) {
                                    #[cfg(feature = "mpd")]
                                    if !args.controller {
                                        if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                            let current_pos = app.library_selected as u32;
                                            let new_pos = current_pos + 1;

                                            if mpd.shift(current_pos, new_pos as usize).is_ok() {
                                                 app.library_selected = new_pos as usize;
                                            }
                                        }
                                    }
                                }
                            },
                            KeyCode::Char('K') if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                                if app.library_selected > 0 {
                                    #[cfg(feature = "mpd")]
                                    if !args.controller {
                                        if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                            let current_pos = app.library_selected as u32;
                                            let new_pos = current_pos - 1;
                                            if mpd.shift(current_pos, new_pos as usize).is_ok() {

                                                 app.library_selected = new_pos as usize;
                                            }
                                        }
                                    }
                                }
                            },

                            // Custom EQ Preset Management
                            // Save Preset: Shift+S
                            KeyCode::Char('S') if app.view_mode == app::ViewMode::EQ => {
                                app.input_state = Some(app::InputState::new(
                                    app::InputMode::EqSave,
                                    "Save Preset As",
                                    ""
                                ));
                            },
                            // Delete Preset: Shift+X (protected defaults in app impl)
                            KeyCode::Char('X') if app.view_mode == app::ViewMode::EQ => {
                                if let Err(e) = app.delete_preset() {
                                    app.show_toast(&format!("❌ {}", e));
                                } else {
                                    app.show_toast("🗑️ Preset Deleted");
                                }
                            },
                            // View Mode Switching 🎛️
                            // Controller mode: Lyrics only (no audio input for Cava)
                            // MPD mode: All cards (Lyrics, Cava, Library, EQ)
                            KeyCode::Char('1') => app.view_mode = app::ViewMode::Lyrics,
                            #[cfg(feature = "mpd")]
                            KeyCode::Char('2') if !args.controller => app.view_mode = app::ViewMode::Visualizer,
                            #[cfg(feature = "mpd")]
                            KeyCode::Char('3') if !args.controller => app.view_mode = app::ViewMode::Library,
                            #[cfg(feature = "mpd")]
                            KeyCode::Char('4') if !args.controller => app.view_mode = app::ViewMode::EQ,

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
                                        app.show_toast(&format!("🎤 Jump to {}:{:02}", mins, secs));
                                        app.lyrics_selected = None; // Exit selection mode
                                        app.lyrics_offset = None; // Return to auto-sync
                                        app.last_scroll_time = None; // Allow immediate auto-follow
                                    }
                                }
                            }
                        },
                        // Audio Device Switching: d/D (Global in Controller Mode / Lyrics / Cava) 🎧
                        KeyCode::Char('d') if matches!(app.view_mode, app::ViewMode::Lyrics | app::ViewMode::Visualizer) => {
                            app.next_device();
                        },
                        KeyCode::Char('D') if matches!(app.view_mode, app::ViewMode::Lyrics | app::ViewMode::Visualizer) => {
                            app.prev_device();
                        },
                        // EQ Controls (only when in EQ view) 🎚️
                        // Navigation: Left/Right or h/l ↔️
                        KeyCode::Left | KeyCode::Char('h') if app.view_mode == app::ViewMode::EQ => {
                            app.eq_selected = app.eq_selected.saturating_sub(1);
                        },
                        KeyCode::Right | KeyCode::Char('l') if app.view_mode == app::ViewMode::EQ => {
                            if app.eq_selected < 9 { app.eq_selected += 1; }
                        },
                        // Gain: Up/Down or k/j ↕️
                        KeyCode::Up | KeyCode::Char('k') if app.view_mode == app::ViewMode::EQ => {
                            let band = &mut app.eq_bands[app.eq_selected];
                            *band = (*band + 0.05).min(1.0); // +1.2dB
                            app.mark_custom();
                            app.sync_band_to_dsp(app.eq_selected);
                            let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
                            app.show_toast(&format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db));
                        },
                        KeyCode::Down | KeyCode::Char('j') if app.view_mode == app::ViewMode::EQ => {
                            let band = &mut app.eq_bands[app.eq_selected];
                            *band = (*band - 0.05).max(0.0); // -1.2dB
                            app.mark_custom();
                            app.sync_band_to_dsp(app.eq_selected);
                            let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
                            app.show_toast(&format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db));
                        },
                        KeyCode::Char('e') if app.view_mode == app::ViewMode::EQ => {
                            app.toggle_eq();
                            app.show_toast(&format!("🎛 EQ: {}", if app.eq_enabled { "ON" } else { "OFF" }));
                        },
                        KeyCode::Char('r') if app.view_mode == app::ViewMode::EQ => {
                            app.reset_eq();
                            app.show_toast("🔄 EQ Reset");
                        },
                        // Reset Preamp + Balance: 0
                        KeyCode::Char('0') if app.view_mode == app::ViewMode::EQ => {
                            // Reset Preamp & Balance only (Level controls)
                            app.reset_preamp();
                            app.reset_balance();
                            
                            app.mark_custom();
                            app.sync_band_to_dsp(app.eq_selected); // Just update state
                            app.show_toast("🎯 Levels Reset"); // "Levels" = Preamp + Balance
                        },
                        // Preset cycling: Tab for next, Shift+Tab for previous
                        KeyCode::Tab if app.view_mode == app::ViewMode::EQ => {
                            app.next_preset();
                            app.show_toast(&format!("🎵 Preset: {}", app.get_preset_name()));
                        },
                        KeyCode::BackTab if app.view_mode == app::ViewMode::EQ => {
                            app.prev_preset();
                            app.show_toast(&format!("🎵 Preset: {}", app.get_preset_name()));
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
                            if !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    let _ = mpd.crossfade(app.crossfade_secs as i64);
                                }
                            }
                        },
                        // ReplayGain: R (Shift+R) to cycle modes (Off → Track → Album → Auto)
                        KeyCode::Char('R') if app.view_mode == app::ViewMode::EQ => {
                            app.replay_gain_mode = (app.replay_gain_mode + 1) % 4;
                            #[cfg(feature = "mpd")]
                            if !args.controller {
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
                        // Global search: / to jump to search from anywhere (MPD only)

                        KeyCode::Esc if app.show_keyhints || app.show_audio_info => {
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
                                if !args.controller {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        let current_pos = app.library_selected as u32;
                                        let new_pos = current_pos - 1;
                                        if mpd.shift(current_pos, new_pos as usize).is_ok() {
                                             app.library_selected = new_pos as usize;
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) && app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue => {
                             if app.library_selected < app.queue.len().saturating_sub(1) {
                                #[cfg(feature = "mpd")]
                                if !args.controller {
                                    if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                        let current_pos = app.library_selected as u32;
                                        let new_pos = current_pos + 1;
                                        if mpd.shift(current_pos, new_pos as usize).is_ok() {
                                             app.library_selected = new_pos as usize;
                                        }
                                    }
                                }
                            }
                        },

                        // Library Panel Controls (when in Library view) 📚
                        KeyCode::Tab if app.view_mode == app::ViewMode::Library => {
                            // Cycle library modes: Queue → Directory → Playlists
                            app.library_mode = match app.library_mode {
                                app::LibraryMode::Queue => app::LibraryMode::Directory,
                                app::LibraryMode::Directory => app::LibraryMode::Playlists,
                                app::LibraryMode::Search => app::LibraryMode::Playlists, // Search via bar, not tab
                                app::LibraryMode::Playlists => app::LibraryMode::Queue,
                            };
                            // Clear state when switching modes
                            app.library_selected = 0;
                            app.library_items.clear();
                            app.browse_path.clear();
                            app.search_query.clear();
                            app.search_active = false;

                            // Load playlists when entering Playlists mode
                            #[cfg(feature = "mpd")]
                            if app.library_mode == app::LibraryMode::Playlists && !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(pls) = mpd.playlists() {
                                        app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                                    }
                                }
                            }

                            // Load root directory when entering Directory mode
                            #[cfg(feature = "mpd")]
                            if app.library_mode == app::LibraryMode::Directory && !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    // Get directories from listfiles, songs with metadata from lsinfo
                                    if let Ok(items) = fetch_directory_items(&mut mpd, "") {
                                        app.library_items = items;
                                    }
                                }
                            }
                        },
                        KeyCode::BackTab if app.view_mode == app::ViewMode::Library => {
                            // Reverse cycle: Playlists → Directory → Queue
                            app.library_mode = match app.library_mode {
                                app::LibraryMode::Queue => app::LibraryMode::Playlists,
                                app::LibraryMode::Directory => app::LibraryMode::Queue,
                                app::LibraryMode::Search => app::LibraryMode::Directory, // Search via bar, not tab
                                app::LibraryMode::Playlists => app::LibraryMode::Directory,
                            };
                            // Clear state when switching modes
                            app.library_selected = 0;
                            app.library_items.clear();
                            app.browse_path.clear();
                            app.search_query.clear();

                            // Load playlists when entering Playlists mode
                            #[cfg(feature = "mpd")]
                            if app.library_mode == app::LibraryMode::Playlists && !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(pls) = mpd.playlists() {
                                        app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                                    }
                                }
                            }
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
                        // Rename Playlist: 'r' ✏️
                        KeyCode::Char('r') if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Playlists => {
                             if !app.playlists.is_empty() {
                                if let Some(pl_name) = app.playlists.get(app.library_selected) {
                                    app.input_state = Some(app::InputState::new(
                                        app::InputMode::PlaylistRename(pl_name.clone()),
                                        "Rename Playlist",
                                        pl_name
                                    ));
                                }
                            }
                        },
                        // Tag editing: t to edit selected song's tags
                        KeyCode::Char('t') if app.view_mode == app::ViewMode::Library => {
                            match app.library_mode {
                                app::LibraryMode::Queue => {
                                    // Get selected song from queue
                                    if let Some(item) = app.queue.get(app.library_selected) {
                                        // Try to read album from file directly 📖
                                        let mut album = String::new();
                                        #[cfg(feature = "mpd")]
                                        if !args.controller {
                                             let full_path = format!("{}/{}", app.music_directory, item.file_path);
                                             if let Ok(tagged_file) = lofty::read_from_path(&full_path) {
                                                 if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                                                     album = tag.album().as_deref().unwrap_or("").to_string();
                                                 }
                                             }
                                        }

                                        // Open tag editor with extracted values
                                        app.tag_edit = Some(app::TagEditState::new(
                                            &item.file_path,
                                            &item.title,
                                            &item.artist,
                                            &album,
                                        ));
                                    }
                                },
                                app::LibraryMode::Directory => {
                                    // Get selected item from directory
                                    if let Some(item) = app.library_items.get(app.library_selected) {
                                        // Only allow editing Songs (not folders)
                                        if item.item_type == app::LibraryItemType::Song {
                                            if let Some(path) = &item.path {
                                                // Try to read album from file directly 📖
                                                let mut album = String::new();
                                                #[cfg(feature = "mpd")]
                                                if !args.controller {
                                                     let full_path = format!("{}/{}", app.music_directory, path);
                                                     if let Ok(tagged_file) = lofty::read_from_path(&full_path) {
                                                         if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                                                             album = tag.album().as_deref().unwrap_or("").to_string();
                                                         }
                                                     }
                                                }

                                                 app.tag_edit = Some(app::TagEditState::new(
                                                    path,
                                                    &item.name,
                                                    item.artist.as_deref().unwrap_or(""),
                                                    &album,
                                                ));
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }
                        },
                        // Delete: d to delete playlist or remove song from queue
                        KeyCode::Char('d') if app.view_mode == app::ViewMode::Library => {
                            #[cfg(feature = "mpd")]
                            if !args.controller {
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
                            if args.controller {
                                let new_state = !app.shuffle;
                                let _ = player.shuffle(new_state);
                                app.shuffle = new_state;
                                app.show_toast(&format!("🔀 Shuffle: {}", if new_state { "ON" } else { "OFF" }));
                            } else {
                                #[cfg(feature = "mpd")]
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(status) = mpd.status() {
                                        let new_state = !status.random;
                                        let _ = mpd.random(new_state);
                                        app.shuffle = new_state;
                                        app.show_toast(&format!("🔀 Shuffle: {}", if new_state { "ON" } else { "OFF" }));
                                    }
                                }
                            }
                        },
                        // Repeat toggle: x
                        KeyCode::Char('x') => {
                            if args.controller {
                                let new_state = !app.repeat;
                                let _ = player.repeat(new_state);
                                app.repeat = new_state;
                                app.show_toast(&format!("🔁 Repeat: {}", if new_state { "ON" } else { "OFF" }));
                            } else {
                                #[cfg(feature = "mpd")]
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if let Ok(status) = mpd.status() {
                                        let new_state = !status.repeat;
                                        let _ = mpd.repeat(new_state);
                                        app.repeat = new_state;
                                        app.show_toast(&format!("🔁 Repeat: {}", if new_state { "ON" } else { "OFF" }));
                                    }
                                }
                            }
                        },
                        // Add to Queue: 'a' key ➕
                        KeyCode::Char('a') if app.view_mode == app::ViewMode::Library && (app.library_mode == app::LibraryMode::Directory || app.library_mode == app::LibraryMode::Search) => {
                             #[cfg(feature = "mpd")]
                             if !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    if !app.library_items.is_empty() {
                                         if let Some(item) = app.library_items.get(app.library_selected) {
                                            let mut added_name = item.name.clone();
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
                                                     mpd.findadd(mpd::Query::new().and(mpd::Term::Tag("Album".into()), &item.name)).is_ok()
                                                },
                                                app::LibraryItemType::Artist => {
                                                     mpd.findadd(mpd::Query::new().and(mpd::Term::Tag("Artist".into()), &item.name)).is_ok()
                                                },
                                                app::LibraryItemType::Playlist => {
                                                     mpd.load(&item.name, ..).is_ok()
                                                },
                                                app::LibraryItemType::Folder => {
                                                     if let Some(path) = &item.path {
                                                         use crate::app::library_helpers::queue_folder_recursive;
                                                         
                                                         // Call it
                                                         if queue_folder_recursive(&mut mpd, path).is_ok() {
                                                             added_name = item.name.clone(); 
                                                             true
                                                         } else { 
                                                             false 
                                                         }
                                                     } else { false }
                                                },
                                            };
                                            if added {
                                                app.show_toast(&format!("Added: {}", added_name));
                                            }
                                         }
                                    }
                                }
                             }
                        },
                        // Enter key for Library actions (Select/Play/Enter Dir)
                        // 'l' removed to allow seeking
                        KeyCode::Enter if app.view_mode == app::ViewMode::Library => {
                            #[cfg(feature = "mpd")]
                            if !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    match app.library_mode {
                                        app::LibraryMode::Queue => {
                                            // Play selected song in queue
                                            let _ = mpd.switch(app.library_selected as u32);
                                        }
                                        app::LibraryMode::Directory => {
                                            // Direct folder navigation using listfiles
                                            if let Some(item) = app.library_items.get(app.library_selected) {
                                                match &item.item_type {
                                                    app::LibraryItemType::Folder => {
                                                        // Navigate into folder
                                                        if let Some(path) = &item.path {
                                                            app.browse_path.push(item.name.clone());
                                                            app.library_selected = 0;

                                                            // Get directories from listfiles
                                                            if let Ok(items) = fetch_directory_items(&mut mpd, path) {
                                                                app.library_items = items;
                                                            }
                                                        }
                                                    },
                                                    app::LibraryItemType::Song => {
                                                        // Smart Play: Add to queue and switch to it (Don't clear!)
                                                        if let Some(path) = &item.path {
                                                            if let Ok(id) = mpd.push(mpd::song::Song { file: path.clone(), ..Default::default() }) {
                                                                 let _ = mpd.switch(id);
                                                            }
                                                        }
                                                    },
                                                    _ => {}
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
                        // Exit Search Mode (Viewing Results) 🔍
                        KeyCode::Esc if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Search => {
                            // Restore previous mode or default to Directory
                            let target_mode = app.previous_library_mode.take().unwrap_or(app::LibraryMode::Directory);
                            app.library_mode = target_mode;
                            app.search_query.clear();
                            app.library_selected = 0;

                            #[cfg(feature = "mpd")]
                            if !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    match target_mode {
                                        app::LibraryMode::Directory => {
                                            // Restore Directory view
                                            let current_path = app.browse_path.join("/");
                                            let mut new_items: Vec<app::LibraryItem> = Vec::new();

                                            // 1. Folders
                                            if let Ok(files) = mpd.listfiles(&current_path) {
                                                for (kind, name) in files {
                                                    let display_name = name.split('/').next_back().unwrap_or(&name).to_string();
                                                    if display_name.starts_with('.') || display_name.trim().is_empty() { continue; }

                                                    if kind == "directory" {
                                                        let full_path = if current_path.is_empty() { name.clone() } else { format!("{}/{}", current_path, name) };
                                                        new_items.push(app::LibraryItem {
                                                            name: display_name,
                                                            item_type: app::LibraryItemType::Folder,
                                                            artist: None, duration_ms: None, path: Some(full_path)
                                                        });
                                                    }
                                                }
                                            }

                                            // 2. Songs
                                            if let Ok(songs) = mpd.lsinfo(&mpd::Song { file: current_path.clone(), ..Default::default() }) {
                                                for song in songs {
                                                    let filename = song.file.split('/').next_back().unwrap_or(&song.file).to_string();
                                                    if filename.starts_with('.') || filename.trim().is_empty() { continue; }

                                                    let title = match song.title.as_ref().filter(|t| !t.trim().is_empty()) {
                                                        Some(t) => t.clone(),
                                                        None => filename.clone(),
                                                    };

                                                    new_items.push(app::LibraryItem {
                                                        name: title,
                                                        item_type: app::LibraryItemType::Song,
                                                        artist: song.artist.clone(),
                                                        duration_ms: song.duration.map(|d| d.as_millis() as u64),
                                                        path: Some(song.file),
                                                    });
                                                }
                                            }

                                            new_items.sort_by(|a, b| {
                                                match (&a.item_type, &b.item_type) {
                                                    (app::LibraryItemType::Folder, app::LibraryItemType::Song) => std::cmp::Ordering::Less,
                                                    (app::LibraryItemType::Song, app::LibraryItemType::Folder) => std::cmp::Ordering::Greater,
                                                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                                                }
                                            });
                                            app.library_items = new_items;
                                        },
                                        app::LibraryMode::Playlists => {
                                            // Refresh playlists
                                            if let Ok(playlists) = mpd.playlists() {
                                                app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                                                app.library_items = app.playlists.iter().map(|p| app::LibraryItem {
                                                    name: p.clone(),
                                                    item_type: app::LibraryItemType::Playlist,
                                                    artist: None, duration_ms: None, path: None
                                                }).collect();
                                            }
                                        },
                                        app::LibraryMode::Queue => {
                                            app.library_items.clear();
                                        },
                                        _ => {}
                                    }
                                }
                            }
                        },

                        // Backspace/Esc to go back in Browse
                        // 'h' removed to allow seeking
                        KeyCode::Backspace | KeyCode::Esc if app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Directory => {
                            app.browse_path.pop();
                            app.library_items.clear();
                            app.library_selected = 0;

                            // Re-fetch items for the parent level
                            #[cfg(feature = "mpd")]
                            if !args.controller {
                                if let Ok(mut mpd) = mpd::Client::connect(format!("{}:{}", args.mpd_host, args.mpd_port)) {
                                    // Build the path from browse_path
                                    let parent_path = if app.browse_path.is_empty() {
                                        "".to_string()
                                    } else {
                                        app.browse_path.join("/")
                                    };

                                    // Get directory items
                                    if let Ok(items) = fetch_directory_items(&mut mpd, &parent_path) {
                                        app.library_items = items;
                                    }
                                }
                            }
                        },
                        // Navigation keys for Library view (Up/Down or k/j)
                        KeyCode::Up | KeyCode::Char('k') if app.view_mode == app::ViewMode::Library => {
                            app.library_selected = app.library_selected.saturating_sub(1);
                        },
                        KeyCode::Down | KeyCode::Char('j') if app.view_mode == app::ViewMode::Library => {
                            let max_items = match app.library_mode {
                                app::LibraryMode::Queue => app.queue.len().max(1),
                                app::LibraryMode::Directory if app.browse_path.is_empty() => 4,
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
}
