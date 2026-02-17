use crate::app::{App, LibraryMode};
use ratatui::{
    layout::Alignment,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub mod browser;
pub mod playlists;
pub mod queue;
pub mod search;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let title_text = " Library ";

    let lib_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(
            title_text,
            Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(theme.blue))
        .style(Style::default().bg(Color::Reset));

    let inner_area = lib_block.inner(area);
    f.render_widget(lib_block, area);

    let w = inner_area.width as usize;
    let h = inner_area.height as usize;
    let mut lines: Vec<Line> = Vec::new();

    // ═══════════════════════════════════════════════════════════════
    // BEAUTIFUL HEADER DESIGN
    // ═══════════════════════════════════════════════════════════════

    // Search bar with elegant styling
    let search_text = if app.search_active {
        format!(" {}▏", &app.search_query)
    } else if !app.search_query.is_empty() {
        format!(" {}", &app.search_query)
    } else {
        " Press / to search...".to_string()
    };
    let search_color = if app.search_active {
        theme.green
    } else {
        theme.overlay
    };

    // Add padding above search bar (User Request)
    lines.push(Line::raw(""));

    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().fg(search_color)),
        Span::styled(search_text, Style::default().fg(search_color)),
    ]));

    // Elegant thin separator - centered
    lines.push(
        Line::from(Span::styled(
            "─".repeat(w.min(60)), // Slightly wider
            Style::default().fg(theme.surface),
        ))
        .alignment(Alignment::Center),
    );

    // Tab bar with filled dot indicators
    let queue_active = app.library_mode == LibraryMode::Queue;
    let dir_active = app.library_mode == LibraryMode::Directory;
    let pl_active = app.library_mode == LibraryMode::Playlists;

    // Use filled dots for active, empty for inactive
    let q_dot = if queue_active { "●" } else { "○" };
    let d_dot = if dir_active { "●" } else { "○" };
    let p_dot = if pl_active { "●" } else { "○" };

    lines.push(
        Line::from(vec![
            // Queue
            Span::styled(format!("{} ", q_dot), Style::default().fg(theme.green)),
            Span::styled(
                "Queue",
                if queue_active {
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.green)
                },
            ),
            Span::styled("      ", Style::default()),
            // Directory
            Span::styled(format!("{} ", d_dot), Style::default().fg(theme.blue)),
            Span::styled(
                "Directory",
                if dir_active {
                    Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.blue)
                },
            ), // Inactive is dimmed blue
            Span::styled("      ", Style::default()),
            // Playlists
            Span::styled(format!("{} ", p_dot), Style::default().fg(theme.magenta)),
            Span::styled(
                "Playlists",
                if pl_active {
                    Style::default()
                        .fg(theme.magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.magenta)
                },
            ), // Inactive is dimmed magenta
        ])
        .alignment(Alignment::Center),
    );

    lines.push(Line::from(""));

    // Determine remaining height for content
    // We added: "", search, sep, tab, "" -> 5 lines?
    // Let's count properly:
    // 1: ""
    // 2: Search
    // 3: Sep
    // 4: Tab
    // 5: ""
    // 6: (First line of content usually starts with logic)
    // Actually, sub-widgets add 3 header lines ("\n TITLE \n").
    // So we need to pass a height that accounts for this too if we want precision.
    // The original code used `content_h = h.saturating_sub(8)`.
    // Since we consumed 5 lines here, passing `h.saturating_sub(5)` seems right,
    // but the sub-widgets often start with `lines.push("")`.
    // Let's pass `h.saturating_sub(8)` to be safe/consistent with old logic.
    let content_h = h.saturating_sub(8);

    match app.library_mode {
        LibraryMode::Queue => queue::render(app, w, content_h, &mut lines),
        LibraryMode::Directory => browser::render(app, w, content_h, &mut lines),
        LibraryMode::Search => search::render(app, w, content_h, &mut lines),
        LibraryMode::Playlists => playlists::render(app, w, content_h, &mut lines),
    }

    let library_widget =
        Paragraph::new(lines).block(Block::default().style(Style::default().bg(Color::Reset)));
    f.render_widget(library_widget, inner_area);
}
