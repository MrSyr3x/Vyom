use crate::app::App;
use crate::ui::utils::truncate;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn render(app: &mut App, width: usize, height: usize, lines: &mut Vec<Line>) {
    let theme = &app.theme;
    let content_h = height;

    let magenta = theme.magenta;
    let green = theme.green;
    let cream = theme.yellow;
    let muted = theme.overlay;
    let grid = theme.surface;

    // ━━━ CENTERED TITLE ━━━
    lines.push(Line::from(""));
    let playlist_count = app.playlists.len();
    lines.push(
        Line::from(Span::styled(
            format!("  PLAYLISTS  ·  {} saved  ", playlist_count),
            Style::default().fg(magenta),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(Line::from(""));

    // ━━━ CONTENT ━━━
    if app.playlists.is_empty() {
        lines.push(
            Line::from(Span::styled("No playlists", Style::default().fg(muted)))
                .alignment(Alignment::Center),
        );
        lines.push(
            Line::from(Span::styled(
                "Press 's' to save queue as playlist",
                Style::default().fg(grid),
            ))
            .alignment(Alignment::Center),
        );
    } else {
        let start_idx = app
            .library_selected
            .saturating_sub(content_h / 2)
            .min(app.playlists.len().saturating_sub(content_h));

        for (display_idx, pl) in app
            .playlists
            .iter()
            .skip(start_idx)
            .take(content_h)
            .enumerate()
        {
            let actual_idx = start_idx + display_idx;
            let is_sel = actual_idx == app.library_selected;
            let num = actual_idx + 1;

            let name_max = width.saturating_sub(12);
            let name = truncate(pl, name_max);

            let (marker, m_color, n_style) = if is_sel {
                (
                    "●",
                    cream,
                    Style::default().fg(magenta).add_modifier(Modifier::BOLD),
                )
            } else {
                ("○", grid, Style::default().fg(theme.text))
            };
            let icon = "📜"; // Standard playlist icon

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                Span::styled(
                    format!("{:>2}  ", num),
                    Style::default().fg(if is_sel { green } else { muted }),
                ),
                Span::styled(format!("{} ", icon), Style::default().fg(magenta)),
                Span::styled(name, n_style),
            ]));
        }
    }
}
