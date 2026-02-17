use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders},
    style::Modifier,
    layout::Alignment,
    style::Color,
    Frame,
};

pub mod art;
pub mod info;
pub mod progress;
pub mod controls;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    // --- MUSIC CARD ---
    let music_title = Title::from(Line::from(vec![Span::styled(
        " Now Playing ",
        Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
    )]));

    let music_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(music_title)
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(theme.blue))
        .style(Style::default().bg(Color::Reset));

    let inner_music_area = music_block.inner(area);
    f.render_widget(music_block, area);

    // Inner Music Layout
    let m_height = inner_music_area.height;

    // Calculate Info Height (4 if badges, 3 if not)
    let info_height = if let Some(track) = &app.track {
        if track.codec.is_some() || track.sample_rate.is_some() {
            4
        } else {
            3
        }
    } else {
        4
    };

    let mut music_constraints = Vec::new();

    // Extremely small height (< 10): Show only essentials
    if m_height < 10 {
        // Tiny Mode: Artwork 0, Info 1, Controls 1
        music_constraints.push(Constraint::Min(0)); // 0: Artwork (Hidden)
        music_constraints.push(Constraint::Length(m_height.saturating_sub(2).max(1))); // 1: Info (Takes remaining)
        music_constraints.push(Constraint::Length(0)); // 2: Gauge (Hidden)
        music_constraints.push(Constraint::Length(0)); // 3: Time (Hidden)
        music_constraints.push(Constraint::Length(0)); // 4: Spacer 1 (Hidden)
        music_constraints.push(Constraint::Length(1)); // 5: Controls
    } else {
        // Normal Mode: Artwork takes ALL available space
        music_constraints.push(Constraint::Min(0)); // 0: Artwork (Elastic!)
        music_constraints.push(Constraint::Length(info_height)); // 1: Info (Dynamic)
        music_constraints.push(Constraint::Length(1)); // 2: Spacer 1
        music_constraints.push(Constraint::Length(1)); // 3: Gauge
        music_constraints.push(Constraint::Length(1)); // 4: Time
                                                       // Removed Spacer 2 to tighten layout
        music_constraints.push(Constraint::Length(3)); // 5: Controls
    }

    let music_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(music_constraints)
        .split(inner_music_area);

    // 1. Artwork
    let artwork_area = if !music_chunks.is_empty() && music_chunks[0].height > 1 {
        // Only render art if we have at least 2 lines
        let area = music_chunks[0];
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0), // Art
            ])
            .split(area)[0]
    } else {
        Rect::default()
    };

    art::render(f, artwork_area, app);

    // 2. Info
    let info_idx = 1;
    if info_idx < music_chunks.len() {
        info::render(f, music_chunks[info_idx], app);
    }

    // 3. Gauge
    // Cramped mode logic reused from chunks layout
    let gauge_idx = if m_height < 10 { 2 } else { 3 };
    if gauge_idx < music_chunks.len() && music_chunks[gauge_idx].height > 0 {
        progress::render_progress(f, music_chunks[gauge_idx], app);
    }

    // 4. Time
    let time_idx = if m_height < 10 { 3 } else { 4 };
    if time_idx < music_chunks.len() && music_chunks[time_idx].height > 0 {
        progress::render_time(f, music_chunks[time_idx], app);
    }

    // 5. Controls
    let controls_idx = 5;
    if controls_idx < music_chunks.len() && music_chunks[controls_idx].height > 0 {
        controls::render(f, music_chunks[controls_idx], app);
    }
}
