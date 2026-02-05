use crate::app::App;
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
    layout::Rect,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // 🌊 Premium Cava Spectrum Visualizer
    let vis_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(" Visualizer ", Style::default().fg(theme.cyan).add_modifier(ratatui::style::Modifier::BOLD)))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(Color::Reset));

    let inner_area = vis_block.inner(area);
    f.render_widget(vis_block, area);

    let width = inner_area.width as usize;
    let height = inner_area.height as usize;

    if height < 4 || width < 10 {
        let msg = Paragraph::new("♪ Resize for visualizer")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.overlay));
        f.render_widget(msg, inner_area);
    } else {
        // Use single-char bars for cleaner look
        // AUTO-SCALE: Use as many bars as fit (width/2), capped only by practical limits (256)
        let bar_count = (width / 2).clamp(8, 256);

        // 8-color gradient using Theme Colors 🎨
        let gradient = [
            theme.red,
            theme.yellow,
            theme.green,
            theme.cyan,
            theme.blue,
            theme.magenta,
            theme.red,
            theme.text,
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
                // RESAMPLING LOGIC: Map UI Bar (i) to Data Range (start..end)
                // This ensures we show the FULL spectrum regardless of screen width.
                // "No Cutoffs" guarantee.
                let source_len = app.visualizer_bars.len();
                let start_idx = (i * source_len) / bar_count;
                let end_idx =
                    ((i + 1) * source_len).div_ceil(bar_count).min(source_len);
                // Ensure we have at least one index
                let start_idx = start_idx.min(source_len.saturating_sub(1));
                let end_idx = end_idx.max(start_idx + 1);

                // Aggregation: Use MAX to preserve peaks (don't lose the beat)
                let mut bar_height = 0.0f32;
                for j in start_idx..end_idx {
                    let val = app.visualizer_bars.get(j).copied().unwrap_or(0.0);
                    if val > bar_height {
                        bar_height = val;
                    }
                }

                // Map bar position to gradient color
                let color_idx =
                    (i * gradient.len() / bar_count).min(gradient.len() - 1);
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
                let bar_height =
                    app.visualizer_bars.get(bar_idx).copied().unwrap_or(0.3);

                // Dimmed gradient for reflection
                let color_idx =
                    (i * gradient.len() / bar_count).min(gradient.len() - 1);
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
        f.render_widget(visualizer, inner_area);
    }
}
