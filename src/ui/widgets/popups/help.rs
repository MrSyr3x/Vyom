use crate::app::{App, ViewMode};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
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

    // Build popup content first to calculate exact height
    let mut lines: Vec<Line> = Vec::new();

    // Context keys
    for (key, icon, desc) in &keys {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<7} ", key),
                Style::default()
                    .fg(theme.yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   ", Style::default().fg(theme.overlay)), // Cleaner spacer
            Span::styled(format!("{} ", icon), Style::default()),
            Span::styled(*desc, Style::default().fg(theme.text)),
        ]));
    }

    if !keys.is_empty() {
        lines.push(Line::from(""));
    }

    // Global section - Left aligned with divider
    lines.push(Line::from(Span::styled(
        "────── Global ──────",
        Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (key, icon, desc) in &global_keys {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<7} ", key),
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   ", Style::default().fg(theme.overlay)), // Cleaner spacer
            Span::styled(format!("{} ", icon), Style::default()),
            Span::styled(*desc, Style::default().fg(theme.text)),
        ]));
    }

    // Calculate popup size - fit content exactly 📏
    let content_width = keys
        .iter()
        .chain(global_keys.iter())
        .map(|(k, _i, d)| {
            // " kkkkkkk    ii ddddddd"
            // padding(1) + key(max 7) + padding(1) + spacer(3) + icon/space(3) + desc
            // We use fixed 7 for key alignment, but if key > 7 it expands
            2 + k.len().max(7) + 3 + 3 + d.len()
        })
        .max()
        .unwrap_or(20) // Minimum width
        .max(22); // "────── Global ──────" length

    let max_height = f.area().height.saturating_sub(4);
    let popup_height = (lines.len() as u16 + 2).min(max_height); // +2 for borders
    let popup_width = (content_width as u16 + 4).min(f.area().width.saturating_sub(2));

    // Position at bottom-right
    let popup_x = f.area().width.saturating_sub(popup_width + 1);
    let popup_y = f.area().height.saturating_sub(popup_height + 2);
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    f.render_widget(Clear, popup_area);

    let popup = Paragraph::new(lines).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.blue))
            .title(format!(" {} ", title))
            .title_alignment(Alignment::Left)
            .style(Style::default().bg(Color::Reset)),
    );
    f.render_widget(popup, popup_area);
}
