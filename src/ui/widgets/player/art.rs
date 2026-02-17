use crate::app::{App, ArtworkState};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // Early exit if area too small
    if area.height < 1 {
        return;
    }

    match &app.artwork {
        ArtworkState::Loaded(raw_image) => {
            // Calculate available area for artwork in characters
            let available_width = area.width as u32;
            let available_height = area.height as u32;

            let target_width = available_width;
            let target_height = available_height * 2;

            if target_width > 0 && target_height > 0 {
                use image::imageops::FilterType;
                use image::GenericImageView;

                // Resize preserving aspect ratio (Triangle for quality)
                let resized = raw_image.resize(target_width, target_height, FilterType::Triangle);

                // Vertical centering logic
                let img_height_subpixels = resized.height();
                let img_rows = img_height_subpixels.div_ceil(2); // integer ceil

                let total_rows = available_height;
                let padding_top = total_rows.saturating_sub(img_rows) / 2;

                let mut lines = Vec::new();

                // Add top padding
                for _ in 0..padding_top {
                    lines.push(Line::default());
                }

                for y in (0..img_height_subpixels).step_by(2) {
                    let mut spans = Vec::new();
                    for x in 0..resized.width() {
                        let p1 = resized.get_pixel(x, y);
                        let p2 = if y + 1 < img_height_subpixels {
                            resized.get_pixel(x, y + 1)
                        } else {
                            p1
                        };

                        let fg = (p1[0], p1[1], p1[2]);
                        let bg = (p2[0], p2[1], p2[2]);

                        spans.push(Span::styled(
                            "▀",
                            Style::default()
                                .fg(Color::Rgb(fg.0, fg.1, fg.2))
                                .bg(Color::Rgb(bg.0, bg.1, bg.2)),
                        ));
                    }
                    lines.push(Line::from(spans));
                }

                let artwork_widget = Paragraph::new(lines)
                    .alignment(Alignment::Center)
                    .block(Block::default().style(Style::default().bg(Color::Reset)));
                f.render_widget(artwork_widget, area);
            }
        }
        ArtworkState::Loading => {
            let p = Paragraph::new("\n\n\n\n\n        Loading...".to_string())
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(theme.yellow).bg(Color::Reset)));
            f.render_widget(p, area);
        }
        ArtworkState::Failed | ArtworkState::Idle => {
            let text = "\n\n\n\n\n        ♪\n    No Album\n      Art".to_string();
            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(theme.overlay).bg(Color::Reset)));
            f.render_widget(p, area);
        }
    }
}
