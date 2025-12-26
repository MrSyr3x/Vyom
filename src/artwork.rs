use image::DynamicImage;
use anyhow::Result;
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
}

pub struct ArtworkRenderer {
    client: Client,
}

impl ArtworkRenderer {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch_image(&self, url: &str) -> Result<DynamicImage> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        let img = image::load_from_memory(&bytes)?;
        Ok(img)
    }

    pub async fn fetch_itunes_artwork(&self, artist: &str, album: &str) -> Result<String> {
        let term = format!("{} {}", artist, album);
        let url = format!("https://itunes.apple.com/search?term={}&entity=album&limit=1", term);
        
        let resp = self.client.get(&url)
            .send().await?
            .json::<ItunesResponse>().await?;

        if let Some(result) = resp.results.first() {
            // Upgrade resolution: 100x100 -> 600x600 (or 1000x1000)
            let high_res = result.artwork_url.replace("100x100bb", "600x600bb");
            Ok(high_res)
        } else {
             anyhow::bail!("No results found on iTunes")
        }
    }
}
