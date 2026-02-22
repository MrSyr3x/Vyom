use crate::app::{App, ArtStyle, ArtworkState};
use image::{imageops::FilterType, GenericImageView};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    if area.height < 1 || area.width < 1 {
        return;
    }

    match &app.artwork {
        ArtworkState::Loaded(raw_image) => {
            if app.art_style == ArtStyle::Off {
                 return;
            }

            if app.art_style == ArtStyle::Image {
                if app.image_protocol.is_none() {
                    let picker = app.image_picker.clone();
                    app.image_protocol = Some(picker.new_resize_protocol(raw_image.clone()));
                }

                if let Some(protocol) = &mut app.image_protocol {
                    // Center the image within the available area
                    // Use Scale instead of Fit to maximize the area properly 
                    // like the Block art style does.
                    // IMPORTANT: Override default `Nearest` neighbor scaling with `Triangle` (Bilinear).
                    // This provides 80% of the sharpness of Lanczos3 but is literally 10x faster,
                    // completely eliminating the UI rendering latency/lag when the image resizes.
                    let resize = ratatui_image::Resize::Scale(Some(FilterType::Triangle));
                    // size_for calculates the exact rect in cells that the image needs
                    let img_size = protocol.size_for(resize.clone(), area);
                    
                    let centered_x = area.x + area.width.saturating_sub(img_size.width) / 2;
                    let centered_y = area.y + area.height.saturating_sub(img_size.height) / 2;
                    let centered_area = ratatui::layout::Rect::new(
                        centered_x, 
                        centered_y, 
                        img_size.width, 
                        img_size.height
                    );

                    let image = ratatui_image::StatefulImage::default().resize(resize);
                    f.render_stateful_widget(image, centered_area, protocol);
                }
                return;
            }

            let lines = match app.art_style {
                ArtStyle::Block => render_block(raw_image, area),
                ArtStyle::Ascii => render_ascii(raw_image, area),
                ArtStyle::Braille => render_braille(raw_image, area),
                _ => vec![], // Image and Off handled above
            };

            let artwork_widget = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().bg(Color::Reset)));
            f.render_widget(artwork_widget, area);
        }
        ArtworkState::Loading => {
            let p = Paragraph::new("\n\n\n\n\n        Loading...".to_string())
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(app.theme.yellow).bg(Color::Reset)));
            f.render_widget(p, area);
        }
        ArtworkState::Failed | ArtworkState::Idle => {
            // ...
            let text = "\n\n\n\n\n        ♪\n    No Album\n      Art".to_string();
            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(Block::default().style(Style::default().fg(app.theme.overlay).bg(Color::Reset)));
            f.render_widget(p, area);
        }
    }
}

fn render_block(raw_image: &image::DynamicImage, area: Rect) -> Vec<Line<'static>> {
    let available_width = area.width as u32;
    let available_height = area.height as u32;
    let target_width = available_width;
    let target_height = available_height * 2;

    if target_width == 0 || target_height == 0 {
        return vec![];
    }

    let resized = raw_image.resize(target_width, target_height, FilterType::Triangle);
    let img_height_subpixels = resized.height();
    let img_rows = img_height_subpixels.div_ceil(2);
    let padding_top = available_height.saturating_sub(img_rows) / 2;

    let mut lines = Vec::new();
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

            // Simple alpha blending check: if mostly transparent, use default bg?
            // For now, assume opaque.

            spans.push(Span::styled(
                "▀",
                Style::default()
                    .fg(Color::Rgb(fg.0, fg.1, fg.2))
                    .bg(Color::Rgb(bg.0, bg.1, bg.2)),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}


fn render_ascii(raw_image: &image::DynamicImage, area: Rect) -> Vec<Line<'static>> {
    let available_width = area.width as u32;
    let available_height = area.height as u32;

    let target_width = available_width;
    let target_height = available_height; // One char per cell

    if target_width == 0 || target_height == 0 {
        return vec![];
    }

    let (src_w, src_h) = raw_image.dimensions();
    let src_ar = src_w as f32 / src_h as f32;
    
    // We are bounded by area.width and area.height.
    // ASCII chars are tall (approx 1:2 aspect ratio).
    // To preserve specific image aspect ratio W/H:
    // We need (cols * 1) / (rows * 2) = W/H
    // rows = cols / (2 * W/H)
    
    let mut final_w = available_width;
    let mut final_h = (available_width as f32 / src_ar / 2.0) as u32;

    if final_h > available_height {
        final_h = available_height;
        final_w = (available_height as f32 * 2.0 * src_ar) as u32;
    }
    
    let resized = raw_image.resize_exact(final_w, final_h, FilterType::Triangle);
    
    // Center vertically
    let padding_top = available_height.saturating_sub(final_h) / 2;
    let mut lines = Vec::new();
    for _ in 0..padding_top {
        lines.push(Line::default());
    }

    // "Standard" ASCII density ramp (Dark -> Light)
    //  .:-=+*#%@
    // Restore space for transparency
    let scale = b" .:-=+*#%@";
    
    for y in 0..resized.height() {
        let mut spans = Vec::new();
        for x in 0..resized.width() {
            let p = resized.get_pixel(x, y);
            let lumi = (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) as u8;
            
            let len = scale.len();
            let idx = (lumi as f32 / 255.0 * (len - 1) as f32) as usize;
            let char_idx = idx.clamp(0, len - 1);
            let c = scale[char_idx] as char;
            
            spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(Color::Rgb(p[0], p[1], p[2])),
            ));
        }
        lines.push(Line::from(spans));
    }
    
    lines
}

