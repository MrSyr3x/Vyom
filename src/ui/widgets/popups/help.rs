use crate::app::{App, ViewMode};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let theme = &app.theme;

    // 🎹 WhichKey-style floating popup (Helix-inspired, centered)

    // Get context-specific keybindings with icons
    // Use String for key display to support dynamic config
    let (title, keys): (&str, Vec<(String, &str, &str)>) = match app.view_mode {
        ViewMode::EQ => (
            "EQ Controls",
            vec![
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.band_prev),
                        app.keys.display(&app.keys.band_next)
                    ),
                    "🎚️",
                    "Select band",
                ),
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.gain_up),
                        app.keys.display(&app.keys.gain_down)
                    ),
                    "📊",
                    "Adjust gain",
                ),
                (app.keys.display(&app.keys.next_preset), "🎵", "Next preset"),
                (app.keys.display(&app.keys.toggle_eq), "⚡", "Toggle EQ"),
                (app.keys.display(&app.keys.reset_eq), "↺", "Reset EQ"),
                (
                    app.keys.display(&app.keys.reset_levels),
                    "🎯",
                    "Reset Levels",
                ),
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.preamp_up),
                        app.keys.display(&app.keys.preamp_down)
                    ),
                    "🔊",
                    "Preamp ±1dB",
                ),
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.balance_right),
                        app.keys.display(&app.keys.balance_left)
                    ),
                    "⚖️",
                    "Balance ±0.1",
                ),
                (app.keys.display(&app.keys.crossfade), "🔀", "Crossfade"),
                (app.keys.display(&app.keys.replay_gain), "📀", "ReplayGain"),
                (app.keys.display(&app.keys.save_preset), "💾", "Save preset"),
                (
                    app.keys.display(&app.keys.delete_preset),
                    "🗑️",
                    "Delete preset",
                ),
            ],
        ),
        ViewMode::Library => (
            "Library",
            vec![
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.nav_down),
                        app.keys.display(&app.keys.nav_up)
                    ),
                    "📋",
                    "Navigate",
                ),
                (app.keys.display(&app.keys.tab_next), "🔄", "Switch mode"),
                (app.keys.display(&app.keys.enter_dir), "▶️", "Select/Play"),
                (app.keys.display(&app.keys.back_dir), "←", "Go back"),
                (app.keys.display(&app.keys.search_global), "🔍", "Search"),
                (
                    app.keys.display(&app.keys.add_to_queue),
                    "➕",
                    "Add to Queue",
                ),
                (
                    app.keys.display(&app.keys.save_playlist),
                    "💾",
                    "Save playlist",
                ),
                (
                    app.keys.display(&app.keys.rename_playlist),
                    "✏️",
                    "Rename playlist",
                ),
                (
                    app.keys.display(&app.keys.delete_item),
                    "🗑️",
                    "Delete/Remove",
                ),
                (app.keys.display(&app.keys.edit_tags), "🏷️", "Edit tags"),
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.move_down),
                        app.keys.display(&app.keys.move_up)
                    ),
                    "🔃",
                    "Reorder",
                ),
            ],
        ),
        ViewMode::Lyrics => (
            "Lyrics",
            vec![
                (
                    format!(
                        "{}/{}",
                        app.keys.display(&app.keys.nav_down),
                        app.keys.display(&app.keys.nav_up)
                    ),
                    "📜",
                    "Scroll lyrics",
                ),
                (
                    app.keys.display(&app.keys.seek_to_line),
                    "🎤",
                    "Jump to line",
                ),
            ],
        ),
        ViewMode::Visualizer => ("Visualizer", vec![]),
    };

    // Global keys - mode-specific
    let global_keys: Vec<(String, &str, &str)> = if app.is_mpd {
        // MPD mode: full feature set
        vec![
            (app.keys.display(&app.keys.play_pause), "▶️", "Play/Pause"),
            (app.keys.display(&app.keys.next_track), "⏭️", "Next track"),
            (
                app.keys.display(&app.keys.prev_track),
                "⏮️",
                "Previous track",
            ),
            (app.keys.display(&app.keys.shuffle), "🔀", "Shuffle"),
            (app.keys.display(&app.keys.repeat), "🔁", "Repeat"),
            (app.keys.display(&app.keys.search_global), "🔍", "Search"),
            (
                format!(
                    "{}/{}",
                    app.keys.display(&app.keys.volume_up),
                    app.keys.display(&app.keys.volume_down)
                ),
                "🔊",
                "Volume",
            ),
            (format!("1-{}", "4"), "🖼️", "View modes"),
            (
                format!(
                    "{}/{}",
                    app.keys.display(&app.keys.seek_backward),
                    app.keys.display(&app.keys.seek_forward)
                ),
                "⏩",
                "Seek ±5s",
            ),
            (
                format!(
                    "{}/{}",
                    app.keys.display(&app.keys.device_next),
                    app.keys.display(&app.keys.device_prev)
                ),
                "🎧",
                "Output device",
            ),
            (
                app.keys.display(&app.keys.toggle_audio_info),
                "ℹ️",
                "Audio info",
            ),
            (app.keys.display(&app.keys.quit), "🚪", "Quit"),
        ]
    } else {
        // Controller mode: limited keys (no shuffle/repeat - not available)
        vec![
            (app.keys.display(&app.keys.play_pause), "▶️", "Play/Pause"),
            (app.keys.display(&app.keys.next_track), "⏭️", "Next track"),
            (
                app.keys.display(&app.keys.prev_track),
                "⏮️",
                "Previous track",
            ),
            (
                format!(
                    "{}/{}",
                    app.keys.display(&app.keys.volume_up),
                    app.keys.display(&app.keys.volume_down)
                ),
                "🔊",
                "Volume",
            ),
            (
                format!(
                    "{}/{}",
                    app.keys.display(&app.keys.seek_backward),
                    app.keys.display(&app.keys.seek_forward)
                ),
                "⏩",
                "Seek ±5s",
            ),
            (
                format!(
                    "{}/{}",
                    app.keys.display(&app.keys.device_next),
                    app.keys.display(&app.keys.device_prev)
                ),
                "🎧",
                "Output device",
            ),
            (
                app.keys.display(&app.keys.toggle_audio_info),
                "ℹ️",
                "Audio info",
            ),
            (app.keys.display(&app.keys.quit), "🚪", "Quit"),
        ]
    };

    // Mathematical Explicit Tracking - Disjoint Grid Renderer
    // Completely unlinks Emoji width calculations from text rendering shifts!
    use unicode_width::UnicodeWidthStr;

    let max_key_w = keys
        .iter()
        .chain(global_keys.iter())
        .map(|(k, _, _)| k.width())
        .max()
        .unwrap_or(7)
        .max(7);

    let max_desc_w = keys
        .iter()
        .chain(global_keys.iter())
        .map(|(_, _, d)| d.width())
        .max()
        .unwrap_or(20);

    let content_width = (max_key_w + max_desc_w + 7).max(22);

    let mut num_lines = keys.len();
    if !keys.is_empty() {
        num_lines += 1;
    }
    num_lines += 2; // global title + empty line
    num_lines += global_keys.len();

    let max_height = f.area().height.saturating_sub(4);
    let popup_height = (num_lines as u16 + 2).min(max_height);
    let popup_width = (content_width as u16 + 2).min(f.area().width.saturating_sub(2));

    // Anchored identically to the original layout (flush right)
    let margin_right = 1;
    let margin_bottom = 2;
    let popup_x = f.area().width.saturating_sub(popup_width + margin_right);
    let popup_y = f.area().height.saturating_sub(popup_height + margin_bottom);
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // EXACT Wipe Area: Do not over-wipe! Previously, an oversized Clear widget
    // broke the main UI boundaries. Now we only wipe exactly what we draw.
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.blue))
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Left);

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut current_y = inner_area.y;

    let draw_row = |f: &mut Frame, y: u16, key: &str, icon: &str, desc: &str| {
        if y >= inner_area.bottom() {
            return;
        }

        let key_span = Span::styled(
            format!("{:<width$}", key, width = max_key_w),
            Style::default()
                .fg(theme.yellow)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(
            Paragraph::new(key_span),
            Rect::new(inner_area.x + 1, y, max_key_w as u16, 1),
        );

        f.render_widget(
            Paragraph::new(icon.to_string()),
            Rect::new(inner_area.x + 1 + max_key_w as u16 + 2, y, 2, 1),
        );

        let desc_span = Span::styled(desc.to_string(), Style::default().fg(theme.text));
        f.render_widget(
            Paragraph::new(desc_span),
            Rect::new(
                inner_area.x + 1 + max_key_w as u16 + 5,
                y,
                max_desc_w as u16,
                1,
            ),
        );
    };

    // Draw Context Keys
    for (key, icon, desc) in &keys {
        draw_row(f, current_y, key, icon, desc);
        current_y += 1;
    }

    if !keys.is_empty() {
        current_y += 1;
    }

    if current_y < inner_area.bottom() {
        let global_title = Span::styled(
            "────── Global ──────",
            Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
        );
        f.render_widget(
            Paragraph::new(global_title),
            Rect::new(
                inner_area.x + 1,
                current_y,
                inner_area.width.saturating_sub(2),
                1,
            ),
        );
        current_y += 2;
    }

    // Draw Global Keys
    for (key, icon, desc) in &global_keys {
        draw_row(f, current_y, key, icon, desc);
        current_y += 1;
    }
}
