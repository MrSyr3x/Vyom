use crate::app::App;
use crate::ui::utils::truncate;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn render(app: &mut App, width: usize, height: usize, lines: &mut Vec<Line>) {
    let theme = &app.theme;

    // Unified aesthetic: simple list, no split
    let time_w = 6;
    let artist_w = width / 4;
    let title_w = width.saturating_sub(artist_w + time_w + 10);
    let content_h = height;

    let blue = theme.blue;
    let green = theme.green;
    let cream = theme.yellow;
    let muted = theme.overlay;
    let grid = theme.surface;

    // Path breadcrumb
    let path = if app.browse_path.is_empty() {
        "Root".to_string()
    } else {
        app.browse_path.join(" › ")
    };

    // ━━━ CENTERED TITLE ━━━
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled(
            format!("  DIRECTORY  ·  {}  ", path),
            Style::default().fg(blue),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(Line::from(""));

    // ━━━ CONTENT ━━━
    if app.library_items.is_empty() {
        lines.push(
            Line::from(Span::styled(
                "Empty folder",
                Style::default().fg(muted),
            ))
            .alignment(Alignment::Center),
        );
    } else {
        let start_idx = app
            .library_selected
            .saturating_sub(content_h / 2)
            .min(app.library_items.len().saturating_sub(content_h));

        for (display_idx, item) in app
            .library_items
            .iter()
            .skip(start_idx)
            .take(content_h)
            .enumerate()
        {
            let actual_idx = start_idx + display_idx;
            let is_sel = actual_idx == app.library_selected;
            let is_folder =
                matches!(item.item_type, crate::app::LibraryItemType::Folder);

            let raw_name = if item.name.trim().is_empty() {
                item.path.clone().unwrap_or_else(|| "[Unnamed]".to_string())
            } else {
                item.name.clone()
            };

            let name = truncate(&raw_name, title_w.saturating_sub(2));

            if is_folder {
                // Folder row
                let (marker, m_color, n_style) = if is_sel {
                    (
                        "●",
                        cream,
                        Style::default().fg(blue).add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("○", grid, Style::default().fg(theme.text))
                };
                let icon = "📁";

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", marker),
                        Style::default().fg(m_color),
                    ),
                    Span::styled(
                        format!("{} ", icon),
                        Style::default().fg(blue),
                    ),
                    Span::styled(name, n_style),
                ]));
            } else {
                // Song row
                let artist = item.artist.clone().unwrap_or_default();
                let artist_disp = truncate(&artist, artist_w.saturating_sub(1));
                let time = item
                    .duration_ms
                    .map(|ms| {
                        let s = ms / 1000;
                        format!("{}:{:02}", s / 60, s % 60)
                    })
                    .unwrap_or_default();

                let (marker, m_color, t_style, a_style, tm_style) = if is_sel {
                    (
                        "●",
                        cream,
                        Style::default()
                            .fg(theme.text)
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(theme.text),
                        Style::default().fg(green),
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
                let icon = "♪";

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", marker),
                        Style::default().fg(m_color),
                    ),
                    Span::styled(
                        format!("{} ", icon),
                        Style::default().fg(green),
                    ),
                    Span::styled(
                        format!("{:title_w$}", name, title_w = title_w),
                        t_style,
                    ),
                    Span::styled(
                        format!("{:artist_w$}", artist_disp, artist_w = artist_w),
                        a_style,
                    ),
                    Span::styled(
                        format!("{:>time_w$}", time, time_w = time_w),
                        tm_style,
                    ),
                ]));
            }
        }
    }
}
