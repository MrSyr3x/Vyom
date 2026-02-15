use crossterm::event::{KeyCode, KeyEvent};
use crate::app::{self, App, LyricsState};
#[cfg(feature = "mpd")]
use crate::app::with_mpd;
use crate::player::PlayerTrait;
use crate::audio::pipeline::AudioPipeline;
use crate::app::cli::Args;
use crate::app::events::AppEvent;
use crate::app::library_helpers::fetch_directory_items;
use lofty::file::TaggedFileExt;
use lofty::tag::Accessor;
use tokio::sync::mpsc;
use std::sync::Arc;
use reqwest::Client;

pub async fn handle_normal_mode(
    key: KeyEvent,
    app: &mut App,
    player: &Arc<dyn PlayerTrait>,
    audio_pipeline: &mut AudioPipeline,
    args: &Args,
    _tx: &mpsc::Sender<AppEvent>,
    _client: &Client,
) {
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
        return;
    }

    // Play/Pause ('Space')
    if keys.matches(key, &keys.play_pause) {
        if let Ok(is_playing) = player.play_pause() {
            app.show_toast(if is_playing { "▶ Play" } else { "⏸ Pause" });
        }
        return;
    }

    // Next Track ('n')
    if keys.matches(key, &keys.next_track) {
        let _ = player.next();
        app.show_toast("⏭ Next Track");
        return;
    }

    // Prev Track ('p')
    if keys.matches(key, &keys.prev_track) {
        let _ = player.prev();
        app.show_toast("⏮ Previous Track");
        return;
    }

    // Volume Up ('+')
    if keys.matches(key, &keys.volume_up) {
        let new_vol = (app.app_volume.saturating_add(5)).min(100);
        app.app_volume = new_vol;
        app.last_volume_action = Some(std::time::Instant::now());
        audio_pipeline.set_volume(new_vol);
        if let Err(e) = player.set_volume(new_vol) {
             app.show_toast(&format!("Error: {}", e));
        }
        app.show_toast(&format!("🔊 Volume: {}%", new_vol));
        return;
    }

    // Volume Down ('-')
    if keys.matches(key, &keys.volume_down) {
        let new_vol = app.app_volume.saturating_sub(5);
        app.app_volume = new_vol;
        app.last_volume_action = Some(std::time::Instant::now());
        audio_pipeline.set_volume(new_vol);
        if let Err(e) = player.set_volume(new_vol) {
             app.show_toast(&format!("Error: {}", e));
        }
        app.show_toast(&format!("🔉 Volume: {}%", new_vol));
        return;
    }

    // Seek Backward ('h' or 'Left') - blocked in EQ
    if (keys.matches(key, &keys.seek_backward) || keys.matches(key, &keys.nav_left_alt)) && app.view_mode != app::ViewMode::EQ {
        let now = std::time::Instant::now();
        let is_new_sequence = if let Some(last) = app.last_seek_time {
            now.duration_since(last).as_millis() >= 500
        } else { true };

        if is_new_sequence {
            if let Some(_track) = &app.track {
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
            if let Some(track) = &app.track {
                let duration = track.duration_ms as f64 / 1000.0;
                target = target.max(0.0).min(duration);
            } else {
                target = target.max(0.0);
            }

            // Increment Seek ID (Generation Counter) ⏩
            app.seek_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let my_id = app.seek_id.load(std::sync::atomic::Ordering::Relaxed);
            let global_seek_id = app.seek_id.clone();

            let player_bg = player.clone();
            let original_track_key = app.track.as_ref().map(|t| (t.name.clone(), t.artist.clone()));
            tokio::task::spawn_blocking(move || {
                // Check if a newer seek request has come in
                if global_seek_id.load(std::sync::atomic::Ordering::Relaxed) != my_id {
                    return; // Stale request, discard 🗑️
                }

                if let Ok(Some(current_track)) = player_bg.get_current_track() {
                    let current_key = (current_track.name.clone(), current_track.artist.clone());
                    if original_track_key.as_ref() == Some(&current_key) {
                        let _ = player_bg.seek(target);
                    }
                }
            });
            app.show_toast(&format!("⏪ Seek: {:+.0}s", app.seek_accumulator));
        }
        return;
    }

    // Seek Forward ('l' or 'Right') - blocked in EQ
    if (keys.matches(key, &keys.seek_forward) || keys.matches(key, &keys.nav_right_alt)) && app.view_mode != app::ViewMode::EQ {
        let now = std::time::Instant::now();
        let is_new_sequence = if let Some(last) = app.last_seek_time {
            now.duration_since(last).as_millis() >= 500
        } else { true };

        if is_new_sequence {
            if let Some(_track) = &app.track {
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
            if let Some(track) = &app.track {
                let duration = track.duration_ms as f64 / 1000.0;
                target = target.max(0.0).min(duration);
            } else {
                target = target.max(0.0);
            }

            // Increment Seek ID (Generation Counter) ⏩
            app.seek_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let my_id = app.seek_id.load(std::sync::atomic::Ordering::Relaxed);
            let global_seek_id = app.seek_id.clone();

            let player_bg = player.clone();
            let original_track_key = app.track.as_ref().map(|t| (t.name.clone(), t.artist.clone()));
            tokio::task::spawn_blocking(move || {
                 // Check if a newer seek request has come in
                if global_seek_id.load(std::sync::atomic::Ordering::Relaxed) != my_id {
                    return; // Stale request, discard 🗑️
                }

                if let Ok(Some(current_track)) = player_bg.get_current_track() {
                    let current_key = (current_track.name.clone(), current_track.artist.clone());
                    if original_track_key.as_ref() == Some(&current_key) {
                        let _ = player_bg.seek(target);
                    }
                }
            });
            app.show_toast(&format!("⏩ Seek: {:+.0}s", app.seek_accumulator));
        }
        return;
    }

    // Queue Reordering with J/K (Shift+j/k)
    // Note: Matches doesn't account for complex multikey modifiers unless we explicitly check
    // But 'J' implies Shift+j usually in char processing.
    // keys.move_down default is "J".
    if keys.matches(key, &keys.move_down) && app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue {
        if app.library_selected < app.queue.len().saturating_sub(1) {
            #[cfg(feature = "mpd")]
            #[cfg(feature = "mpd")]
            if !args.controller {
                let current_pos = app.library_selected as u32;
                let new_pos = current_pos + 1;
                
                let success = with_mpd(app, args, |mpd| {
                    mpd.shift(current_pos, new_pos as usize).is_ok()
                }).unwrap_or(false);

                if success {
                     app.library_selected = new_pos as usize;
                }
            }
        }
        return;
    }

    if keys.matches(key, &keys.move_up) && app.view_mode == app::ViewMode::Library && app.library_mode == app::LibraryMode::Queue {
        if app.library_selected > 0 {
            #[cfg(feature = "mpd")]
            #[cfg(feature = "mpd")]
            if !args.controller {
                let current_pos = app.library_selected as u32;
                let new_pos = current_pos - 1;
                
                let success = with_mpd(app, args, |mpd| {
                    mpd.shift(current_pos, new_pos as usize).is_ok()
                }).unwrap_or(false);

                if success {
                     app.library_selected = new_pos as usize;
                }
            }
        }
        return;
    }

    // EQ Save Preset (Shift+S)
    if keys.matches(key, &keys.save_preset) && app.view_mode == app::ViewMode::EQ {
        app.input_state = Some(app::InputState::new(
            app::InputMode::EqSave,
            "Save Preset As",
            ""
        ));
        return;
    }

    // EQ Delete Preset (Shift+X)
    if keys.matches(key, &keys.delete_preset) && app.view_mode == app::ViewMode::EQ {
        if let Err(e) = app.delete_preset() {
            app.show_toast(&format!("❌ {}", e));
        } else {
            app.show_toast("🗑️ Preset Deleted");
        }
        return;
    }

    // View Mode Switching
    if keys.matches(key, &keys.view_lyrics) { app.view_mode = app::ViewMode::Lyrics; return; }
    #[cfg(feature = "mpd")]
    if keys.matches(key, &keys.view_visualizer) && !args.controller { app.view_mode = app::ViewMode::Visualizer; return; }
    #[cfg(feature = "mpd")]
    if keys.matches(key, &keys.view_library) && !args.controller { app.view_mode = app::ViewMode::Library; return; }
    #[cfg(feature = "mpd")]
    if keys.matches(key, &keys.view_eq) && !args.controller { app.view_mode = app::ViewMode::EQ; return; }

    // Lyrics Navigation
    if app.view_mode == app::ViewMode::Lyrics {
        if keys.matches(key, &keys.nav_down) || keys.matches(key, &keys.nav_down_alt) {
            if let LyricsState::Loaded(ref lines) = &app.lyrics {
                let max = lines.len().saturating_sub(1);
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
                app.last_scroll_time = Some(std::time::Instant::now());
            }
            return;
        }

        if keys.matches(key, &keys.nav_up) || keys.matches(key, &keys.nav_up_alt) {
            if let LyricsState::Loaded(ref lines) = &app.lyrics {
                let max = lines.len().saturating_sub(1);
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
                app.last_scroll_time = Some(std::time::Instant::now());
            }
            return;
        }

        if keys.matches(key, &keys.seek_to_line) {
            if let LyricsState::Loaded(ref lines) = &app.lyrics {
                if let Some(idx) = app.lyrics_selected {
                    if idx < lines.len() {
                        let target_ms = lines[idx].timestamp_ms;
                        let target_secs = target_ms as f64 / 1000.0;
                        let player_bg = player.clone();
                        tokio::task::spawn_blocking(move || {
                            let _ = player_bg.seek(target_secs);
                        });
                        let mins = target_ms / 60000;
                        let secs = (target_ms % 60000) / 1000;
                        app.show_toast(&format!("🎤 Jump to {}:{:02}", mins, secs));
                        app.lyrics_selected = None;
                        app.lyrics_offset = None;
                        app.last_scroll_time = None;
                    }
                }
            }
            return;
        }
    }

    // Audio Device Switching
    if app.view_mode == app::ViewMode::Lyrics || app.view_mode == app::ViewMode::Visualizer || app.view_mode == app::ViewMode::EQ {
        if keys.matches(key, &keys.device_next) { app.next_device(); return; }
        if keys.matches(key, &keys.device_prev) { app.prev_device(); return; }
    }

    // EQ Controls
    if app.view_mode == app::ViewMode::EQ {
        if keys.matches(key, &keys.nav_left) || keys.matches(key, &keys.nav_left_alt) {
            app.eq_selected = app.eq_selected.saturating_sub(1);
            return;
        }
        if keys.matches(key, &keys.nav_right) || keys.matches(key, &keys.nav_right_alt) {
            if app.eq_selected < 9 { app.eq_selected += 1; }
            return;
        }
        if keys.matches(key, &keys.gain_up) || keys.matches(key, &keys.nav_up_alt) {
            let band = &mut app.eq_bands[app.eq_selected];
            *band = (*band + 0.05).min(1.0);
            app.mark_custom();
            app.sync_band_to_dsp(app.eq_selected);
            let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
            app.show_toast(&format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db));
            return;
        }
        if keys.matches(key, &keys.gain_down) || keys.matches(key, &keys.nav_down_alt) {
            let band = &mut app.eq_bands[app.eq_selected];
            *band = (*band - 0.05).max(0.0);
            app.mark_custom();
            app.sync_band_to_dsp(app.eq_selected);
            let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
            app.show_toast(&format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db));
            return;
        }
        if keys.matches(key, &keys.toggle_eq) {
            app.toggle_eq();
            app.show_toast(&format!("🎛 EQ: {}", if app.eq_enabled { "ON" } else { "OFF" }));
            return;
        }
        if keys.matches(key, &keys.reset_eq) {
             app.reset_eq();
             app.show_toast("🔄 EQ Reset");
             return;
        }
        if keys.matches(key, &keys.reset_levels) {
            app.reset_preamp();
            app.reset_balance();
            // app.mark_custom(); // Removed to keep current preset
            app.sync_band_to_dsp(app.eq_selected);
            app.show_toast("🎯 Levels Reset");
            return;
        }
        if keys.matches(key, &keys.tab_next) {
            app.next_preset();
            app.show_toast(&format!("🎵 Preset: {}", app.get_preset_name()));
            return;
        }
        if keys.matches(key, &keys.tab_prev) {
            app.prev_preset();
            app.show_toast(&format!("🎵 Preset: {}", app.get_preset_name()));
            return;
        }
        if keys.matches(key, &keys.preamp_up) { app.adjust_preamp(1.0); return; }
        if keys.matches(key, &keys.preamp_down) { app.adjust_preamp(-1.0); return; }
        if keys.matches(key, &keys.balance_right) { app.adjust_balance(0.1); return; }
        if keys.matches(key, &keys.balance_left) { app.adjust_balance(-0.1); return; }
        if keys.matches(key, &keys.crossfade) {
             app.toggle_crossfade();
             #[cfg(feature = "mpd")]
             if !args.controller {
                let secs = app.crossfade_secs as i64;
                with_mpd(app, args, |mpd| {
                    let _ = mpd.crossfade(secs);
                });
             }
             return;
        }
        if keys.matches(key, &keys.replay_gain) {
            app.replay_gain_mode = (app.replay_gain_mode + 1) % 4;
            #[cfg(feature = "mpd")]
            #[cfg(feature = "mpd")]
            if !args.controller {
                let mode = match app.replay_gain_mode {
                    1 => mpd::status::ReplayGain::Track,
                    2 => mpd::status::ReplayGain::Album,
                    3 => mpd::status::ReplayGain::Auto,
                    _ => mpd::status::ReplayGain::Off,
                };
                with_mpd(app, args, |mpd| {
                    let _ = mpd.replaygain(mode);
                });
            }
            return;
        }
    }

    if keys.matches(key, &keys.toggle_keyhints) {
        app.show_keyhints = !app.show_keyhints;
        return;
    }
    if keys.matches(key, &keys.toggle_audio_info) {
        app.show_audio_info = !app.show_audio_info;
        return;
    }
    
    // Global Popup Close (Esc)
    if (keys.matches(key, &keys.back_dir_alt) || key.code == KeyCode::Esc) && (app.show_keyhints || app.show_audio_info) {
        if app.show_keyhints { app.show_keyhints = false; }
        if app.show_audio_info { app.show_audio_info = false; }
        return;
    }

    // Explicit Esc check for search exit logic if not captured elsewhere
    // In inputs.rs, we see "Exit Search Mode" at the bottom for Library View
    
    // Library Panel Controls
    if app.view_mode == app::ViewMode::Library {
        if keys.matches(key, &keys.tab_next) {
             app.library_mode = match app.library_mode {
                app::LibraryMode::Queue => app::LibraryMode::Directory,
                app::LibraryMode::Directory => app::LibraryMode::Playlists,
                app::LibraryMode::Search => app::LibraryMode::Playlists,
                app::LibraryMode::Playlists => app::LibraryMode::Queue,
            };
            app.library_selected = 0;
            app.library_items.clear();
            app.browse_path.clear();
            app.search_query.clear();
            app.search_active = false;

            #[cfg(feature = "mpd")]
            if app.library_mode == app::LibraryMode::Playlists && !args.controller {
                if let Some(playlists) = with_mpd(app, args, |mpd| {
                    mpd.playlists().ok()
                }).flatten() {
                    app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                }
            }
             #[cfg(feature = "mpd")]
             #[cfg(feature = "mpd")]
            if app.library_mode == app::LibraryMode::Directory && !args.controller {
                if let Some(Some(items)) = with_mpd(app, args, |mpd| {
                    fetch_directory_items(mpd, "").ok()
                }) {
                    app.library_items = items;
                }
            }
            return;
        }

        if keys.matches(key, &keys.tab_prev) {
             app.library_mode = match app.library_mode {
                app::LibraryMode::Queue => app::LibraryMode::Playlists,
                app::LibraryMode::Directory => app::LibraryMode::Queue,
                app::LibraryMode::Search => app::LibraryMode::Directory,
                app::LibraryMode::Playlists => app::LibraryMode::Directory,
            };
            app.library_selected = 0;
            app.library_items.clear();
            app.browse_path.clear();
            app.search_query.clear();
            
             #[cfg(feature = "mpd")]
            if app.library_mode == app::LibraryMode::Playlists && !args.controller {
                if let Some(playlists) = with_mpd(app, args, |mpd| {
                    mpd.playlists().ok()
                }).flatten() {
                    app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                }
            }
            return;
        }

        if keys.matches(key, &keys.save_playlist) {
             app.input_state = Some(app::InputState::new(
                app::InputMode::PlaylistSave,
                "Save Playlist As:",
                ""
            ));
            return;
        }

        if keys.matches(key, &keys.rename_playlist) && app.library_mode == app::LibraryMode::Playlists {
             if !app.playlists.is_empty() {
                if let Some(pl_name) = app.playlists.get(app.library_selected) {
                    app.input_state = Some(app::InputState::new(
                        app::InputMode::PlaylistRename(pl_name.clone()),
                        "Rename Playlist",
                        pl_name
                    ));
                }
            }
            return;
        }

        if keys.matches(key, &keys.edit_tags) {
             // Logic copied from inputs.rs lines 814-870...
             // Simplified for brevity in thought trace, will write full logic in file
             // Copied logic:
             match app.library_mode {
                app::LibraryMode::Queue => {
                    if let Some(item) = app.queue.get(app.library_selected) {
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
                        app.tag_edit = Some(app::TagEditState::new(
                            &item.file_path,
                            &item.title,
                            &item.artist,
                            &album,
                        ));
                    }
                },
                app::LibraryMode::Directory => {
                    if let Some(item) = app.library_items.get(app.library_selected) {
                        if item.item_type == app::LibraryItemType::Song {
                            if let Some(path) = &item.path {
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
            return;
        }
        
        if keys.matches(key, &keys.delete_item) {
             #[cfg(feature = "mpd")]
             if !args.controller {
                use crate::app::LibraryMode;
                let mode = app.library_mode;
                let selected = app.library_selected;
                let pl_name = if mode == LibraryMode::Playlists {
                    app.playlists.get(selected).cloned()
                } else { None };

                let new_playlists = with_mpd(app, args, |mpd| {
                    match mode {
                        LibraryMode::Queue => { let _ = mpd.delete(selected as u32); None },
                        LibraryMode::Playlists => {
                             if let Some(name) = pl_name {
                                let _ = mpd.pl_remove(&name);
                                mpd.playlists().ok() // Return new playlists
                            } else { None }
                        },
                        _ => None
                    }
                });

                if let Some(Some(pls)) = new_playlists {
                    app.playlists = pls.iter().map(|p| p.name.clone()).collect();
                }
                
                if mode == LibraryMode::Playlists && app.library_selected > 0 && app.library_selected >= app.playlists.len() {
                     app.library_selected = app.library_selected.saturating_sub(1);
                }
             }
             return;
        }

        if keys.matches(key, &keys.add_to_queue) && (app.library_mode == app::LibraryMode::Directory || app.library_mode == app::LibraryMode::Search) {
             #[cfg(feature = "mpd")]
             if !args.controller && !app.library_items.is_empty() {
                     let item = app.library_items.get(app.library_selected).cloned();
                     
                     if let Some(target_item) = item {
                         let (result, shuffle_on) = with_mpd(app, args, |mpd| {
                             let mut added_name = target_item.name.clone();
                             let added = match target_item.item_type {
                                app::LibraryItemType::Song => {
                                    if let Some(path) = &target_item.path {
                                        let song = mpd::Song {
                                            file: path.clone(),
                                            ..Default::default()
                                        };
                                        mpd.push(song).is_ok()
                                    } else { false }
                                },
                                app::LibraryItemType::Album => {
                                     mpd.findadd(mpd::Query::new().and(mpd::Term::Tag("Album".into()), &target_item.name)).is_ok()
                                },
                                app::LibraryItemType::Artist => {
                                     mpd.findadd(mpd::Query::new().and(mpd::Term::Tag("Artist".into()), &target_item.name)).is_ok()
                                },
                                app::LibraryItemType::Playlist => {
                                     mpd.load(&target_item.name, ..).is_ok()
                                },
                                app::LibraryItemType::Folder => {
                                     if let Some(path) = &target_item.path {
                                         use crate::app::library_helpers::queue_folder_recursive;
                                         if queue_folder_recursive(mpd, path).is_ok() {
                                             added_name = target_item.name.clone(); 
                                             true
                                         } else { 
                                             false 
                                         }
                                     } else { false }
                                },
                            };
                            let shuffle = if let Ok(s) = mpd.status() { s.random } else { false };
                            (if added { Some(added_name) } else { None }, shuffle)
                         }).unwrap_or((None, false)); // Default to no add false shuffle if connection failed

                         if let Some(added_name) = result {
                             let shuffle_msg = if shuffle_on { " (Shuffle ON)" } else { "" };
                             app.show_toast(&format!("Added: {}{}", added_name, shuffle_msg));
                         }
                     }
             }
             return;
        }

        if keys.matches(key, &keys.enter_dir) {
             #[cfg(feature = "mpd")]
             if !args.controller {
                 // Clone needed state
                 let mode = app.library_mode;
                 let item = app.library_items.get(app.library_selected).cloned();
                 let pl_name = if mode == app::LibraryMode::Playlists {
                     app.playlists.get(app.library_selected).cloned()
                 } else { None };
                 let queue_idx = if mode == app::LibraryMode::Queue { Some(app.library_selected as u32) } else { None };
                 
                 let item_clone = item.clone();
                 let result_items = with_mpd(app, args, |mpd| {
                    match mode {
                        app::LibraryMode::Queue => {
                            if let Some(idx) = queue_idx { let _ = mpd.switch(idx); }
                            None
                        }
                        app::LibraryMode::Directory => {
                            if let Some(target) = item_clone {
                                match target.item_type {
                                    app::LibraryItemType::Folder => {
                                        if let Some(path) = &target.path {
                                            return fetch_directory_items(mpd, path).ok();
                                        }
                                    },
                                    app::LibraryItemType::Song => {
                                        if let Some(path) = &target.path {
                                            if let Ok(id) = mpd.push(mpd::song::Song { file: path.clone(), ..Default::default() }) {
                                                 let _ = mpd.switch(id);
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            None
                        }
                        app::LibraryMode::Search => { 
                            if let Some(target) = item_clone {
                                if let Some(path) = &target.path {
                                    let song = mpd::Song { file: path.clone(), ..Default::default() };
                                    if let Ok(id) = mpd.push(&song) {
                                        let _ = mpd.switch(id);
                                    }
                                }
                            }
                            None
                        },
                        app::LibraryMode::Playlists => {
                            if let Some(pl) = pl_name {
                                let _ = mpd.load(&pl, ..);
                            }
                            None
                        }
                    }
                 });

                 // Post-processing for Directory change
                 if mode == app::LibraryMode::Directory {
                    if let Some(Some(items)) = result_items {
                         // We successfully fetched items, meaning we descended
                         if let Some(target) = item {
                             if let Some(_path) = target.path {
                                 // Update path in app *after* (so we don't break logic if fetch fails?) 
                                 // Actually safe to update now
                                 // Wait, `path` here is full path. `browse_path` is usually a list of components?
                                 // Looking at old code: `app.browse_path.push(item.name.clone());`
                                 // We need to replicate that.
                                 if target.item_type == app::LibraryItemType::Folder {
                                     app.browse_path.push(target.name);
                                     app.library_selected = 0;
                                     app.library_items = items;
                                 }
                             }
                         }
                    }
                 }
             }
             return;
        }

        // Search Exit (Esc)
        if (keys.matches(key, &keys.back_dir_alt) || keys.matches(key, &keys.back_dir)) && app.library_mode == app::LibraryMode::Search {
            // Restore previous mode or default to Directory
            let target_mode = app.previous_library_mode.take().unwrap_or(app::LibraryMode::Directory);
            app.library_mode = target_mode;
            app.search_query.clear();
            app.library_selected = 0;

            #[cfg(feature = "mpd")]
            if !args.controller {
                let current_path = app.browse_path.join("/");
                
                let (new_items, new_playlists) = with_mpd(app, args, |mpd| {
                    match target_mode {
                        app::LibraryMode::Directory => {
                            (fetch_directory_items(mpd, &current_path).ok(), None)
                        },
                        app::LibraryMode::Playlists => {
                            if let Ok(playlists) = mpd.playlists() {
                                let items = playlists.iter().map(|p| app::LibraryItem {
                                    name: p.name.clone(),
                                    item_type: app::LibraryItemType::Playlist,
                                    artist: None, duration_ms: None, path: None
                                }).collect();
                                (Some(items), Some(playlists))
                            } else { (None, None) }
                        },
                        _ => (None, None)
                    }
                }).unwrap_or((None, None));

                if let Some(items) = new_items {
                    app.library_items = items;
                }
                if let Some(playlists) = new_playlists {
                    app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                }
                if target_mode == app::LibraryMode::Queue {
                    app.library_items.clear();
                }
            }
            return;
        }

        // Backspace Browser
        if (keys.matches(key, &keys.back_dir) || keys.matches(key, &keys.back_dir_alt)) && app.library_mode == app::LibraryMode::Directory {
            app.browse_path.pop();
            app.library_items.clear();
            app.library_selected = 0;

            // Re-fetch items for the parent level
            #[cfg(feature = "mpd")]
            if !args.controller {
                // Build the path from browse_path
                let parent_path = if app.browse_path.is_empty() {
                    "".to_string()
                } else {
                    app.browse_path.join("/")
                };

                if let Some(items) = with_mpd(app, args, |mpd| {
                    fetch_directory_items(mpd, &parent_path).ok()
                }).flatten() {
                    app.library_items = items;
                }
            }
             return;
        }

        // Navigation
        if keys.matches(key, &keys.nav_up) || keys.matches(key, &keys.nav_up_alt) {
            app.library_selected = app.library_selected.saturating_sub(1);
            return;
        }
        if keys.matches(key, &keys.nav_down) || keys.matches(key, &keys.nav_down_alt) {
             let max_items = match app.library_mode {
                app::LibraryMode::Queue => app.queue.len().max(1),
                app::LibraryMode::Directory if app.browse_path.is_empty() => 4,
                app::LibraryMode::Playlists => app.playlists.len().max(1),
                _ => app.library_items.len().max(1),
            };
            if app.library_selected < max_items.saturating_sub(1) {
                app.library_selected += 1;
            }
            return;
        }
    }

    // Shuffle toggle
    if keys.matches(key, &keys.shuffle) {
        if args.controller {
            let new_state = !app.shuffle;
            let _ = player.shuffle(new_state);
            app.shuffle = new_state;
            app.show_toast(&format!("🔀 Shuffle: {}", if new_state { "ON" } else { "OFF" }));
        } else {
            #[cfg(feature = "mpd")]
            {
                let new_shuffle_state = with_mpd(app, args, |mpd| {
                    if let Ok(status) = mpd.status() {
                         let new_state = !status.random;
                         let _ = mpd.random(new_state);
                         Some(new_state)
                    } else { None }
                }).flatten();

                if let Some(state) = new_shuffle_state {
                    app.shuffle = state;
                    app.show_toast(&format!("🔀 Shuffle: {}", if state { "ON" } else { "OFF" }));
                }
            }

        }
         return;
    }

    // Repeat toggle
    if keys.matches(key, &keys.repeat) {
        if args.controller {
            let new_state = !app.repeat;
            let _ = player.repeat(new_state);
            app.repeat = new_state;
            app.show_toast(&format!("🔁 Repeat: {}", if new_state { "ON" } else { "OFF" }));
        } else {
            #[cfg(feature = "mpd")]
            {
                let new_repeat_state = with_mpd(app, args, |mpd| {
                    if let Ok(status) = mpd.status() {
                         let new_state = !status.repeat;
                         let _ = mpd.repeat(new_state);
                         Some(new_state)
                    } else { None }
                }).flatten();

                if let Some(state) = new_repeat_state {
                    app.repeat = state;
                    app.show_toast(&format!("🔁 Repeat: {}", if state { "ON" } else { "OFF" }));
                }
            }
        }

    }
}


