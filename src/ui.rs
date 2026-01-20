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
            if height < 30 {
                // Too short for stack -> Hide Lyrics (Compressed)
                (body_area, None, false)
            } else {
                // Stack Mode: User requested 45% Top, 55% Bottom
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(45),
                        Constraint::Percentage(55),
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
    let is_cramped = m_height < 10; // Redefined threshold for Tiny Mode

    
    // Elastic Priority Stack 🧠
    // We strictly prioritize: Controls > Info > Gauge > Time > Artwork
    // Artwork gets whatever is left (Constraint::Min(0)).
    
    // Calculate Info Height (4 if badges, 3 if not)
    let info_height = if let Some(track) = &app.track {
        if track.codec.is_some() || track.sample_rate.is_some() { 4 } else { 3 }
    } else {
        4
    };

    let mut music_constraints = Vec::new();
    
    // Extremely small height (< 10): Show only essentials
    if m_height < 10 {
         // Tiny Mode: Artwork 0, Info 1, Controls 1
         music_constraints.push(Constraint::Min(0));    // 0: Artwork (Hidden)
         music_constraints.push(Constraint::Length(m_height.saturating_sub(2).max(1)));  // 1: Info (Takes remaining)
         music_constraints.push(Constraint::Length(0));  // 2: Gauge (Hidden)
         music_constraints.push(Constraint::Length(0));  // 3: Time (Hidden)
         music_constraints.push(Constraint::Length(0));  // 4: Spacer 1 (Hidden)
         music_constraints.push(Constraint::Length(1));  // 5: Controls
    } else {
        // Normal Mode: Artwork takes ALL available space
        music_constraints.push(Constraint::Min(0));           // 0: Artwork (Elastic!)
        music_constraints.push(Constraint::Length(info_height)); // 1: Info (Dynamic)
        music_constraints.push(Constraint::Length(1));        // 2: Spacer 1
        music_constraints.push(Constraint::Length(1));        // 3: Gauge
        music_constraints.push(Constraint::Length(1));        // 4: Time
        // Removed Spacer 2 to tighten layout
        music_constraints.push(Constraint::Length(3));        // 5: Controls
    }

    let music_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(music_constraints)
        .split(inner_music_area);

    // 1. Artwork
    let artwork_area = if music_chunks.len() > 0 && music_chunks[0].height > 1 {
         // Only render art if we have at least 2 lines
         let area = music_chunks[0];
         Layout::default()
             .direction(Direction::Vertical)
             .constraints([
                  Constraint::Min(0),    // Art
              ])
              .split(area)[0]
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
        let gauge_idx = if is_cramped { 2 } else { 3 };
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

            let current_pos = app.get_current_position_ms();
            let ratio = if track.duration_ms > 0 {
                current_pos as f64 / track.duration_ms as f64
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
                    if i == occupied_width.saturating_sub(1) {
                        // Playhead knob
                        bar_spans.push(Span::styled("●", fill_style));
                    } else {
                        // Filled pipe
                        bar_spans.push(Span::styled("━", fill_style));
                    }
                } else {
                    // Empty track
                    bar_spans.push(Span::styled("─", empty_style));
                }
            }

            let gauge_p = Paragraph::new(Line::from(bar_spans))
                .alignment(Alignment::Left)
                .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(gauge_p, gauge_area_rect);

        }

        // 4. Time
        let time_idx = if is_cramped { 3 } else { 4 };
        if time_idx < music_chunks.len() {
            let current_pos = app.get_current_position_ms();
            let time_str = format!(
                "{:02}:{:02} / {:02}:{:02}",
                current_pos / 60000,
                (current_pos % 60000) / 1000,
                track.duration_ms / 60000,
                (track.duration_ms % 60000) / 1000
            );
            let time_label = Paragraph::new(time_str)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.overlay));
            f.render_widget(time_label, music_chunks[time_idx]);
        }
        
        // 5. Controls
        let controls_idx = 5;
        
        if controls_idx < music_chunks.len() {
            let play_icon = if track.state == PlayerState::Playing { "⏸" } else { "▶" };
            let btn_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
            
            let prev_str = "   ⏮   ";
            let next_str = "   ⏭   ";
            let play_str = format!("   {}   ", play_icon); 
            
            // Split Controls Area: Top for Buttons, Bottom for Volume
            let controls_area = music_chunks[controls_idx];
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Buttons
                    Constraint::Length(1), // Spacer
                    Constraint::Length(1), // Volume Bar
                ])
                .split(controls_area);

            // 1. Buttons (Top)
            let controls_text = Line::from(vec![
                Span::styled(prev_str, btn_style),
                Span::raw("   "), 
                Span::styled(play_str, btn_style),
                Span::raw("   "), 
                Span::styled(next_str, btn_style),
            ]);
            
            let controls_widget = Paragraph::new(controls_text)
                .alignment(Alignment::Center)
                .block(Block::default());
            f.render_widget(controls_widget, chunks[0]);

            // 2. Volume Bar (Bottom) - if we have space
            if chunks.len() >= 3 {
                // Calculate Volume Bar
                let vol_ratio = app.app_volume as f64 / 100.0;
                let bar_width = 20; // Fixed width for clean look
                let filled_width = (bar_width as f64 * vol_ratio).round() as usize;
                
                let mut bar_spans = Vec::new();
                
                // "- " 
                bar_spans.push(Span::styled("- ", Style::default().fg(theme.overlay)));
                
                for i in 0..bar_width {
                    if i < filled_width {
                        bar_spans.push(Span::styled("━", Style::default().fg(theme.magenta)));
                    } else {
                        bar_spans.push(Span::styled("─", Style::default().fg(theme.surface)));
                    }
                }
                
                // " + "
                bar_spans.push(Span::styled(" +", Style::default().fg(theme.overlay)));

                let vol_widget = Paragraph::new(Line::from(bar_spans))
                    .alignment(Alignment::Center)
                    .block(Block::default());
                f.render_widget(vol_widget, chunks[2]);
            }

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
                             
                             if dist_from_center <= 6 && target_idx_isize >= 0 && target_idx_isize < lyrics.len() as isize {
                                 let idx = target_idx_isize as usize;
                                 let line = &lyrics[idx];
                                 
                                 let is_active = idx == current_idx;
                                 let is_selected = app.lyrics_selected == Some(idx);
                                 
                                 let style = if is_selected {
                                    // User-selected line (j/k navigation)
                                    Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).fg(theme.yellow)
                                 } else if is_active {
                                    Style::default().add_modifier(Modifier::BOLD).fg(theme.green)
                                 } else {
                                    match dist_from_center {
                                        1..=2 => Style::default().fg(theme.text),
                                        3..=4 => Style::default().fg(theme.text).add_modifier(Modifier::DIM),
                                        5..=6 => Style::default().fg(theme.overlay),
                                        _ => Style::default().fg(theme.base),
                                    }
                                 };

                                let prefix = if is_selected { "▶ " } else if is_active { "● " } else { "  " };
                                let prefix_span = if is_selected {
                                    Span::styled(prefix, Style::default().fg(theme.yellow))
                                } else if is_active {
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
                    
                    let mut lines = Vec::new();

                    // PADDING TOP: Push the visualizer down (15% air)
                    let padding_top = (height * 15 / 100).max(1);
                    for _ in 0..padding_top {
                        lines.push(Line::default());
                    }
                    
                    let available_height = height.saturating_sub(padding_top);
                    
                    // Split remaining height: main bars (65%) + reflection (35%)
                    let main_height = (available_height * 65 / 100).max(2);
                    let reflection_height = available_height.saturating_sub(main_height);
                    
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
                    
                    // === REFLECTION (dimmed, mirrored from center) ===
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
                let search_color = if app.search_active { theme.green } else { theme.overlay };
                
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().fg(search_color)),
                    Span::styled(search_text, Style::default().fg(search_color)),
                ]));
                
                // Elegant thin separator - centered
                lines.push(Line::from(Span::styled(
                    "─".repeat(w.min(60)), // Slightly wider
                    Style::default().fg(theme.surface)
                )).alignment(Alignment::Center));
                
                // Tab bar with filled dot indicators
                let queue_active = app.library_mode == LibraryMode::Queue;
                let dir_active = app.library_mode == LibraryMode::Directory;
                let pl_active = app.library_mode == LibraryMode::Playlists;
                
                // Use filled dots for active, empty for inactive
                let q_dot = if queue_active { "●" } else { "○" };
                let d_dot = if dir_active { "●" } else { "○" };
                let p_dot = if pl_active { "●" } else { "○" };
                
                // Center tabs by adding padding logic or just centering the line
                
                lines.push(Line::from(vec![
                    // Queue
                    Span::styled(format!("{} ", q_dot), 
                        Style::default().fg(if queue_active { theme.green } else { theme.green })), // Always green, just dimmed if inactive? No, let's keep it clean.
                    Span::styled("Queue", 
                        if queue_active { Style::default().fg(theme.green).add_modifier(Modifier::BOLD) } 
                        else { Style::default().fg(theme.green) }), // Inactive is now dimmed green instead of gray
                    Span::styled("      ", Style::default()),
                    
                    // Directory
                    Span::styled(format!("{} ", d_dot), 
                        Style::default().fg(if dir_active { theme.blue } else { theme.blue })),
                    Span::styled("Directory", 
                        if dir_active { Style::default().fg(theme.blue).add_modifier(Modifier::BOLD) } 
                        else { Style::default().fg(theme.blue) }), // Inactive is dimmed blue
                    Span::styled("      ", Style::default()),
                    
                    // Playlists
                    Span::styled(format!("{} ", p_dot), 
                        Style::default().fg(if pl_active { theme.magenta } else { theme.magenta })),
                    Span::styled("Playlists", 
                        if pl_active { Style::default().fg(theme.magenta).add_modifier(Modifier::BOLD) } 
                        else { Style::default().fg(theme.magenta) }), // Inactive is dimmed magenta
                ]).alignment(Alignment::Center)); // Use center alignment!
                
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
                        lines.push(Line::from(Span::styled(
                            format!("  QUEUE  ·  {} songs  ", queue_count), 
                            Style::default().fg(green)
                        )).alignment(Alignment::Center));
                        lines.push(Line::from(""));
                        
                        // ━━━ CONTENT ━━━
                        if app.queue.is_empty() {
                            lines.push(Line::from(Span::styled("Empty queue", Style::default().fg(muted))).alignment(Alignment::Center));
                            lines.push(Line::from(Span::styled("Browse Directory to add songs", Style::default().fg(grid))).alignment(Alignment::Center));
                        } else {
                            let start_idx = app.library_selected.saturating_sub(content_h / 2).min(app.queue.len().saturating_sub(content_h));
                            
                            for (display_idx, (_, item)) in app.queue.iter().enumerate().skip(start_idx).take(content_h).enumerate() {
                                let actual_idx = start_idx + display_idx;
                                let is_sel = actual_idx == app.library_selected;
                                let num = actual_idx + 1;
                                
                                let title = if item.title.len() > title_w.saturating_sub(2) { format!("{}…", &item.title[..title_w.saturating_sub(3)]) } else { item.title.clone() };
                                let artist = if item.artist.len() > artist_w.saturating_sub(1) { format!("{}…", &item.artist[..artist_w.saturating_sub(2)]) } else { item.artist.clone() };
                                let time = { let s = item.duration_ms / 1000; format!("{}:{:02}", s / 60, s % 60) };
                                
                                // Selection markers: ● for selected, ◉ for playing, ○ for normal
                                let (marker, m_color, t_style, a_style, tm_style) = if is_sel {
                                    ("●", cream, Style::default().fg(theme.text).add_modifier(Modifier::BOLD), Style::default().fg(theme.text), Style::default().fg(green))
                                } else if item.is_current {
                                    ("◉", pink, Style::default().fg(pink), Style::default().fg(pink), Style::default().fg(pink))
                                } else {
                                    ("○", grid, Style::default().fg(theme.text), Style::default().fg(muted), Style::default().fg(muted))
                                };
                                
                                lines.push(Line::from(vec![
                                    Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                                    Span::styled(format!("{:>2}  ", num), Style::default().fg(if is_sel { green } else { muted })),
                                    Span::styled("♪ ", Style::default().fg(if item.is_current { pink } else { green })), 
                                    Span::styled(format!("{:title_w$}", title, title_w = title_w.saturating_sub(2)), t_style), 
                                    Span::styled(format!("{:artist_w$}", artist, artist_w = artist_w), a_style),
                                    Span::styled(format!("{:>time_w$}", time, time_w = time_w), tm_style),
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
                        lines.push(Line::from(Span::styled(
                            format!("  DIRECTORY  ·  {}  ", path), 
                            Style::default().fg(blue)
                        )).alignment(Alignment::Center));
                        lines.push(Line::from(""));
                        
                        // ━━━ CONTENT ━━━
                        if app.library_items.is_empty() {
                            lines.push(Line::from(Span::styled("Empty folder", Style::default().fg(muted))).alignment(Alignment::Center));
                        } else {
                            let start_idx = app.library_selected.saturating_sub(content_h / 2).min(app.library_items.len().saturating_sub(content_h));
                            
                            for (display_idx, item) in app.library_items.iter().skip(start_idx).take(content_h).enumerate() {
                                let actual_idx = start_idx + display_idx;
                                let is_sel = actual_idx == app.library_selected;
                                let is_folder = matches!(item.item_type, crate::app::LibraryItemType::Folder);
                                
                                let raw_name = if item.name.trim().is_empty() {
                                    item.path.clone().unwrap_or_else(|| "[Unnamed]".to_string())
                                } else {
                                    item.name.clone()
                                };
                                
                                let name = if raw_name.len() > title_w.saturating_sub(2) { 
                                    format!("{}…", &raw_name[..title_w.saturating_sub(3)]) 
                                } else { 
                                    raw_name
                                };
                                
                                if is_folder {
                                    // Folder row
                                    let (marker, m_color, n_style) = if is_sel {
                                        ("●", cream, Style::default().fg(blue).add_modifier(Modifier::BOLD))
                                    } else {
                                        ("○", grid, Style::default().fg(theme.text))
                                    };
                                    let icon = "📁";
                                    
                                    lines.push(Line::from(vec![
                                        Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                                        Span::styled(format!("{} ", icon), Style::default().fg(blue)),
                                        Span::styled(name, n_style),
                                    ]));
                                } else {
                                    // Song row
                                    let artist = item.artist.clone().unwrap_or_default();
                                    let artist_disp = if artist.len() > artist_w.saturating_sub(1) { 
                                        format!("{}…", &artist[..artist_w.saturating_sub(2)]) 
                                    } else { 
                                        artist 
                                    };
                                    let time = item.duration_ms.map(|ms| { 
                                        let s = ms / 1000; 
                                        format!("{}:{:02}", s / 60, s % 60) 
                                    }).unwrap_or_default();
                                    
                                    let (marker, m_color, t_style, a_style, tm_style) = if is_sel {
                                        ("●", cream, Style::default().fg(theme.text).add_modifier(Modifier::BOLD), Style::default().fg(theme.text), Style::default().fg(green))
                                    } else {
                                        ("○", grid, Style::default().fg(theme.text), Style::default().fg(muted), Style::default().fg(muted))
                                    };
                                    let icon = "♪";
                                    
                                    lines.push(Line::from(vec![
                                        Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                                        Span::styled(format!("{} ", icon), Style::default().fg(green)),
                                        Span::styled(format!("{:title_w$}", name, title_w = title_w), t_style),
                                        Span::styled(format!("{:artist_w$}", artist_disp, artist_w = artist_w), a_style),
                                        Span::styled(format!("{:>time_w$}", time, time_w = time_w), tm_style),
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
                        
                        let lavender = theme.magenta;
                        let green = theme.green;
                        let cream = theme.yellow;
                        let muted = theme.overlay;
                        let grid = theme.surface;
                        
                        // ━━━ CENTERED TITLE ━━━
                        lines.push(Line::from(""));
                        let result_count = app.library_items.len();
                        let title = if app.search_query.is_empty() {
                            "  SEARCH  ".to_string()
                        } else {
                            format!("  \"{}\"  ·  {} results  ", app.search_query, result_count)
                        };
                        lines.push(Line::from(Span::styled(title, Style::default().fg(lavender))).alignment(Alignment::Center));
                        lines.push(Line::from(""));
                        
                        // ━━━ CONTENT ━━━
                        if app.library_items.is_empty() && !app.search_query.is_empty() {
                            lines.push(Line::from(Span::styled("No results found", Style::default().fg(muted))).alignment(Alignment::Center));
                            lines.push(Line::from(Span::styled("Try a different search", Style::default().fg(grid))).alignment(Alignment::Center));
                        } else if app.library_items.is_empty() {
                            lines.push(Line::from(Span::styled("Type to search your library", Style::default().fg(muted))).alignment(Alignment::Center));
                        } else {
                            let start_idx = app.library_selected.saturating_sub(content_h / 2).min(app.library_items.len().saturating_sub(content_h));
                            
                            for (display_idx, item) in app.library_items.iter().skip(start_idx).take(content_h).enumerate() {
                                let actual_idx = start_idx + display_idx;
                                let is_sel = actual_idx == app.library_selected;
                                
                                let name = if item.name.len() > title_w.saturating_sub(2) { 
                                    format!("{}…", &item.name[..title_w.saturating_sub(3)]) 
                                } else { 
                                    item.name.clone() 
                                };
                                let artist = item.artist.clone().unwrap_or_default();
                                let artist_disp = if artist.len() > artist_w.saturating_sub(1) { 
                                    format!("{}…", &artist[..artist_w.saturating_sub(2)]) 
                                } else { 
                                    artist 
                                };
                                let time = item.duration_ms.map(|ms| { 
                                    let s = ms / 1000; 
                                    format!("{}:{:02}", s / 60, s % 60) 
                                }).unwrap_or_default();
                                
                                let (marker, m_color, t_style, a_style, tm_style) = if is_sel {
                                    ("●", cream, Style::default().fg(theme.text).add_modifier(Modifier::BOLD), Style::default().fg(theme.text), Style::default().fg(green))
                                } else {
                                    ("○", grid, Style::default().fg(theme.text), Style::default().fg(muted), Style::default().fg(muted))
                                };
                                
                                // Only show ♪ for songs
                                let icon = match item.item_type {
                                    crate::app::LibraryItemType::Song => "♪",
                                    crate::app::LibraryItemType::Folder => "📁",
                                    crate::app::LibraryItemType::Playlist => "📜",
                                    _ => " "
                                };

                                lines.push(Line::from(vec![
                                    Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                                    Span::styled(format!("{} {:title_w$}", icon, name, title_w = title_w.saturating_sub(2)), t_style),
                                    Span::styled(format!("{:artist_w$}", artist_disp, artist_w = artist_w), a_style),
                                    Span::styled(format!("{:>time_w$}", time, time_w = time_w), tm_style),
                                ]));
                            }
                        }
                    }
                    LibraryMode::Playlists => {
                        // Unified aesthetic for Playlists
                        let content_h = h.saturating_sub(8);
                        let playlist_count = app.playlists.len();
                        
                        let magenta = theme.magenta;
                        let green = theme.green;
                        let cream = theme.yellow;
                        let muted = theme.overlay;
                        let grid = theme.surface;
                        
                        // ━━━ CENTERED TITLE ━━━
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            format!("  PLAYLISTS  ·  {} saved  ", playlist_count), 
                            Style::default().fg(magenta)
                        )).alignment(Alignment::Center));
                        lines.push(Line::from(""));
                        
                        // ━━━ CONTENT ━━━
                        if app.playlists.is_empty() {
                            lines.push(Line::from(Span::styled("No playlists", Style::default().fg(muted))).alignment(Alignment::Center));
                            lines.push(Line::from(Span::styled("Press 's' to save queue as playlist", Style::default().fg(grid))).alignment(Alignment::Center));
                        } else {
                            let start_idx = app.library_selected.saturating_sub(content_h / 2).min(app.playlists.len().saturating_sub(content_h));
                            
                            for (display_idx, pl) in app.playlists.iter().skip(start_idx).take(content_h).enumerate() {
                                let actual_idx = start_idx + display_idx;
                                let is_sel = actual_idx == app.library_selected;
                                let num = actual_idx + 1;
                                
                                let name_max = w.saturating_sub(12);
                                let name = if pl.len() > name_max { format!("{}…", &pl[..name_max.saturating_sub(1)]) } else { pl.clone() };
                                
                                let (marker, m_color, n_style) = if is_sel {
                                    ("●", cream, Style::default().fg(magenta).add_modifier(Modifier::BOLD))
                                } else {
                                    ("○", grid, Style::default().fg(theme.text))
                                };
                                let icon = "📜"; // Standard playlist icon
                                
                                lines.push(Line::from(vec![
                                    Span::styled(format!("  {} ", marker), Style::default().fg(m_color)),
                                    Span::styled(format!("{:>2}  ", num), Style::default().fg(if is_sel { green } else { muted })),
                                    Span::styled(format!("{} ", icon), Style::default().fg(magenta)),
                                    Span::styled(name, n_style),
                                ]));
                            }
                        }
                    }
                }
                
                let library_widget = Paragraph::new(lines)
                    .block(Block::default().style(Style::default().bg(Color::Reset)));
                f.render_widget(library_widget, inner_lyrics_area);
            },
            ViewMode::EQ => {
                // 🎛️ EQ Card - User's Design with Dotted Grid & Filled Curve
                let w = inner_lyrics_area.width as usize;
                let h = inner_lyrics_area.height as usize;
                
                if h < 14 || w < 40 {
                    let msg = Paragraph::new("♪ Resize for EQ")
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(theme.overlay));
                    f.render_widget(msg, inner_lyrics_area);
                } else {
                    let mut lines: Vec<Line> = Vec::new();
                    
                    // Color palette from theme
                    let green = theme.green;
                    let pink = theme.red;       // Catppuccin red = pink
                    let blue = theme.blue;
                    let lavender = theme.magenta;
                    let cream = theme.yellow;   // Use yellow as highlight/cream
                    let grid_dim = theme.surface;
                    let muted = theme.overlay;
                    // Dynamic label color based on EQ state
                    let label_color = if app.eq_enabled { green } else { muted };
                    
                    let freqs = ["32", "64", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
                    let bands = 10;
                    
                    // ━━━ TOP PADDING ━━━
                    lines.push(Line::from(""));
                    
                    // ━━━ BALANCE SLIDER ━━━
                    let slider_w = (w * 50 / 100).max(20);
                    let pad = (w.saturating_sub(slider_w + 6)) / 2;
                    let bal_pos = ((app.balance + 1.0) / 2.0 * (slider_w - 1) as f32) as usize;
                    let center_pos = slider_w / 2;
                    let show_center_marker = (bal_pos as i32 - center_pos as i32).abs() > 2;
                    
                    // Label colors based on balance direction - use actual value to avoid resize rounding issues
                    let is_panned_left = app.balance < -0.02;
                    let is_panned_right = app.balance > 0.02;
                    let l_color = if is_panned_left { green } else { muted };
                    let r_color = if is_panned_right { pink } else { muted };
                    let bal_label_color = if is_panned_left { green } else if is_panned_right { pink } else { muted };
                    
                    let mut bal: Vec<Span> = Vec::new();
                    bal.push(Span::raw(" ".repeat(pad)));
                    bal.push(Span::styled("L ", Style::default().fg(l_color)));
                    for i in 0..slider_w {
                        if i == bal_pos {
                            // Current position marker - colored based on actual value
                            let color = if is_panned_left { green } else if is_panned_right { pink } else { cream };
                            bal.push(Span::styled("○", Style::default().fg(color)));
                        } else if i == center_pos && show_center_marker {
                            // Center marker - only show when away from center
                            bal.push(Span::styled("│", Style::default().fg(muted)));
                        } else {
                            // Color dots between slider and center - use actual value
                            let is_left_fill = is_panned_left && i > bal_pos && i < center_pos;
                            let is_right_fill = is_panned_right && i < bal_pos && i > center_pos;
                            let dot_color = if is_left_fill { green } else if is_right_fill { pink } else { grid_dim };
                            bal.push(Span::styled("·", Style::default().fg(dot_color)));
                        }
                    }
                    bal.push(Span::styled(" R", Style::default().fg(r_color)));
                    lines.push(Line::from(bal));
                    lines.push(Line::from(Span::styled("BALANCE", Style::default().fg(bal_label_color))).alignment(Alignment::Center));
                    lines.push(Line::from(""));
                    
                    // ━━━ EQ GRAPH with High Resolution ━━━
                    // Scale graph height based on available space (7-25 rows)
                    let available_rows = h.saturating_sub(14); // Reserve space for other elements
                    // Smart scaling: compact for tmux (7-13), expanded for fullscreen (up to 25)
                    let max_graph_h = if h >= 40 { 25 } else { 13 };
                    let graph_h = available_rows.max(7).min(max_graph_h);
                    let label_w = 5;
                    let graph_w = w.saturating_sub(label_w + 1);
                    
                    // Band X positions
                    let band_x: Vec<usize> = (0..bands).map(|i| (graph_w * (i * 2 + 1)) / (bands * 2)).collect();
                    
                    // Calculate Y for each band with higher precision
                    // Convert eq value (0.0-1.0) to row position
                    // v=1.0 -> top (row 0, +12dB), v=0.5 -> center (0dB), v=0.0 -> bottom (-12dB)
                    let center_row = graph_h / 2;
                    
                    // Store precise Y values as floats first
                    let band_y_precise: Vec<f32> = app.eq_bands.iter().map(|&v| {
                        (1.0 - v) * (graph_h - 1) as f32
                    }).collect();
                    
                    // Interpolate curve Y for each column (with sub-row precision)
                    let mut curve_y_precise: Vec<f32> = vec![center_row as f32; graph_w];
                    for col in 0..graph_w {
                        let mut left_band = 0;
                        for i in 0..bands {
                            if col >= band_x[i] { left_band = i; }
                        }
                        let right_band = (left_band + 1).min(bands - 1);
                        
                        if col <= band_x[0] {
                            curve_y_precise[col] = band_y_precise[0];
                        } else if col >= band_x[bands - 1] {
                            curve_y_precise[col] = band_y_precise[bands - 1];
                        } else {
                            let x1 = band_x[left_band];
                            let x2 = band_x[right_band];
                            if x2 > x1 {
                                let t = (col - x1) as f32 / (x2 - x1) as f32;
                                let t = t * t * (3.0 - 2.0 * t); // smoothstep
                                curve_y_precise[col] = band_y_precise[left_band] * (1.0 - t) + band_y_precise[right_band] * t;
                            }
                        }
                    }
                    
                    // Generate dB labels based on graph height
                    let db_step = 24.0 / (graph_h - 1) as f32; // dB per row
                    
                    for row in 0..graph_h {
                        let mut spans: Vec<Span> = Vec::new();
                        
                        // Y-axis label
                        let db_val = 12.0 - (row as f32 * db_step);
                        let db_label = if db_val.abs() < 0.1 {
                            "0dB ".to_string()
                        } else if db_val > 0.0 {
                            format!("{:+.0} ", db_val)
                        } else {
                            format!("{:.0} ", db_val)
                        };
                        spans.push(Span::styled(format!("{:>4}", db_label), Style::default().fg(muted)));
                        
                        for col in 0..graph_w {
                            let cy = curve_y_precise[col];
                            let cy_row = cy.round() as usize;
                            let is_on_curve = row == cy_row;
                            let is_band_col = band_x.contains(&col);
                            let is_band_point = is_band_col && is_on_curve;
                            let is_center_row = row == center_row;
                            
                            // Fill regions - check if this row is between curve and center
                            let is_boost_fill = cy < center_row as f32 && (row as f32) > cy && row <= center_row;
                            let is_cut_fill = cy > center_row as f32 && (row as f32) < cy && row >= center_row;
                            
                            // Check if selected band
                            let band_idx = band_x.iter().position(|&x| x == col);
                            let is_selected = band_idx.map(|i| i == app.eq_selected).unwrap_or(false);
                            
                            if is_band_point {
                                // Circle marker at band points
                                // Use epsilon for floating point comparison at center
                                let at_or_above_center = (cy - center_row as f32) < 0.1;
                                let marker_col = if is_selected { cream } else { 
                                    if at_or_above_center { green } else { pink } 
                                };
                                spans.push(Span::styled("○", Style::default().fg(marker_col)));
                            } else if is_on_curve {
                                // Bold curve line - use bullet for thicker dots
                                let at_or_above_center = (cy - center_row as f32) < 0.1;
                                let curve_col = if at_or_above_center { green } else { pink };
                                spans.push(Span::styled("•", Style::default().fg(curve_col)));
                            } else if is_boost_fill {
                                // Solid fill for boost
                                spans.push(Span::styled("░", Style::default().fg(green)));
                            } else if is_cut_fill {
                                // Solid fill for cut
                                spans.push(Span::styled("░", Style::default().fg(pink)));
                            } else if is_band_col {
                                // Dotted vertical grid line at band positions
                                spans.push(Span::styled("┊", Style::default().fg(grid_dim)));
                            } else if is_center_row {
                                // Dashed horizontal 0dB line
                                spans.push(Span::styled("─", Style::default().fg(grid_dim)));
                            } else {
                                spans.push(Span::raw(" "));
                            }
                        }
                        lines.push(Line::from(spans));
                    }
                    
                    // Frequency labels
                    let mut freq_line: Vec<Span> = Vec::new();
                    freq_line.push(Span::raw(" ".repeat(label_w)));
                    let mut pos = 0;
                    for (i, freq) in freqs.iter().enumerate() {
                        let target = band_x[i];
                        while pos < target.saturating_sub(freq.len() / 2) && pos < graph_w {
                            freq_line.push(Span::raw(" "));
                            pos += 1;
                        }
                        let style = if i == app.eq_selected { Style::default().fg(cream).add_modifier(Modifier::BOLD) } else { Style::default().fg(muted) };
                        freq_line.push(Span::styled(*freq, style));
                        pos += freq.len();
                    }
                    lines.push(Line::from(freq_line));
                    
                    // ━━━ EQUALISER + PRESET ━━━
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled("EQUALISER", Style::default().fg(muted))).alignment(Alignment::Center));
                    let preset = format!("PRESET: {}", app.get_preset_name());
                    lines.push(Line::from(Span::styled(preset, Style::default().fg(if app.eq_enabled { green } else { muted }))).alignment(Alignment::Center));
                    
                    // ━━━ PREAMP SLIDER ━━━
                    lines.push(Line::from(""));
                    let pre_w = (w * 45 / 100).max(16);
                    let pre_pad = (w.saturating_sub(pre_w + 10)) / 2;
                    let pre_norm = (app.preamp_db + 12.0) / 24.0;
                    let pre_pos = (pre_norm * (pre_w - 1) as f32) as usize;
                    let pre_center = pre_w / 2;
                    let show_pre_center = (pre_pos as i32 - pre_center as i32).abs() > 2;
                    // Label colors based on preamp direction - use actual value to avoid resize rounding issues
                    // Slider layout: -12 (left) ----center---- +12 (right)
                    // Boosting = positive dB = slider moves RIGHT towards +12 = green
                    // Cutting = negative dB = slider moves LEFT towards -12 = pink
                    let is_boosting = app.preamp_db > 0.5;
                    let is_cutting = app.preamp_db < -0.5;
                    let left_label_color = if is_cutting { pink } else { muted };   // -12 label
                    let right_label_color = if is_boosting { green } else { muted }; // +12 label
                    let pre_label_color = if is_boosting { green } else if is_cutting { pink } else { muted };
                    let mut pre: Vec<Span> = Vec::new();
                    pre.push(Span::raw(" ".repeat(pre_pad)));
                    pre.push(Span::styled("-12 ", Style::default().fg(left_label_color)));
                    for i in 0..pre_w {
                        if i == pre_pos {
                            // Current position marker - colored based on actual value
                            let color = if is_boosting { green } else if is_cutting { pink } else { cream };
                            pre.push(Span::styled("○", Style::default().fg(color)));
                        } else if i == pre_center && show_pre_center {
                            // Center marker (0dB) - only show when away from center
                            pre.push(Span::styled("│", Style::default().fg(muted)));
                        } else {
                            // Color dots between slider and center - use actual value
                            let is_right_fill = is_boosting && i < pre_pos && i > pre_center;
                            let is_left_fill = is_cutting && i > pre_pos && i < pre_center;
                            let dot_color = if is_right_fill { green } else if is_left_fill { pink } else { grid_dim };
                            pre.push(Span::styled("·", Style::default().fg(dot_color)));
                        }
                    }
                    pre.push(Span::styled(" +12", Style::default().fg(right_label_color)));
                    lines.push(Line::from(pre));
                    lines.push(Line::from(Span::styled("PREAMP", Style::default().fg(pre_label_color))).alignment(Alignment::Center));
                    
                    // ━━━ CROSSFADE (own line) ━━━
                    lines.push(Line::from(""));
                    let xf_opts = ["Off", "2s", "4s", "6s"];
                    let xf_sel = match app.crossfade_secs { 2 => 1, 4 => 2, 6 => 3, _ => 0 };
                    
                    let mut xf_line: Vec<Span> = Vec::new();
                    xf_line.push(Span::styled("CROSSFADE:  ", Style::default().fg(muted)));
                    for (i, o) in xf_opts.iter().enumerate() {
                        let s = if i == xf_sel { Style::default().fg(green) } else { Style::default().fg(grid_dim) };
                        xf_line.push(Span::styled(*o, s));
                        xf_line.push(Span::raw("  "));
                    }
                    lines.push(Line::from(xf_line).alignment(Alignment::Center));
                    
                    // ━━━ REPLAYGAIN (own line) ━━━
                    let rg_opts = ["Off", "Track", "Album", "Auto"];
                    let rg_sel = app.replay_gain_mode as usize;
                    
                    let mut rg_line: Vec<Span> = Vec::new();
                    rg_line.push(Span::styled("REPLAYGAIN:  ", Style::default().fg(muted)));
                    for (i, o) in rg_opts.iter().enumerate() {
                        let s = if i == rg_sel { Style::default().fg(green) } else { Style::default().fg(grid_dim) };
                        rg_line.push(Span::styled(*o, s));
                        rg_line.push(Span::raw("  "));
                    }
                    lines.push(Line::from(rg_line).alignment(Alignment::Center));
                    
                    // ━━━ DEVICE PILL ━━━
                    lines.push(Line::from(""));
                    let status = if app.dsp_available {
                        if app.eq_enabled { ("● ON", green) } else { ("○ OFF", muted) }
                    } else { ("⚠ N/A", pink) };
                    
                    lines.push(Line::from(vec![
                        Span::styled(format!(" {} ", app.output_device), Style::default().fg(theme.base).bg(green).add_modifier(ratatui::style::Modifier::BOLD)),
                        Span::raw("  "),
                        Span::styled(status.0, Style::default().fg(status.1)),
                    ]).alignment(Alignment::Center));
                    
                    let widget = Paragraph::new(lines).block(Block::default().style(Style::default().bg(Color::Reset)));
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
        
        // Shuffle/Repeat only shown in MPD mode (not available in controller)
        if app.is_mpd {
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
        }
        
        let gapless_str = if app.gapless_mode { "Active" } else { "OFF" };
        lines.push(Line::from(vec![
            Span::styled("  Gapless: ", Style::default().fg(theme.overlay)),
            Span::styled(gapless_str, Style::default().fg(if app.gapless_mode { theme.blue } else { theme.overlay })),
        ]));
        
        lines.push(Line::from(""));
        
        // Mode-specific section
        if app.is_mpd {
            // DSP/EQ Section (MPD mode only)
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
        } else {
            // Streaming Source Section (Controller mode)
            lines.push(Line::from(vec![
                Span::styled("📡 ", Style::default().fg(theme.blue)),
                Span::styled("Source", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
            ]));
            
            lines.push(Line::from(vec![
                Span::styled("  Streaming: ", Style::default().fg(theme.overlay)),
                Span::styled(&app.source_app, Style::default().fg(theme.green)),
            ]));
            
            lines.push(Line::from(vec![
                Span::styled("  Mode: ", Style::default().fg(theme.overlay)),
                Span::styled("Controller", Style::default().fg(theme.magenta)),
            ]));
        }
        
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
    if let Some(ref toast) = app.toast {
        use ratatui::widgets::{Clear, Paragraph};
        
        let now = std::time::Instant::now();
        
        // Auto-dismiss if deadline passed
        if now > toast.deadline {
            app.toast = None;
        } else {
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
                        .border_type(ratatui::widgets::BorderType::Rounded)
                        .border_style(Style::default().fg(theme.green))
                        .style(Style::default().bg(theme.base));
                    
                    let style = Style::default().fg(theme.green).add_modifier(Modifier::BOLD);
                    
                    let text = Paragraph::new(Line::from(vec![
                        Span::styled("✓ ", style),
                        Span::styled(message.as_str(), style),
                    ]))
                    .alignment(Alignment::Center)
                    .block(block);
                    
                    f.render_widget(text, visible_area);
                }
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
                ("h/l", "🎚️", "Select band"),
                ("k/j", "📊", "Adjust gain"),
                ("e", "⚡", "Toggle EQ"),
                ("0", "↺", "Reset band"),
                ("g/G", "🔊", "Preamp ±1dB"),
                ("b/B", "⚖️", "Balance ±0.1"),
                ("c", "🔀", "Crossfade"),
                ("R", "📀", "ReplayGain"),
                ("d/D", "🎧", "Output device"),
                ("S", "💾", "Save preset"),
                ("X", "🗑️", "Delete preset"),
            ]),
            ViewMode::Library => ("Library", vec![
                ("j/k", "📋", "Navigate"),
                ("Tab", "🔄", "Switch mode"),
                ("l/Ent", "▶️", "Select/Play"),
                ("h/Bksp", "←", "Go back"),
                ("/", "🔍", "Search"),
                ("a", "➕", "Add to Queue"),
                ("s", "💾", "Save playlist"),
                ("d", "🗑️", "Delete/Remove"),
                ("t", "🏷️", "Edit tags"),
                ("J/K", "🔃", "Reorder"),
            ]),
            ViewMode::Lyrics => ("Lyrics", vec![
                ("j/k", "📜", "Scroll lyrics"),
                ("Enter", "🎤", "Jump to line"),
            ]),
            ViewMode::Cava => ("Visualizer", vec![]),
        };
        
        // Global keys - mode-specific
        let global_keys: Vec<(&str, &str, &str)> = if app.is_mpd {
            // MPD mode: full feature set
            vec![
                ("Space", "▶️", "Play/Pause"),
                ("n", "⏭️", "Next track"),
                ("p", "⏮️", "Previous track"),
                ("z", "🔀", "Shuffle"),
                ("x", "🔁", "Repeat"),
                ("/", "🔍", "Search"),
                ("+/-", "🔊", "Volume"),
                ("z", "🔀", "Shuffle"),
                ("x", "🔁", "Repeat"),
                ("1-4", "🖼️", "View modes"),
                ("h/l", "⏩", "Seek ±5s"),
                ("i", "ℹ️", "Audio info"),
                ("q", "🚪", "Quit"),
            ]
        } else {
            // Controller mode: limited keys (no shuffle/repeat - not available)
            vec![
                ("Space", "▶️", "Play/Pause"),
                ("n", "⏭️", "Next track"),
                ("p", "⏮️", "Previous track"),
                ("+/-", "🔊", "Volume"),
                ("h/l", "⏩", "Seek ±5s"),
                ("i", "ℹ️", "Audio info"),
                ("q", "🚪", "Quit"),
            ]
        };
        
        // Calculate popup size - fit content exactly (no extra space)
        // Calculate popup size - fit content exactly (no extra space)
        // Calculate popup size - fit content exactly (no extra space)
        let mut total_items = keys.len() + global_keys.len() + 4; // +4 for title, empty, global header, empty
        if !keys.is_empty() { total_items += 1; } // +1 for separator between sections
        // Use 80% of screen height as max, or at least fit content if possible
        let max_height = (f.area().height as u16).saturating_sub(4); 
        let popup_height = (total_items as u16 + 2).min(max_height); // +2 for borders
        let popup_width = 32u16.min(f.area().width.saturating_sub(2)); // Clamp to window width
        
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
                Span::styled(format!(" {:>7} ", key), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                Span::styled("→ ", Style::default().fg(theme.overlay)),
                Span::styled(format!("{} ", icon), Style::default()),
                Span::styled(*desc, Style::default().fg(theme.text)),
            ]));
        }
        
        if !keys.is_empty() {
            lines.push(Line::from(""));
        }
        
        // Global section
        let global_title = "Global";
        let global_pad = (popup_width as usize - global_title.len() - 4) / 2;
        lines.push(Line::from(Span::styled(
            format!("{:>pad$}{}", "", global_title, pad = global_pad),
            Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)
        )));
        lines.push(Line::from(""));
        
        for (key, icon, desc) in &global_keys {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>7} ", key), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
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
                .border_style(Style::default().fg(theme.blue))
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
        let new_start = selected.saturating_sub(content_h / 2).min(list_len.saturating_sub(content_h)); 
        // term1 = 38
        // term2 = 50 - 22 = 28
        // min(38, 28) = 28
        
        let new_visible_count = list_len - new_start; // 50 - 28 = 22 items
        
        println!("Content Height Capacity: {}", content_h);
        println!("Old Logic: Start {}, Items Visible: {} ({} empty spaces)", old_start, old_visible_count, content_h - old_visible_count);
        println!("New Logic: Start {}, Items Visible: {} ({} empty spaces)", new_start, new_visible_count, content_h - new_visible_count);
        
        // Assertions
        assert_eq!(new_visible_count, content_h, "List should be fully filled");
        assert!(new_start <= selected, "Start index must be before or equal to selected");
        assert!(new_start + content_h > selected, "Selected item must be within view");
    }
}
