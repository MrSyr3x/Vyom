use super::common::build_audio_stream;
use crate::audio::dsp::{DspEqualizer, EqGains};
use crate::audio::types::AudioInputFormat;
use cpal::traits::{HostTrait};
use cpal::StreamConfig;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Connect to MPD HTTP stream
fn connect_to_http_stream(host: &str, port: u16) -> Result<TcpStream, String> {
    let addr = format!("{}:{}", host, port);
    let mut stream =
        TcpStream::connect(&addr).map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}:{}\r\nConnection: keep-alive\r\n\r\n",
        host, port
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // Read HTTP headers
    let cloned_stream = stream
        .try_clone()
        .map_err(|e| format!("Failed to clone stream: {}", e))?;
    let mut reader = BufReader::new(cloned_stream);
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
pub fn run_http_audio_loop(
    host: &str,
    port: u16,
    initial_format: &AudioInputFormat,
    eq_gains: EqGains,
    running: Arc<AtomicBool>,
    global_volume: Arc<std::sync::atomic::AtomicU8>,
    vis_buffer: Option<Arc<Mutex<VecDeque<f32>>>>,
) -> Result<(), String> {
    // Get output device
    let audio_host = cpal::default_host();
    let device = audio_host
        .default_output_device()
        .ok_or("No output device available")?;

    let mut _current_stream: Option<cpal::Stream> = None;
    let mut current_sample_rate = initial_format.sample_rate;
    let mut current_channels = initial_format.channels;

    let ring_buffer = Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::<f32>::with_capacity(32768),
    ));
    
    let fade_level = Arc::new(std::sync::atomic::AtomicU32::new(0));
    
    // VISUALIZER: Clone buffer ref
    let vis_buffer_orig = vis_buffer.clone();

    // Helper to build stream with correct params for HTTP loop
    let build_stream = |sample_rate: u32, channels: u16| -> Result<cpal::Stream, String> {
        let stream_config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        build_audio_stream(
            &device,
            &stream_config,
            ring_buffer.clone(),
            fade_level.clone(),
            global_volume.clone(),
            vis_buffer_orig.clone(),
            0.005, // FADE_SPEED for HTTP
        )
    };

    // Initial stream build (fallback)
    _current_stream = Some(build_stream(current_sample_rate, current_channels)?);

    let mut read_buffer = vec![0u8; 8192];
    
    // EQ instance for processing loop (needs to match sample rate too!)
    // We'll recreate it if rate changes.
    let mut processing_eq = DspEqualizer::new(current_sample_rate as f32, eq_gains.clone());

    while running.load(Ordering::SeqCst) {
        let tcp_stream = match connect_to_http_stream(host, port) {
            Ok(s) => s,
            Err(_) => {
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let mut reader = BufReader::with_capacity(16384, tcp_stream);

        // 1. Read WAV Header (44 bytes) for TRUTH 🕵️‍♂️
        let mut header = [0u8; 44];
        if reader.read_exact(&mut header).is_err() {
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        
        // 2. Parse Format
        let mut new_rate = current_sample_rate;
        let mut new_channels = current_channels;
        
        // Check for RIFF/WAVE signature
        if &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" && &header[12..16] == b"fmt " {
             // channels at offset 22 (u16)
             new_channels = u16::from_le_bytes([header[22], header[23]]);
             // sample rate at offset 24 (u32)
             new_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
             
             // Sanity checks
             if new_channels == 0 || new_channels > 8 { new_channels = 2; }
             if !(8000..=192000).contains(&new_rate) { new_rate = 44100; }
        }

        // 3. Reconfigure Stream if changed
        if new_rate != current_sample_rate || new_channels != current_channels {
             eprintln!("⟳ Audio Format Changed: {}Hz / {}ch", new_rate, new_channels);
             current_sample_rate = new_rate;
             current_channels = new_channels;
             
             // Update EQ for processing loop
             processing_eq = DspEqualizer::new(new_rate as f32, eq_gains.clone());
             
             // Rebuild cpal stream
             // Dropping old stream (by overwriting Option) stops it
             _current_stream = match build_stream(new_rate, new_channels) {
                 Ok(s) => Some(s),
                 Err(e) => {
                     eprintln!("Failed to rebuild stream: {}", e);
                     // Try to keep old one? Or fallback?
                     // If rebuild fails, we are kind of stuck. Wait and retry?
                     thread::sleep(Duration::from_secs(1));
                     continue;
                 }
             };
        }

        if let Ok(mut buffer) = ring_buffer.lock() {
            buffer.clear();
        }

        while running.load(Ordering::SeqCst) {
            match reader.read(&mut read_buffer) {
                Ok(0) => {
                    processing_eq.reset_filters();
                    break;
                }
                Ok(bytes_read) => {
                    // 16-bit PCM to f32
                    let samples = bytes_read / 2;
                    let mut float_buffer = Vec::with_capacity(samples);

                    for i in 0..samples {
                        let idx = i * 2;
                        if idx + 1 < bytes_read {
                            let sample_i16 =
                                i16::from_le_bytes([read_buffer[idx], read_buffer[idx + 1]]);
                            float_buffer.push(sample_i16 as f32 / 32768.0);
                        }
                    }

                    processing_eq.process_buffer(&mut float_buffer);

                    // Backpressure: Wait for space (prevents skipping) 🛑
                    let max_buffer_size = 32768;
                    loop {
                        let len = ring_buffer.lock().map(|b| b.len()).unwrap_or(0);
                        // Allow slight overflow for current batch
                        if len < max_buffer_size {
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                        if !running.load(Ordering::SeqCst) {
                             processing_eq.reset_filters(); // Ensure cleanup
                             return Ok(()); 
                        }
                    }

                    if let Ok(mut buffer) = ring_buffer.lock() {
                        for sample in float_buffer {
                            buffer.push_back(sample);
                        }
                        // Safety valve: only drop if we are excessively behind (e.g. 2x)
                        while buffer.len() > max_buffer_size * 2 {
                            buffer.pop_front();
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    continue
                }
                Err(_) => {
                    processing_eq.reset_filters();
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}
