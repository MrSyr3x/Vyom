use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct LrclibResponse {
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(default)] 
    pub instrumental: bool,
    pub duration: Option<f64>,
}

#[derive(Debug)]
pub enum LyricsFetchResult {
    Found(Vec<LyricLine>),
    Instrumental,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLine {
    pub timestamp_ms: u64,
    pub text: String,
}

pub struct LyricsFetcher {
    client: Client,
}

impl LyricsFetcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn get_cache_path(&self, artist: &str, title: &str) -> Option<PathBuf> {
         let home = std::env::var("HOME").ok()?;
         let safe_artist = artist.replace("/", "_");
         let safe_title = title.replace("/", "_");
         let filename = format!("{}_{}.json", safe_artist, safe_title);
         
         let path = Path::new(&home).join(".cache").join("vyom").join("lyrics").join(filename);
         Some(path)
    }

    fn load_from_cache(&self, path: &PathBuf) -> Option<Vec<LyricLine>> {
        if path.exists() {
            if let Ok(file) = fs::File::open(path) {
                if let Ok(lyrics) = serde_json::from_reader(file) {
                    return Some(lyrics);
                }
            }
        }
        None
    }

    fn save_to_cache(&self, path: &PathBuf, lyrics: &Vec<LyricLine>) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(file) = fs::File::create(path) {
            let _ = serde_json::to_writer(file, lyrics);
        }
    }

    fn clean_title(title: &str) -> String {
        // Remove junk that confuses fuzzy search
        let t = title.to_lowercase();
        // Cut off at common delimiters
        let t = t.split("feat.").next().unwrap_or(&t);
        let t = t.split("(feat").next().unwrap_or(&t);
        let t = t.split("with").next().unwrap_or(&t);
        
        // Remove specific phrases
        let t = t.replace("remastered", "")
                 .replace("remaster", "")
                 .replace("studio version", "")
                 .replace("stereo mix", "")
                 .replace("mono mix", "")
                 .replace("single version", "")
                 .replace("original mix", "");
                 
        // Remove bracket contents if they look like metadata
        // Regex: \([^)]*\) -> but manually
        let mut clean = String::new();
        let mut in_bracket = false;
        for c in t.chars() {
            if c == '(' || c == '[' { in_bracket = true; }
            else if c == ')' || c == ']' { in_bracket = false; }
            else if !in_bracket { clean.push(c); }
        }
        
        clean.trim().to_string()
    }

    fn clean_artist(artist: &str) -> String {
        // "Drake, Future" -> "Drake"
        // "Drake & Future" -> "Drake"
        // "Drake feat. Future" -> "Drake"
        let t = artist.to_lowercase();
        // Split by common separators
        let separators = [",", "&", " feat.", " ft.", " featuring"];
        let mut primary = t.as_str();
        
        for sep in separators {
            if let Some(idx) = primary.find(sep) {
                primary = &primary[..idx];
            }
        }
        
        primary.trim().to_string()
    }

    pub async fn fetch(&self, artist: &str, title: &str, duration_ms: u64) -> Result<LyricsFetchResult> {
        // 0. Check Disk Cache 💾
        let cache_path = self.get_cache_path(artist, title);
        if let Some(path) = &cache_path {
            if let Some(cached_lyrics) = self.load_from_cache(path) {
                // Disk cache only stores Found lyrics for now
                return Ok(LyricsFetchResult::Found(cached_lyrics));
            }
        }

        let url = "https://lrclib.net/api/get";
        let duration_sec = duration_ms as f64 / 1000.0;
        let duration_str = duration_sec.to_string();
        
        let safe_title = Self::clean_title(title); 
        
        let params = [
            ("artist_name", artist),
            ("track_name", title),
            ("duration", duration_str.as_str()),
        ];

        // 1. Try Exact (/get)
        let resp = self.client.get(url).query(&params).send().await?;
        if resp.status().is_success() {
             let data: LrclibResponse = resp.json().await?;
             let result = self.parse(data);
             
             if let LyricsFetchResult::Found(ref lines) = result {
                 if let Some(path) = &cache_path {
                     self.save_to_cache(path, lines);
                 }
             }
             
             match result {
                 LyricsFetchResult::None => {}, // fallthrough
                 _ => return Ok(result),
             }
        }

        // 2. Try Search (/search) with CLEAN title and ORIGINAL artist
        let search_res = self.search(artist, &safe_title, duration_ms).await?;
        if let LyricsFetchResult::Found(ref lines) = search_res {
            if let Some(path) = &cache_path {
                 self.save_to_cache(path, lines);
            }
            return Ok(search_res);
        }
        if let LyricsFetchResult::Instrumental = search_res {
             return Ok(search_res);
        }

        // 3. Try Search with PRIMARY artist (NEW Fallback) 🎯
        let safe_artist = Self::clean_artist(artist);
        if safe_artist != artist.to_lowercase() {
             let primary_res = self.search(&safe_artist, &safe_title, duration_ms).await?;
             if let LyricsFetchResult::Found(ref lines) = primary_res {
                if let Some(path) = &cache_path {
                     self.save_to_cache(path, lines);
                }
             }
             Ok(primary_res)
        } else {
             // Already tried with this artist name (it was clean)
             Ok(search_res)
        }
    }

    async fn search(&self, artist: &str, title: &str, duration_ms: u64) -> Result<LyricsFetchResult> {
        let url = "https://lrclib.net/api/search";
        let q = format!("{} {}", artist, title);
        let params = [("q", q.as_str())];

        let resp = self.client.get(url).query(&params).send().await?;
        let results: Vec<LrclibResponse> = resp.json().await?;
        
        let target_dur = duration_ms as f64 / 1000.0;

        // Helper filter closure
        let is_valid = |r: &LrclibResponse| -> bool {
            if let Some(dur) = r.duration {
                (dur - target_dur).abs() <= 3.0
            } else {
                // If API doesn't return duration, strict mode says NO? 
                // Or YES to be safe? 
                // Let's say NO to be strict. Most entries have duration.
                false
            }
        };

        // Find first synced OR instrumental THAT MATCHES DURATION
        // Prioritize Synced loops
        if let Some(found) = results.iter().find(|r| r.synced_lyrics.is_some() && is_valid(r)) {
             return Ok(self.parse_ref(found));
        }
        
        // If no synced, check if any match is instrumental
        if let Some(_found) = results.iter().find(|r| r.instrumental && is_valid(r)) {
             return Ok(LyricsFetchResult::Instrumental);
        }

        Ok(LyricsFetchResult::None)
    }

    fn parse(&self, data: LrclibResponse) -> LyricsFetchResult {
        if data.instrumental {
            return LyricsFetchResult::Instrumental;
        }
        
        // Reuse parsing logic via helper or just inline? 
        // LrclibResponse ownership consumed here
        let raw = data.synced_lyrics.or(data.plain_lyrics);
        if raw.is_none() { return LyricsFetchResult::None; }
        
        let mut lines = Vec::new();
        for line in raw.unwrap().lines() {
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
        
        if lines.is_empty() { LyricsFetchResult::None } else { LyricsFetchResult::Found(lines) }
    }
    
    // Helper for reference (Search)
    fn parse_ref(&self, data: &LrclibResponse) -> LyricsFetchResult {
        if data.instrumental {
            return LyricsFetchResult::Instrumental;
        }
        let raw = data.synced_lyrics.as_ref().or(data.plain_lyrics.as_ref());
         if raw.is_none() { return LyricsFetchResult::None; }
         
         // ... duplicate logic or clone? Clone data is cheap (strings)
         // Let's just clone and call parse
         // Actually LrclibResponse is not Clone. 
         // Manual parse:
        let mut lines = Vec::new();
        for line in raw.unwrap().lines() {
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
         if lines.is_empty() { LyricsFetchResult::None } else { LyricsFetchResult::Found(lines) }
    }

    fn parse_timestamp(&self, ts: &str) -> Option<u64> {
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
