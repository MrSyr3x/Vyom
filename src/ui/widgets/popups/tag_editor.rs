use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if let Some(ref tag_state) = app.tag_edit {
        let theme = &app.theme;

        // Make popup responsive to terminal size
        let max_popup_width = f.area().width.saturating_sub(4).min(50);
        let max_popup_height = f.area().height.saturating_sub(4).min(12);

        // Only show if terminal is big enough
        if max_popup_width >= 30 && max_popup_height >= 8 {
            let popup_x = (f.area().width.saturating_sub(max_popup_width)) / 2;
            let popup_y = (f.area().height.saturating_sub(max_popup_height)) / 2;
            let popup_area = Rect::new(popup_x, popup_y, max_popup_width, max_popup_height);

            // SEAMLESS Z-INDEX FIX: Un-skip cells so Ratatui overwrites Kitty images
            let buf = f.buffer_mut();
            for y in popup_area.top()..popup_area.bottom() {
                for x in popup_area.left()..popup_area.right() {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_skip(false);
                        cell.set_char(' ');
                        cell.set_bg(ratatui::style::Color::Reset);
                        cell.set_fg(ratatui::style::Color::Reset);
                    }
                }
            }

            f.render_widget(Clear, popup_area);

            let mut lines: Vec<Line> = Vec::new();

            // Title
            lines.push(Line::from(vec![Span::styled(
                "🏷️ Edit Tags",
                Style::default()
                    .fg(theme.magenta)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            // Fields with active highlighting
            let fields = ["Title", "Artist", "Album"];
            let values = [&tag_state.title, &tag_state.artist, &tag_state.album];

            for (i, (field, value)) in fields.iter().zip(values.iter()).enumerate() {
                let is_active = i == tag_state.active_field;
                let field_style = if is_active {
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.overlay)
                };
                let value_style = if is_active {
                    Style::default()
                        .fg(theme.text)
                        .add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.text)
                };

                let cursor = if is_active { "▌" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(format!("{:>8}: ", field), field_style),
                    Span::styled(value.to_string(), value_style),
                    Span::styled(cursor, Style::default().fg(theme.green)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "Tab",
                    Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" next  ", Style::default().fg(theme.overlay)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" save  ", Style::default().fg(theme.overlay)),
                Span::styled(
                    "Esc",
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" cancel", Style::default().fg(theme.overlay)),
            ]));

            let popup = Paragraph::new(lines).alignment(Alignment::Left).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.blue))
                    .title(Span::styled(
                        " Edit Song Tags ",
                        Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
                    ))
                    .title_alignment(Alignment::Left)
                    .style(Style::default().bg(Color::Reset)),
            );
            f.render_widget(popup, popup_area);
        }
    }
}
