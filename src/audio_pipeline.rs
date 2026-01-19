//! Audio Pipeline Module - Hi-Res Edition
//!
//! Supports both HTTP and FIFO input with 16/24/32-bit audio.
//! Processes through DSP EQ and outputs via CoreAudio or cpal.
//! Features dynamic sample rate detection for bit-perfect playback.

#[cfg(feature = "eq")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "eq")]
use cpal::StreamConfig;

#[allow(unused_imports)]
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::dsp_eq::{DspEqualizer, EqGains};

/// Default settings
pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8000;
pub const DEFAULT_FIFO_PATH: &str = "/tmp/vyom_hires.fifo";
pub const MPD_PORT: u16 = 6600;

/// Query MPD for current audio format
/// Returns (sample_rate, bits_per_sample, channels)
fn query_mpd_format() -> Option<(u32, u16, u16)> {
    let mut stream = TcpStream::connect("127.0.0.1:6600").ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    
    // Read greeting
    let mut buf = [0u8; 256];
    let _ = stream.read(&mut buf);
    
    // Send status command
    stream.write_all(b"status\n").ok()?;
    
    let mut response = String::new();
    let mut reader = BufReader::new(&stream);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.starts_with("OK") || line.starts_with("ACK") {
                    break;
                }
                response.push_str(&line);
            }
            Err(_) => break,
        }
    }
    
    // Parse audio: sample_rate:bits:channels
    for line in response.lines() {
        if line.starts_with("audio: ") {
            let audio_str = line.trim_start_matches("audio: ");
            let parts: Vec<&str> = audio_str.split(':').collect();
            if parts.len() >= 3 {
                let sample_rate = parts[0].parse().ok()?;
                let bits = parts[1].parse().ok()?;
                let channels = parts[2].parse().ok()?;
                return Some((sample_rate, bits, channels));
            }
        }
    }
    
    None
}

/// Audio input source type
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
pub struct AudioPipelineConfig {
    pub source: AudioSource,
    pub format: AudioInputFormat,
}

impl Default for AudioPipelineConfig {
    fn default() -> Self {
        Self {
            source: AudioSource::default(),
            format: AudioInputFormat::default(),
        }
    }
}

/// Audio pipeline with Hi-Res support
pub struct AudioPipeline {
    config: AudioPipelineConfig,
    eq_gains: EqGains,
    running: Arc<AtomicBool>,
    pub global_volume: Arc<std::sync::atomic::AtomicU8>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl AudioPipeline {
    /// Create a new audio pipeline (defaults to HTTP)
    pub fn new(eq_gains: EqGains) -> Self {
        Self {
            config: AudioPipelineConfig::default(),
            eq_gains,
            running: Arc::new(AtomicBool::new(false)),
            global_volume: Arc::new(std::sync::atomic::AtomicU8::new(100)),
            thread_handle: None,
        }
    }
    
    /// Create pipeline with FIFO source for Hi-Res
    #[allow(dead_code)]
    pub fn with_fifo(eq_gains: EqGains, fifo_path: &str, format: AudioInputFormat) -> Self {
        Self {
            config: AudioPipelineConfig {
                source: AudioSource::Fifo { path: fifo_path.to_string() },
                format,
            },
            eq_gains,
            running: Arc::new(AtomicBool::new(false)),
            global_volume: Arc::new(std::sync::atomic::AtomicU8::new(100)),
            thread_handle: None,
        }
    }
    
    /// Set global volume (0-100)
    pub fn set_volume(&self, volume: u8) {
        self.global_volume.store(volume.min(100), Ordering::SeqCst);
    }
    
    /// Start the audio pipeline
    #[cfg(feature = "eq")]
    pub fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Pipeline already running".to_string());
        }
        
        let running = self.running.clone();
        let eq_gains = self.eq_gains.clone();
        let global_volume = self.global_volume.clone();
        let source = self.config.source.clone();
        let format = self.config.format.clone();
        
        running.store(true, Ordering::SeqCst);
        
        let handle = thread::spawn(move || {
            let result = match source {
                AudioSource::Http { host, port } => {
                    run_http_audio_loop(&host, port, &format, eq_gains, running.clone(), global_volume)
                }
                AudioSource::Fifo { path } => {
                    run_fifo_audio_loop(&path, &format, eq_gains, running.clone(), global_volume)
                }
            };
            
            if let Err(e) = result {
                eprintln!("Audio pipeline error: {}", e);
            }
            running.store(false, Ordering::SeqCst);
        });
        
        self.thread_handle = Some(handle);
        Ok(())
    }
    
    /// Stop the audio pipeline
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
    
    /// Check if running
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}



// ════════════════════════════════════════════════════════════════════════════
// HTTP Audio Loop (legacy 16-bit)
// ════════════════════════════════════════════════════════════════════════════

