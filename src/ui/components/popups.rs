use crate::app::{App, ViewMode};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    // AUDIO INFO POPUP
    if app.show_audio_info {
        render_audio_info(f, app);
    }

    // TOAST NOTIFICATION
    if let Some(ref _val) = app.toast {
        // Need to clone strictly necessary data to avoid borrow checker issues if we pass &mut app
        // But app.toast is accessible.
        // The render logic modifies nothing, so &App or &mut App is fine.
        // We need to implement the toast logic here.
        render_toast(f, app);
    }

    // INPUT POPUP
    if app.input_state.is_some() {
        render_input(f, app);
    }

    // TAG EDITOR POPUP
    if app.tag_edit.is_some() {
        render_tag_editor(f, app);
    }

    // FOOTER / WHICHKEY POPUP
    if app.show_keyhints {
        render_keyhints(f, app);
    }
}

fn render_audio_info(f: &mut Frame, app: &App) {
    let theme = &app.theme;

    // 1. Generate Content First
    let mut lines: Vec<Line> = Vec::new();

    // Track Info Section
    lines.push(Line::from(vec![
        Span::styled("♫ ", Style::default().fg(theme.magenta)),
        Span::styled(
            "Track",
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Current song info from TrackInfo
    if let Some(ref track) = app.track {
        lines.push(Line::from(vec![
            Span::styled("  Title: ", Style::default().fg(theme.overlay)),
            Span::styled(&track.name, Style::default().fg(theme.text)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Artist: ", Style::default().fg(theme.overlay)),
            Span::styled(&track.artist, Style::default().fg(theme.text)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Album: ", Style::default().fg(theme.overlay)),
            Span::styled(&track.album, Style::default().fg(theme.text)),
        ]));

        // Audiophile metadata if available
        if let Some(ref codec) = track.codec {
            let mut format_parts = vec![codec.clone()];
            if let Some(depth) = track.bit_depth {
                format_parts.push(format!("{} bit", depth));
            }
            if let Some(rate) = track.sample_rate {
                format_parts.push(format!("{:.1} kHz", rate as f32 / 1000.0));
            }
            if let Some(bitrate) = track.bitrate {
                format_parts.push(format!("{} kbps", bitrate));
            }
            lines.push(Line::from(vec![
                Span::styled("  Format: ", Style::default().fg(theme.overlay)),
                Span::styled(format_parts.join(" / "), Style::default().fg(theme.green)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No track playing",
            Style::default().fg(theme.overlay),
        )]));
    }

    // Queue position
    let queue_pos = format!("{} / {}", app.library_selected + 1, app.queue.len());
    lines.push(Line::from(vec![
        Span::styled("  Queue: ", Style::default().fg(theme.overlay)),
        Span::styled(queue_pos, Style::default().fg(theme.text)),
    ]));

    lines.push(Line::from(""));

    // Playback Status Section
    lines.push(Line::from(vec![
        Span::styled("▶ ", Style::default().fg(theme.green)),
        Span::styled(
            "Playback",
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    let is_paused = app
        .track
        .as_ref()
        .map(|t| t.state == crate::player::PlayerState::Paused)
        .unwrap_or(true);
    let status = if is_paused { "Paused" } else { "Playing" };
    lines.push(Line::from(vec![
        Span::styled("  Status: ", Style::default().fg(theme.overlay)),
        Span::styled(
            status,
            Style::default().fg(if is_paused {
                theme.yellow
            } else {
                theme.green
            }),
        ),
    ]));

    // Shuffle/Repeat only shown in MPD mode (not available in controller)
    if app.is_mpd {
        let shuffle_str = if app.shuffle { "ON" } else { "OFF" };
        lines.push(Line::from(vec![
            Span::styled("  Shuffle: ", Style::default().fg(theme.overlay)),
            Span::styled(
                shuffle_str,
                Style::default().fg(if app.shuffle {
                    theme.green
                } else {
                    theme.overlay
                }),
            ),
        ]));

        let repeat_str = if app.repeat { "ON" } else { "OFF" };
        lines.push(Line::from(vec![
            Span::styled("  Repeat: ", Style::default().fg(theme.overlay)),
            Span::styled(
                repeat_str,
                Style::default().fg(if app.repeat {
                    theme.green
                } else {
                    theme.overlay
                }),
            ),
        ]));
    }

    let gapless_str = if app.gapless_mode { "Active" } else { "OFF" };
    lines.push(Line::from(vec![
        Span::styled("  Gapless: ", Style::default().fg(theme.overlay)),
        Span::styled(
            gapless_str,
            Style::default().fg(if app.gapless_mode {
                theme.blue
            } else {
                theme.overlay
            }),
        ),
    ]));

    lines.push(Line::from(""));

    // Mode-specific section
    if app.is_mpd {
        // DSP/EQ Section (MPD mode only)
        lines.push(Line::from(vec![
            Span::styled("🎛 ", Style::default().fg(theme.blue)),
            Span::styled(
                "DSP / EQ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));

        let eq_status = if app.eq_enabled {
            "Enabled"
        } else {
            "Disabled"
        };
        lines.push(Line::from(vec![
            Span::styled("  Equalizer: ", Style::default().fg(theme.overlay)),
            Span::styled(
                eq_status,
                Style::default().fg(if app.eq_enabled {
                    theme.green
                } else {
                    theme.overlay
                }),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Preset: ", Style::default().fg(theme.overlay)),
            Span::styled(app.get_preset_name(), Style::default().fg(theme.magenta)),
        ]));

        lines.push(Line::from(""));

        // Output Section
        lines.push(Line::from(vec![
            Span::styled("🔊 ", Style::default().fg(theme.yellow)),
            Span::styled(
                "Output",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Device: ", Style::default().fg(theme.overlay)),
            Span::styled(&app.output_device, Style::default().fg(theme.cyan)),
        ]));

        let (mode_text, mode_color) = if app.eq_enabled {
            ("DSP Active (EQ Enabled)", theme.yellow)
        } else {
            ("Bit-Perfect (No DSP)", theme.green)
        };
        lines.push(Line::from(vec![
            Span::styled("  Mode: ", Style::default().fg(theme.overlay)),
            Span::styled(mode_text, Style::default().fg(mode_color)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Backend: ", Style::default().fg(theme.overlay)),
            Span::styled("MPD", Style::default().fg(theme.text)),
        ]));
    } else {
        // Streaming Source Section (Controller mode)
        lines.push(Line::from(vec![
            Span::styled("📡 ", Style::default().fg(theme.blue)),
            Span::styled(
                "Source",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Streaming: ", Style::default().fg(theme.overlay)),
            Span::styled(&app.source_app, Style::default().fg(theme.green)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Mode: ", Style::default().fg(theme.overlay)),
            Span::styled("Controller", Style::default().fg(theme.magenta)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(theme.surface)),
        Span::styled(
            "i",
            Style::default()
                .fg(theme.green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" or ", Style::default().fg(theme.surface)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(theme.green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to close", Style::default().fg(theme.surface)),
    ]));

    // 2. Calculate Scalable Height
    let height = (lines.len() as u16 + 2).min(f.area().height.saturating_sub(4)); // +2 for borders
    let width = 60.min(f.area().width.saturating_sub(4));
    let x = (f.area().width.saturating_sub(width)) / 2;
    let y = (f.area().height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    // 3. Clear and Render
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.blue))
        .title(" Audio Info ")
        .title_alignment(Alignment::Left)
        .style(Style::default().bg(Color::Reset));

    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, area);
}

fn render_toast(f: &mut Frame, app: &App) {
    if let Some(ref toast) = app.toast {
        let theme = &app.theme;
        let now = std::time::Instant::now();

        // Auto-dismiss handled in App::on_tick()
        let message = &toast.message;
        let width = (message.len() as u16 + 6).min(f.area().width.saturating_sub(4));
        let height = 3;
        let target_x = f.area().width.saturating_sub(width + 1); // Top-right fixed
        let mut x = target_x;

        let entrance_elapsed = now.duration_since(toast.start_time).as_millis();
        let time_remaining = toast.deadline.saturating_duration_since(now).as_millis();

        // Animation: Slide In/Out 🌊
        if entrance_elapsed < 300 {
            // Entrance (0-300ms from start): Slide LEFT
            let t = entrance_elapsed as f32 / 300.0;
            let ease = 1.0 - (1.0 - t).powi(3); // Cubic Out
            let offset = (width as f32 * (1.0 - ease)) as u16;
            x += offset;
        } else if time_remaining < 300 {
            // Exit (Last 300ms before deadline): Slide RIGHT
            // t goes 0 -> 1 as we approach deadline
            let t = (300 - time_remaining) as f32 / 300.0;
            let ease = t.powi(3); // Cubic In
            let offset = (width as f32 * ease) as u16;
            x += offset;
        }
        // Else: Hold position

        // Don't render if off-screen (start/end)
        if x < f.area().width {
            let y = 1; // Near top
            let full_area = Rect::new(x, y, width, height);
            // Clip to screen bounds to avoid panic
            let visible_area = full_area.intersection(f.area());

            if !visible_area.is_empty() {
                f.render_widget(Clear, visible_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.blue))
                    .style(Style::default().bg(Color::Reset));

                let style = Style::default().fg(theme.blue).add_modifier(Modifier::BOLD);

                let text = Paragraph::new(Line::from(vec![
                    Span::styled(message.as_str(), style),
                ]))
                .alignment(Alignment::Center)
                .block(block);

                f.render_widget(text, visible_area);
            }
        }
    }
}

fn render_input(f: &mut Frame, app: &App) {
    if let Some(ref input) = app.input_state {
        let theme = &app.theme;

        let width = 60.min(f.area().width.saturating_sub(4));
        let height = 5;
        let x = (f.area().width.saturating_sub(width)) / 2;
        let y = (f.area().height.saturating_sub(height)) / 2;
        let area = Rect::new(x, y, width, height);

        f.render_widget(Clear, area);

        let lines: Vec<Line> = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " > ",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&input.value, Style::default().fg(theme.text)),
                Span::styled(
                    "▌",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.blue))
            .title(format!(" {} ", input.title))
            .title_alignment(Alignment::Left)
            .style(Style::default().bg(Color::Reset));

        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
    }
}

fn render_tag_editor(f: &mut Frame, app: &App) {
    if let Some(ref tag_state) = app.tag_edit {
        let theme = &app.theme;

        // Make popup responsive to terminal size
        let max_popup_width = f.area().width.saturating_sub(4).min(50);
        let max_popup_height = f.area().height.saturating_sub(4).min(12);

        // Only show if terminal is big enough
        if max_popup_width >= 30 && max_popup_height >= 8 {
            let popup_x = (f.area().width.saturating_sub(max_popup_width)) / 2;
            let popup_y = (f.area().height.saturating_sub(max_popup_height)) / 2;
            let popup_area = Rect::new(popup_x, popup_y, max_popup_width, max_popup_height);

            f.render_widget(Clear, popup_area);

            let mut lines: Vec<Line> = Vec::new();

            // Title
            lines.push(Line::from(vec![Span::styled(
                "🏷️ Edit Tags",
                Style::default()
                    .fg(theme.magenta)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            // Fields with active highlighting
            let fields = ["Title", "Artist", "Album"];
            let values = [&tag_state.title, &tag_state.artist, &tag_state.album];

            for (i, (field, value)) in fields.iter().zip(values.iter()).enumerate() {
                let is_active = i == tag_state.active_field;
                let field_style = if is_active {
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.overlay)
                };
                let value_style = if is_active {
                    Style::default()
                        .fg(theme.text)
                        .add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.text)
                };

                let cursor = if is_active { "▌" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(format!("{:>8}: ", field), field_style),
                    Span::styled(value.to_string(), value_style),
                    Span::styled(cursor, Style::default().fg(theme.green)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "Tab",
                    Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" next  ", Style::default().fg(theme.overlay)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" save  ", Style::default().fg(theme.overlay)),
                Span::styled(
                    "Esc",
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" cancel", Style::default().fg(theme.overlay)),
            ]));

            let popup = Paragraph::new(lines).alignment(Alignment::Left).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.blue))
                    .title(Span::styled(" Edit Song Tags ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)))
                    .title_alignment(Alignment::Left)
                    .style(Style::default().bg(Color::Reset)),
            );
            f.render_widget(popup, popup_area);
        }
    }
}

fn render_keyhints(f: &mut Frame, app: &App) {
    let theme = &app.theme;

    // 🎹 WhichKey-style floating popup (Helix-inspired, centered)

    // Get context-specific keybindings with icons
    // Use String for key display to support dynamic config
    let (title, keys): (&str, Vec<(String, &str, &str)>) = match app.view_mode {
        ViewMode::EQ => (
            "EQ Controls",
            vec![
                (format!("{}/{}", app.keys.display(&app.keys.band_prev), app.keys.display(&app.keys.band_next)), "🎚️", "Select band"),
                (format!("{}/{}", app.keys.display(&app.keys.gain_up), app.keys.display(&app.keys.gain_down)), "📊", "Adjust gain"),
                (app.keys.display(&app.keys.next_preset), "🎵", "Next preset"),
                (app.keys.display(&app.keys.toggle_eq), "⚡", "Toggle EQ"),
                (app.keys.display(&app.keys.reset_eq), "↺", "Reset EQ"),
                (app.keys.display(&app.keys.reset_levels), "🎯", "Reset Levels"),
                (format!("{}/{}", app.keys.display(&app.keys.preamp_up), app.keys.display(&app.keys.preamp_down)), "🔊", "Preamp ±1dB"),
                (format!("{}/{}", app.keys.display(&app.keys.balance_right), app.keys.display(&app.keys.balance_left)), "⚖️", "Balance ±0.1"),
                (app.keys.display(&app.keys.crossfade), "🔀", "Crossfade"),
                (app.keys.display(&app.keys.replay_gain), "📀", "ReplayGain"),
                (app.keys.display(&app.keys.save_preset), "💾", "Save preset"),
                (app.keys.display(&app.keys.delete_preset), "🗑️", "Delete preset"),
            ],
        ),
        ViewMode::Library => (
            "Library",
            vec![
                (format!("{}/{}", app.keys.display(&app.keys.nav_down), app.keys.display(&app.keys.nav_up)), "📋", "Navigate"),
                (app.keys.display(&app.keys.tab_next), "🔄", "Switch mode"),
                (app.keys.display(&app.keys.enter_dir), "▶️", "Select/Play"),
                (app.keys.display(&app.keys.back_dir), "←", "Go back"),
                (app.keys.display(&app.keys.search_global), "🔍", "Search"),
                (app.keys.display(&app.keys.add_to_queue), "➕", "Add to Queue"),
                (app.keys.display(&app.keys.save_playlist), "💾", "Save playlist"),
                (app.keys.display(&app.keys.rename_playlist), "✏️", "Rename playlist"),
                (app.keys.display(&app.keys.delete_item), "🗑️", "Delete/Remove"),
                (app.keys.display(&app.keys.edit_tags), "🏷️", "Edit tags"),
                (format!("{}/{}", app.keys.display(&app.keys.move_down), app.keys.display(&app.keys.move_up)), "🔃", "Reorder"),
            ],
        ),
        ViewMode::Lyrics => (
            "Lyrics",
            vec![
                (format!("{}/{}", app.keys.display(&app.keys.nav_down), app.keys.display(&app.keys.nav_up)), "📜", "Scroll lyrics"),
                (app.keys.display(&app.keys.seek_to_line), "🎤", "Jump to line"),
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
            (app.keys.display(&app.keys.prev_track), "⏮️", "Previous track"),
            (app.keys.display(&app.keys.shuffle), "🔀", "Shuffle"),
            (app.keys.display(&app.keys.repeat), "🔁", "Repeat"),
            (app.keys.display(&app.keys.search_global), "🔍", "Search"),
            (format!("{}/{}", app.keys.display(&app.keys.volume_up), app.keys.display(&app.keys.volume_down)), "🔊", "Volume"),
            (format!("1-{}", "4"), "🖼️", "View modes"),
            (format!("{}/{}", app.keys.display(&app.keys.seek_backward), app.keys.display(&app.keys.seek_forward)), "⏩", "Seek ±5s"),
            (format!("{}/{}", app.keys.display(&app.keys.device_next), app.keys.display(&app.keys.device_prev)), "🎧", "Output device"),
            (app.keys.display(&app.keys.toggle_audio_info), "ℹ️", "Audio info"),
            (app.keys.display(&app.keys.quit), "🚪", "Quit"),
        ]
    } else {
        // Controller mode: limited keys (no shuffle/repeat - not available)
        vec![
            (app.keys.display(&app.keys.play_pause), "▶️", "Play/Pause"),
            (app.keys.display(&app.keys.next_track), "⏭️", "Next track"),
            (app.keys.display(&app.keys.prev_track), "⏮️", "Previous track"),
            (format!("{}/{}", app.keys.display(&app.keys.volume_up), app.keys.display(&app.keys.volume_down)), "🔊", "Volume"),
            (format!("{}/{}", app.keys.display(&app.keys.seek_backward), app.keys.display(&app.keys.seek_forward)), "⏩", "Seek ±5s"),
            (format!("{}/{}", app.keys.display(&app.keys.device_next), app.keys.display(&app.keys.device_prev)), "🎧", "Output device"),
            (app.keys.display(&app.keys.toggle_audio_info), "ℹ️", "Audio info"),
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
    let content_width = keys.iter().chain(global_keys.iter())
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
