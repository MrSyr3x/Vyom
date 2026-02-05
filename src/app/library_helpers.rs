#[cfg(feature = "mpd")]
use crate::app::{LibraryItem, LibraryItemType};

// Helper to fetch directory contents (folders + songs)
#[cfg(feature = "mpd")]
pub fn fetch_directory_items(
    mpd: &mut mpd::Client,
    path: &str,
) -> Result<Vec<LibraryItem>, mpd::error::Error> {
    let mut items: Vec<LibraryItem> = Vec::new();

    // 1. Folders
    if let Ok(files) = mpd.listfiles(path) {
        for (kind, name) in files {
            let display_name = name.split('/').next_back().unwrap_or(&name).to_string();
            if display_name.starts_with('.') || display_name.trim().is_empty() {
                continue;
            }

            if kind == "directory" {
                let full_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", path, name)
                };
                items.push(LibraryItem {
                    name: display_name,
                    item_type: LibraryItemType::Folder,
                    artist: None,
                    duration_ms: None,
                    path: Some(full_path),
                });
            }
        }
    }

    // 2. Songs
    if let Ok(songs) = mpd.lsinfo(&mpd::Song {
        file: path.to_string(),
        ..Default::default()
    }) {
        for song in songs {
            let filename = song
                .file
                .split('/')
                .next_back()
                .unwrap_or(&song.file)
                .to_string();
            if filename.starts_with('.') || filename.trim().is_empty() {
                continue;
            }

            let title = match song.title.as_ref().filter(|t| !t.trim().is_empty()) {
                Some(t) => t.clone(),
                None => filename.clone(),
            };

            items.push(LibraryItem {
                name: title,
                item_type: LibraryItemType::Song,
                artist: song.artist.clone(),
                duration_ms: song.duration.map(|d| d.as_millis() as u64),
                path: Some(song.file),
            });
        }
    }

    // Sort: folders first
    items.sort_by(|a, b| match (&a.item_type, &b.item_type) {
        (LibraryItemType::Folder, LibraryItemType::Song) => std::cmp::Ordering::Less,
        (LibraryItemType::Song, LibraryItemType::Folder) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(items)
}

// Recursive Add Helper
#[cfg(feature = "mpd")]
pub fn queue_folder_recursive(
    mpd: &mut mpd::Client,
    path: &str,
) -> Result<(), mpd::error::Error> {
    // Determine path for listfiles
    // Note: listfiles logic in correct MPD (and fetch_directory_items) usually returns components
    
    // We use listfiles to get EVERYTHING (files + dirs) cheaply
    if let Ok(mut entries) = mpd.listfiles(path) {
        // Sort: Directory < File, then Name alpha
        entries.sort_by(|(ka, na), (kb, nb)| {
             // Directories first
             if ka == "directory" && kb != "directory" { return std::cmp::Ordering::Less; }
             if ka != "directory" && kb == "directory" { return std::cmp::Ordering::Greater; }
             // Alpha sort by name
             na.to_lowercase().cmp(&nb.to_lowercase())
        });
        
        for (kind, name) in entries {
             let full_path = if path.is_empty() {
                 name.clone()
             } else {
                 format!("{}/{}", path, name)
             };
             
             if kind == "directory" {
                 // Recurse (ignore errors to keep processing siblings)
                 let _ = queue_folder_recursive(mpd, &full_path);
             } else if kind == "file" {
                 // Add song (ignore errors)
                 let _ = mpd.push(mpd::Song { file: full_path, ..Default::default() });
             }
        }
    }
    Ok(())
}
