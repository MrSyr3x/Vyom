use super::dsp::{DspEqualizer, EqGains};
use super::types::AudioInputFormat;
use super::visualizer::Visualizer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Query MPD for current audio format
/// Returns (sample_rate, bits_per_sample, channels)
pub fn query_mpd_format() -> Option<(u32, u16, u16)> {
    let mut stream = TcpStream::connect("127.0.0.1:6600").ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok()?;

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

    parse_mpd_status(&response)
}

/// Pure function to parse MPD status response 🧪
fn parse_mpd_status(response: &str) -> Option<(u32, u16, u16)> {
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

/// FIFO audio loop (Hi-Res 16/24/32-bit)
#[cfg(feature = "eq")]
pub fn run_fifo_audio_loop(
    fifo_path: &str,
    format: &AudioInputFormat,
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

    // Query MPD for actual format (dynamic detection!)
    let (initial_sample_rate, initial_bits, initial_channels) =
        query_mpd_format().unwrap_or((format.sample_rate, format.bits_per_sample, format.channels));

    eprintln!(
        "🎵 Hi-Res Audio: {}Hz/{}bit/{}ch (bit-perfect)",
        initial_sample_rate, initial_bits, initial_channels
    );

    // Dynamic State
    let mut current_sample_rate = initial_sample_rate;
    let mut current_bits_per_sample = initial_bits;
    let mut current_channels = initial_channels;

    // Use detected sample rate for bit-perfect output
    let stream_config = StreamConfig {
        channels: current_channels,
        sample_rate: cpal::SampleRate(current_sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    // Create EQ at correct sample rate
    // Clone eq_gains because we might need it again later for dynamic updates
    let mut equalizer = DspEqualizer::new(current_sample_rate as f32, eq_gains.clone());

    let ring_buffer = Arc::new(std::sync::Mutex::new(
        std::collections::VecDeque::<f32>::with_capacity(65536), // Larger for Hi-Res
    ));

    let fade_level = Arc::new(std::sync::atomic::AtomicU32::new(0));

    // Initial Stream
    let stream = build_audio_stream(
        &device,
        &stream_config,
        ring_buffer.clone(),
        fade_level.clone(),
        global_volume.clone(),
        vis_buffer.clone(),
        0.003, // FADE_SPEED for FIFO
    )?;

    // Calculate bytes per sample based on detected bit depth
    let mut bytes_per_sample_val = (current_bits_per_sample / 8) as usize;
    let mut frame_size = bytes_per_sample_val * current_channels as usize;
    let buffer_frames = 2048;
    // Buffer needs to adapt. We start with max reasonable size?
    let mut read_buffer = vec![0u8; frame_size * buffer_frames];

    // State for dynamic rate detection
    let mut last_format_check = std::time::Instant::now();
    let format_check_interval = std::time::Duration::from_secs(2);
    let mut _active_stream = stream; // Keep stream alive

    while running.load(Ordering::SeqCst) {
        // Dynamic Format Check 🕵️‍♂️ (Poll MPD periodically)
        if last_format_check.elapsed() > format_check_interval {
            if let Some((new_rate, new_bits, new_ch)) = query_mpd_format() {
                if new_rate != current_sample_rate || new_bits != current_bits_per_sample || new_ch != current_channels {
                     eprintln!("⟳ Audio Format Changed: {}Hz/{}bit/{}ch", new_rate, new_bits, new_ch);
                     
                     // 1. Update State
                     current_sample_rate = new_rate;
                     current_bits_per_sample = new_bits;
                     current_channels = new_ch;
                     
                     bytes_per_sample_val = (current_bits_per_sample / 8) as usize;
                     frame_size = bytes_per_sample_val * current_channels as usize;
                     
                     // 2. Re-allocate Read Buffer
                     read_buffer = vec![0u8; frame_size * buffer_frames];
                     
                     // 3. Re-create EQ
                     equalizer = DspEqualizer::new(current_sample_rate as f32, eq_gains.clone());
                     
                     // 4. Rebuild Stream
                     // Note: We can't easily change channels on the fly without complex buffer re-mapping if cpal doesn't support it.
                     // For now, we assume channels (stereo) don't change often.
                     // Restarting loop is hard here. Let's try to rebuild stream.
                     
                     let new_config = StreamConfig {
                        channels: current_channels,
                        sample_rate: cpal::SampleRate(current_sample_rate),
                        buffer_size: cpal::BufferSize::Default,
                    };
                    
                    match build_audio_stream(
                        &device,
                        &new_config,
                        ring_buffer.clone(),
                        fade_level.clone(),
                        global_volume.clone(),
                        vis_buffer.clone(),
                        0.003, // FADE_SPEED for FIFO
                    ) {
                        Ok(s) => {
                            _active_stream = s; // Replace old stream
                        },
                        Err(e) => eprintln!("Failed to rebuild stream: {}", e),
                    }
                }
            }
            last_format_check = std::time::Instant::now();
        }

        // Open FIFO (blocking)
        let fifo = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(fifo_path)
        {
            Ok(f) => f,
            Err(_e) => {
                // If FIFO is gone (MPD restarted?), sleep and retry
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        let mut reader = BufReader::with_capacity(65536, fifo);
        let mut float_buffer = Vec::with_capacity(buffer_frames * current_channels as usize);

        while running.load(Ordering::SeqCst) {
             
            match reader.read(&mut read_buffer) {
                Ok(0) => {
                    thread::sleep(Duration::from_millis(10));
                    // Check format here too if stalled?
                    if last_format_check.elapsed() > format_check_interval {
                        break; // Break inner loop to check format in outer loop
                    }
                    continue;
                }
                Ok(bytes_read) => {
                    let frames = bytes_read / frame_size;
                    
                    float_buffer.clear();
                    
                    for frame in 0..frames {
                        for ch in 0..current_channels as usize {
                            let offset = frame * frame_size + ch * bytes_per_sample_val;

                            let sample_f32 = match current_bits_per_sample {
                                16 => {
                                    if offset + 1 < bytes_read {
                                        let s = i16::from_le_bytes([read_buffer[offset], read_buffer[offset + 1]]);
                                        s as f32 / 32768.0
                                    } else { 0.0 }
                                },
                                24 => {
                                    if offset + 2 < bytes_read {
                                        let b0 = read_buffer[offset] as i32;
                                        let b1 = read_buffer[offset + 1] as i32;
                                        let b2 = read_buffer[offset + 2] as i32;
                                        let s = (b2 << 24) | (b1 << 16) | (b0 << 8);
                                        (s >> 8) as f32 / 8388608.0
                                    } else { 0.0 }
                                },
                                32 => {
                                     if offset + 3 < bytes_read {
                                        let s = i32::from_le_bytes([read_buffer[offset], read_buffer[offset+1], read_buffer[offset+2], read_buffer[offset+3]]);
                                        s as f32 / 2147483648.0
                                    } else { 0.0 }
                                },
                                _ => 0.0
                            };
                            float_buffer.push(sample_f32);
                        }
                    }

                    equalizer.process_buffer(&mut float_buffer);

                    // Backpressure: Wait for space 🛑
                    let max_size = 65536;
                    loop {
                        let len = ring_buffer.lock().map(|b| b.len()).unwrap_or(0);
                        if len < max_size {
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                        if !running.load(Ordering::SeqCst) {
                            equalizer.reset_filters();
                            return Ok(());
                        }
                    }

                    if let Ok(mut buffer) = ring_buffer.lock() {
                        for sample in &float_buffer {
                            buffer.push_back(*sample);
                        }
                        while buffer.len() > max_size * 2 {
                            buffer.pop_front();
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                     thread::sleep(Duration::from_millis(5));
                     if last_format_check.elapsed() > format_check_interval {
                        break; // Break to check format
                     }
                     continue;
                }
                Err(_) => {
                    equalizer.reset_filters();
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(100)); // Sleep if outer loop continues (re-open fifo)
    }

    Ok(())
}

/// Helper to build audio output stream with consistent volume/fade/visualizer logic
fn build_audio_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    fade_level: Arc<std::sync::atomic::AtomicU32>,
    global_volume: Arc<std::sync::atomic::AtomicU8>,
    vis_buffer: Option<Arc<Mutex<VecDeque<f32>>>>,
    fade_speed: f32,
) -> Result<cpal::Stream, String> {
    let rb_clone = ring_buffer.clone();
    let fl_clone = fade_level.clone();
    let gv_clone = global_volume.clone();
    let vb_clone = vis_buffer.clone();
    let channels = config.channels as usize;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buffer = if let Ok(buf) = rb_clone.lock() {
                    buf
                } else {
                    return;
                };
                let mut fade = f32::from_bits(fl_clone.load(Ordering::Relaxed));

                // Calculate Gain 🎚️
                let vol = gv_clone.load(Ordering::Relaxed);
                let gain = (vol as f32 / 100.0).powf(3.0);

                for sample in data.iter_mut() {
                    if let Some(s) = buffer.pop_front() {
                        if fade < 1.0 {
                            fade = (fade + fade_speed).min(1.0);
                        }
                        *sample = s * fade * gain;
                    } else {
                        if fade > 0.0 {
                            fade = (fade - fade_speed).max(0.0);
                        }
                        *sample = 0.0;
                    }
                }

                // Visualize
                if let Some(vis) = &vb_clone {
                     Visualizer::push_samples(vis, data, channels);
                }

                fl_clone.store(fade.to_bits(), Ordering::Relaxed);
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )
        .map_err(|e| format!("Failed to build output stream: {}", e))?;
        
    stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
    Ok(stream)
}
