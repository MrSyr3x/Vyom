use crate::app::cli::Args;
#[cfg(feature = "mpd")]
use crate::app::with_mpd;
use crate::app::{self, App};
use crossterm::event::KeyEvent;

pub fn handle_eq_events(key: KeyEvent, app: &mut App, args: &Args) -> bool {
    let keys = &app.keys;

    if app.view_mode != app::ViewMode::EQ {
        return false;
    }

    // EQ Save Preset (Shift+S)
    if keys.matches(key, &keys.save_preset) {
        app.input_state = Some(app::InputState::new(
            app::InputMode::EqSave,
            "Save Preset As",
            "",
        ));
        return true;
    }

    // EQ Delete Preset (Shift+X)
    if keys.matches(key, &keys.delete_preset) {
        if let Err(e) = app.delete_preset() {
            app.show_toast(&format!("❌ {}", e));
        } else {
            app.show_toast("🗑️ Preset Deleted");
        }
        return true;
    }

    if keys.matches(key, &keys.nav_left) || keys.matches(key, &keys.nav_left_alt) {
        app.eq_selected = app.eq_selected.saturating_sub(1);
        return true;
    }
    if keys.matches(key, &keys.nav_right) || keys.matches(key, &keys.nav_right_alt) {
        if app.eq_selected < 9 {
            app.eq_selected += 1;
        }
        return true;
    }
    if keys.matches(key, &keys.gain_up) || keys.matches(key, &keys.nav_up_alt) {
        let band = &mut app.eq_bands[app.eq_selected];
        *band = (*band + 0.05).min(1.0);
        app.mark_custom();
        app.sync_band_to_dsp(app.eq_selected);
        let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
        app.show_toast(&format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db));
        return true;
    }
    if keys.matches(key, &keys.gain_down) || keys.matches(key, &keys.nav_down_alt) {
        let band = &mut app.eq_bands[app.eq_selected];
        *band = (*band - 0.05).max(0.0);
        app.mark_custom();
        app.sync_band_to_dsp(app.eq_selected);
        let db = (app.eq_bands[app.eq_selected] - 0.5) * 24.0;
        app.show_toast(&format!("🎚 Band {}: {:+.1}dB", app.eq_selected + 1, db));
        return true;
    }
    if keys.matches(key, &keys.toggle_eq) {
        app.toggle_eq();
        app.show_toast(&format!(
            "🎛 EQ: {}",
            if app.eq_enabled { "ON" } else { "OFF" }
        ));
        return true;
    }
    if keys.matches(key, &keys.reset_eq) {
        app.reset_eq();
        app.show_toast("🔄 EQ Reset");
        return true;
    }
    if keys.matches(key, &keys.reset_levels) {
        app.reset_preamp();
        app.reset_balance();
        // app.mark_custom(); // Removed to keep current preset
        app.sync_band_to_dsp(app.eq_selected);
        app.show_toast("🎯 Levels Reset");
        return true;
    }
    if keys.matches(key, &keys.tab_next) {
        app.next_preset();
        app.show_toast(&format!("🎵 Preset: {}", app.get_preset_name()));
        return true;
    }
    if keys.matches(key, &keys.tab_prev) {
        app.prev_preset();
        app.show_toast(&format!("🎵 Preset: {}", app.get_preset_name()));
        return true;
    }
    if keys.matches(key, &keys.preamp_up) {
        app.adjust_preamp(1.0);
        return true;
    }
    if keys.matches(key, &keys.preamp_down) {
        app.adjust_preamp(-1.0);
        return true;
    }
    if keys.matches(key, &keys.balance_right) {
        app.adjust_balance(0.1);
        return true;
    }
    if keys.matches(key, &keys.balance_left) {
        app.adjust_balance(-0.1);
        return true;
    }
    if keys.matches(key, &keys.crossfade) {
        app.toggle_crossfade();
        #[cfg(feature = "mpd")]
        if !args.controller {
            let secs = app.crossfade_secs as i64;
            with_mpd(app, args, |mpd| {
                let _ = mpd.crossfade(secs);
            });
        }
        return true;
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
        return true;
    }

    false
}
