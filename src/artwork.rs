use image::DynamicImage;
use anyhow::Result;
use reqwest::Client;

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
}
