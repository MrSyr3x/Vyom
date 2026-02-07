use crossterm::event::KeyCode;
use crate::app::{self, App};
use crate::player::PlayerTrait;
use crate::audio::pipeline::AudioPipeline;
use crate::app::cli::Args;
use crate::app::events::AppEvent;
#[cfg(feature = "mpd")]
use crate::app::library_helpers::fetch_directory_items;
#[cfg(feature = "mpd")]
use crate::app::with_mpd;

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
                                                    let val = input.value.clone();
                                                    let result = with_mpd(app, args, |mpd| {
                                                        mpd.save(&val).map_err(|e| e.to_string())
                                                    });

                                                    if let Some(res) = result {
                                                        match res {
                                                            Ok(_) => {
                                                                app.show_toast(&format!("💾 Saved: {}", val));
                                                                app.playlists.push(val);
                                                            },
                                                            Err(e) => app.show_toast(&format!("❌ Error: {}", e)),
                                                        }
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
                                                    let new_name = input.value.clone();
                                                    let old = old_name.clone();
                                                    
                                                    let result = with_mpd(app, args, |mpd| {
                                                        match mpd.pl_rename(&old, &new_name) {
                                                            Ok(_) => mpd.playlists().map_err(|e| e.to_string()),
                                                            Err(e) => Err(e.to_string())
                                                        }
                                                    });

                                                    if let Some(res) = result {
                                                        match res {
                                                            Ok(playlists) => {
                                                                app.show_toast(&format!("✏️ Renamed: {} -> {}", old, new_name));
                                                                app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                                                                if app.library_mode == app::LibraryMode::Playlists {
                                                                     app.library_items = app.playlists.iter().map(|p| app::LibraryItem {
                                                                         name: p.clone(),
                                                                         item_type: app::LibraryItemType::Playlist,
                                                                         artist: None, duration_ms: None, path: None
                                                                     }).collect();
                                                                }
                                                            },
                                                            Err(e) => app.show_toast(&format!("❌ Error: {}", e)),
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
                                    let target = target_mode;
                                    let current_path = app.browse_path.join("/");

                                    if let Some(items) = with_mpd(app, args, |mpd| {
                                        match target {
                                            app::LibraryMode::Directory => {
                                                fetch_directory_items(mpd, &current_path).ok().map(|i| (None, i))
                                            },
                                            app::LibraryMode::Playlists => {
                                                if let Ok(playlists) = mpd.playlists() {
                                                    let items = playlists.iter().map(|p| app::LibraryItem {
                                                        name: p.name.clone(),
                                                        item_type: app::LibraryItemType::Playlist,
                                                        artist: None, duration_ms: None, path: None
                                                    }).collect();
                                                    Some((Some(playlists), items))
                                                } else { None }
                                            },
                                            _ => Some((None, Vec::new()))
                                        }
                                    }).flatten() {
                                        let (playlists_opt, items) = items;
                                        if let Some(playlists) = playlists_opt {
                                             app.playlists = playlists.iter().map(|p| p.name.clone()).collect();
                                        }
                                        
                                        if target == app::LibraryMode::Directory || target == app::LibraryMode::Playlists {
                                             app.library_items = items;
                                        } else if target == app::LibraryMode::Queue {
                                             app.library_items.clear();
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
                                    if let Some(songs) = with_mpd(app, args, |mpd| mpd.listall().ok()).flatten() {
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
