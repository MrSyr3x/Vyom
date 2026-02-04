use crate::app::{App, LibraryMode};
use crate::ui::utils::truncate;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
    layout::Rect,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // Smart Library Panel 📚
    // let theme = &app.theme; // Already defined above

    let title_text = match app.library_mode {
        LibraryMode::Queue => " Queue ",
        LibraryMode::Directory => " Directory ",
        LibraryMode::Playlists => " Playlists ",
        LibraryMode::Search => " Search ",
    };

    let lib_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(title_text, Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)))
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

    // Center tabs by adding padding logic or just centering the line

    lines.push(
        Line::from(vec![
            // Queue
            Span::styled(
                format!("{} ", q_dot),
                Style::default().fg(if queue_active {
                    theme.green
                } else {
                    theme.green
                }),
            ), // Always green, just dimmed if inactive? No, let's keep it clean.
            Span::styled(
                "Queue",
                if queue_active {
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.green)
                },
            ), // Inactive is now dimmed green instead of gray
            Span::styled("      ", Style::default()),
            // Directory
            Span::styled(
                format!("{} ", d_dot),
                Style::default().fg(if dir_active { theme.blue } else { theme.blue }),
            ),
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
            Span::styled(
                format!("{} ", p_dot),
                Style::default().fg(if pl_active {
                    theme.magenta
                } else {
                    theme.magenta
                }),
            ),
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
    ); // Use center alignment!

    lines.push(Line::from(""));

    match app.library_mode {
        LibraryMode::Queue => {
            // Unified aesthetic: spacious, centered, clean
            let time_w = 6;
            let artist_w = w / 4;
            let title_w = w.saturating_sub(artist_w + time_w + 10);
            let content_h = h.saturating_sub(8);

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
                            Style::default()
                                .fg(theme.text)
                                .add_modifier(Modifier::BOLD),
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
                        Span::styled(
                            format!("  {} ", marker),
                            Style::default().fg(m_color),
                        ),
                        Span::styled(
                            format!("{:>2}  ", num),
                            Style::default().fg(if is_sel { green } else { muted }),
                        ),
                        Span::styled(
                            "♪ ",
                            Style::default().fg(if item.is_current {
                                pink
                            } else {
                                green
                            }),
                        ),
                        Span::styled(
                            format!(
                                "{:title_w$}",
                                title,
                                title_w = title_w.saturating_sub(2)
                            ),
                            t_style,
                        ),
                        Span::styled(
                            format!("{:artist_w$}", artist, artist_w = artist_w),
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
        LibraryMode::Directory => {
            // Unified aesthetic: simple list, no split
            let time_w = 6;
            let artist_w = w / 4;
            let title_w = w.saturating_sub(artist_w + time_w + 10);
            let content_h = h.saturating_sub(8);

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
        LibraryMode::Search => {
            // Unified aesthetic for Search
            let time_w = 6;
            let artist_w = w / 4;
            let title_w = w.saturating_sub(artist_w + time_w + 10);
            let content_h = h.saturating_sub(8);

            let green = theme.green;
            let cream = theme.yellow;
            let muted = theme.overlay;
            let grid = theme.surface;

            // ━━━ CENTERED TITLE (Removed - Moved to Border) ━━━
            lines.push(Line::from(""));
            // let title = ... removed ...
            // lines.push(Line::from(""));

            // ━━━ CONTENT ━━━
            if app.library_items.is_empty() && !app.search_query.is_empty() {
                lines.push(
                    Line::from(Span::styled(
                        "No results found",
                        Style::default().fg(muted),
                    ))
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

                    let name = truncate(&item.name, title_w.saturating_sub(2));
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

                    // Only show ♪ for songs
                    let icon = match item.item_type {
                        crate::app::LibraryItemType::Song => "♪",
                        crate::app::LibraryItemType::Folder => "📁",
                        crate::app::LibraryItemType::Playlist => "📜",
                        _ => " ",
                    };

                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {} ", marker),
                            Style::default().fg(m_color),
                        ),
                        Span::styled(
                            format!(
                                "{} {:title_w$}",
                                icon,
                                name,
                                title_w = title_w.saturating_sub(2)
                            ),
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
        LibraryMode::Playlists => {
            // Unified aesthetic for Playlists
            let content_h = h.saturating_sub(8);
            let _playlist_count = app.playlists.len();

            let magenta = theme.magenta;
            let green = theme.green;
            let cream = theme.yellow;
            let muted = theme.overlay;
            let grid = theme.surface;

            // ━━━ CENTERED TITLE (Removed - Moved to Border) ━━━
            lines.push(Line::from(""));
            // lines.push(Line::from(Span::styled(
            //     format!("  PLAYLISTS  ·  {} saved  ", playlist_count),
            //     Style::default().fg(magenta)
            // )).alignment(Alignment::Center));
            // lines.push(Line::from(""));

            // ━━━ CONTENT ━━━
            if app.playlists.is_empty() {
                lines.push(
                    Line::from(Span::styled(
                        "No playlists",
                        Style::default().fg(muted),
                    ))
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

                    let name_max = w.saturating_sub(12);
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
                        Span::styled(
                            format!("  {} ", marker),
                            Style::default().fg(m_color),
                        ),
                        Span::styled(
                            format!("{:>2}  ", num),
                            Style::default().fg(if is_sel { green } else { muted }),
                        ),
                        Span::styled(
                            format!("{} ", icon),
                            Style::default().fg(magenta),
                        ),
                        Span::styled(name, n_style),
                    ]));
                }
            }
        }
    }

    let library_widget = Paragraph::new(lines)
        .block(Block::default().style(Style::default().bg(Color::Reset)));
    f.render_widget(library_widget, inner_area);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_scroll_clamping() {
        // Simulation parameters
        let list_len: usize = 50;
        let h: usize = 30; // Virtual terminal height
        let header_footer: usize = 8;
        let content_h: usize = h - header_footer; // 22 items visible

        // Scenario 1: Select last item (49)
        let selected: usize = 49;

        // Old Logic replication
        let old_start = selected.saturating_sub(content_h / 2); // 49 - 11 = 38
        let old_visible_count = list_len - old_start; // 50 - 38 = 12 items

        // New Logic replication
        let new_start = selected
            .saturating_sub(content_h / 2)
            .min(list_len.saturating_sub(content_h));
        // term1 = 38
        // term2 = 50 - 22 = 28
        // min(38, 28) = 28

        let new_visible_count = list_len - new_start; // 50 - 28 = 22 items

        println!("Content Height Capacity: {}", content_h);
        println!(
            "Old Logic: Start {}, Items Visible: {} ({} empty spaces)",
            old_start,
            old_visible_count,
            content_h - old_visible_count
        );
        println!(
            "New Logic: Start {}, Items Visible: {} ({} empty spaces)",
            new_start,
            new_visible_count,
            content_h - new_visible_count
        );

        // Assertions
        assert_eq!(new_visible_count, content_h, "List should be fully filled");
        assert!(
            new_start <= selected,
            "Start index must be before or equal to selected"
        );
        assert!(
            new_start + content_h > selected,
            "Selected item must be within view"
        );
    }
}
