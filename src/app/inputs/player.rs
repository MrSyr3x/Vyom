use crate::app::cli::Args;
#[cfg(feature = "mpd")]
use crate::app::with_mpd;
use crate::app::{self, App};
use crate::audio::pipeline::AudioPipeline;
use crate::player::PlayerTrait;
use crossterm::event::KeyEvent;
use std::sync::Arc;

pub async fn handle_player_events(
    key: KeyEvent,
    app: &mut App,
    player: &Arc<dyn PlayerTrait>,
    audio_pipeline: &mut AudioPipeline,
    args: &Args,
) -> bool {
    let keys = &app.keys;

    // Play/Pause ('Space')
    if keys.matches(key, &keys.play_pause) {
        if let Ok(is_playing) = player.play_pause() {
            app.show_toast(if is_playing { "▶ Play" } else { "⏸ Pause" });
        }
        return true;
    }

    // Next Track ('n')
    if keys.matches(key, &keys.next_track) {
        let _ = player.next();
        app.show_toast("⏭ Next Track");
        return true;
    }

    // Prev Track ('p')
    if keys.matches(key, &keys.prev_track) {
        let _ = player.prev();
        app.show_toast("⏮ Previous Track");
        return true;
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
        return true;
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
        return true;
    }

    // Seek Backward ('h' or 'Left') - blocked in EQ
    if (keys.matches(key, &keys.seek_backward) || keys.matches(key, &keys.nav_left_alt))
        && app.view_mode != app::ViewMode::EQ
    {
        let now = std::time::Instant::now();
        let is_new_sequence = if let Some(last) = app.last_seek_time {
            now.duration_since(last).as_millis() >= 500
        } else {
            true
        };

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
            app.seek_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let my_id = app.seek_id.load(std::sync::atomic::Ordering::Relaxed);
            let global_seek_id = app.seek_id.clone();

            let player_bg = player.clone();
            let original_track_key = app
                .track
                .as_ref()
                .map(|t| (t.name.clone(), t.artist.clone()));
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
        return true;
    }

    // Seek Forward ('l' or 'Right') - blocked in EQ
    if (keys.matches(key, &keys.seek_forward) || keys.matches(key, &keys.nav_right_alt))
        && app.view_mode != app::ViewMode::EQ
    {
        let now = std::time::Instant::now();
        let is_new_sequence = if let Some(last) = app.last_seek_time {
            now.duration_since(last).as_millis() >= 500
        } else {
            true
        };

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
            app.seek_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let my_id = app.seek_id.load(std::sync::atomic::Ordering::Relaxed);
            let global_seek_id = app.seek_id.clone();

            let player_bg = player.clone();
            let original_track_key = app
                .track
                .as_ref()
                .map(|t| (t.name.clone(), t.artist.clone()));
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
        return true;
    }

    // Shuffle toggle
    if keys.matches(key, &keys.shuffle) {
        if args.controller {
            let new_state = !app.shuffle;
            let _ = player.shuffle(new_state);
            app.shuffle = new_state;
            app.show_toast(&format!(
                "🔀 Shuffle: {}",
                if new_state { "ON" } else { "OFF" }
            ));
        } else {
            #[cfg(feature = "mpd")]
            {
                let new_shuffle_state = with_mpd(app, args, |mpd| {
                    if let Ok(status) = mpd.status() {
                        let new_state = !status.random;
                        let _ = mpd.random(new_state);
                        Some(new_state)
                    } else {
                        None
                    }
                })
                .flatten();

                if let Some(state) = new_shuffle_state {
                    app.shuffle = state;
                    app.show_toast(&format!("🔀 Shuffle: {}", if state { "ON" } else { "OFF" }));
                }
            }
        }
        return true;
    }

    // Repeat toggle
    if keys.matches(key, &keys.repeat) {
        use crate::player::RepeatMode;

        let next_mode = match app.repeat {
            RepeatMode::Off => RepeatMode::Playlist,
            RepeatMode::Playlist => RepeatMode::Single,
            RepeatMode::Single => RepeatMode::Off,
        };

        if args.controller {
            let _ = player.repeat(next_mode);
            app.repeat = next_mode;
            let icon = match next_mode {
                RepeatMode::Off => "OFF",
                RepeatMode::Playlist => "🔁 All",
                RepeatMode::Single => "🔂 One",
            };
            app.show_toast(&format!("Repeat: {}", icon));
        } else {
            #[cfg(feature = "mpd")]
            {
                let new_mode = with_mpd(app, args, |mpd| {
                    // We need to set repeat and single flags manually based on mode
                    let (repeat, single) = match next_mode {
                        RepeatMode::Off => (false, false),
                        RepeatMode::Playlist => (true, false),
                        RepeatMode::Single => (true, true),
                    };

                    if mpd.repeat(repeat).is_ok() {
                        let _ = mpd.single(single);
                        Some(next_mode)
                    } else {
                        None
                    }
                })
                .flatten();

                if let Some(mode) = new_mode {
                    app.repeat = mode;
                    let icon = match mode {
                        RepeatMode::Off => "OFF",
                        RepeatMode::Playlist => "🔁 All",
                        RepeatMode::Single => "🔂 One",
                    };
                    app.show_toast(&format!("Repeat: {}", icon));
                }
            }
        }
        return true;
    }

    // Audio Device Switching
    if app.view_mode == app::ViewMode::Lyrics
        || app.view_mode == app::ViewMode::Visualizer
        || app.view_mode == app::ViewMode::EQ
    {
        if keys.matches(key, &keys.device_next) {
            app.next_device();
            return true;
        }
        if keys.matches(key, &keys.device_prev) {
            app.prev_device();
            return true;
        }
    }

    false
}
