use anyhow::Result;
use image::DynamicImage;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ItunesResponse {
    results: Vec<ItunesResult>,
}

#[derive(Debug, Deserialize)]
struct ItunesResult {
    #[serde(rename = "artworkUrl100")]
    artwork_url: String,
    #[serde(rename = "collectionName")]
    collection_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
}

pub struct ArtworkRenderer {
    client: Client,
}

pub type DualPixelColor = (u8, u8, u8, u8, u8, u8);
pub type AsciiArtLine = (String, Vec<DualPixelColor>);

impl ArtworkRenderer {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch_image(&self, url: &str) -> Result<DynamicImage> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        let img = image::load_from_memory(&bytes)?;
        Ok(img)
    }

    /// Extract embedded album art from audio file (FLAC, MP3, etc.)
    #[cfg(feature = "mpd")]
    pub fn extract_embedded_art(file_path: &str) -> Result<DynamicImage> {
        use lofty::file::TaggedFileExt;
        use lofty::picture::PictureType;

        let tagged_file = lofty::read_from_path(file_path)?;

        // Try primary tag first, then all tags
        for tag in tagged_file.tags() {
            for picture in tag.pictures() {
                if picture.pic_type() == PictureType::CoverFront
                    || picture.pic_type() == PictureType::Other
                {
                    let img = image::load_from_memory(picture.data())?;
                    return Ok(img);
                }
            }
        }

        // If no cover found in tags, return error
        anyhow::bail!("No embedded artwork found in {}", file_path)
    }

    fn clean_string(s: &str) -> String {
        // Remove content in (), [], and "feat."
        let s = s.to_lowercase();
        let s = s.split('(').next().unwrap_or("");
        let s = s.split('[').next().unwrap_or("");
        let s = s.split("feat").next().unwrap_or("");
        s.trim().to_string()
    }

    pub async fn fetch_itunes_artwork(&self, artist: &str, album: &str) -> Result<String> {
        let clean_artist = Self::clean_string(artist);
        let clean_album = Self::clean_string(album);
        let term = format!("{} {}", clean_artist, clean_album);

        // Try US first (global default), then IN (for regional content)
        // We can add more regions if needed, or make it configurable later.
        let countries = ["US", "IN"];

        for country in countries {
            let params = [
                ("term", term.as_str()),
                ("entity", "album"),
                ("limit", "5"),
                ("country", country),
            ];

            let resp_result = self
                .client
                .get("https://itunes.apple.com/search")
                .query(&params)
                .send()
                .await;

            // If request failed entirely (network), probably fails for all. But let's proceed.
            if let Ok(resp) = resp_result {
                if let Ok(data) = resp.json::<ItunesResponse>().await {
                    // 2. Filter Candidates (Strict Artist Check)
                    let candidates: Vec<&ItunesResult> = data
                        .results
                        .iter()
                        .filter(|r| {
                            if let Some(r_artist) = &r.artist_name {
                                let r_clean = Self::clean_string(r_artist);
                                r_clean.contains(&clean_artist) || clean_artist.contains(&r_clean)
                            } else {
                                true
                            }
                        })
                        .collect();

                    if candidates.is_empty() {
                        continue; // Try next country
                    }

                    // 3. Find Best Match
                    let best_match = candidates.iter().find(|r| {
                        if let Some(name) = &r.collection_name {
                            let r_clean = Self::clean_string(name);
                            r_clean == clean_album
                                || r_clean.contains(&clean_album)
                                || clean_album.contains(&r_clean)
                        } else {
                            false
                        }
                    });

                    let result = best_match.or(candidates.first()).copied();

                    if let Some(result) = result {
                        let high_res = result.artwork_url.replace("100x100bb", "600x600bb");
                        return Ok(high_res);
                    }
                }
            }
        }

        anyhow::bail!("No results found on iTunes")
    }

    pub fn render_to_lines(
        img: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> Vec<AsciiArtLine> {
        use image::GenericImageView;

        // Resize image to target dimensions (height should be even for half-blocks)
        let actual_height = target_height * 2; // 2 pixels per terminal row
        let resized = img.resize_exact(
            target_width,
            actual_height,
            image::imageops::FilterType::Triangle,
        );

        let mut lines: Vec<AsciiArtLine> = Vec::new();

        // Process 2 rows at a time
        for y in (0..actual_height).step_by(2) {
            let mut line_chars = String::new();
            let mut line_colors: Vec<DualPixelColor> = Vec::new();

            for x in 0..target_width {
                let top_pixel = resized.get_pixel(x, y);
                let bottom_pixel = if y + 1 < actual_height {
                    resized.get_pixel(x, y + 1)
                } else {
                    top_pixel
                };

                // Get RGB values
                let (tr, tg, tb) = (top_pixel[0], top_pixel[1], top_pixel[2]);
                let (br, bg, bb) = (bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);

                // Use upper half block (▀) with fg=top, bg=bottom
                line_chars.push('▀');
                line_colors.push((tr, tg, tb, br, bg, bb));
            }

            lines.push((line_chars, line_colors));
        }

        lines
    }

    /// Render a tiny thumbnail (4 chars wide, 2 lines tall) for inline display
    /// Returns Vec of ratatui Lines ready for rendering
    pub fn render_tiny(img: &DynamicImage) -> Vec<ratatui::text::Line<'static>> {
        use image::GenericImageView;
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        // Resize to 4x4 pixels (2 lines of 4 chars each using half-blocks)
        let resized = img.resize_exact(4, 4, image::imageops::FilterType::Triangle);

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Process 2 rows at a time (4 pixels → 2 lines)
        for y in (0..4).step_by(2) {
            let mut spans: Vec<Span> = Vec::new();

            for x in 0..4 {
                let top = resized.get_pixel(x, y);
                let bottom = if y + 1 < 4 {
                    resized.get_pixel(x, y + 1)
                } else {
                    top
                };

                let fg = Color::Rgb(top[0], top[1], top[2]);
                let bg = Color::Rgb(bottom[0], bottom[1], bottom[2]);

                spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
            }

            lines.push(Line::from(spans));
        }

        lines
    }

    /// Render a small thumbnail (8 chars wide, 4 lines tall)
    /// Better quality than tiny, still compact
    pub fn render_small(img: &DynamicImage) -> Vec<ratatui::text::Line<'static>> {
        use image::GenericImageView;
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        // Resize to 8x8 pixels (4 lines of 8 chars each)
        let resized = img.resize_exact(8, 8, image::imageops::FilterType::Triangle);

        let mut lines: Vec<Line<'static>> = Vec::new();

        for y in (0..8).step_by(2) {
            let mut spans: Vec<Span> = Vec::new();

            for x in 0..8 {
                let top = resized.get_pixel(x, y);
                let bottom = if y + 1 < 8 {
                    resized.get_pixel(x, y + 1)
                } else {
                    top
                };

                let fg = Color::Rgb(top[0], top[1], top[2]);
                let bg = Color::Rgb(bottom[0], bottom[1], bottom[2]);

                spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
            }

            lines.push(Line::from(spans));
        }

        lines
    }
}
