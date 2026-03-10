use super::common::build_audio_stream;
use crate::audio::dsp::{DspEqualizer, EqGains};
use crate::audio::types::AudioInputFormat;
use cpal::traits::HostTrait;
use cpal::StreamConfig;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Connect to MPD HTTP stream and return a BufReader that preserves all data.
/// 
/// CRITICAL: We return the BufReader directly (not the raw TcpStream) because
/// the BufReader pre-reads data into its internal buffer during header parsing.
/// Returning the raw stream would lose that pre-read audio data, causing a
/// ~1 second buzz/distortion on startup.
fn connect_to_http_stream(host: &str, port: u16) -> Result<BufReader<TcpStream>, String> {
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

    // Wrap in BufReader FIRST, then read headers through it.
    // This way any audio data that gets pre-read into the BufReader's
    // internal buffer stays available for the caller.
    let mut reader = BufReader::with_capacity(16384, stream);
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

    Ok(reader)
}

/// HTTP audio loop (16-bit PCM)
#[cfg(feature = "eq")]
#[allow(clippy::too_many_arguments)]
pub fn run_http_audio_loop(
    host: &str,
    port: u16,
    initial_format: &AudioInputFormat,
    eq_gains: EqGains,
    running: Arc<AtomicBool>,
    global_volume: Arc<std::sync::atomic::AtomicU8>,
    vis_buffer: Option<Arc<Mutex<VecDeque<f32>>>>,
    flush_signal: Arc<AtomicBool>,
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
    let flush_sig_orig = flush_signal.clone();
    let build_stream = |sample_rate: u32, channels: u16| -> Result<cpal::Stream, String> {
        let stream_config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Fixed(1024),
        };

        build_audio_stream(
            &device,
            &stream_config,
            ring_buffer.clone(),
            fade_level.clone(),
            global_volume.clone(),
            vis_buffer_orig.clone(),
            0.001, // FADE_SPEED for HTTP (~30ms fade-in at 44100Hz)
            flush_sig_orig.clone(),
        )
    };

    // Initial stream build (fallback)
    _current_stream = Some(build_stream(current_sample_rate, current_channels)?);

    let mut read_buffer = vec![0u8; 8192];

    // EQ instance for processing loop (needs to match sample rate too!)
    // We'll recreate it if rate changes.
    let mut processing_eq = DspEqualizer::new(current_sample_rate as f32, eq_gains.clone());

    while running.load(Ordering::SeqCst) {
        let mut reader = match connect_to_http_stream(host, port) {
            Ok(r) => r,
            Err(_) => {
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

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
            if new_channels == 0 || new_channels > 8 {
                new_channels = 2;
            }
            if !(8000..=192000).contains(&new_rate) {
                new_rate = 44100;
            }
        }

        // 3. Reconfigure Stream if changed
        if new_rate != current_sample_rate || new_channels != current_channels {
            tracing::info!(
                "⟳ Audio Format Changed: {}Hz / {}ch",
                new_rate, new_channels
            );
            current_sample_rate = new_rate;
            current_channels = new_channels;

            // Update EQ for processing loop
            processing_eq = DspEqualizer::new(new_rate as f32, eq_gains.clone());

            // Rebuild cpal stream
            // Dropping old stream (by overwriting Option) stops it
            _current_stream = match build_stream(new_rate, new_channels) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::error!("Failed to rebuild stream: {}", e);
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

        // Frame alignment accumulator: carries leftover bytes between reads.
        // TCP read() returns arbitrary byte counts (e.g. 8191 bytes for 16-bit
        // stereo where frame_size=4). Without this, remainder bytes are silently
        // dropped, causing sustained sample misalignment → distortion/buzz.
        let frame_size = (current_channels * 2) as usize; // 16-bit = 2 bytes per sample
        let mut leftover = Vec::<u8>::with_capacity(frame_size);

        while running.load(Ordering::SeqCst) {
            // Immediate Audio Flush Check ⚡
            if flush_signal.load(Ordering::SeqCst) {
                flush_signal.store(false, Ordering::SeqCst);
                if let Ok(mut buffer) = ring_buffer.lock() {
                    buffer.clear(); // Drop 1s+ of old audio
                }
                // Reset fade to 0 so audio fades in smoothly on resume
                fade_level.store(0f32.to_bits(), Ordering::SeqCst);
                processing_eq.reset_filters();
                leftover.clear(); // Reset alignment state
                // Break out of the socket read loop to force MPD buffer drop!
                break;
            }

            match reader.read(&mut read_buffer) {
                Ok(0) => {
                    processing_eq.reset_filters();
                    break;
                }
                Ok(bytes_read) => {
                    // Prepend any leftover bytes from the previous read
                    let work_buf: &[u8];
                    let mut combined;
                    if leftover.is_empty() {
                        work_buf = &read_buffer[..bytes_read];
                    } else {
                        combined = Vec::with_capacity(leftover.len() + bytes_read);
                        combined.extend_from_slice(&leftover);
                        combined.extend_from_slice(&read_buffer[..bytes_read]);
                        leftover.clear();
                        work_buf = &combined;
                    }

                    // Mid-stream WAV header detection 🛡️
                    // MPD httpd with `always_on "yes"` resends a 44-byte WAV
                    // header when tracks change. If we decode those bytes as
                    // PCM samples, they produce audible garbage/buzz.
                    // Scan for embedded RIFF headers and skip past them.
                    let pcm_start = if work_buf.len() >= 44 {
                        // Check first 4 bytes for RIFF signature
                        if &work_buf[0..4] == b"RIFF" {
                            tracing::debug!("Skipped mid-stream WAV header (44 bytes)");
                            44 // Skip the entire WAV header
                        } else {
                            // Also scan for RIFF embedded deeper in the buffer
                            // (could arrive mid-read)
                            let mut skip_to = 0;
                            for pos in 0..work_buf.len().saturating_sub(44) {
                                if &work_buf[pos..pos + 4] == b"RIFF" {
                                    // Found embedded header — skip everything
                                    // up to and including it
                                    skip_to = pos + 44;
                                    tracing::debug!("Skipped embedded WAV header at offset {}", pos);
                                    break;
                                }
                            }
                            skip_to
                        }
                    } else {
                        0
                    };

                    let pcm_data = &work_buf[pcm_start..];
                    let total_bytes = pcm_data.len();
                    let aligned_bytes = (total_bytes / frame_size) * frame_size;
                    let remainder = total_bytes - aligned_bytes;

                    // Save unaligned trailing bytes for the next iteration
                    if remainder > 0 {
                        leftover.extend_from_slice(&pcm_data[aligned_bytes..]);
                    }

                    // 16-bit PCM to f32 (only process frame-aligned bytes)
                    let samples = aligned_bytes / 2;
                    let mut float_buffer = Vec::with_capacity(samples);

                    for i in 0..samples {
                        let idx = i * 2;
                        let sample_i16 =
                            i16::from_le_bytes([pcm_data[idx], pcm_data[idx + 1]]);
                        float_buffer.push(sample_i16 as f32 / 32768.0);
                    }

                    processing_eq.process_buffer(&mut float_buffer);

                    // Backpressure: Wait for space (prevents skipping) 🛑
                    let max_buffer_size = 32768;
                    loop {
                        if flush_signal.load(Ordering::SeqCst) {
                            break; // Abort wait immediately!
                        }
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
