use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    if let Some(ref toast) = app.toast {
        let theme = &app.theme;
        let now = std::time::Instant::now();

        // Auto-dismiss handled in App::on_tick()
        let message = &toast.message;
        let width = (message.len() as u16 + 6).min(f.area().width.saturating_sub(4));
        let height = 3;
        let target_x = f.area().width.saturating_sub(width + 1); // Top-right fixed
        let mut x = target_x;

        let entrance_elapsed = now.duration_since(toast.start_time).as_millis();
        let time_remaining = toast.deadline.saturating_duration_since(now).as_millis();

        // Animation: Slide In/Out 🌊
        if entrance_elapsed < 300 {
            // Entrance (0-300ms from start): Slide LEFT
            let t = entrance_elapsed as f32 / 300.0;
            let ease = 1.0 - (1.0 - t).powi(3); // Cubic Out
            let offset = (width as f32 * (1.0 - ease)) as u16;
            x += offset;
        } else if time_remaining < 300 {
            // Exit (Last 300ms before deadline): Slide RIGHT
            // t goes 0 -> 1 as we approach deadline
            let t = (300 - time_remaining) as f32 / 300.0;
            let ease = t.powi(3); // Cubic In
            let offset = (width as f32 * ease) as u16;
            x += offset;
        }
        // Else: Hold position

        // Don't render if off-screen (start/end)
        if x < f.area().width {
            let y = 1; // Near top
            let full_area = Rect::new(x, y, width, height);
            // Clip to screen bounds to avoid panic
            let visible_area = full_area.intersection(f.area());

            if !visible_area.is_empty() {
                f.render_widget(Clear, visible_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.blue))
                    .style(Style::default().bg(Color::Reset));

                let style = Style::default().fg(theme.blue).add_modifier(Modifier::BOLD);

                let text = Paragraph::new(Line::from(vec![Span::styled(message.as_str(), style)]))
                    .alignment(Alignment::Center)
                    .block(block);

                f.render_widget(text, visible_area);
            }
        }
    }
}
