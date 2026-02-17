use crate::app::App;
use crate::ui::utils::truncate;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn render(app: &mut App, width: usize, height: usize, lines: &mut Vec<Line>) {
    let theme = &app.theme;

    // Unified aesthetic for Search
    let _time_w = 6;
    // Match Directory Layout: 25% Artist
    let artist_w = width / 4;
    // let title_w = w.saturating_sub(artist_w + time_w + 10); // Unused
    let content_h = height;

    let green = theme.green;
    let cream = theme.yellow;
    let muted = theme.overlay;
    let grid = theme.surface;

    // ━━━ CENTERED TITLE ━━━
    lines.push(Line::from(""));
    let search_title = if app.search_query.is_empty() {
        "  SEARCH  ".to_string()
    } else {
        format!("  SEARCH RESULTS: \"{}\"  ", app.search_query)
    };
    lines.push(
        Line::from(Span::styled(search_title, Style::default().fg(green)))
            .alignment(Alignment::Center),
    );
    lines.push(Line::from(""));

    // ━━━ CONTENT ━━━
    if app.library_items.is_empty() && !app.search_query.is_empty() {
        lines.push(
            Line::from(Span::styled("No results found", Style::default().fg(muted)))
                .alignment(Alignment::Center),
        );
        lines.push(
            Line::from(Span::styled(
                "Try a different search",
                Style::default().fg(grid),
            ))
            .alignment(Alignment::Center),
        );
    } else if app.library_items.is_empty() {
        lines.push(
            Line::from(Span::styled(
                "Type to search your library",
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

            // Clean up name by removing path components if present
            let clean_name = item.name.split('/').next_back().unwrap_or(&item.name);

            // Dynamic Layout Calculation 📐
            // Prefix width: "  ● " (4) + "♪ " (2) = 6 chars
            let prefix_w = 6;

            let has_artist = !item.artist.as_deref().unwrap_or("").trim().is_empty();
            let has_time = item.duration_ms.unwrap_or(0) > 0;

            let row_time_w = if has_time { 6 } else { 0 };
            let row_artist_w = if has_artist { artist_w } else { 0 };

            // Give ALL remaining space to Title
            let row_title_w = width.saturating_sub(row_artist_w + row_time_w + prefix_w);

            let name = truncate(clean_name, row_title_w.saturating_sub(1));
            let artist = item.artist.clone().unwrap_or_default();
            let artist_disp = truncate(&artist, row_artist_w.saturating_sub(1));
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
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
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

            // Only show ♪ for songs
            let icon = match item.item_type {
                crate::app::LibraryItemType::Song => "♪",
                crate::app::LibraryItemType::Folder => "📁",
                crate::app::LibraryItemType::Playlist => "📜",
                _ => " ",
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                Span::styled(
                    format!(
                        "{} {:title_w$}",
                        icon,
                        name,
                        title_w = row_title_w.saturating_sub(1)
                    ),
                    t_style,
                ),
                Span::styled(
                    format!("{:artist_w$}", artist_disp, artist_w = row_artist_w),
                    a_style,
                ),
                Span::styled(format!("{:>time_w$}", time, time_w = row_time_w), tm_style),
            ]));
        }
    }
}
