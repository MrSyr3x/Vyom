use crossterm::event::KeyCode;
use crate::app::{self, App};
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
                        // Don't process other key handlers while tag editor is open
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
                        // Priority Handler for Global Search 🔍
                        // Ensures / works even if Directory mode or other states would otherwise shadow it
                        #[cfg(feature = "mpd")]
                        if key.code == KeyCode::Char('/') && !args.controller {
                            app.view_mode = app::ViewMode::Library;
                            // Save current mode before switching to Search, unless we are already searching
                            if app.library_mode != app::LibraryMode::Search {
                                app.previous_library_mode = Some(app.library_mode);
                            }
                            app.library_mode = app::LibraryMode::Search;
                            app.search_active = true;
                            // Critical: Clear items so we don't see previous Directory contents
                            app.library_items.clear();
                            app.library_selected = 0;
                            return; // Consume event immediately
                        }

                        // Normal key handling when NOT typing in search
                        // Dynamic Key Handler (delegated)
                        // This allows for user-configurable keybindings via config.toml
                        crate::app::input_handler::handle_normal_mode(
                            key,
                            app,
                            player,
                            audio_pipeline,
                            args,
                            _tx,
                            _client
                        ).await;
                    }
}
