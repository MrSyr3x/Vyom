use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct MainLayout {
    pub body_area: Rect,
    pub footer_area: Rect,
}

pub fn get_main_layout(area: Rect) -> MainLayout {
    // Responsive Logic 🧠
    // 1. Footer needs 1 line at the bottom always.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(area);

    MainLayout {
        body_area: chunks[0],
        footer_area: chunks[1],
    }
}

pub struct ContentLayout {
    pub left: Rect,
    pub right: Option<Rect>,
    pub is_horizontal: bool,
}

pub fn get_content_layout(
    area: Rect,
    show_lyrics: bool,
    wide_mode: bool,
    height: u16,
) -> ContentLayout {
    if show_lyrics {
        if wide_mode {
            // Unified Horizontal Mode: Music Dominant (65%)
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(65), // Bigger Music
                    Constraint::Min(10),        // Lyrics
                ])
                .split(area);
            ContentLayout {
                left: chunks[0],
                right: Some(chunks[1]),
                is_horizontal: true,
            }
        } else {
            // Vertical Mode
            if height < 30 {
                // Too short for stack -> Hide Lyrics (Compressed)
                ContentLayout {
                    left: area,
                    right: None,
                    is_horizontal: false,
                }
            } else {
                // Stack Mode: User requested 45% Top, 55% Bottom
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                    .split(area);
                ContentLayout {
                    left: chunks[0],
                    right: Some(chunks[1]),
                    is_horizontal: false,
                }
            }
        }
    } else {
        // No Lyrics Mode
        ContentLayout {
            left: area,
            right: None,
            is_horizontal: false,
        }
    }
}
