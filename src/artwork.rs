use image::GenericImageView;
use image::imageops::FilterType;
use anyhow::Result;

use reqwest::Client;

// (R, G, B)
pub type Rgb = (u8, u8, u8);
// Row of (Foreground, Background) colors
pub type ArtworkRow = Vec<(Rgb, Rgb)>;
pub type ArtworkData = Vec<ArtworkRow>;

pub struct ArtworkRenderer {
    client: Client,
}

impl ArtworkRenderer {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn render_from_url(&self, url: &str, width: u32, height: u32) -> Result<ArtworkData> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        let img = image::load_from_memory(&bytes)?;

        // Resize
        // We use height * 2 because we use half-blocks (2 pixels per char vertically)
        let resized = img.resize_exact(width, height * 2, FilterType::Lanczos3);
        
        let mut rows = Vec::new();

        for y in (0..height * 2).step_by(2) {
            let mut row = Vec::new();
            for x in 0..width {
                let p1 = resized.get_pixel(x, y);
                let p2 = if y + 1 < height * 2 {
                    resized.get_pixel(x, y + 1)
                } else {
                    p1
                };

                let fg = (p1[0], p1[1], p1[2]);
                let bg = (p2[0], p2[1], p2[2]);
                
                row.push((fg, bg));
            }
            rows.push(row);
        }

        Ok(rows)
    }
}
