use serde::{Deserialize, Serialize};

/// Default settings
pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8000;
pub const DEFAULT_FIFO_PATH: &str = "/tmp/vyom_hires.fifo";
pub const MPD_PORT: u16 = 6600;

/// Audio input source type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioSource {
    /// HTTP stream from MPD (16-bit only)
    Http { host: String, port: u16 },
    /// FIFO for Hi-Res audio (16/24/32-bit)
    Fifo { path: String },
}

impl Default for AudioSource {
    fn default() -> Self {
        AudioSource::Http {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
        }
    }
}

/// Audio format detected from input
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioInputFormat {
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub channels: u16,
}

impl Default for AudioInputFormat {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            bits_per_sample: 16,
            channels: 2,
        }
    }
}

impl AudioInputFormat {
    pub fn is_hi_res(&self) -> bool {
        self.sample_rate > 44100 || self.bits_per_sample > 16
    }
}

/// Audio pipeline configuration
#[derive(Default, Clone, Debug)]
pub struct AudioPipelineConfig {
    pub source: AudioSource,
    pub format: AudioInputFormat,
}
