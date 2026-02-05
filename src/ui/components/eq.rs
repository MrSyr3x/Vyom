use crate::app::App;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
    layout::Rect,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // 🎛️ EQ Card
    let eq_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(Span::styled(" Equalizer ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)))
        .border_style(Style::default().fg(theme.green))
        .style(Style::default().bg(Color::Reset));

    let inner_area = eq_block.inner(area);
    f.render_widget(eq_block, area);

    let w = inner_area.width as usize;
    let h = inner_area.height as usize;

    if h < 14 || w < 40 {
        let msg = Paragraph::new("♪ Resize for EQ")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.overlay));
        f.render_widget(msg, inner_area);
    } else {
        let mut lines: Vec<Line> = Vec::new();

        // Color palette from theme
        let green = theme.green;
        let pink = theme.red; // Catppuccin red = pink
        let _blue = theme.blue;
        let _lavender = theme.magenta;
        let cream = theme.yellow; // Use yellow as highlight/cream
        let grid_dim = theme.surface;
        let muted = theme.overlay;
        // Dynamic label color based on EQ state
        let _label_color = if app.eq_enabled { green } else { muted };

        let freqs = [
            "32", "64", "125", "250", "500", "1K", "2K", "4K", "8K", "16K",
        ];
        let bands = 10;

        // ━━━ TOP PADDING ━━━
        lines.push(Line::from(""));

        // ━━━ BALANCE SLIDER ━━━
        let mut slider_w = (w * 50 / 100).max(21);
        if slider_w.is_multiple_of(2) {
            slider_w += 1;
        } // Force odd width for perfect center
        let pad = (w.saturating_sub(slider_w + 6)) / 2;
        let bal_pos =
            ((app.balance + 1.0) / 2.0 * (slider_w - 1) as f32).round() as usize;
        let center_pos = slider_w / 2;
        let show_center_marker = bal_pos != center_pos;

        // Label colors based on balance direction - use actual value to avoid resize rounding issues
        let is_panned_left = app.balance < -0.02;
        let is_panned_right = app.balance > 0.02;
        let l_color = if is_panned_left { green } else { muted };
        let r_color = if is_panned_right { pink } else { muted };
        let bal_label_color = if is_panned_left {
            green
        } else if is_panned_right {
            pink
        } else {
            muted
        };

        let mut bal: Vec<Span> = Vec::new();
        bal.push(Span::raw(" ".repeat(pad)));
        bal.push(Span::styled("L ", Style::default().fg(l_color)));
        for i in 0..slider_w {
            if i == bal_pos {
                // Current position marker - colored based on actual value
                let color = if is_panned_left {
                    green
                } else if is_panned_right {
                    pink
                } else {
                    cream
                };
                bal.push(Span::styled("○", Style::default().fg(color)));
            } else if i == center_pos && show_center_marker {
                // Center marker - only show when away from center
                bal.push(Span::styled("│", Style::default().fg(muted)));
            } else {
                // Color dots between slider and center - use actual value
                let is_left_fill = is_panned_left && i > bal_pos && i < center_pos;
                let is_right_fill = is_panned_right && i < bal_pos && i > center_pos;
                let dot_color = if is_left_fill {
                    green
                } else if is_right_fill {
                    pink
                } else {
                    grid_dim
                };
                bal.push(Span::styled("·", Style::default().fg(dot_color)));
            }
        }
        bal.push(Span::styled(" R", Style::default().fg(r_color)));
        lines.push(Line::from(bal));
        lines.push(
            Line::from(Span::styled(
                "BALANCE",
                Style::default().fg(bal_label_color),
            ))
            .alignment(Alignment::Center),
        );
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
        let band_x: Vec<usize> = (0..bands)
            .map(|i| (graph_w * (i * 2 + 1)) / (bands * 2))
            .collect();

        // Calculate Y for each band with higher precision
        // Convert eq value (0.0-1.0) to row position
        // v=1.0 -> top (row 0, +12dB), v=0.5 -> center (0dB), v=0.0 -> bottom (-12dB)
        let center_row = graph_h / 2;

        // Store precise Y values as floats first
        let band_y_precise: Vec<f32> = app
            .eq_bands
            .iter()
            .map(|&v| (1.0 - v) * (graph_h - 1) as f32)
            .collect();

        // Interpolate curve Y for each column (with sub-row precision)
        let mut curve_y_precise: Vec<f32> = vec![center_row as f32; graph_w];
        for (col, val) in curve_y_precise.iter_mut().enumerate().take(graph_w) {
            let mut left_band = 0;
            for (i, &band_pos) in band_x.iter().enumerate().take(bands) {
                if col >= band_pos {
                    left_band = i;
                }
            }
            let right_band = (left_band + 1).min(bands - 1);

            if col <= band_x[0] {
                *val = band_y_precise[0];
            } else if col >= band_x[bands - 1] {
                *val = band_y_precise[bands - 1];
            } else {
                let x1 = band_x[left_band];
                let x2 = band_x[right_band];
                if x2 > x1 {
                    let t = (col - x1) as f32 / (x2 - x1) as f32;
                    let t = t * t * (3.0 - 2.0 * t); // smoothstep
                    *val = band_y_precise[left_band] * (1.0 - t)
                        + band_y_precise[right_band] * t;
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
            spans.push(Span::styled(
                format!("{:>4}", db_label),
                Style::default().fg(muted),
            ));

            for (col, &cy) in curve_y_precise.iter().enumerate().take(graph_w) {
                let cy_row = cy.round() as usize;
                let is_on_curve = row == cy_row;
                let is_band_col = band_x.contains(&col);
                let is_band_point = is_band_col && is_on_curve;
                let is_center_row = row == center_row;

                // Fill regions - check if this row is between curve and center
                let is_boost_fill =
                    cy < center_row as f32 && (row as f32) > cy && row <= center_row;
                let is_cut_fill =
                    cy > center_row as f32 && (row as f32) < cy && row >= center_row;

                // Check if selected band
                let band_idx = band_x.iter().position(|&x| x == col);
                let is_selected =
                    band_idx.map(|i| i == app.eq_selected).unwrap_or(false);

                if is_band_point {
                    // Circle marker at band points
                    // Use epsilon for floating point comparison at center
                    let at_or_above_center = (cy - center_row as f32) < 0.1;
                    let marker_col = if is_selected {
                        cream
                    } else if at_or_above_center {
                        green
                    } else {
                        pink
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
            let style = if i == app.eq_selected {
                Style::default().fg(cream).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(muted)
            };
            freq_line.push(Span::styled(*freq, style));
            pos += freq.len();
        }
        lines.push(Line::from(freq_line));

        // ━━━ EQUALISER + PRESET ━━━
        lines.push(Line::from(""));
        lines.push(
            Line::from(Span::styled("EQUALISER", Style::default().fg(muted)))
                .alignment(Alignment::Center),
        );
        let preset = format!("PRESET: {}", app.get_preset_name());
        lines.push(
            Line::from(Span::styled(
                preset,
                Style::default().fg(if app.eq_enabled { green } else { muted }),
            ))
            .alignment(Alignment::Center),
        );

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
        let left_label_color = if is_cutting { pink } else { muted }; // -12 label
        let right_label_color = if is_boosting { green } else { muted }; // +12 label
        let pre_label_color = if is_boosting {
            green
        } else if is_cutting {
            pink
        } else {
            muted
        };
        let mut pre: Vec<Span> = Vec::new();
        pre.push(Span::raw(" ".repeat(pre_pad)));
        pre.push(Span::styled("-12 ", Style::default().fg(left_label_color)));
        for i in 0..pre_w {
            if i == pre_pos {
                // Current position marker - colored based on actual value
                let color = if is_boosting {
                    green
                } else if is_cutting {
                    pink
                } else {
                    cream
                };
                pre.push(Span::styled("○", Style::default().fg(color)));
            } else if i == pre_center && show_pre_center {
                // Center marker (0dB) - only show when away from center
                pre.push(Span::styled("│", Style::default().fg(muted)));
            } else {
                // Color dots between slider and center - use actual value
                let is_right_fill = is_boosting && i < pre_pos && i > pre_center;
                let is_left_fill = is_cutting && i > pre_pos && i < pre_center;
                let dot_color = if is_right_fill {
                    green
                } else if is_left_fill {
                    pink
                } else {
                    grid_dim
                };
                pre.push(Span::styled("·", Style::default().fg(dot_color)));
            }
        }
        pre.push(Span::styled(" +12", Style::default().fg(right_label_color)));
        lines.push(Line::from(pre));
        lines.push(
            Line::from(Span::styled("PREAMP", Style::default().fg(pre_label_color)))
                .alignment(Alignment::Center),
        );

        // ━━━ CROSSFADE (own line) ━━━
        lines.push(Line::from(""));
        let xf_opts = ["Off", "2s", "4s", "6s"];
        let xf_sel = match app.crossfade_secs {
            2 => 1,
            4 => 2,
            6 => 3,
            _ => 0,
        };

        let mut xf_line: Vec<Span> = Vec::new();
        xf_line.push(Span::styled("CROSSFADE:  ", Style::default().fg(muted)));
        for (i, o) in xf_opts.iter().enumerate() {
            let s = if i == xf_sel {
                Style::default().fg(green)
            } else {
                Style::default().fg(grid_dim)
            };
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
            let s = if i == rg_sel {
                Style::default().fg(green)
            } else {
                Style::default().fg(grid_dim)
            };
            rg_line.push(Span::styled(*o, s));
            rg_line.push(Span::raw("  "));
        }
        lines.push(Line::from(rg_line).alignment(Alignment::Center));

        // ━━━ DEVICE PILL ━━━
        lines.push(Line::from(""));
        let status = if app.dsp_available {
            if app.eq_enabled {
                ("● ON", green)
            } else {
                ("○ OFF", muted)
            }
        } else {
            ("⚠ N/A", pink)
        };

        lines.push(
            Line::from(vec![
                Span::styled(
                    format!(" {} ", app.output_device),
                    Style::default()
                        .fg(theme.base)
                        .bg(green)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(status.0, Style::default().fg(status.1)),
            ])
            .alignment(Alignment::Center),
        );

        let widget = Paragraph::new(lines)
            .block(Block::default().style(Style::default().bg(Color::Reset)));
        f.render_widget(widget, inner_area);
    }
}
