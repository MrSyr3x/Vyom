use crate::app::{App, ArtworkState};
use crate::player::PlayerState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // --- MUSIC CARD ---
    let music_title = Title::from(Line::from(vec![Span::styled(
        " Now Playing ",
        Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
    )]));

    let music_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(music_title)
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(theme.blue))
        .style(Style::default().bg(Color::Reset));

    let inner_music_area = music_block.inner(area);
    f.render_widget(music_block, area);

    // Inner Music Layout
    let m_height = inner_music_area.height;
    let is_cramped = m_height < 10; // Redefined threshold for Tiny Mode

    // Elastic Priority Stack 🧠
    // We strictly prioritize: Controls > Info > Gauge > Time > Artwork
    // Artwork gets whatever is left (Constraint::Min(0)).

    // Calculate Info Height (4 if badges, 3 if not)
    let info_height = if let Some(track) = &app.track {
        if track.codec.is_some() || track.sample_rate.is_some() {
            4
        } else {
            3
        }
    } else {
        4
    };

    let mut music_constraints = Vec::new();

    // Extremely small height (< 10): Show only essentials
    if m_height < 10 {
        // Tiny Mode: Artwork 0, Info 1, Controls 1
        music_constraints.push(Constraint::Min(0)); // 0: Artwork (Hidden)
        music_constraints.push(Constraint::Length(m_height.saturating_sub(2).max(1))); // 1: Info (Takes remaining)
        music_constraints.push(Constraint::Length(0)); // 2: Gauge (Hidden)
        music_constraints.push(Constraint::Length(0)); // 3: Time (Hidden)
        music_constraints.push(Constraint::Length(0)); // 4: Spacer 1 (Hidden)
        music_constraints.push(Constraint::Length(1)); // 5: Controls
    } else {
        // Normal Mode: Artwork takes ALL available space
        music_constraints.push(Constraint::Min(0)); // 0: Artwork (Elastic!)
        music_constraints.push(Constraint::Length(info_height)); // 1: Info (Dynamic)
        music_constraints.push(Constraint::Length(1)); // 2: Spacer 1
        music_constraints.push(Constraint::Length(1)); // 3: Gauge
        music_constraints.push(Constraint::Length(1)); // 4: Time
                                                       // Removed Spacer 2 to tighten layout
        music_constraints.push(Constraint::Length(3)); // 5: Controls
    }

    let music_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(music_constraints)
        .split(inner_music_area);

    // 1. Artwork
    let artwork_area = if !music_chunks.is_empty() && music_chunks[0].height > 1 {
        // Only render art if we have at least 2 lines
        let area = music_chunks[0];
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0), // Art
            ])
            .split(area)[0]
    } else {
        Rect::default()
    };

    match &app.artwork {
        ArtworkState::Loaded(raw_image) => {
            // Calculate available area for artwork in characters
            let available_width = artwork_area.width as u32;
            let available_height = artwork_area.height as u32;

            let target_width = available_width;
            let target_height = available_height * 2;

            if target_width > 0 && target_height > 0 {
                use image::imageops::FilterType;
                use image::GenericImageView;

                // Resize preserving aspect ratio (Triangle for quality)
                let resized = raw_image.resize(target_width, target_height, FilterType::Triangle);

                // Vertical centering logic
                let img_height_subpixels = resized.height();
                let img_rows = img_height_subpixels.div_ceil(2); // integer ceil

                let total_rows = available_height;
                let padding_top = total_rows.saturating_sub(img_rows) / 2;

                let mut lines = Vec::new();

                // Add top padding
                for _ in 0..padding_top {
                    lines.push(Line::default());
                }

                for y in (0..img_height_subpixels).step_by(2) {
                    let mut spans = Vec::new();
                    for x in 0..resized.width() {
                        let p1 = resized.get_pixel(x, y);
                        let p2 = if y + 1 < img_height_subpixels {
                            resized.get_pixel(x, y + 1)
                        } else {
                            p1
                        };

                        let fg = (p1[0], p1[1], p1[2]);
                        let bg = (p2[0], p2[1], p2[2]);

                        spans.push(Span::styled(
                            "▀",
                            Style::default()
                                .fg(Color::Rgb(fg.0, fg.1, fg.2))
                                .bg(Color::Rgb(bg.0, bg.1, bg.2)),
                        ));
                    }
                    lines.push(Line::from(spans));
                }

                let artwork_widget = Paragraph::new(lines)
                    .alignment(Alignment::Center)
                    .block(Block::default().style(Style::default().bg(Color::Reset)));
                f.render_widget(artwork_widget, artwork_area);
            }
        }
        ArtworkState::Loading => {
            let p = Paragraph::new("\n\n\n\n\n        Loading...".to_string())
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(theme.yellow).bg(Color::Reset)));
            f.render_widget(p, artwork_area);
        }
        ArtworkState::Failed | ArtworkState::Idle => {
            let text = "\n\n\n\n\n        ♪\n    No Album\n      Art".to_string();
            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(theme.overlay).bg(Color::Reset)));
            f.render_widget(p, artwork_area);
        }
    }

    // 2. Info
    let info_idx = 1;
    if let Some(track) = &app.track {
        // Build audio quality badge 🎵
        let audio_badge: Option<Line> = if track.codec.is_some() || track.sample_rate.is_some() {
            let mut spans = Vec::new();

            // Audio Quality Badges 🎵
            // Hi-Res: 24bit+ or sample rate > 44.1kHz
            // CD Quality: 16bit/44.1kHz lossless
            // Lossy: MP3, AAC, OGG, etc.

            let is_hires = track.bit_depth.map(|b| b >= 24).unwrap_or(false)
                || track.sample_rate.map(|r| r > 44100).unwrap_or(false);

            let is_lossless = track
                .codec
                .as_ref()
                .map(|c| {
                    matches!(
                        c.to_uppercase().as_str(),
                        "FLAC" | "ALAC" | "WAV" | "AIFF" | "APE" | "DSD"
                    )
                })
                .unwrap_or(false);

            let is_lossy = track
                .codec
                .as_ref()
                .map(|c| {
                    matches!(
                        c.to_uppercase().as_str(),
                        "MP3" | "AAC" | "OGG" | "OPUS" | "M4A" | "WMA"
                    )
                })
                .unwrap_or(false);

            if is_hires {
                spans.push(Span::styled(
                    "\u{00A0}Hi-Res\u{00A0}",
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            } else if is_lossless && track.bit_depth == Some(16) {
                spans.push(Span::styled(
                    "\u{00A0}CD\u{00A0}",
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.blue)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            } else if is_lossy {
                spans.push(Span::styled(
                    "\u{00A0}Lossy\u{00A0}",
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.overlay)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            } else if is_lossless {
                spans.push(Span::styled(
                    "\u{00A0}Lossless\u{00A0}",
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.cyan)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            }

            // Gapless badge (when consecutive tracks from same album)
            if app.gapless_mode {
                spans.push(Span::styled(
                    "\u{00A0}Gapless\u{00A0}",
                    Style::default()
                        .fg(theme.base)
                        .bg(theme.magenta)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            }

            // Codec
            if let Some(codec) = &track.codec {
                spans.push(Span::styled(codec, Style::default().fg(theme.cyan)));
                spans.push(Span::raw(" "));
            }

            // Bit depth + Sample rate (e.g., "24bit/96kHz")
            if let (Some(depth), Some(rate)) = (track.bit_depth, track.sample_rate) {
                let khz = rate as f32 / 1000.0;
                spans.push(Span::styled(
                    format!("{}bit/{}kHz", depth, khz),
                    Style::default().fg(theme.overlay),
                ));
            } else if let Some(rate) = track.sample_rate {
                let khz = rate as f32 / 1000.0;
                spans.push(Span::styled(
                    format!("{}kHz", khz),
                    Style::default().fg(theme.overlay),
                ));
            }

            // Bitrate (for lossy)
            if let Some(kbps) = track.bitrate {
                if kbps > 0 {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("{}kbps", kbps),
                        Style::default().fg(theme.overlay),
                    ));
                }
            }

            if !spans.is_empty() {
                Some(Line::from(spans))
            } else {
                None
            }
        } else {
            None
        };

        // Helper to truncate strings that are too long
        let max_width = music_chunks[info_idx].width.saturating_sub(4) as usize; // -4 for padding/prefixes

        use crate::ui::utils::truncate;

        let mut info_text = vec![
            Line::from(Span::styled(
                format!("🎵 {}", truncate(&track.name, max_width.saturating_sub(2))),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::raw("🎤 "),
                Span::styled(
                    truncate(&track.artist, max_width.saturating_sub(2)),
                    Style::default().fg(theme.magenta),
                ),
            ]),
            Line::from(vec![
                Span::raw("💿 "),
                Span::styled(
                    truncate(&track.album, max_width.saturating_sub(2)),
                    Style::default().fg(theme.cyan).add_modifier(Modifier::DIM),
                ),
            ]),
        ];

        // Add audio badge if available
        if let Some(badge) = audio_badge {
            info_text.push(badge);
        }

        let info = Paragraph::new(info_text)
            .alignment(Alignment::Center)
            // .wrap(ratatui::widgets::Wrap { trim: true }) // Removed wrapping to prevent layout shift
            .block(Block::default().style(Style::default().bg(Color::Reset)));
        f.render_widget(info, music_chunks[info_idx]);

        // 3. Gauge
        let gauge_idx = if is_cramped { 2 } else { 3 };
        // Check if we have enough chunks. If cramped, we don't have spacers.
        // We used indices 0..4 for cramped.
        // music_chunks length check?

        // Helper to safely get chunk
        if gauge_idx < music_chunks.len() {
            let gauge_area_rect = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(10),
                    Constraint::Percentage(80),
                    Constraint::Percentage(10),
                ])
                .split(music_chunks[gauge_idx])[1];

            let current_pos = app.get_current_position_ms();
            let ratio = if track.duration_ms > 0 {
                current_pos as f64 / track.duration_ms as f64
            } else {
                0.0
            };

            let width = gauge_area_rect.width as usize;
            let occupied_width = (width as f64 * ratio.min(1.0).max(0.0)) as usize;
            let fill_style = Style::default().fg(theme.magenta);
            let empty_style = Style::default().fg(theme.surface);

            let mut bar_spans: Vec<Span> = Vec::with_capacity(width);
            for i in 0..width {
                if i < occupied_width {
                    if i == occupied_width.saturating_sub(1) {
                        // Playhead knob
                        bar_spans.push(Span::styled("●", fill_style));
                    } else {
                        // Filled pipe
                        bar_spans.push(Span::styled("━", fill_style));
                    }
                } else {
                    // Empty track
                    bar_spans.push(Span::styled("─", empty_style));
                }
            }

            let gauge_p = Paragraph::new(Line::from(bar_spans))
                .alignment(Alignment::Left)
                .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(gauge_p, gauge_area_rect);
        }

        // 4. Time
        let time_idx = if is_cramped { 3 } else { 4 };
        if time_idx < music_chunks.len() {
            let current_pos = app.get_current_position_ms();
            let time_str = format!(
                "{:02}:{:02} / {:02}:{:02}",
                current_pos / 60000,
                (current_pos % 60000) / 1000,
                track.duration_ms / 60000,
                (track.duration_ms % 60000) / 1000
            );
            let time_label = Paragraph::new(time_str)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.overlay));
            f.render_widget(time_label, music_chunks[time_idx]);
        }

        // 5. Controls
        let controls_idx = 5;

        if controls_idx < music_chunks.len() {
            let play_icon = if track.state == PlayerState::Playing {
                "⏸"
            } else {
                "▶"
            };
            let btn_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);

            let prev_str = "   ⏮   ";
            let next_str = "   ⏭   ";
            let play_str = format!("   {}   ", play_icon);

            // Split Controls Area: Top for Buttons, Bottom for Volume
            let controls_area = music_chunks[controls_idx];
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Buttons
                    Constraint::Length(1), // Spacer
                    Constraint::Length(1), // Volume Bar
                ])
                .split(controls_area);

            // 1. Buttons (Top) - 3-Column Layout for seamless centering
            let button_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(36), // Fixed width for center controls ensuring absolute center
                    Constraint::Fill(1),
                ])
                .split(chunks[0]);

            // Left: Shuffle (Align Right)
            if app.shuffle {
                let shuffle_widget =
                    Paragraph::new(Span::styled("🔀 ", Style::default().fg(theme.green)))
                        .alignment(Alignment::Right)
                        .block(Block::default());
                f.render_widget(shuffle_widget, button_layout[0]);
            }

            // Center: Prev / Play / Next (Always Centered)
            let center_spans = Line::from(vec![
                Span::styled(prev_str, btn_style),
                Span::raw("   "),
                Span::styled(play_str, btn_style),
                Span::raw("   "),
                Span::styled(next_str, btn_style),
            ]);
            let center_widget = Paragraph::new(center_spans)
                .alignment(Alignment::Center)
                .block(Block::default());
            f.render_widget(center_widget, button_layout[1]);

            // Right: Repeat (Align Left)
            if app.repeat {
                let repeat_widget =
                    Paragraph::new(Span::styled(" 🔁", Style::default().fg(theme.blue)))
                        .alignment(Alignment::Left)
                        .block(Block::default());
                f.render_widget(repeat_widget, button_layout[2]);
            }

            // 2. Volume Bar (Bottom) - if we have space
            if chunks.len() >= 3 {
                // Calculate Volume Bar
                let vol_ratio = app.app_volume as f64 / 100.0;
                let bar_width = 20; // Fixed width for clean look
                let filled_width = (bar_width as f64 * vol_ratio).round() as usize;

                let mut bar_spans = Vec::new();

                // "- "
                bar_spans.push(Span::styled("- ", Style::default().fg(theme.overlay)));

                for i in 0..bar_width {
                    if i < filled_width {
                        bar_spans.push(Span::styled("━", Style::default().fg(theme.magenta)));
                    } else {
                        bar_spans.push(Span::styled("─", Style::default().fg(theme.surface)));
                    }
                }

                // " + "
                bar_spans.push(Span::styled(" +", Style::default().fg(theme.overlay)));

                // Match button layout for perfect alignment
                let volume_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Length(36), // Same as buttons
                        Constraint::Fill(1),
                    ])
                    .split(chunks[2]);

                let vol_widget = Paragraph::new(Line::from(bar_spans))
                    .alignment(Alignment::Center)
                    .block(Block::default());
                f.render_widget(vol_widget, volume_layout[1]);
            }
        }
    } else {
        // IDLE STATE
        let t = Paragraph::new("Music Paused / Not Running")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text));

        // Just center it in available space
        f.render_widget(t, inner_music_area);
    }
}
