use super::common::{build_audio_stream, query_mpd_format};
use crate::audio::dsp::{DspEqualizer, EqGains};
use crate::audio::types::AudioInputFormat;
use cpal::traits::HostTrait;
use cpal::StreamConfig;
use std::collections::VecDeque;
use std::io::{BufReader, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
                if new_rate != current_sample_rate
                    || new_bits != current_bits_per_sample
                    || new_ch != current_channels
                {
                    eprintln!(
                        "⟳ Audio Format Changed: {}Hz/{}bit/{}ch",
                        new_rate, new_bits, new_ch
                    );

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
                        }
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
                                        let s = i16::from_le_bytes([
                                            read_buffer[offset],
                                            read_buffer[offset + 1],
                                        ]);
                                        s as f32 / 32768.0
                                    } else {
                                        0.0
                                    }
                                }
                                24 => {
                                    if offset + 2 < bytes_read {
                                        let b0 = read_buffer[offset] as i32;
                                        let b1 = read_buffer[offset + 1] as i32;
                                        let b2 = read_buffer[offset + 2] as i32;
                                        let s = (b2 << 24) | (b1 << 16) | (b0 << 8);
                                        (s >> 8) as f32 / 8388608.0
                                    } else {
                                        0.0
                                    }
                                }
                                32 => {
                                    if offset + 3 < bytes_read {
                                        let s = i32::from_le_bytes([
                                            read_buffer[offset],
                                            read_buffer[offset + 1],
                                            read_buffer[offset + 2],
                                            read_buffer[offset + 3],
                                        ]);
                                        s as f32 / 2147483648.0
                                    } else {
                                        0.0
                                    }
                                }
                                _ => 0.0,
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