fn render_braille(raw_image: &image::DynamicImage, area: Rect) -> Vec<Line<'static>> {
    let available_width = area.width as u32;
    let available_height = area.height as u32;

    let target_w = available_width * 2;
    let target_h = available_height * 4;
    
    let (src_w, src_h) = raw_image.dimensions();
    let src_ar = src_w as f32 / src_h as f32;
    
    let mut final_dot_w = target_w;
    let mut final_dot_h = (target_w as f32 / src_ar) as u32;

    if final_dot_h > target_h {
        final_dot_h = target_h;
        final_dot_w = (target_h as f32 * src_ar) as u32;
    }
    
    // 1. Resize for dot resolution
    let resized = raw_image.resize_exact(final_dot_w, final_dot_h, FilterType::Triangle);
    
    // 2. Apply Floyd-Steinberg Dithering to get binary (on/off) state matrix 🔳
    let dithered_width = resized.width() as usize;
    let dithered_height = resized.height() as usize;
    let dithered = floyd_steinberg_dithering(&resized);

    // Padding in CELLS
    let used_cells_h = final_dot_h.div_ceil(4);
    let padding_top = available_height.saturating_sub(used_cells_h) / 2;
    
    let mut lines = Vec::new();
    for _ in 0..padding_top {
        lines.push(Line::default());
    }

    // Process 2x4 blocks
    for y in (0..dithered_height).step_by(4) {
        let mut spans = Vec::new();
        
        for x in (0..dithered_width).step_by(2) {
            let mut mask: u32 = 0;
            
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut count = 0;
            
            // Standard Unicode Braille ordering
            let dots = [
                (0,0,0x1), (0,1,0x2), (0,2,0x4), (0,3,0x40),
                (1,0,0x8), (1,1,0x10), (1,2,0x20), (1,3,0x80)
            ];
            
            for &(dx, dy, bit) in &dots {
                if x + dx < dithered_width && y + dy < dithered_height {
                    // Use dithered buffer for shape
                    if dithered[y + dy][x + dx] {
                        mask |= bit;
                        
                        // Use original image for color (averaging)
                        let p = resized.get_pixel((x + dx) as u32, (y + dy) as u32);
                        r_sum += p[0] as u32;
                        g_sum += p[1] as u32;
                        b_sum += p[2] as u32;
                        count += 1;
                    }
                }
            }
            
            let ch = char::from_u32(0x2800 + mask).unwrap_or(' ');
            
            let (r, g, b) = if count > 0 {
                ((r_sum / count) as u8, (g_sum / count) as u8, (b_sum / count) as u8)
            } else {
                 (255, 255, 255) 
            };

            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(Color::Rgb(r, g, b)),
            ));
        }
        lines.push(Line::from(spans));
    }
    
    lines
}

/// Floyd-Steinberg Dithering Implementation 🎨
/// Converts a color image to a boolean matrix (true = on/white, false = off/black)
fn floyd_steinberg_dithering(img: &image::DynamicImage) -> Vec<Vec<bool>> {
    let w = img.width() as usize;
    let h = img.height() as usize;
    
    // Create a buffer of luminance values (float for error diffusion)
    let mut buffer: Vec<Vec<f32>> = vec![vec![0.0; w]; h];
    
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x as u32, y as u32);
            // Standard Luminance
            buffer[y][x] = 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
        }
    }
    
    let mut output = vec![vec![false; w]; h];
    
    for y in 0..h {
        for x in 0..w {
            let old_pixel = buffer[y][x];
            let new_pixel = if old_pixel > 127.0 { 255.0 } else { 0.0 };
            
            output[y][x] = new_pixel > 127.0;
            
            let quant_error = old_pixel - new_pixel;
            
            // Distribute error to neighbors
            if x + 1 < w {
                buffer[y][x + 1] += quant_error * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    buffer[y + 1][x - 1] += quant_error * 3.0 / 16.0;
                }
                buffer[y + 1][x] += quant_error * 5.0 / 16.0;
                if x + 1 < w {
                    buffer[y + 1][x + 1] += quant_error * 1.0 / 16.0;
                }
            }
        }
    }
    
    output
}
