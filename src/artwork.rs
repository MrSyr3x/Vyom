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
    #[serde(rename = "collectionName")]
    collection_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
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
            
            let resp_result = self.client.get("https://itunes.apple.com/search")
                .query(&params)
                .send().await;
                
            // If request failed entirely (network), probably fails for all. But let's proceed.
            if let Ok(resp) = resp_result {
                if let Ok(data) = resp.json::<ItunesResponse>().await {
                    // 2. Filter Candidates (Strict Artist Check)
                    let candidates: Vec<&ItunesResult> = data.results.iter().filter(|r| {
                        if let Some(r_artist) = &r.artist_name {
                            let r_clean = Self::clean_string(r_artist);
                            r_clean.contains(&clean_artist) || clean_artist.contains(&r_clean)
                        } else {
                            true 
                        }
                    }).collect();

                    if candidates.is_empty() {
                        continue; // Try next country
                    }

                    // 3. Find Best Match
                    let best_match = candidates.iter().find(|r| {
                        if let Some(name) = &r.collection_name {
                             let r_clean = Self::clean_string(name);
                             r_clean == clean_album || r_clean.contains(&clean_album) || clean_album.contains(&r_clean)
                        } else {
                             false
                        }
                    });
                    
                    let result = best_match.or(candidates.first()).map(|&r| r);
                    
                    if let Some(result) = result {
                        let high_res = result.artwork_url.replace("100x100bb", "600x600bb");
                        return Ok(high_res);
                    }
                }
            }
        }
        
        anyhow::bail!("No results found on iTunes")
    }
}
