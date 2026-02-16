use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "mpd")]
use lofty::{file::TaggedFileExt, tag::ItemKey};

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

    fn get_cache_path(artist: &str, title: &str) -> Option<PathBuf> {
        // Use dirs::cache_dir() for correct XDG_CACHE_HOME support
        let cache_root = dirs::cache_dir().unwrap_or_else(|| {
             let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
             PathBuf::from(home).join(".cache")
        });

        // Use SipHash for safe, fixed-length filename (Prevents path traversal)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        format!("{}{}", artist, title).hash(&mut hasher);
        let hash = hasher.finish();
        let filename = format!("{:016x}.json", hash);

        let path = cache_root
            .join("vyom")
            .join("lyrics")
            .join(filename);
        Some(path)
    }

    fn load_from_cache(path: &PathBuf) -> Option<Vec<LyricLine>> {
        if path.exists() {
            if let Ok(file) = fs::File::open(path) {
                if let Ok(lyrics) = serde_json::from_reader(file) {
                    return Some(lyrics);
                }
            }
        }
        None
    }

    fn save_to_cache(path: &PathBuf, lyrics: &Vec<LyricLine>) {
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
        let t = t.split("(feat").next().unwrap_or(t);
        let t = t.split("with").next().unwrap_or(t);

        // Remove specific phrases
        let t = t
            .replace("remastered", "")
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
            if c == '(' || c == '[' {
                in_bracket = true;
            } else if c == ')' || c == ']' {
                in_bracket = false;
            } else if !in_bracket {
                clean.push(c);
            }
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

    pub async fn fetch(
        &self,
        artist: &str,
        title: &str,
        duration_ms: u64,
        file_path: Option<&String>,
    ) -> Result<LyricsFetchResult> {
        // 0. Check Local File (Embedded or LRC) 📂 - BLOCKING WRAPPER
        if let Some(path_str) = file_path {
            let p = path_str.clone();
            let local_res = tokio::task::spawn_blocking(move || {
                Self::fetch_impl_local(&p)
            }).await?;
            
            if let Some(res) = local_res {
                return Ok(res);
            }
        }

        // 1. Check Disk Cache 💾 - BLOCKING WRAPPER
        let artist_owned = artist.to_string();
        let title_owned = title.to_string();
        
        let (cache_path, cached_lyrics) = tokio::task::spawn_blocking(move || {
            let path = Self::get_cache_path(&artist_owned, &title_owned);
            let lyrics = if let Some(p) = &path {
                Self::load_from_cache(p)
            } else {
                None
            };
            (path, lyrics)
        }).await?;

        if let Some(lyrics) = cached_lyrics {
             return Ok(LyricsFetchResult::Found(lyrics));
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

        // 2. Try Exact (/get) with Retry 🔄
        let mut attempt = 0;
        const MAX_RETRIES: u8 = 2;

        while attempt <= MAX_RETRIES {
            let resp_result = self.client.get(url).query(&params).send().await;

            match resp_result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // If JSON parse fails, it's a data issue, don't retry network
                        if let Ok(data) = resp.json::<LrclibResponse>().await {
                            let result = self.parse(data);
                            if let LyricsFetchResult::Found(ref lines) = result {
                                if let Some(path) = &cache_path {
                                    let path_clone = path.clone();
                                    let lines_clone = lines.clone();
                                    // Save in background, don't await
                                    tokio::task::spawn_blocking(move || {
                                        Self::save_to_cache(&path_clone, &lines_clone);
                                    });
                                }
                            }
                            match result {
                                LyricsFetchResult::None => break, // Fallthrough to Search
                                _ => return Ok(result),
                            }
                        }
                    }
                    break; // Request succeeded (even if 404), proceed to Search
                }
                Err(_) => {
                    attempt += 1;
                    if attempt > MAX_RETRIES {
                        // Don't error out completely, just log/proceed to search
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        // 3. Try Search (/search) with CLEAN title and ORIGINAL artist
        let search_res = self.search(artist, &safe_title, duration_ms).await?;
        if let LyricsFetchResult::Found(ref lines) = search_res {
            if let Some(path) = &cache_path {
                let path_clone = path.clone();
                let lines_clone = lines.clone();
                 tokio::task::spawn_blocking(move || {
                    Self::save_to_cache(&path_clone, &lines_clone);
                });
            }
            return Ok(search_res);
        }
        if let LyricsFetchResult::Instrumental = search_res {
            return Ok(search_res);
        }

        // 4. Try Search with PRIMARY artist (NEW Fallback) 🎯
        let safe_artist = Self::clean_artist(artist);
        if safe_artist != artist.to_lowercase() {
            let primary_res = self.search(&safe_artist, &safe_title, duration_ms).await?;
            if let LyricsFetchResult::Found(ref lines) = primary_res {
                if let Some(path) = &cache_path {
                    let path_clone = path.clone();
                    let lines_clone = lines.clone();
                    tokio::task::spawn_blocking(move || {
                        Self::save_to_cache(&path_clone, &lines_clone);
                    });
                }
            }
            Ok(primary_res)
        } else {
            // Already tried with this artist name (it was clean)
            Ok(search_res)
        }
    }

    fn fetch_impl_local(path_str: &str) -> Option<LyricsFetchResult> {
        let path = Path::new(path_str);

        // A. Check sidecar .lrc file
        let lrc_path = path.with_extension("lrc");
        if lrc_path.exists() {
            if let Ok(content) = fs::read_to_string(lrc_path) {
                // Determine if synced or plain by checking for timestamps
                let is_synced = content.contains(']');
                if is_synced {
                    let lines = Self::parse_lrc_content(&content);
                    if !lines.is_empty() {
                        return Some(LyricsFetchResult::Found(lines));
                    }
                }
            }
        }

        // B. Check Embedded Lyrics (lofty)
        #[cfg(feature = "mpd")]
        {
            if let Ok(tagged_file) = lofty::read_from_path(path) {
                // Try primary tag first, then first tag
                let tag = tagged_file
                    .primary_tag()
                    .or_else(|| tagged_file.first_tag());

                if let Some(tag) = tag {
                    if let Some(lyrics) = tag.get_string(&ItemKey::Lyrics).map(|s| s.to_string()) {
                        let is_synced = lyrics.contains(']');
                        if is_synced {
                            let lines = Self::parse_lrc_content(&lyrics);
                            if !lines.is_empty() {
                                return Some(LyricsFetchResult::Found(lines));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn parse_lrc_content(content: &str) -> Vec<LyricLine> {
        let mut lines = Vec::new();
        for line in content.lines() {
            if let Some(idx) = line.find(']') {
                if line.starts_with('[') {
                    let timestamp_str = &line[1..idx];
                    let text = line[idx + 1..].trim().to_string();
                    if let Some(ms) = Self::parse_timestamp(timestamp_str) {
                        lines.push(LyricLine {
                            timestamp_ms: ms,
                            text,
                        });
                    }
                }
            }
        }
        lines
    }

    async fn search(
        &self,
        artist: &str,
        title: &str,
        duration_ms: u64,
    ) -> Result<LyricsFetchResult> {
        let url = "https://lrclib.net/api/search";
        let q = format!("{} {}", artist, title);
        let params = [("q", q.as_str())];

        let mut attempt = 0;
        const MAX_RETRIES: u8 = 2;

        let resp = loop {
            match self.client.get(url).query(&params).send().await {
                Ok(response) => break Ok(response),
                Err(_) => {
                    attempt += 1;
                    if attempt > MAX_RETRIES {
                        break Err(anyhow::anyhow!("Lyrics Network Error"));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }?;

        let results: Vec<LrclibResponse> = resp.json().await.unwrap_or_default();

        let target_dur = duration_ms as f64 / 1000.0;

        // Helper filter closure
        let is_valid = |r: &LrclibResponse| -> bool {
            if let Some(dur) = r.duration {
                (dur - target_dur).abs() <= 3.0
            } else {
                false
            }
        };

        // Find first synced OR instrumental THAT MATCHES DURATION
        // Prioritize Synced loops
        if let Some(found) = results
            .iter()
            .find(|r| r.synced_lyrics.is_some() && is_valid(r))
        {
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

        let raw_opt = data.synced_lyrics.or(data.plain_lyrics);
        if let Some(raw) = raw_opt {
            let lines = Self::parse_lrc_content(&raw);
            if lines.is_empty() {
                LyricsFetchResult::None
            } else {
                LyricsFetchResult::Found(lines)
            }
        } else {
            LyricsFetchResult::None
        }
    }

    // Helper for reference (Search)
    fn parse_ref(&self, data: &LrclibResponse) -> LyricsFetchResult {
        if data.instrumental {
            return LyricsFetchResult::Instrumental;
        }
        let raw_opt = data.synced_lyrics.as_ref().or(data.plain_lyrics.as_ref());
        if let Some(raw) = raw_opt {
            let lines = Self::parse_lrc_content(raw);
            if lines.is_empty() {
                LyricsFetchResult::None
            } else {
                LyricsFetchResult::Found(lines)
            }
        } else {
            LyricsFetchResult::None
        }
    }

    fn parse_timestamp(ts: &str) -> Option<u64> {
        let parts: Vec<&str> = ts.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

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
