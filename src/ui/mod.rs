pub mod components;
pub mod layout;
pub mod theme;
pub mod utils;
pub mod widgets;

pub use theme::Theme;

use crate::app::{App, ViewMode};
use ratatui::Frame;

pub fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // 1. Layout
    let main_layout = layout::get_main_layout(area);

    // 2. Content Layout
    let show_lyrics = app.app_show_lyrics;
    let wide_mode = !app.is_tmux && area.width >= 90;

    let content_layout =
        layout::get_content_layout(main_layout.body_area, show_lyrics, wide_mode, area.height);

    // 3. Render Music Card (Left)
    widgets::player::render(f, content_layout.left, app);

    // 4. Render Right Panel (Lyrics / Visualizer / Library / EQ)
    if let Some(right_area) = content_layout.right {
        match app.view_mode {
            ViewMode::Lyrics => components::lyrics::render(f, right_area, app),
            ViewMode::Visualizer => components::visualizer::render(f, right_area, app),
            ViewMode::Library => widgets::library::render(f, right_area, app),
            ViewMode::EQ => components::eq::render(f, right_area, app),
        }
    }

    // 5. Render Footer Hint (if no popup active)
    if !app.show_keyhints {
        use ratatui::layout::Alignment;
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let theme = &app.theme;
        let hint = Line::from(vec![
            Span::styled(
                " ? ",
                Style::default()
                    .fg(theme.overlay)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("keys", Style::default().fg(theme.overlay)),
        ]);
        let footer = Paragraph::new(hint).alignment(Alignment::Right);
        f.render_widget(footer, main_layout.footer_area);
    }

    // 6. Render Popups (Overlays)
    // Note: widgets::popups::render handles active states internally
    widgets::popups::render(f, app);
}
