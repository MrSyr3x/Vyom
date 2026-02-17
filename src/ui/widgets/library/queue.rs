use crate::app::App;
use crate::ui::utils::truncate;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn render(app: &mut App, width: usize, height: usize, lines: &mut Vec<Line>) {
    let theme = &app.theme;

    // Unified aesthetic: spacious, centered, clean
    let time_w = 6;
    let artist_w = width / 4;
    let title_w = width.saturating_sub(artist_w + time_w + 10);
    // content_h is passed as height.
    // In original code, it subtracted 8 from inner height, but here we might pass the remaining height?
    // Let's assume height IS the content height available.
    let content_h = height;

    let green = theme.green;
    let pink = theme.red;
    let cream = theme.yellow;
    let muted = theme.overlay;
    let grid = theme.surface;

    // ━━━ CENTERED TITLE ━━━
    lines.push(Line::from(""));
    let queue_count = app.queue.len();
    lines.push(
        Line::from(Span::styled(
            format!("  QUEUE  ·  {} songs  ", queue_count),
            Style::default().fg(green),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(Line::from(""));

    // ━━━ CONTENT ━━━
    if app.queue.is_empty() {
        lines.push(
            Line::from(Span::styled("Empty queue", Style::default().fg(muted)))
                .alignment(Alignment::Center),
        );
        lines.push(
            Line::from(Span::styled(
                "Browse Directory to add songs",
                Style::default().fg(grid),
            ))
            .alignment(Alignment::Center),
        );
    } else {
        let start_idx = app
            .library_selected
            .saturating_sub(content_h / 2)
            .min(app.queue.len().saturating_sub(content_h));

        for (display_idx, (_, item)) in app
            .queue
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(content_h)
            .enumerate()
        {
            let actual_idx = start_idx + display_idx;
            let is_sel = actual_idx == app.library_selected;
            let num = actual_idx + 1;

            let title = truncate(&item.title, title_w.saturating_sub(2));
            let artist = truncate(&item.artist, artist_w.saturating_sub(1));
            let time = {
                let s = item.duration_ms / 1000;
                format!("{}:{:02}", s / 60, s % 60)
            };

            // Selection markers: ● for selected, ◉ for playing, ○ for normal
            let (marker, m_color, t_style, a_style, tm_style) = if is_sel {
                (
                    "●",
                    cream,
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                    Style::default().fg(theme.text),
                    Style::default().fg(green),
                )
            } else if item.is_current {
                (
                    "◉",
                    pink,
                    Style::default().fg(pink),
                    Style::default().fg(pink),
                    Style::default().fg(pink),
                )
            } else {
                (
                    "○",
                    grid,
                    Style::default().fg(theme.text),
                    Style::default().fg(muted),
                    Style::default().fg(muted),
                )
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                Span::styled(
                    format!("{:>2}  ", num),
                    Style::default().fg(if is_sel { green } else { muted }),
                ),
                Span::styled(
                    "♪ ",
                    Style::default().fg(if item.is_current { pink } else { green }),
                ),
                Span::styled(
                    format!("{:title_w$}", title, title_w = title_w.saturating_sub(2)),
                    t_style,
                ),
                Span::styled(
                    format!("{:artist_w$}", artist, artist_w = artist_w),
                    a_style,
                ),
                Span::styled(format!("{:>time_w$}", time, time_w = time_w), tm_style),
            ]));
        }
    }
}
