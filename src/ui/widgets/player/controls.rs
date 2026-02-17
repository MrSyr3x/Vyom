use crate::app::App;
use crate::player::{PlayerState, RepeatMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    if let Some(track) = &app.track {
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Buttons
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Volume Bar
            ])
            .split(area);

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
        if app.repeat != RepeatMode::Off {
            let repeat_spans = match app.repeat {
                RepeatMode::Single => vec![
                    Span::styled(" 🔂", Style::default().fg(theme.green)),
                    Span::styled("1", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                ],
                _ => vec![
                    Span::styled(" 🔁", Style::default().fg(theme.green)),
                ],
            };
            let repeat_widget =
                Paragraph::new(Line::from(repeat_spans))
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
}