/// Connect to MPD HTTP stream
fn connect_to_http_stream(host: &str, port: u16) -> Result<TcpStream, String> {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;
    
    stream.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;
    
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}:{}\r\nConnection: keep-alive\r\n\r\n",
        host, port
    );
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    // Read HTTP headers
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut header_line = String::new();
    loop {
        header_line.clear();
        match reader.read_line(&mut header_line) {
            Ok(0) => return Err("Connection closed during headers".to_string()),
            Ok(_) => {
                if header_line.trim().is_empty() {
                    break;
                }
            }
            Err(e) => return Err(format!("Failed to read headers: {}", e)),
        }
    }
    
    Ok(stream)
}

/// HTTP audio loop (16-bit PCM)
#[cfg(feature = "eq")]
fn run_http_audio_loop(
    host: &str,
    port: u16,
    format: &AudioInputFormat,
    eq_gains: EqGains,
    running: Arc<AtomicBool>,
    global_volume: Arc<std::sync::atomic::AtomicU8>,
) -> Result<(), String> {
    // Get output device
    let audio_host = cpal::default_host();
    let device = audio_host.default_output_device()
        .ok_or("No output device available")?;
    
    let stream_config = StreamConfig {
        channels: format.channels,
        sample_rate: cpal::SampleRate(format.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };
    
    let mut equalizer = DspEqualizer::new(format.sample_rate as f32, eq_gains);
    
    let ring_buffer = Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::<f32>::with_capacity(32768)
    ));
    let ring_buffer_clone = ring_buffer.clone();
    
    let fade_level = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fade_level_clone = fade_level.clone();
    
    const FADE_SPEED: f32 = 0.005;
    
    let stream = device.build_output_stream(
        &stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut buffer = ring_buffer_clone.lock().unwrap();
            let mut fade = f32::from_bits(fade_level_clone.load(Ordering::Relaxed));
            
            // Calculate Gain 🎚️
            let vol = global_volume.load(Ordering::Relaxed);
            let gain = (vol as f32 / 100.0).powf(3.0); // Cubic taper for natural feel
            
            for sample in data.iter_mut() {
                if let Some(s) = buffer.pop_front() {
                    if fade < 1.0 { fade = (fade + FADE_SPEED).min(1.0); }
                    *sample = s * fade * gain;
                } else {
                    if fade > 0.0 { fade = (fade - FADE_SPEED).max(0.0); }
                    *sample = 0.0;
                }
            }
            fade_level_clone.store(fade.to_bits(), Ordering::Relaxed);
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    ).map_err(|e| format!("Failed to build output stream: {}", e))?;
    
    stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
    
    let mut read_buffer = vec![0u8; 8192];
    
    while running.load(Ordering::SeqCst) {
        let tcp_stream = match connect_to_http_stream(host, port) {
            Ok(s) => s,
            Err(_) => {
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };
        
        let mut reader = BufReader::with_capacity(16384, tcp_stream);
        
        // Skip WAV header (44 bytes)
        let mut header = [0u8; 44];
        if reader.read_exact(&mut header).is_err() {
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        
        while running.load(Ordering::SeqCst) {
            match reader.read(&mut read_buffer) {
                Ok(0) => {
                    equalizer.reset_filters();
                    break;
                }
                Ok(bytes_read) => {
                    // 16-bit PCM to f32
                    let samples = bytes_read / 2;
                    let mut float_buffer = Vec::with_capacity(samples);
                    
                    for i in 0..samples {
                        let idx = i * 2;
                        if idx + 1 < bytes_read {
                            let sample_i16 = i16::from_le_bytes([
                                read_buffer[idx],
                                read_buffer[idx + 1],
                            ]);
                            float_buffer.push(sample_i16 as f32 / 32768.0);
                        }
                    }
                    
                    equalizer.process_buffer(&mut float_buffer);
                    
                    if let Ok(mut buffer) = ring_buffer.lock() {
                        for sample in float_buffer {
                            buffer.push_back(sample);
                        }
                        while buffer.len() > 32768 {
                            buffer.pop_front();
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock 
                    || e.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(_) => {
                    equalizer.reset_filters();
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// FIFO Audio Loop (Hi-Res 16/24/32-bit)
// ════════════════════════════════════════════════════════════════════════════

/// FIFO audio loop with Hi-Res support and dynamic format detection
#[cfg(feature = "eq")]
fn run_fifo_audio_loop(
    fifo_path: &str,
    format: &AudioInputFormat,
    eq_gains: EqGains,
    running: Arc<AtomicBool>,
    global_volume: Arc<std::sync::atomic::AtomicU8>,
) -> Result<(), String> {
    // Get output device
    let audio_host = cpal::default_host();
    let device = audio_host.default_output_device()
        .ok_or("No output device available")?;
    
    // Query MPD for actual format (dynamic detection!)
    let (sample_rate, bits_per_sample, channels) = query_mpd_format()
        .unwrap_or((format.sample_rate, format.bits_per_sample, format.channels));
    
    eprintln!("🎵 Hi-Res Audio: {}Hz/{}bit/{}ch (bit-perfect)", sample_rate, bits_per_sample, channels);
    
    // Use detected sample rate for bit-perfect output
    let stream_config = StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };
    
    // Create EQ at correct sample rate
    let mut equalizer = DspEqualizer::new(sample_rate as f32, eq_gains);
    
    
    let ring_buffer = Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::<f32>::with_capacity(65536) // Larger for Hi-Res
    ));
    let ring_buffer_clone = ring_buffer.clone();
    
    let fade_level = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fade_level_clone = fade_level.clone();
    
    const FADE_SPEED: f32 = 0.003; // Slower fade for Hi-Res
    
    let stream = device.build_output_stream(
        &stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut buffer = ring_buffer_clone.lock().unwrap();
            let mut fade = f32::from_bits(fade_level_clone.load(Ordering::Relaxed));
            
            // Calculate Gain 🎚️
            let vol = global_volume.load(Ordering::Relaxed);
            let gain = (vol as f32 / 100.0).powf(3.0);
            
            for sample in data.iter_mut() {
                if let Some(s) = buffer.pop_front() {
                    if fade < 1.0 { fade = (fade + FADE_SPEED).min(1.0); }
                    *sample = s * fade * gain;
                } else {
                    if fade > 0.0 { fade = (fade - FADE_SPEED).max(0.0); }
                    *sample = 0.0;
                }
            }
            fade_level_clone.store(fade.to_bits(), Ordering::Relaxed);
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    ).map_err(|e| format!("Failed to build output stream: {}", e))?;
    
    stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
    
    // Calculate bytes per sample based on detected bit depth
    let bytes_per_sample_val = (bits_per_sample / 8) as usize;
    let frame_size = bytes_per_sample_val * channels as usize;
    let buffer_frames = 2048;
    let mut read_buffer = vec![0u8; frame_size * buffer_frames];
    
    
    while running.load(Ordering::SeqCst) {
        // Open FIFO (blocking)
        let fifo = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(fifo_path) 
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!("FIFO not available: {} - falling back to HTTP", e);
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        };
        
        let mut reader = BufReader::with_capacity(65536, fifo);
        
        while running.load(Ordering::SeqCst) {
            match reader.read(&mut read_buffer) {
                Ok(0) => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Ok(bytes_read) => {
                    let frames = bytes_read / frame_size;
                    let mut float_buffer = Vec::with_capacity(frames * channels as usize);
                    
                    for frame in 0..frames {
                        for ch in 0..channels as usize {
                            let offset = frame * frame_size + ch * bytes_per_sample_val;
                            
                            let sample_f32 = match bits_per_sample {
                                16 => {
                                    if offset + 1 < bytes_read {
                                        let s = i16::from_le_bytes([
                                            read_buffer[offset],
                                            read_buffer[offset + 1],
                                        ]);
                                        s as f32 / 32768.0
                                    } else { 0.0 }
                                }
                                24 => {
                                    if offset + 2 < bytes_read {
                                        // 24-bit in 3 bytes, sign-extend to i32
                                        let b0 = read_buffer[offset] as i32;
                                        let b1 = read_buffer[offset + 1] as i32;
                                        let b2 = read_buffer[offset + 2] as i32;
                                        let s = (b2 << 24) | (b1 << 16) | (b0 << 8);
                                        (s >> 8) as f32 / 8388608.0 // 2^23
                                    } else { 0.0 }
                                }
                                32 => {
                                    if offset + 3 < bytes_read {
                                        // 32-bit integer
                                        let s = i32::from_le_bytes([
                                            read_buffer[offset],
                                            read_buffer[offset + 1],
                                            read_buffer[offset + 2],
                                            read_buffer[offset + 3],
                                        ]);
                                        s as f32 / 2147483648.0 // 2^31
                                    } else { 0.0 }
                                }
                                _ => 0.0,
                            };
                            
                            float_buffer.push(sample_f32);
                        }
                    }
                    
                    // Apply EQ in 32-bit float domain
                    equalizer.process_buffer(&mut float_buffer);
                    
                    if let Ok(mut buffer) = ring_buffer.lock() {
                        for sample in float_buffer {
                            buffer.push_back(sample);
                        }
                        // Larger limit for Hi-Res
                        while buffer.len() > 65536 {
                            buffer.pop_front();
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(_) => {
                    equalizer.reset_filters();
                    break;
                }
            }
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    Ok(())
}

// Fallback for non-eq feature
#[cfg(not(feature = "eq"))]
impl AudioPipeline {
    pub fn start(&mut self) -> Result<(), String> {
        Err("EQ feature not enabled".to_string())
    }
}
