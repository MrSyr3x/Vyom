use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ArtStyle {
    #[default]
    Block,   // Half-block truecolor
    Ascii,   // Character based on luminance
    Braille, // 2x4 dot pattern
    Image,   // ratatui-image (Sixel/Kitty)
    Off,     // Hidden
}

pub enum ArtworkState {
    Idle,
    Loading,
    Loaded(DynamicImage),
    Failed,
}
