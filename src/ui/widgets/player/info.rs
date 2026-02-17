use crate::app::App;
use crate::ui::utils::truncate;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    if let Some(track) = &app.track {
        // Build audio quality badge 🎵
        let audio_badge: Option<Line> = if track.codec.is_some() || track.sample_rate.is_some() {
            let mut spans = Vec::new();

            // Audio Quality Badges 🎵
            // Hi-Res: 24bit+ or sample rate > 44.1kHz
            // CD Quality: 16bit/44.1kHz lossless
            // Lossy: MP3, AAC, OGG, etc.

            let is_hires = track.bit_depth.map(|b| b > 16).unwrap_or(false)
                || track.sample_rate.map(|r| r > 48000).unwrap_or(false);
            
            // DSD is technically 1-bit, but effectively Studio Master quality
            let is_dsd = track.codec.as_ref().map(|c| c.to_uppercase().contains("DSD")).unwrap_or(false);

            let is_studio = (track.bit_depth.map(|b| b >= 24).unwrap_or(false) 
                && track.sample_rate.map(|r| r >= 192000).unwrap_or(false))
                || is_dsd;

            let is_lossless = track
                .codec
                .as_ref()
                .map(|c| {
                    matches!(
                        c.to_uppercase().as_str(),
                        "FLAC" | "ALAC" | "WAV" | "AIFF" | "APE" | "PCM"
                    )
                })
                .unwrap_or(false)
                || track.bitrate.map(|b| b > 600).unwrap_or(false);

            // Determine Quality Tier
            if is_studio {
                spans.push(Span::styled(
                    if is_dsd { "\u{00A0}DSD\u{00A0}" } else { "\u{00A0}Studio\u{00A0}" },
                    Style::default().fg(theme.base).bg(theme.magenta).add_modifier(Modifier::BOLD),
                ));
            } else if is_hires {
                 spans.push(Span::styled(
                    "\u{00A0}Hi-Res\u{00A0}",
                    Style::default().fg(theme.base).bg(theme.blue).add_modifier(Modifier::BOLD),
                ));
            } else if is_lossless {
                 // CD Quality (16/44.1 or similar)
                 spans.push(Span::styled(
                    "\u{00A0}Lossless\u{00A0}",
                    Style::default().fg(theme.base).bg(theme.cyan).add_modifier(Modifier::BOLD),
                ));
            } else {
                // Lossy Logic based on Bitrate
                let bitrate = track.bitrate.unwrap_or(0);
                if bitrate > 0 {
                    if bitrate >= 256 {
                        spans.push(Span::styled(
                            "\u{00A0}HQ\u{00A0}",
                            Style::default().fg(theme.base).bg(theme.green).add_modifier(Modifier::BOLD),
                        ));
                    } else if bitrate >= 128 { // Standard Quality (e.g. 128k AAC/Opus)
                         spans.push(Span::styled(
                            "\u{00A0}SQ\u{00A0}",
                            Style::default().fg(theme.base).bg(theme.yellow).add_modifier(Modifier::BOLD),
                        ));
                    } else {
                         spans.push(Span::styled(
                            "\u{00A0}LQ\u{00A0}",
                            Style::default().fg(theme.base).bg(theme.red).add_modifier(Modifier::BOLD),
                        ));
                    }
                } else {
                     // Fallback if no bitrate known but known lossy codec
                      spans.push(Span::styled(
                            "\u{00A0}Lossy\u{00A0}",
                            Style::default().fg(theme.base).bg(theme.overlay).add_modifier(Modifier::BOLD),
                        ));
                }
            }

            // Bit depth + Sample rate (e.g., "24bit/96kHz")
            if let (Some(depth), Some(rate)) = (track.bit_depth, track.sample_rate) {
                spans.push(Span::styled(" • ", Style::default().fg(theme.overlay)));
                let khz = rate as f32 / 1000.0;
                spans.push(Span::styled(
                    format!("{}bit/{}kHz", depth, khz),
                    Style::default().fg(theme.overlay),
                ));
            } else if let Some(rate) = track.sample_rate {
                spans.push(Span::styled(" • ", Style::default().fg(theme.overlay)));
                let khz = rate as f32 / 1000.0;
                spans.push(Span::styled(
                    format!("{}kHz", khz),
                    Style::default().fg(theme.overlay),
                ));
            }

            // Bitrate (for lossy)
            if let Some(kbps) = track.bitrate {
                if kbps > 0 {
                    spans.push(Span::styled(" • ", Style::default().fg(theme.overlay)));
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
        let max_width = area.width.saturating_sub(4) as usize; // -4 for padding/prefixes

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
            .block(Block::default().style(Style::default().bg(Color::Reset)));
        f.render_widget(info, area);
    }
}
