use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    style::{Color, Style, Modifier},
    text::{Span, Line, Text},
    widgets::{block::Title, Block, Paragraph, Borders, BorderType},
    Frame,
};
use crate::app::{App, ArtworkState, LyricsState, ViewMode};
use crate::player::PlayerState;



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
    // Only enable horizontal split if NOT in Tmux (as per user request) AND wide enough.
    let wide_mode = !app.is_tmux && width >= 90;
    
    // Logic:
    // If we want lyrics:
    //    If wide -> Horizontal Split.
    //    If narrow -> Vertical Split.
    //       If too short (height < 40) -> Hide Lyrics (Compressed).
    // If we don't want lyrics -> Music Card only.

    let show_lyrics = app.app_show_lyrics;
    
    let (music_area, lyrics_area, _is_horizontal) = if show_lyrics {
        if wide_mode {
             // Unified Horizontal Mode: Music Dominant (65%)
             let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(65), // Bigger Music
                    Constraint::Min(10),        // Lyrics
                ])
                .split(body_area);
             (chunks[0], Some(chunks[1]), true)
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

    match &app.artwork {
        ArtworkState::Loaded(raw_image) => {
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
        },
        ArtworkState::Loading => {
            let p = Paragraph::new("\n\n\n\n\n        Loading...".to_string())
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(theme.yellow).bg(Color::Reset)));
             f.render_widget(p, artwork_area);
        },
        ArtworkState::Failed | ArtworkState::Idle => {
            let text = "\n\n\n\n\n        ♪\n    No Album\n      Art".to_string();
            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(theme.overlay).bg(Color::Reset)));
            f.render_widget(p, artwork_area);
        }
    }

    // 2. Info
    let info_idx = 1;
    if let Some(track) = &app.track {
        // Build audio quality badge 🎵
        let audio_badge: Option<Line> = if track.codec.is_some() || track.sample_rate.is_some() {
            let mut spans = Vec::new();
            
            // Audio Quality Badges 🎵
            // Hi-Res: 24bit+ or sample rate > 44.1kHz
            // CD Quality: 16bit/44.1kHz lossless
            // Lossy: MP3, AAC, OGG, etc.
            
            let is_hires = track.bit_depth.map(|b| b >= 24).unwrap_or(false)
                || track.sample_rate.map(|r| r > 44100).unwrap_or(false);
            
            let is_lossless = track.codec.as_ref()
                .map(|c| matches!(c.to_uppercase().as_str(), "FLAC" | "ALAC" | "WAV" | "AIFF" | "APE" | "DSD"))
                .unwrap_or(false);
            
            let is_lossy = track.codec.as_ref()
                .map(|c| matches!(c.to_uppercase().as_str(), "MP3" | "AAC" | "OGG" | "OPUS" | "M4A" | "WMA"))
                .unwrap_or(false);
            
            if is_hires {
                spans.push(Span::styled(" Hi-Res ", Style::default()
                    .fg(theme.base).bg(theme.green).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" "));
            } else if is_lossless && track.bit_depth == Some(16) {
                spans.push(Span::styled(" CD ", Style::default()
                    .fg(theme.base).bg(theme.blue).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" "));
            } else if is_lossy {
                spans.push(Span::styled(" Lossy ", Style::default()
                    .fg(theme.base).bg(theme.overlay).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" "));
            } else if is_lossless {
                spans.push(Span::styled(" Lossless ", Style::default()
                    .fg(theme.base).bg(theme.cyan).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" "));
            }
            
            // Gapless badge (when consecutive tracks from same album)
            if app.gapless_mode {
                spans.push(Span::styled(" Gapless ", Style::default()
                    .fg(theme.base).bg(theme.magenta).add_modifier(Modifier::BOLD)));
                spans.push(Span::raw(" "));
            }
            
            // Codec
            if let Some(codec) = &track.codec {
                spans.push(Span::styled(codec, Style::default().fg(theme.cyan)));
                spans.push(Span::raw(" "));
            }
            
            // Bit depth + Sample rate (e.g., "24bit/96kHz")
            if let (Some(depth), Some(rate)) = (track.bit_depth, track.sample_rate) {
                let khz = rate / 1000;
                spans.push(Span::styled(
                    format!("{}bit/{}kHz", depth, khz),
                    Style::default().fg(theme.overlay)
                ));
            } else if let Some(rate) = track.sample_rate {
                let khz = rate / 1000;
                spans.push(Span::styled(format!("{}kHz", khz), Style::default().fg(theme.overlay)));
            }
            
            // Bitrate (for lossy)
            if let Some(kbps) = track.bitrate {
                if kbps > 0 {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(format!("{}kbps", kbps), Style::default().fg(theme.overlay)));
                }
            }
            
            if !spans.is_empty() {
                Some(Line::from(spans))
            } else {
                None
            }
        } else {
            None
        };
        
        let mut info_text = vec![
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
        
        // Add audio badge if available
        if let Some(badge) = audio_badge {
            info_text.push(badge);
        }
        
        let info = Paragraph::new(info_text)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true })
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
            

        }

    } else {
        // IDLE STATE
        let t = Paragraph::new("Music Paused / Not Running")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text));
        
        // Just center it in available space
        f.render_widget(t, inner_music_area);
    }
    
    // --- RIGHT PANEL (Lyrics/Cava/Queue/EQ) ---
    if let Some(lyrics_area_rect) = lyrics_area {
        // Dynamic title based on view mode 🎛️
        let mode_title = match app.view_mode {
            ViewMode::Lyrics => " Lyrics ",
            ViewMode::Cava => " Cava ",
            ViewMode::Library => " Library ",
            ViewMode::EQ => " EQ ",
        };
        
        let lyrics_title = Title::from(Line::from(vec![
            Span::styled(mode_title, Style::default().fg(theme.base).bg(theme.magenta).add_modifier(Modifier::BOLD))
        ]));

        // Shuffle/Repeat status icons
        let shuffle_icon = if app.shuffle { " 🔀 " } else { "" };
        let repeat_icon = if app.repeat { " 🔁 " } else { "" };

        let credits_title = Line::from(vec![
            Span::styled(shuffle_icon, Style::default().fg(theme.green)),
            Span::styled(repeat_icon, Style::default().fg(theme.blue)),
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

        // Content based on current view mode 🎛️
        match app.view_mode {
            ViewMode::Lyrics => {
                match &app.lyrics {
                    LyricsState::Loaded(lyrics) => {
                        let height = inner_lyrics_area.height as usize;
                        let track_ms = app.track.as_ref().map(|t| t.position_ms).unwrap_or(0);
                        
                        let current_idx = lyrics.iter()
                           .position(|l| l.timestamp_ms > track_ms)
                           .map(|i| if i > 0 { i - 1 } else { 0 })
                           .unwrap_or(lyrics.len().saturating_sub(1));

                        let mut lines = Vec::new();
                        let half_height = height / 2;
                        let center_idx = app.lyrics_offset.unwrap_or(current_idx);

                        for row in 0..height {
                             let dist_from_center: isize = (row as isize - half_height as isize).abs();
                             let target_idx_isize = (center_idx as isize) - (half_height as isize) + (row as isize);
                             
                             if dist_from_center <= 8 && target_idx_isize >= 0 && target_idx_isize < lyrics.len() as isize {
                                 let idx = target_idx_isize as usize;
                                 let line = &lyrics[idx];
                                 
                                 let is_active = idx == current_idx;
                                 
                                 let style = if is_active {
                                    Style::default().add_modifier(Modifier::BOLD).fg(theme.green)
                                 } else {
                                    match dist_from_center {
                                        1..=2 => Style::default().fg(theme.text),
                                        3..=4 => Style::default().fg(theme.text).add_modifier(Modifier::DIM),
                                        5..=6 => Style::default().fg(theme.overlay),
                                        7..=8 => Style::default().fg(theme.surface).add_modifier(Modifier::DIM),
                                        _ => Style::default().fg(theme.base),
                                    }
                                 };

                                let prefix = if is_active { "● " } else { "  " };
                                let prefix_span = if is_active {
                                    Span::styled(prefix, Style::default().fg(theme.green))
                                } else {
                                     Span::styled(prefix, style)
                                };

                                lines.push(Line::from(vec![
                                    prefix_span,
                                    Span::styled(line.text.clone(), style)
                                ]));
                                
                                let line_y = inner_lyrics_area.y + row as u16;
                                let hitbox = Rect::new(inner_lyrics_area.x, line_y, inner_lyrics_area.width, 1);


                             } else {
                                 lines.push(Line::from(""));
                             }
                        }
                        
                        let lyrics_widget = Paragraph::new(lines)
                            .alignment(Alignment::Center)
                            .wrap(ratatui::widgets::Wrap { trim: true }) 
                            .block(Block::default().style(Style::default().bg(Color::Reset)));
                            
                        f.render_widget(lyrics_widget, inner_lyrics_area);
                    },
                    LyricsState::Loading => {
                        let text = Paragraph::new(Text::styled("\nFetching Lyrics...", Style::default().fg(theme.yellow)))
                            .alignment(Alignment::Center)
                            .block(Block::default().style(Style::default().bg(Color::Reset)));
                        f.render_widget(text, inner_lyrics_area);
                    },
                    LyricsState::Instrumental => {
                        let text = Paragraph::new(Text::styled("\n\n\n\n♫ Instrumental ♫", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)))
                            .alignment(Alignment::Center)
                            .block(Block::default().style(Style::default().bg(Color::Reset)));
                        f.render_widget(text, inner_lyrics_area);
                    },
                    LyricsState::Failed(err) => {
                         let text = Paragraph::new(Text::styled(format!("\nLyrics Failed: {}", err), Style::default().fg(theme.red)))
                            .alignment(Alignment::Center)
                             .block(Block::default().style(Style::default().bg(Color::Reset)));
                         f.render_widget(text, inner_lyrics_area);
                    },
                    LyricsState::Idle | LyricsState::NotFound => {
                        let no_lyrics = Paragraph::new(Text::styled("\nNo Lyrics Found", Style::default().fg(theme.overlay)))
                            .alignment(Alignment::Center)
                             .block(Block::default().style(Style::default().bg(Color::Reset)));
                         f.render_widget(no_lyrics, inner_lyrics_area);
                    }
                }
            },
            ViewMode::Cava => {
                // 🌊 Premium Cava Spectrum Visualizer with Mirror Effect
                let width = inner_lyrics_area.width as usize;
                let height = inner_lyrics_area.height as usize;
                
                if height < 4 || width < 10 {
                    let msg = Paragraph::new("♪ Resize for visualizer")
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(theme.overlay));
                    f.render_widget(msg, inner_lyrics_area);
                } else {
                    // Use single-char bars for cleaner look
                    let bar_count = (width / 2).max(8).min(64);
                    
                    // Split height: main bars (75%) + reflection (25%)
                    let main_height = (height * 75 / 100).max(2);
                    let reflection_height = height.saturating_sub(main_height + 1);
                    
                    let mut lines = Vec::new();
                    
                    // 8-color gradient for smooth transitions 🌈
                    let gradient = [
                        Color::Rgb(237, 135, 150), // Red/Pink
                        Color::Rgb(245, 169, 127), // Peach
                        Color::Rgb(238, 212, 159), // Yellow
                        Color::Rgb(166, 218, 149), // Green
                        Color::Rgb(139, 213, 202), // Teal
                        Color::Rgb(138, 173, 244), // Blue
                        Color::Rgb(183, 189, 248), // Lavender
                        Color::Rgb(198, 160, 246), // Mauve
                    ];
                    
                    // === MAIN BARS (grow upward from center) ===
                    for row in 0..main_height {
                        let mut spans = Vec::new();
                        let threshold = 1.0 - (row as f32 / main_height as f32);
                        
                        // Center padding (3 chars per bar: 2 for bar + 1 gap)
                        let total_bar_width = bar_count * 3 - 1;
                        let padding = (width.saturating_sub(total_bar_width)) / 2;
                        if padding > 0 {
                            spans.push(Span::raw(" ".repeat(padding)));
                        }
                        
                        for i in 0..bar_count {
                            let bar_idx = i % app.visualizer_bars.len().max(1);
                            let bar_height = app.visualizer_bars.get(bar_idx).copied().unwrap_or(0.3);
                            
                            // Map bar position to gradient color
                            let color_idx = (i * gradient.len() / bar_count).min(gradient.len() - 1);
                            let bar_color = gradient[color_idx];
                            
                            // Draw bar segment with smooth caps
                            let char = if bar_height > threshold {
                                "██"
                            } else if bar_height > threshold - 0.06 {
                                "▓▓"
                            } else if bar_height > threshold - 0.12 {
                                "▒▒"
                            } else {
                                "  "
                            };
                            
                            spans.push(Span::styled(char, Style::default().fg(bar_color)));
                            
                            // Gap between bars
                            if i < bar_count - 1 {
                                spans.push(Span::raw(" "));
                            }
                        }
                        
                        lines.push(Line::from(spans));
                    }
                    
                    // === CENTER LINE ===
                    let center_width = bar_count * 3 - 1;
                    let center_padding = (width.saturating_sub(center_width)) / 2;
                    let center_line = format!("{}{}",
                        " ".repeat(center_padding),
                        "─".repeat(center_width.min(width.saturating_sub(center_padding)))
                    );
                    lines.push(Line::from(Span::styled(center_line, Style::default().fg(theme.magenta))));
                    
                    // === REFLECTION (dimmed, inverted) ===
                    for row in 0..reflection_height {
                        let mut spans = Vec::new();
                        // Inverted threshold for mirror effect
                        let threshold = (row as f32 / reflection_height as f32) * 0.6; // Damped
                        
                        // Center padding (3 chars per bar: 2 for bar + 1 gap)
                        let total_bar_width = bar_count * 3 - 1;
                        let padding = (width.saturating_sub(total_bar_width)) / 2;
                        if padding > 0 {
                            spans.push(Span::raw(" ".repeat(padding)));
                        }
                        
                        for i in 0..bar_count {
                            let bar_idx = i % app.visualizer_bars.len().max(1);
                            let bar_height = app.visualizer_bars.get(bar_idx).copied().unwrap_or(0.3);
                            
                            // Dimmed gradient for reflection
                            let color_idx = (i * gradient.len() / bar_count).min(gradient.len() - 1);
                            let base = gradient[color_idx];
                            let dimmed = match base {
                                Color::Rgb(r, g, b) => Color::Rgb(r / 3, g / 3, b / 3),
                                _ => theme.surface,
                            };
                            
                            // Reflection is inverted and fades out
                            let char = if bar_height * 0.5 > threshold {
                                "░░"
                            } else {
                                "  "
                            };
                            
                            spans.push(Span::styled(char, Style::default().fg(dimmed)));
                            
                            if i < bar_count - 1 {
                                spans.push(Span::raw(" "));
                            }
                        }
                        
                        lines.push(Line::from(spans));
                    }
                    
                    let visualizer = Paragraph::new(lines)
                        .block(Block::default().style(Style::default().bg(Color::Reset)));
                    f.render_widget(visualizer, inner_lyrics_area);
                }
            },
            ViewMode::Library => {
                // Smart Library Panel 📚
                use crate::app::LibraryMode;
                
                let w = inner_lyrics_area.width as usize;
                let h = inner_lyrics_area.height as usize;
                let mut lines: Vec<Line> = Vec::new();
                
                // Mode tabs at top
                let queue_tab = if app.library_mode == LibraryMode::Queue { "│ ▶ Queue │" } else { "  Queue  " };
                let browse_tab = if app.library_mode == LibraryMode::Browse { "│ 📂 Browse │" } else { "  Browse  " };
                let search_tab = if app.library_mode == LibraryMode::Search { "│ 🔍 Search │" } else { "  Search  " };
                let playlist_tab = if app.library_mode == LibraryMode::Playlists { "│ 📋 Playlists │" } else { "  Playlists  " };
                
                // Calculate tab widths for hitboxes (unicode-width accounting)
                let tab_y = inner_lyrics_area.y;

                
                lines.push(Line::from(vec![
                    Span::styled(queue_tab, Style::default().fg(if app.library_mode == LibraryMode::Queue { theme.green } else { theme.overlay })),
                    Span::styled(browse_tab, Style::default().fg(if app.library_mode == LibraryMode::Browse { theme.blue } else { theme.overlay })),
                    Span::styled(search_tab, Style::default().fg(if app.library_mode == LibraryMode::Search { theme.magenta } else { theme.overlay })),
                    Span::styled(playlist_tab, Style::default().fg(if app.library_mode == LibraryMode::Playlists { theme.text } else { theme.overlay })),
                ]));
                lines.push(Line::from(Span::styled("─".repeat(w), Style::default().fg(theme.overlay))));
                
                match app.library_mode {
                    LibraryMode::Queue => {
                        // Queue View

                        if app.queue.is_empty() {
                            lines.push(Line::from(""));
                            lines.push(Line::from(Span::styled("📋 Queue Empty", Style::default().fg(theme.overlay))));
                            lines.push(Line::from(Span::styled("Add songs from Browse or Search", Style::default().fg(theme.overlay))));
                        } else {
                            let content_height = h.saturating_sub(3);
                            let start_idx = app.library_selected.saturating_sub(content_height / 2);
                            
                            for (display_idx, (i, item)) in app.queue.iter().enumerate().skip(start_idx).take(content_height).enumerate() {
                                let actual_idx = start_idx + display_idx;
                                let is_selected = actual_idx == app.library_selected;
                                let prefix = if item.is_current { "▶ " } else if is_selected { "› " } else { "  " };
                                
                                // Add hitbox for this queue item
                                let item_y = inner_lyrics_area.y + 2 + display_idx as u16; // +2 for tabs and separator

                                
                                // Highlight selected item, show playing item differently
                                let title_style = if is_selected {
                                    Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
                                } else if item.is_current {
                                    Style::default().fg(theme.magenta)
                                } else {
                                    Style::default().fg(theme.text)
                                };
                                let max_len = w.saturating_sub(10);
                                let title = if item.title.len() > max_len {
                                    format!("{}...", &item.title[..max_len.saturating_sub(3)])
                                } else {
                                    item.title.clone()
                                };
                                lines.push(Line::from(vec![
                                    Span::styled(prefix, title_style),
                                    Span::styled(title, title_style),
                                ]));
                            }
                        }
                    }
                    LibraryMode::Browse => {
                        // Browse View
                        if app.browse_path.is_empty() {
                            // Show root categories
                            let categories = ["🎤 Artists", "💿 Albums", "🎭 Genres", "📁 Folders"];
                            lines.push(Line::from(""));
                            for (i, cat) in categories.iter().enumerate() {
                                let is_sel = i == app.library_selected;
                                let style = if is_sel {
                                    Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(theme.text)
                                };
                                let prefix = if is_sel { "▶ " } else { "  " };
                                lines.push(Line::from(Span::styled(format!("{}{}", prefix, cat), style)));
                            }
                        } else {
                            // Show current path as breadcrumb
                            let path = app.browse_path.join(" > ");
                            lines.push(Line::from(Span::styled(format!("📂 {}", path), Style::default().fg(theme.blue))));
                            lines.push(Line::from(""));
                            
                            // Show items
                            let content_height = h.saturating_sub(5);
                            for (i, item) in app.library_items.iter().take(content_height).enumerate() {
                                let is_sel = i == app.library_selected;
                                let style = if is_sel {
                                    Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(theme.text)
                                };
                                let prefix = if is_sel { "▶ " } else { "  " };
                                lines.push(Line::from(Span::styled(format!("{}{}", prefix, item.name), style)));
                            }
                        }
                    }
                    LibraryMode::Search => {
                        // Search View
                        let search_box = if app.search_active {
                            format!("🔍 [{}▌]", app.search_query)
                        } else {
                            format!("🔍 [{}] (press / to search)", app.search_query)
                        };
                        lines.push(Line::from(Span::styled(search_box, Style::default().fg(theme.magenta))));
                        lines.push(Line::from(""));
                        
                        if app.library_items.is_empty() && !app.search_query.is_empty() {
                            lines.push(Line::from(Span::styled("No results", Style::default().fg(theme.overlay))));
                        } else {
                            let content_height = h.saturating_sub(4);
                            for (i, item) in app.library_items.iter().take(content_height).enumerate() {
                                let is_sel = i == app.library_selected;
                                let style = if is_sel {
                                    Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(theme.text)
                                };
                                let prefix = if is_sel { "▶ " } else { "  " };
                                let artist = item.artist.as_deref().unwrap_or("");
                                lines.push(Line::from(vec![
                                    Span::styled(prefix, style),
                                    Span::styled(&item.name, style),
                                    Span::styled(format!(" - {}", artist), Style::default().fg(theme.overlay)),
                                ]));
                            }
                        }
                    }
                    LibraryMode::Playlists => {
                        // Playlists View
                        lines.push(Line::from(""));
                        if app.playlists.is_empty() {
                            lines.push(Line::from(Span::styled("📋 No saved playlists", Style::default().fg(theme.overlay))));
                            lines.push(Line::from(Span::styled("Press 's' to save current queue", Style::default().fg(theme.overlay))));
                        } else {
                            for (i, pl) in app.playlists.iter().enumerate() {
                                let is_sel = i == app.library_selected;
                                let style = if is_sel {
                                    Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(theme.text)
                                };
                                let prefix = if is_sel { "▶ " } else { "  " };
                                lines.push(Line::from(Span::styled(format!("{}📋 {}", prefix, pl), style)));
                            }
                        }
                    }
                }
                
                let library_widget = Paragraph::new(lines)
                    .block(Block::default().style(Style::default().bg(Color::Reset)));
                f.render_widget(library_widget, inner_lyrics_area);
            },
            ViewMode::EQ => {
                // 🎛️ Beautiful 10-Band Equalizer with Axes
                let w = inner_lyrics_area.width as usize;
                let h = inner_lyrics_area.height as usize;
                
                if h < 12 || w < 30 {
                    let msg = Paragraph::new("♪ Resize for EQ")
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(theme.overlay));
                    f.render_widget(msg, inner_lyrics_area);
                } else {
                    let mut lines: Vec<Line> = Vec::new();
                    
                    // Config
                    let bands = 10;
                    let freqs = ["32", "64", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
                    
                    // Colors
                    let _purple = Color::Rgb(180, 142, 255);  // Purple axis (reserved)
                    let blue = Color::Rgb(137, 180, 250);    // Blue curve
                    let green = Color::Rgb(166, 218, 149);   // Green boost
                    let pink = Color::Rgb(243, 139, 168);    // Pink cut
                    let cream = Color::Rgb(245, 224, 220);   // Cream selected
                    let dim = theme.surface;
                    
                    // ─────────────────────────────────────
                    // CLEAN MINIMAL FREQUENCY RESPONSE
                    // ─────────────────────────────────────
                    let label_width = 4;
                    let actual_graph_cols = w.saturating_sub(label_width);
                    let graph_rows = (h * 45 / 100).max(6);
                    
                    // Grid line positions for each band
                    let band_positions: Vec<usize> = (0..bands)
                        .map(|i| (actual_graph_cols * (2 * i + 1)) / (2 * bands))
                        .collect();
                    
                    // Calculate smooth curve positions
                    let mut curve_y: Vec<usize> = vec![graph_rows / 2; actual_graph_cols];
                    for col in 0..actual_graph_cols {
                        let mut band_idx = 0;
                        for i in 0..bands {
                            if col >= band_positions[i] {
                                band_idx = i;
                            }
                        }
                        
                        if col <= band_positions[0] {
                            curve_y[col] = ((1.0 - app.eq_bands[0]) * (graph_rows - 1) as f32).round() as usize;
                        } else if col >= band_positions[bands - 1] {
                            curve_y[col] = ((1.0 - app.eq_bands[bands - 1]) * (graph_rows - 1) as f32).round() as usize;
                        } else {
                            let next_band = (band_idx + 1).min(bands - 1);
                            let x1 = band_positions[band_idx];
                            let x2 = band_positions[next_band];
                            let y1 = (1.0 - app.eq_bands[band_idx]) * (graph_rows - 1) as f32;
                            let y2 = (1.0 - app.eq_bands[next_band]) * (graph_rows - 1) as f32;
                            
                            if x2 > x1 {
                                let t = (col - x1) as f32 / (x2 - x1) as f32;
                                let t_smooth = t * t * (3.0 - 2.0 * t);
                                curve_y[col] = (y1 + t_smooth * (y2 - y1)).round() as usize;
                            }
                        }
                    }
                    
                    let center_row = graph_rows / 2;
                    
                    // Render clean graph
                    for row in 0..graph_rows {
                        let mut spans: Vec<Span> = Vec::new();
                        
                        // dB labels - subtle
                        let db_label = if row == 0 {
                            "+12"
                        } else if row == center_row {
                            "  0"
                        } else if row == graph_rows - 1 {
                            "-12"
                        } else {
                            "   "
                        };
                        
                        spans.push(Span::styled(db_label, Style::default().fg(dim)));
                        spans.push(Span::styled("│", Style::default().fg(dim)));
                        
                        for col in 0..actual_graph_cols {
                            let is_on_curve = row == curve_y[col];
                            let is_band_point = band_positions.contains(&col) && is_on_curve;
                            let is_center = row == center_row;
                            
                            if is_band_point {
                                // Band control points - slightly larger
                                spans.push(Span::styled("●", Style::default().fg(blue)));
                            } else if is_on_curve {
                                // Smooth curve line
                                spans.push(Span::styled("─", Style::default().fg(blue)));
                            } else if is_center {
                                // Center line - very subtle dotted
                                if col % 4 == 0 {
                                    spans.push(Span::styled("·", Style::default().fg(dim)));
                                } else {
                                    spans.push(Span::raw(" "));
                                }
                            } else {
                                spans.push(Span::raw(" "));
                            }
                        }
                        
                        lines.push(Line::from(spans));
                    }
                    
                    // Bottom axis - simple line
                    let mut axis_spans: Vec<Span> = Vec::new();
                    axis_spans.push(Span::styled(" Hz ", Style::default().fg(dim)));
                    axis_spans.push(Span::styled("─".repeat(actual_graph_cols), Style::default().fg(dim)));
                    lines.push(Line::from(axis_spans));
                    
                    // Frequency labels - clean and aligned
                    let mut freq_spans: Vec<Span> = Vec::new();
                    freq_spans.push(Span::raw("    "));
                    
                    for i in 0..bands {
                        let pos = band_positions[i];
                        let label = freqs[i];
                        let label_len = label.len();
                        
                        let prev_end = if i == 0 { 0 } else { 
                            band_positions[i - 1] + freqs[i - 1].len() / 2 + 1 
                        };
                        let spaces_before = pos.saturating_sub(label_len / 2).saturating_sub(prev_end);
                        
                        if spaces_before > 0 {
                            freq_spans.push(Span::raw(" ".repeat(spaces_before)));
                        }
                        freq_spans.push(Span::styled(label, Style::default().fg(theme.overlay)));
                    }
                    lines.push(Line::from(freq_spans));
                    
                    // Simple spacer
                    lines.push(Line::from(""));
                    
                    // ─────────────────────────────────────
                    // CLEAN MINIMAL SLIDERS
                    // ─────────────────────────────────────
                    let slider_rows = (h * 30 / 100).max(5);
                    let center = slider_rows / 2;
                    
                    for row in 0..slider_rows {
                        let mut chars: Vec<char> = vec![' '; w];
                        let mut colors: Vec<Color> = vec![theme.base; w];
                        
                        // Center line - subtle dots
                        if row == center {
                            for c in 0..w {
                                if c % 4 == 0 {
                                    chars[c] = '·';
                                    colors[c] = dim;
                                }
                            }
                        }
                        
                        // Sliders - clean minimal colors
                        for band in 0..bands {
                            let graph_cols = w.saturating_sub(2);
                            let x = 2 + (graph_cols * (2 * band + 1)) / (2 * bands);
                            if x >= w { continue; }
                            
                            let val = app.eq_bands[band];
                            let knob_row = ((1.0 - val) * (slider_rows - 1) as f32).round() as usize;
                            let is_sel = band == app.eq_selected;
                            
                            if row == knob_row {
                                // Knob
                                chars[x] = if is_sel { '◉' } else { '●' };
                                colors[x] = if is_sel { cream } else { blue };
                            } else if val > 0.5 && row > knob_row && row <= center {
                                // Boost - green
                                chars[x] = '│';
                                colors[x] = green;
                            } else if val < 0.5 && row < knob_row && row >= center {
                                // Cut - pink
                                chars[x] = '│';
                                colors[x] = pink;
                            } else {
                                // Track
                                chars[x] = '│';
                                colors[x] = dim;
                            }
                        }
                        
                        let spans: Vec<Span> = chars.iter().zip(colors.iter())
                            .map(|(c, col)| Span::styled(c.to_string(), Style::default().fg(*col)))
                            .collect();
                        lines.push(Line::from(spans));
                    }
                    
                    // ─────────────────────────────────────
                    // CLEAN FREQUENCY LABELS
                    // ─────────────────────────────────────
                    let mut lbl_spans: Vec<Span> = Vec::new();
                    let mut pos = 0;
                    let graph_cols = w.saturating_sub(2);
                    for (i, freq) in freqs.iter().enumerate() {
                        let target = 2 + (graph_cols * (2 * i + 1)) / (2 * bands);
                        let start = target.saturating_sub(freq.len() / 2);
                        
                        while pos < start {
                            lbl_spans.push(Span::raw(" "));
                            pos += 1;
                        }
                        
                        let is_sel = i == app.eq_selected;
                        let style = if is_sel {
                            Style::default().fg(cream).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.overlay)
                        };
                        lbl_spans.push(Span::styled(*freq, style));
                        pos += freq.len();
                    }
                    lines.push(Line::from(lbl_spans));
                    
                    // Bottom divider
                    lines.push(Line::from(Span::styled("─".repeat(w), Style::default().fg(dim))));
                    
                    // Audiophile Controls Row 🎚️
                    let preamp_str = format!("Preamp {:+.0}dB", app.preamp_db);
                    let balance_str = if app.balance.abs() < 0.05 { 
                        "Balance ◉".to_string() 
                    } else if app.balance > 0.0 {
                        format!("Balance ─{}▶", "●".repeat((app.balance * 5.0) as usize))
                    } else {
                        format!("Balance ◀{}─", "●".repeat((-app.balance * 5.0) as usize))
                    };
                    let xfade_str = if app.crossfade_secs > 0 {
                        format!("Xfade {}s", app.crossfade_secs)
                    } else {
                        "Xfade Off".to_string()
                    };
                    let rg_str = match app.replay_gain_mode {
                        1 => "RG Track",
                        2 => "RG Album",
                        3 => "RG Auto",
                        _ => "RG Off",
                    };
                    
                    let total_ctl_len = preamp_str.len() + balance_str.chars().count() + xfade_str.len() + rg_str.len() + 9;
                    let ctl_pad = (w.saturating_sub(total_ctl_len)) / 5;
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(ctl_pad)),
                        Span::styled(&preamp_str, Style::default().fg(if app.preamp_db != 0.0 { green } else { theme.overlay })),
                        Span::raw("  "),
                        Span::styled(&balance_str, Style::default().fg(if app.balance != 0.0 { pink } else { theme.overlay })),
                        Span::raw("  "),
                        Span::styled(&xfade_str, Style::default().fg(if app.crossfade_secs > 0 { blue } else { theme.overlay })),
                        Span::raw("  "),
                        Span::styled(rg_str, Style::default().fg(if app.replay_gain_mode > 0 { green } else { theme.overlay })),
                    ]));
                    
                    // Device name (from actual audio output)
                    let device = format!("🔊 {}", app.output_device);
                    
                    // Preset name
                    let preset = format!(" [{}]", app.get_preset_name());
                    
                    // DSP EQ status indicator
                    let status = if app.dsp_available {
                        if app.eq_enabled { " ● ON" } else { " ○ OFF" }
                    } else {
                        " ⚠ N/A"
                    };
                    let status_color = if app.dsp_available {
                        if app.eq_enabled { green } else { theme.overlay }
                    } else {
                        pink
                    };
                    
                    let total_len = device.chars().count() + preset.len() + status.len();
                    let device_pad = (w.saturating_sub(total_len)) / 2;
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(device_pad)),
                        Span::styled(device, Style::default().fg(green)),
                        Span::styled(preset, Style::default().fg(blue)),
                        Span::styled(status, Style::default().fg(status_color)),
                    ]));
                    
                    let widget = Paragraph::new(lines)
                        .block(Block::default().style(Style::default().bg(Color::Reset)));
                    f.render_widget(widget, inner_lyrics_area);
                }
            },
        }
    }

    // --- AUDIO INFO POPUP (like Poweramp) ---
    if app.show_audio_info {
        use ratatui::widgets::Clear;
        
        let width = 50.min(f.area().width.saturating_sub(4));
        let height = 20.min(f.area().height.saturating_sub(4));
        let x = (f.area().width.saturating_sub(width)) / 2;
        let y = (f.area().height.saturating_sub(height)) / 2;
        let area = Rect::new(x, y, width, height);
        
        f.render_widget(Clear, area);
        
        let mut lines: Vec<Line> = Vec::new();
        
        // Header
        lines.push(Line::from(vec![
            Span::styled("◉ ", Style::default().fg(theme.green)),
            Span::styled("Audio Info", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        
        // Track Info Section
        lines.push(Line::from(vec![
            Span::styled("♫ ", Style::default().fg(theme.magenta)),
            Span::styled("Track", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]));
        
        // Current song info from TrackInfo
        if let Some(ref track) = app.track {
            lines.push(Line::from(vec![
                Span::styled("  Title: ", Style::default().fg(theme.overlay)),
                Span::styled(&track.name, Style::default().fg(theme.text)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Artist: ", Style::default().fg(theme.overlay)),
                Span::styled(&track.artist, Style::default().fg(theme.text)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Album: ", Style::default().fg(theme.overlay)),
                Span::styled(&track.album, Style::default().fg(theme.text)),
            ]));
            
            // Audiophile metadata if available
            if let Some(ref codec) = track.codec {
                let mut format_parts = vec![codec.clone()];
                if let Some(depth) = track.bit_depth {
                    format_parts.push(format!("{} bit", depth));
                }
                if let Some(rate) = track.sample_rate {
                    format_parts.push(format!("{:.1} kHz", rate as f32 / 1000.0));
                }
                if let Some(bitrate) = track.bitrate {
                    format_parts.push(format!("{} kbps", bitrate));
                }
                lines.push(Line::from(vec![
                    Span::styled("  Format: ", Style::default().fg(theme.overlay)),
                    Span::styled(format_parts.join(" / "), Style::default().fg(theme.green)),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("  No track playing", Style::default().fg(theme.overlay)),
            ]));
        }
        
        // Queue position
        let queue_pos = format!("{} / {}", app.library_selected + 1, app.queue.len());
        lines.push(Line::from(vec![
            Span::styled("  Queue: ", Style::default().fg(theme.overlay)),
            Span::styled(queue_pos, Style::default().fg(theme.text)),
        ]));
        
        lines.push(Line::from(""));
        
        // Playback Status Section
        lines.push(Line::from(vec![
            Span::styled("▶ ", Style::default().fg(theme.green)),
            Span::styled("Playback", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]));
        
        let is_paused = app.track.as_ref().map(|t| t.state == crate::player::PlayerState::Paused).unwrap_or(true);
        let status = if is_paused { "Paused" } else { "Playing" };
        lines.push(Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(theme.overlay)),
            Span::styled(status, Style::default().fg(if is_paused { theme.yellow } else { theme.green })),
        ]));
        
        let shuffle_str = if app.shuffle { "ON" } else { "OFF" };
        lines.push(Line::from(vec![
            Span::styled("  Shuffle: ", Style::default().fg(theme.overlay)),
            Span::styled(shuffle_str, Style::default().fg(if app.shuffle { theme.green } else { theme.overlay })),
        ]));
        
        let repeat_str = if app.repeat { "ON" } else { "OFF" };
        lines.push(Line::from(vec![
            Span::styled("  Repeat: ", Style::default().fg(theme.overlay)),
            Span::styled(repeat_str, Style::default().fg(if app.repeat { theme.green } else { theme.overlay })),
        ]));
        
        let gapless_str = if app.gapless_mode { "Active" } else { "OFF" };
        lines.push(Line::from(vec![
            Span::styled("  Gapless: ", Style::default().fg(theme.overlay)),
            Span::styled(gapless_str, Style::default().fg(if app.gapless_mode { theme.blue } else { theme.overlay })),
        ]));
        
        lines.push(Line::from(""));
        
        // DSP/EQ Section
        lines.push(Line::from(vec![
            Span::styled("🎛 ", Style::default().fg(theme.blue)),
            Span::styled("DSP / EQ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]));
        
        let eq_status = if app.eq_enabled { "Enabled" } else { "Disabled" };
        lines.push(Line::from(vec![
            Span::styled("  Equalizer: ", Style::default().fg(theme.overlay)),
            Span::styled(eq_status, Style::default().fg(if app.eq_enabled { theme.green } else { theme.overlay })),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("  Preset: ", Style::default().fg(theme.overlay)),
            Span::styled(app.get_preset_name(), Style::default().fg(theme.magenta)),
        ]));
        
        lines.push(Line::from(""));
        
        // Output Section
        lines.push(Line::from(vec![
            Span::styled("🔊 ", Style::default().fg(theme.yellow)),
            Span::styled("Output", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]));
        
        let (mode_text, mode_color) = if app.eq_enabled {
            ("DSP Active (EQ Enabled)", theme.yellow)
        } else {
            ("Bit-Perfect (No DSP)", theme.green)
        };
        lines.push(Line::from(vec![
            Span::styled("  Mode: ", Style::default().fg(theme.overlay)),
            Span::styled(mode_text, Style::default().fg(mode_color)),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("  Backend: ", Style::default().fg(theme.overlay)),
            Span::styled("MPD", Style::default().fg(theme.text)),
        ]));
        
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(theme.surface)),
            Span::styled("i", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", Style::default().fg(theme.surface)),
            Span::styled("Esc", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled(" to close", Style::default().fg(theme.surface)),
        ]));
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.blue))
            .style(Style::default().bg(theme.base));
        
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
    }

    // --- TOAST NOTIFICATION ---
    if let Some((message, shown_at)) = &app.toast {
        let elapsed = shown_at.elapsed().as_millis() as u64;
        let duration_ms = 2000; // 2 seconds
        
        if elapsed < duration_ms {
            use ratatui::widgets::Clear;
            
            // Calculate opacity based on time (fade out in last 500ms)
            let fade_start = duration_ms - 500;
            let opacity = if elapsed > fade_start {
                ((duration_ms - elapsed) as f32 / 500.0).max(0.0)
            } else {
                1.0
            };
            
            // Only render if visible
            if opacity > 0.1 {
                let width = (message.len() as u16 + 6).min(f.area().width.saturating_sub(4));
                let height = 3;
                let x = f.area().width.saturating_sub(width + 1); // Top-right
                let y = 1; // Near top
                let area = Rect::new(x, y, width, height);
                
                f.render_widget(Clear, area);
                
                let style = if opacity > 0.5 {
                    Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.overlay)
                };
                
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(if opacity > 0.5 { theme.green } else { theme.surface }))
                    .style(Style::default().bg(theme.base));
                
                let text = Paragraph::new(Line::from(vec![
                    Span::styled("✓ ", style),
                    Span::styled(message.as_str(), style),
                ]))
                .alignment(Alignment::Center)
                .block(block);
                
                f.render_widget(text, area);
            }
        }
    }

    // --- INPUT POPUP ---
    if let Some(ref input) = app.input_state {
        use ratatui::widgets::Clear;
        
        let width = 60.min(f.area().width.saturating_sub(4));
        let height = 5;
        let x = (f.area().width.saturating_sub(width)) / 2;
        let y = (f.area().height.saturating_sub(height)) / 2;
        let area = Rect::new(x, y, width, height);

        f.render_widget(Clear, area);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" > ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled(&input.value, Style::default().fg(theme.text)),
            Span::styled("▌", Style::default().fg(theme.green).add_modifier(Modifier::SLOW_BLINK)),
        ]));
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.blue))
            .title(input.title.as_str())
            .title_style(Style::default().fg(theme.magenta).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(theme.base));
        
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
    }

    // --- TAG EDITOR POPUP ---
    if let Some(ref tag_state) = app.tag_edit {
        use ratatui::widgets::Clear;
        
        // Make popup responsive to terminal size
        let max_popup_width = f.area().width.saturating_sub(4).min(50);
        let max_popup_height = f.area().height.saturating_sub(4).min(12);
        
        // Only show if terminal is big enough
        if max_popup_width >= 30 && max_popup_height >= 8 {
            let popup_x = (f.area().width.saturating_sub(max_popup_width)) / 2;
            let popup_y = (f.area().height.saturating_sub(max_popup_height)) / 2;
            let popup_area = Rect::new(popup_x, popup_y, max_popup_width, max_popup_height);
        
        f.render_widget(Clear, popup_area);
        
        let mut lines: Vec<Line> = Vec::new();
        
        // Title
        lines.push(Line::from(vec![
            Span::styled("🏷️ Edit Tags", Style::default().fg(theme.magenta).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        
        // Fields with active highlighting
        let fields = ["Title", "Artist", "Album"];
        let values = [&tag_state.title, &tag_state.artist, &tag_state.album];
        
        for (i, (field, value)) in fields.iter().zip(values.iter()).enumerate() {
            let is_active = i == tag_state.active_field;
            let field_style = if is_active {
                Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.overlay)
            };
            let value_style = if is_active {
                Style::default().fg(theme.text).add_modifier(Modifier::UNDERLINED)
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
            Span::styled("Tab", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
            Span::styled(" next  ", Style::default().fg(theme.overlay)),
            Span::styled("Enter", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled(" save  ", Style::default().fg(theme.overlay)),
            Span::styled("Esc", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(theme.overlay)),
        ]));
        
        let popup = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.surface))
                .title(" Edit Song Tags ")
                .title_style(Style::default().fg(theme.magenta))
                .style(Style::default().bg(theme.base)));
        f.render_widget(popup, popup_area);
        } // Close min size check
    }

    // --- FOOTER / WHICHKEY POPUP ---
    if app.show_keyhints {
        // 🎹 WhichKey-style floating popup (Helix-inspired, centered)
        use ratatui::widgets::Clear;
        
        // Get context-specific keybindings with icons
        let (title, keys): (&str, Vec<(&str, &str, &str)>) = match app.view_mode {
            ViewMode::EQ => ("EQ Controls", vec![
                ("←/→", "🎚️", "Select band"),
                ("↑/↓", "📊", "Adjust gain"),
                ("e", "⚡", "Toggle EQ"),
                ("0", "↺", "Reset band"),
                ("g/G", "🔊", "Preamp ±1dB"),
                ("b/B", "⚖️", "Balance ±0.1"),
                ("c", "🔀", "Crossfade"),
                ("R", "📀", "ReplayGain"),
                ("d/D", "🎧", "Output device"),
            ]),
            ViewMode::Library => ("Library", vec![
                ("↑/↓", "📋", "Navigate"),
                ("Tab", "🔄", "Switch mode"),
                ("Enter", "▶️", "Select/Play"),
                ("⌫", "←", "Go back"),
                ("/", "🔍", "Search"),
                ("s", "💾", "Save playlist"),
                ("d", "🗑️", "Delete/Remove"),
                ("t", "🏷️", "Edit tags"),
            ]),
            ViewMode::Lyrics => ("Lyrics", vec![
                ("↑/↓", "📜", "Scroll lyrics"),
            ]),
            ViewMode::Cava => ("Visualizer", vec![]),
        };
        
        // Global keys with icons
        let global_keys: Vec<(&str, &str, &str)> = vec![
            ("Space", "▶️", "Play/Pause"),
            ("n", "⏭️", "Next track"),
            ("p", "⏮️", "Previous track"),
            ("+/-", "🔊", "Volume"),
            ("z", "🔀", "Shuffle"),
            ("x", "🔁", "Repeat"),
            ("1-4", "🖼️", "View modes"),
            ("q", "🚪", "Quit"),
        ];
        
        // Calculate popup size - fit content exactly (no extra space)
        let total_items = keys.len() + global_keys.len() + 3; // +3 for title, empty line, global header
        let popup_height = (total_items as u16 + 2).min(22); // +2 for top/bottom borders
        let popup_width = 32u16;
        
        // Position at bottom-right (like Neovim which-key)
        let popup_x = f.area().width.saturating_sub(popup_width + 1);
        let popup_y = f.area().height.saturating_sub(popup_height + 2);
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);
        
        // Clear background
        f.render_widget(Clear, popup_area);
        
        // Build popup content with Helix-style formatting
        let mut lines: Vec<Line> = Vec::new();
        
        // Title centered
        let title_pad = (popup_width as usize - title.len() - 4) / 2;
        lines.push(Line::from(Span::styled(
            format!("{:>pad$}{}", "", title, pad = title_pad),
            Style::default().fg(theme.magenta).add_modifier(Modifier::BOLD)
        )));
        lines.push(Line::from(""));
        
        // Context keys
        for (key, icon, desc) in &keys {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>3} ", key), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                Span::styled("→ ", Style::default().fg(theme.overlay)),
                Span::styled(format!("{} ", icon), Style::default()),
                Span::styled(*desc, Style::default().fg(theme.text)),
            ]));
        }
        
        if !keys.is_empty() {
            lines.push(Line::from(""));
        }
        
        // Global section
        lines.push(Line::from(Span::styled(
            "─── Global ───",
            Style::default().fg(theme.blue)
        )));
        
        for (key, icon, desc) in &global_keys {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>3} ", key), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                Span::styled("→ ", Style::default().fg(theme.overlay)),
                Span::styled(format!("{} ", icon), Style::default()),
                Span::styled(*desc, Style::default().fg(theme.text)),
            ]));
        }
        
        let popup = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.surface))
                .style(Style::default().bg(theme.base)));
        f.render_widget(popup, popup_area);
    } else {
        // Minimal footer hint
        let hint = Line::from(vec![
            Span::styled(" ? ", Style::default().fg(theme.overlay).add_modifier(Modifier::BOLD)),
            Span::styled("keys", Style::default().fg(theme.overlay)),
        ]);
        let footer = Paragraph::new(hint).alignment(Alignment::Right);
        f.render_widget(footer, footer_area);
    }
}
