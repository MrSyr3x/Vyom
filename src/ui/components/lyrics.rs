use crate::app::{App, LyricsState, ViewMode};
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{block::Title, Block, BorderType, Borders, Paragraph},
    Frame,
    layout::Rect,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // Dynamic title based on view mode 🎛️
    let mode_title = match app.view_mode {
        ViewMode::Lyrics => " Lyrics ".to_string(),
        ViewMode::Visualizer => " Visualizer ".to_string(),
        ViewMode::Library => " Library ".to_string(),
        ViewMode::EQ => " Sound ".to_string(),
    };

    let lyrics_title = Title::from(Line::from(vec![Span::styled(
        mode_title,
        Style::default()
            .fg(theme.magenta)
            .add_modifier(Modifier::BOLD),
    )]));

    let credits_title = Line::from(vec![Span::styled(
        " ~ by syr3x </3 ",
        Style::default()
            .bg(Color::Rgb(235, 111, 146)) // #eb6f92
            .fg(theme.base)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC),
    )])
    .alignment(Alignment::Center);

    let lyrics_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(lyrics_title)
        .title_alignment(Alignment::Left)
        .title_bottom(credits_title)
        .border_style(Style::default().fg(theme.magenta))
        .style(Style::default().bg(Color::Reset));

    let inner_lyrics_area = lyrics_block.inner(area);
    f.render_widget(lyrics_block, area);

    // Content based on current view mode 🎛️
    // Note: This function only renders the lyrics content itself.
    // The orchestration for switching between Lyrics/Visualizer/Library/EQ happens in main ui function usually,
    // but the original code had this block wrapping everything.
    // However, if we split components, this file should arguably only handle Lyrics logic?
    // But the TITLE block is shared.
    //
    // Design Decision: I will keep this file focused on LYRICS content.
    // The shared container/title logic should probably be in `ui/mod.rs` or a `wrapper` helper.
    // BUT for now, to replicate exact behavior, I will render the lyrics content here.
    //
    // Actually, looking at the original code, the wrapper with title "Lyrics"/"Visualizer" etc.
    // surrounds the content.
    // So `lyrics.rs` should probably just render the lyrics content into `inner_lyrics_area`.
    // The container block rendering might be better placed in `ui.rs` or a shared helper.
    //
    // Let's assume for this refactor that `ui/mod.rs` will handle the container if it switches title,
    // OR we duplicate the container logic in each component (Library, EQ, etc.) which allows custom titles easily.
    // The original code used one big `if let Some(lyrics_area_rect)` then `match app.view_mode`.
    //
    // I entered this file thinking it's just lyrics.rs.
    // But `ViewMode::Visualizer` was also inside this "Right Panel".
    // 
    // I will refactor so `ui/mod.rs` handles the RIGHT PANEL CONTAINER if it's generic,
    // OR each component renders its own container.
    //
    // Given the `mode_title` logic, it seems the container is generic but the title changes.
    // I will implement JUST the lyrics rendering here. The container should be handled by the caller or a wrapper.
    //
    // WAIT. If I don't render the block here, I need to render it in `ui.rs`.
    // Let's stick to the plan: `lyrics.rs` renders the lyrics content.
    // I will replicate the container logic in `ui.rs` or `ui/components/wrapper.rs`?
    //
    // Let's make `rect_container.rs` or similar? No, I'll just put the container logic in `ui/mod.rs` 
    // and pass the `inner` rect to these specific components.
    
    match &app.lyrics {
        LyricsState::Loaded(lyrics) => {
            let height = inner_lyrics_area.height as usize;
            let track_ms = app.track.as_ref().map(|t| t.position_ms).unwrap_or(0);

            let current_idx = lyrics
                .iter()
                .position(|l| l.timestamp_ms > track_ms)
                .map(|i| if i > 0 { i - 1 } else { 0 })
                .unwrap_or(lyrics.len().saturating_sub(1));

            let mut lines = Vec::new();
            let half_height = height / 2;
            let center_idx = app.lyrics_offset.unwrap_or(current_idx);

            for row in 0..height {
                let dist_from_center: isize =
                    (row as isize - half_height as isize).abs();
                let target_idx_isize =
                    (center_idx as isize) - (half_height as isize) + (row as isize);

                if dist_from_center <= 6
                    && target_idx_isize >= 0
                    && target_idx_isize < lyrics.len() as isize
                {
                    let idx = target_idx_isize as usize;
                    let line = &lyrics[idx];

                    let is_active = idx == current_idx;
                    let is_selected = app.lyrics_selected == Some(idx);

                    let style = if is_selected {
                        // User-selected line (j/k navigation)
                        Style::default()
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                            .fg(theme.yellow)
                    } else if is_active {
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(theme.green)
                    } else {
                        match dist_from_center {
                            1..=2 => Style::default().fg(theme.text),
                            3..=4 => Style::default()
                                .fg(theme.text)
                                .add_modifier(Modifier::DIM),
                            5..=6 => Style::default().fg(theme.overlay),
                            _ => Style::default().fg(theme.base),
                        }
                    };

                    let prefix = if is_selected {
                        "▶ "
                    } else if is_active {
                        "● "
                    } else {
                        "  "
                    };
                    let prefix_span = if is_selected {
                        Span::styled(prefix, Style::default().fg(theme.yellow))
                    } else if is_active {
                        Span::styled(prefix, Style::default().fg(theme.green))
                    } else {
                        Span::styled(prefix, style)
                    };

                    lines.push(Line::from(vec![
                        prefix_span,
                        Span::styled(line.text.clone(), style),
                    ]));
                } else {
                    lines.push(Line::from(""));
                }
            }

            let lyrics_widget = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(Block::default().style(Style::default().bg(Color::Reset)));

            f.render_widget(lyrics_widget, inner_lyrics_area);
        }
        LyricsState::Loading => {
            let text = Paragraph::new(Text::styled(
                "\nFetching Lyrics...",
                Style::default().fg(theme.yellow),
            ))
            .alignment(Alignment::Center)
            .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(text, inner_lyrics_area);
        }
        LyricsState::Instrumental => {
            let text = Paragraph::new(Text::styled(
                "\n\n\n\n♫ Instrumental ♫",
                Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center)
            .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(text, inner_lyrics_area);
        }
        LyricsState::Failed(err) => {
            let text = Paragraph::new(Text::styled(
                format!("\nLyrics Failed: {}", err),
                Style::default().fg(theme.red),
            ))
            .alignment(Alignment::Center)
            .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(text, inner_lyrics_area);
        }
        LyricsState::Idle | LyricsState::NotFound => {
            let no_lyrics = Paragraph::new(Text::styled(
                "\nNo Lyrics Found",
                Style::default().fg(theme.overlay),
            ))
            .alignment(Alignment::Center)
            .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(no_lyrics, inner_lyrics_area);
        }
    }
}
