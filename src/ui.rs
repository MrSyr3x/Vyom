use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    style::{Color, Style, Modifier},
    text::{Span, Line, Text},
    widgets::{block::Title, Block, Paragraph, Borders, BorderType},
    Frame,
};
use crate::app::App;
use crate::spotify::PlayerState;

pub fn ui(f: &mut Frame, app: &mut App) {
    let theme = &app.theme;
    let area = f.area();

    // Responsive Logic 🧠
    // 1. Footer needs 1 line at the bottom always.
    let root_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(area);

    let body_area = root_layout[0];
    let footer_area = root_layout[1];

    // 2. Decide Layout Direction
    // - Horizontal: If width >= 100 && user wants lyrics.
    // - Vertical: Standard.
    // - Compressed: If Vertical AND height < 40 (Hide Lyrics).
    let width = area.width;
    let height = area.height;
    
    // Thresholds
    let wide_mode = width >= 90; // Slightly relaxed from 100
    
    // Logic:
    // If we want lyrics:
    //    If wide -> Horizontal Split.
    //    If narrow -> Vertical Split.
    //       If too short (height < 40) -> Hide Lyrics (Compressed).
    // If we don't want lyrics -> Music Card only.

    let show_lyrics = app.show_lyrics;
    
    let (music_area, lyrics_area, _is_horizontal) = if show_lyrics {
        if wide_mode {
            // Check if SUPER WIDE (Standalone Fullscreen?)
            if width > 120 {
                 // Scalable Mode (Apple Music Style-ish, but balanced)
                 let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(60), // Music scales (Bigger Art)
                        Constraint::Percentage(40), // Lyrics scales
                    ])
                    .split(body_area);
                 (chunks[0], Some(chunks[1]), true)
            } else {
                 // Standard Wide (Tmux Sidebar) -> Fixed 45
                 let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(45), // Fixed width for music card
                        Constraint::Min(10),    // Lyrics take rest
                    ])
                    .split(body_area);
                 (chunks[0], Some(chunks[1]), true)
            }
        } else {
            // Vertical Mode
            if height < 40 {
                // Too short for stack -> Hide Lyrics
                (body_area, None, false)
            } else {
                // Stack Mode: Music Top (36), Lyrics Bottom
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(36),
                        Constraint::Min(0),
                    ])
                    .split(body_area);
                (chunks[0], Some(chunks[1]), false)
            }
        }
    } else {
        // No Lyrics Mode
        (body_area, None, false)
    };

    // --- MUSIC CARD ---
    let music_title = Title::from(Line::from(vec![
        Span::styled(" Vyom ", Style::default().fg(theme.base).bg(theme.blue).add_modifier(Modifier::BOLD))
    ]));

    let music_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(music_title)
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(theme.blue)) 
        .style(Style::default().bg(Color::Reset));
    
    let inner_music_area = music_block.inner(music_area);
    f.render_widget(music_block, music_area);

    // Inner Music Layout
    let m_height = inner_music_area.height;
    let is_cramped = m_height < 30; 

    let music_constraints = if is_cramped {
         vec![
            Constraint::Min(10),    // 0: Artwork (Shrinkable)
            Constraint::Length(4),  // 1: Info 
            Constraint::Length(1),  // 2: Gauge
            Constraint::Length(1),  // 3: Time
            Constraint::Length(1),  // 4: Controls
         ]
    } else {
        // Normal
         vec![
            Constraint::Min(20),    // 0: Artwork (Takes available space!)
            Constraint::Length(4),  // 1: Info 
            Constraint::Length(1),  // 2: Gauge
            Constraint::Length(1),  // 3: Time
            Constraint::Length(1),  // 4: Spacer
            Constraint::Length(1),  // 5: Controls
            Constraint::Length(1),  // 6: Bottom Padding
        ]
    };

    let music_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(music_constraints)
        .split(inner_music_area);

    // 1. Artwork
    let _art_idx = 0;
    
    // Add 2 lines of padding at top of artwork chunk itself to separate from Border Title (Vyom)
    let artwork_area = if music_chunks.len() > 0 {
         let area = music_chunks[0];
         // Only shrink if we have space, else use as is
         if area.height > 2 {
             Layout::default()
                 .direction(Direction::Vertical)
                 .constraints([
                     Constraint::Length(1), // Top Padding
                     Constraint::Min(1),    // Art
                 ])
                 .split(area)[1]
         } else {
             area
         }
    } else {
        Rect::default()
    };

    
    if let Some(raw_image) = &app.artwork {
        // Calculate available area for artwork in characters
        let available_width = artwork_area.width as u32;
        let available_height = artwork_area.height as u32;
        
        let target_width = available_width;
        let target_height = available_height * 2;
        
        if target_width > 0 && target_height > 0 {
            use image::imageops::FilterType;
            use image::GenericImageView;
            
            // Resize preserving aspect ratio (Triangle for quality)
            let resized = raw_image.resize(target_width, target_height, FilterType::Triangle);
            
            // Vertical centering logic
            let img_height_subpixels = resized.height();
            let img_rows = (img_height_subpixels + 1) / 2; // integer ceil
            
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
                            .bg(Color::Rgb(bg.0, bg.1, bg.2))
                    ));
                }
                lines.push(Line::from(spans));
            }
            
            let artwork_widget = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(artwork_widget, artwork_area);
        }

    } else {
       // Placeholder
       let text = if app.track.is_some() {
           "\n\n\n\n\n        Loading...".to_string()
       } else {
           "\n\n\n\n\n        ♪\n    No Album\n      Art".to_string()
       };
       let p = Paragraph::new(text)
           .alignment(Alignment::Center)
           .block(Block::default().style(Style::default().fg(theme.overlay).bg(Color::Reset)));
       f.render_widget(p, artwork_area);
    }

    // 2. Info
    let info_idx = 1;
    if let Some(track) = &app.track {
        let info_text = vec![
            Line::from(Span::styled(
                format!("🎵 {}", track.name),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD)
            )),
            Line::from(vec![
                Span::raw("🎤 "),
                Span::styled(&track.artist, Style::default().fg(theme.magenta)), 
            ]),
            Line::from(vec![
                Span::raw("💿 "),
                Span::styled(&track.album, Style::default().fg(theme.cyan).add_modifier(Modifier::DIM)), 
            ]),
        ];
        
        let info = Paragraph::new(info_text)
            .alignment(Alignment::Center)
             .block(Block::default().style(Style::default().bg(Color::Reset)));
        f.render_widget(info, music_chunks[info_idx]);

        // 3. Gauge
        let gauge_idx = 2;
        // Check if we have enough chunks. If cramped, we don't have spacers.
        // We used indices 0..4 for cramped.
        // music_chunks length check? 
        
        // Helper to safely get chunk
        if gauge_idx < music_chunks.len() {
             let gauge_area_rect = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(10), 
                    Constraint::Percentage(80), 
                    Constraint::Percentage(10), 
                ])
                .split(music_chunks[gauge_idx])[1];

            let ratio = if track.duration_ms > 0 {
                track.position_ms as f64 / track.duration_ms as f64
            } else {
                0.0
            };
            
            let width = gauge_area_rect.width as usize;
            let occupied_width = (width as f64 * ratio.min(1.0).max(0.0)) as usize;
            let fill_style = Style::default().fg(theme.magenta);
            let empty_style = Style::default().fg(theme.surface);
            
            let mut bar_spans: Vec<Span> = Vec::with_capacity(width);
            for i in 0..width {
                 if i < occupied_width {
                    if i >= occupied_width.saturating_sub(1) {
                        bar_spans.push(Span::styled("▓", fill_style));
                    } else if i >= occupied_width.saturating_sub(2) {
                        bar_spans.push(Span::styled("▒", fill_style));
                    } else {
                        bar_spans.push(Span::styled("█", fill_style));
                    }
                } else {
                    bar_spans.push(Span::styled("░", empty_style));
                }
            }

            let gauge_p = Paragraph::new(Line::from(bar_spans))
                .alignment(Alignment::Left)
                .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(gauge_p, gauge_area_rect);
            app.progress_rect = gauge_area_rect;
        }

        // 4. Time
        let time_idx = 3;
        if time_idx < music_chunks.len() {
            let time_str = format!(
                "{:02}:{:02} / {:02}:{:02}",
                track.position_ms / 60000,
                (track.position_ms % 60000) / 1000,
                track.duration_ms / 60000,
                (track.duration_ms % 60000) / 1000
            );
            let time_label = Paragraph::new(time_str)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.overlay));
            f.render_widget(time_label, music_chunks[time_idx]);
        }
        
        // 5. Controls
        // If cramped: index 4. If normal: index 5 (index 4 is spacer)
        let controls_idx = if is_cramped { 4 } else { 5 };
        
        if controls_idx < music_chunks.len() {
            let play_icon = if track.state == PlayerState::Playing { "⏸" } else { "▶" };
            let btn_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
            
            let prev_str = "   ⏮   ";
            let next_str = "   ⏭   ";
            let play_str = format!("   {}   ", play_icon); 
            
            let controls_text = Line::from(vec![
                Span::styled(prev_str, btn_style),
                Span::raw("   "), 
                Span::styled(play_str, btn_style),
                Span::raw("   "), 
                Span::styled(next_str, btn_style),
            ]);
            
            let controls = Paragraph::new(controls_text)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().bg(Color::Reset)));
            
            f.render_widget(controls, music_chunks[controls_idx]);

            let area = music_chunks[controls_idx];
            let mid_x = area.x + area.width / 2;
            let y = area.y;
            
            app.prev_btn = ratatui::layout::Rect::new(mid_x.saturating_sub(13), y, 7, 1);
            app.play_btn = ratatui::layout::Rect::new(mid_x.saturating_sub(3), y, 7, 1);
            app.next_btn = ratatui::layout::Rect::new(mid_x + 7, y, 7, 1);
        }

    } else {
        // IDLE STATE
        let t = Paragraph::new("Music Paused / Not Running")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text));
        
        // Just center it in available space
        f.render_widget(t, inner_music_area);
    }
    
    // --- LYRICS CARD ---
    if let Some(lyrics_area_rect) = lyrics_area {
        let lyrics_title = Title::from(Line::from(vec![
            Span::styled(" Lyrics ", Style::default().fg(theme.base).bg(theme.magenta).add_modifier(Modifier::BOLD))
        ]));

        let credits_title = Line::from(vec![
            Span::styled(" ~ by syr3x </3 ", Style::default()
                .bg(Color::Rgb(235, 111, 146)) // #eb6f92
                .fg(theme.base) 
                .add_modifier(Modifier::BOLD | Modifier::ITALIC))
        ]);

        let lyrics_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(lyrics_title)
            .title_alignment(Alignment::Center)
            .title_bottom(credits_title)
            .border_style(Style::default().fg(theme.magenta))
            .style(Style::default().bg(Color::Reset));
        
        let inner_lyrics_area = lyrics_block.inner(lyrics_area_rect);
        f.render_widget(lyrics_block, lyrics_area_rect);

        app.lyrics_hitboxes.clear(); 
        
        if let Some(lyrics) = &app.lyrics {
            let height = inner_lyrics_area.height as usize;
            let track_ms = app.track.as_ref().map(|t| t.position_ms).unwrap_or(0);
            
            let current_idx = lyrics.iter()
               .position(|l| l.timestamp_ms > track_ms)
               .map(|i| if i > 0 { i - 1 } else { 0 })
               .unwrap_or(lyrics.len().saturating_sub(1));

           let start_idx = if let Some(offset) = app.lyrics_offset {
                offset.min(lyrics.len().saturating_sub(1))
           } else {
                let half_height = height / 2;
                current_idx.saturating_sub(half_height)
           };
           
           let end_idx = (start_idx + height).min(lyrics.len());
           
           let mut lines = Vec::new();
           
           for (offset, (i, line)) in lyrics.iter().enumerate().skip(start_idx).take(end_idx - start_idx).enumerate() {
               let style = if i == current_idx {
                   Style::default().add_modifier(Modifier::BOLD).fg(theme.green)
               } else {
                   Style::default().fg(theme.overlay)
               };
               
               let prefix = if i == current_idx { "● " } else { "  " };
               let prefix_span = if i == current_idx {
                   Span::styled(prefix, Style::default().fg(theme.green))
               } else {
                    Span::styled(prefix, style)
               };

               lines.push(Line::from(vec![
                   prefix_span,
                   Span::styled(line.text.clone(), style)
               ]));
               
               let line_y = inner_lyrics_area.y + offset as u16;
               let hitbox = Rect::new(inner_lyrics_area.x, line_y, inner_lyrics_area.width, 1);
               app.lyrics_hitboxes.push((hitbox, line.timestamp_ms));
           }
           
           let lyrics_widget = Paragraph::new(lines)
               .alignment(Alignment::Center)
               .block(Block::default().style(Style::default().bg(Color::Reset)));
               
           f.render_widget(lyrics_widget, inner_lyrics_area);

        } else {
            let no_lyrics = Paragraph::new(Text::styled("\nNo Lyrics Found", Style::default().fg(theme.overlay)))
                .alignment(Alignment::Center)
                 .block(Block::default().style(Style::default().bg(Color::Reset)));
             f.render_widget(no_lyrics, inner_lyrics_area);
        }
    }

    // --- FOOTER ---
    let desc_style = Style::default().fg(theme.overlay);
    
    // Split footer into 2 chunks: Left (Controls) and Right (Volume)
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),    // Left: Main Controls
            Constraint::Length(12), // Right: Volume Control
        ])
        .split(footer_area);

    let left_footer_text = Line::from(vec![
        Span::styled(" q ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)), 
        Span::styled("Exit   ", desc_style),
        
        Span::styled(" n ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)), 
        Span::styled("Next   ", desc_style),
        
        Span::styled(" p ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)), 
        Span::styled("Prev   ", desc_style),
        
        Span::styled(" Space ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)), 
        Span::styled("Play/Pause", desc_style),
    ]);
    
    let left_footer = Paragraph::new(left_footer_text)
        .alignment(Alignment::Right)
        .block(Block::default().style(Style::default().bg(Color::Reset)));
    f.render_widget(left_footer, footer_chunks[0]);

    let right_footer_text = Line::from(vec![
        Span::styled(" +/- ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)), 
        Span::styled("Vol ", desc_style),
    ]);

    let right_footer = Paragraph::new(right_footer_text)
        .alignment(Alignment::Right)
        .block(Block::default().style(Style::default().bg(Color::Reset)));
    f.render_widget(right_footer, footer_chunks[1]);
}
