use crossterm::event::KeyEvent;
use crate::app::{self, App};
#[cfg(feature = "mpd")]
use crate::app::with_mpd;
use crate::app::cli::Args;
#[cfg(feature = "mpd")]
use crate::app::library_helpers::fetch_directory_items;
#[cfg(feature = "mpd")]
use lofty::file::TaggedFileExt;
#[cfg(feature = "mpd")]
use lofty::tag::Accessor;

pub fn handle_library_events(
    key: KeyEvent,
    app: &mut App,
    args: &Args,
) -> bool {
    let keys = &app.keys;



    if app.view_mode != app::ViewMode::Library {
        return false;
    }

    // Queue Reordering with J/K (Shift+j/k)
    if keys.matches(key, &keys.move_down) && app.library_mode == app::LibraryMode::Queue {
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
        return true;
    }

    if keys.matches(key, &keys.move_up) && app.library_mode == app::LibraryMode::Queue {
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
        return true;
    }

    // Tab Navigation
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
        return true;
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
        return true;
    }

    if keys.matches(key, &keys.save_playlist) {
            app.input_state = Some(app::InputState::new(
            app::InputMode::PlaylistSave,
            "Save Playlist As:",
            ""
        ));
        return true;
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
        return true;
    }

    if keys.matches(key, &keys.edit_tags) {
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
        return true;
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

            let (new_playlists, success) = with_mpd(app, args, |mpd| {
                match mode {
                    LibraryMode::Queue => { 
                        let ok = mpd.delete(selected as u32).is_ok();
                        (None, ok) 
                    },
                    LibraryMode::Playlists => {
                            if let Some(name) = pl_name {
                            let ok = mpd.pl_remove(&name).is_ok();
                            (mpd.playlists().ok(), ok) // Return new playlists and success status
                        } else { (None, false) }
                    },
                    _ => (None, false)
                }
            }).unwrap_or((None, false)); // Default to failure if MPD connection fails

            if let Some(pls) = new_playlists {
                app.playlists = pls.iter().map(|p| p.name.clone()).collect();
            }

            if success {
                match mode {
                    LibraryMode::Queue => app.show_toast("🗑️ Removed from Queue"),
                    LibraryMode::Playlists => app.show_toast("🗑️ Playlist Deleted"),
                    _ => {}
                }
            }
            
            if mode == LibraryMode::Playlists && app.library_selected > 0 && app.library_selected >= app.playlists.len() {
                    app.library_selected = app.library_selected.saturating_sub(1);
            }
            }
            return true;
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
            return true;
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
            return true;
    }

    // Search Exit (Esc) or Back Directory
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
        return true;
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
            return true;
    }

    // Navigation
    if keys.matches(key, &keys.nav_up) || keys.matches(key, &keys.nav_up_alt) {
        app.library_selected = app.library_selected.saturating_sub(1);
        return true;
    }
    if keys.matches(key, &keys.nav_down) || keys.matches(key, &keys.nav_down_alt) {
            let max_items = match app.library_mode {
            app::LibraryMode::Queue => app.queue.len().max(1),
            // Fix: No hardcoded check
            app::LibraryMode::Playlists => app.playlists.len().max(1),
            _ => app.library_items.len().max(1),
        };
        if app.library_selected < max_items.saturating_sub(1) {
            app.library_selected += 1;
        }
        return true;
    }
    
    false
}
