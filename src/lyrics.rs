use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LrclibResponse {
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub timestamp_ms: u64,
    pub text: String,
}

pub struct LyricsFetcher {
    client: Client,
}

impl LyricsFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("vyom-rs/0.1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn fetch(&self, artist: &str, title: &str, duration_ms: u64) -> Result<Option<Vec<LyricLine>>> {
        let url = "https://lrclib.net/api/get";
        let duration_sec = duration_ms as f64 / 1000.0;
        let duration_str = duration_sec.to_string();
        
        let params = [
            ("artist_name", artist),
            ("track_name", title),
            ("duration", duration_str.as_str()),
        ];

        let resp = self.client.get(url)
            .query(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            // Try search endpoint if get fails
            return self.search(artist, title).await;
        }

        let data: LrclibResponse = resp.json().await?;
        Ok(self.parse(data))
    }

    async fn search(&self, artist: &str, title: &str) -> Result<Option<Vec<LyricLine>>> {
        let url = "https://lrclib.net/api/search";
        let q = format!("{} {}", artist, title);
        let params = [
            ("q", q.as_str()),
        ];

        let resp = self.client.get(url)
            .query(&params)
            .send()
            .await?;
            
        let results: Vec<LrclibResponse> = resp.json().await?;
        if let Some(first) = results.into_iter().next() {
            Ok(self.parse(first))
        } else {
            Ok(None)
        }
    }

    fn parse(&self, data: LrclibResponse) -> Option<Vec<LyricLine>> {
        let raw = data.synced_lyrics.or(data.plain_lyrics)?;
        
        let mut lines = Vec::new();
        // Parse basic LRC format [mm:ss.xx]Text
        // Regex is overkill, lets do manual parsing for speed
        
        for line in raw.lines() {
            if let Some(idx) = line.find(']') {
                if line.starts_with('[') {
                    let timestamp_str = &line[1..idx];
                    let text = line[idx+1..].trim().to_string();
                    
                    if let Some(ms) = self.parse_timestamp(timestamp_str) {
                         lines.push(LyricLine { timestamp_ms: ms, text });
                    }
                }
            }
        }
        
        if lines.is_empty() && !raw.is_empty() {
             // Plain lyrics? return simple list without timestamps? 
             // Or construct fake timestamps?
             // For now return raw lines with 0 ts if parsing failed but we had plain text
             // Actually better to just return what we found.
        }

        if lines.is_empty() { None } else { Some(lines) }
    }
    
    fn parse_timestamp(&self, ts: &str) -> Option<u64> {
        // mm:ss.xx
        let parts: Vec<&str> = ts.split(':').collect();
        if parts.len() != 2 { return None; }
        
        let min: u64 = parts[0].parse().ok()?;
        let sec_parts: Vec<&str> = parts[1].split('.').collect();
        let sec: u64 = sec_parts[0].parse().ok()?;
        let ms: u64 = if sec_parts.len() > 1 {
            let frac = sec_parts[1];
            if frac.len() == 2 {
                frac.parse::<u64>().ok()? * 10
            } else {
                frac.parse::<u64>().ok()?
            }
        } else {
            0
        };
        
        Some(min * 60000 + sec * 1000 + ms)
    }
}
