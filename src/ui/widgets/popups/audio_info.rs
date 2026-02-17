use crate::app::App;
use crate::player::RepeatMode;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
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

    // Lyrics Source Info (Requested by User) 🎤
    if let crate::app::LyricsState::Loaded(_, source) = &app.lyrics {
        lines.push(Line::from(vec![
            Span::styled("  Lyrics: ", Style::default().fg(theme.overlay)),
            Span::styled(source, Style::default().fg(theme.cyan)),
        ]));
    } else if let crate::app::LyricsState::Instrumental = &app.lyrics {
        lines.push(Line::from(vec![
            Span::styled("  Lyrics: ", Style::default().fg(theme.overlay)),
            Span::styled("Instrumental", Style::default().fg(theme.yellow)),
        ]));
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
            Style::default().fg(if is_paused { theme.yellow } else { theme.green }),
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

        let repeat_str = match app.repeat {
            RepeatMode::Off => "OFF",
            RepeatMode::Playlist => "All",
            RepeatMode::Single => "One",
        };

        lines.push(Line::from(vec![
            Span::styled("  Repeat: ", Style::default().fg(theme.overlay)),
            Span::styled(
                repeat_str,
                Style::default().fg(if app.repeat != RepeatMode::Off {
                    theme.green
                } else {
                    theme.overlay
                }),
            ),
        ]));
    }

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
