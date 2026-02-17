use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render_progress(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    if let Some(track) = &app.track {
        let gauge_area_rect = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(area)[1];

        let current_pos = app.get_current_position_ms();
        let ratio = if track.duration_ms > 0 {
            current_pos as f64 / track.duration_ms as f64
        } else {
            0.0
        };

        let width = gauge_area_rect.width as usize;
        let occupied_width = (width as f64 * ratio.clamp(0.0, 1.0)) as usize;
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
}

pub fn render_time(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;
    if let Some(track) = &app.track {
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
        f.render_widget(time_label, area);
    }
}
