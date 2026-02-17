use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if let Some(ref input) = app.input_state {
        let theme = &app.theme;

        let width = 60.min(f.area().width.saturating_sub(4));
        let height = 5;
        let x = (f.area().width.saturating_sub(width)) / 2;
        let y = (f.area().height.saturating_sub(height)) / 2;
        let area = Rect::new(x, y, width, height);

        f.render_widget(Clear, area);

        let lines: Vec<Line> = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " > ",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&input.value, Style::default().fg(theme.text)),
                Span::styled(
                    "▌",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.blue))
            .title(format!(" {} ", input.title))
            .title_alignment(Alignment::Left)
            .style(Style::default().bg(Color::Reset));

        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
    }
}
