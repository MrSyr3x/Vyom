use super::common::build_audio_stream;
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
#[allow(clippy::too_many_arguments)]
pub fn run_fifo_audio_loop(
    fifo_path: &str,
    format: &AudioInputFormat,
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

    let current_sample_rate = format.sample_rate;
    let current_bits_per_sample = format.bits_per_sample;
    let current_channels = format.channels;

    tracing::info!(
        "🎵 FIFO Audio: {}Hz/{}bit/{}ch (Fixed format pipeline)",
        current_sample_rate,
        current_bits_per_sample,
        current_channels
    );

    // Use detected sample rate for bit-perfect output
    let stream_config = StreamConfig {
        channels: current_channels,
        sample_rate: cpal::SampleRate(current_sample_rate),
        buffer_size: cpal::BufferSize::Fixed(1024),
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
        0.001, // FADE_SPEED for FIFO (~30ms fade-in at 44100Hz)
        flush_signal.clone(),
    )?;

    // Calculate bytes per sample based on detected bit depth
    let bytes_per_sample_val = (current_bits_per_sample / 8) as usize;
    let frame_size = bytes_per_sample_val * current_channels as usize;
    let buffer_frames = 2048;
    // Buffer needs to adapt. We start with max reasonable size?
    let mut read_buffer = vec![0u8; frame_size * buffer_frames];

    let mut _active_stream = stream; // Keep stream alive

    while running.load(Ordering::SeqCst) {
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

        // Drain stale FIFO OS kernel buffer on reopen 🔇
        // After a pause/resume cycle, the kernel FIFO pipe buffer may contain
        // old discontinuous PCM data. Read and discard everything immediately
        // available before feeding into the ring buffer.
        {
            let mut drain_buf = [0u8; 65536];
            loop {
                match reader.read(&mut drain_buf) {
                    Ok(0) => break,
                    Ok(_) => continue, // Discard stale data
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }

        // Frame alignment accumulator: carries leftover bytes between reads
        let mut leftover = Vec::<u8>::with_capacity(frame_size);

        while running.load(Ordering::SeqCst) {
            // Immediate Audio Flush Check ⚡
            if flush_signal.load(Ordering::SeqCst) {
                flush_signal.store(false, Ordering::SeqCst);
                if let Ok(mut buffer) = ring_buffer.lock() {
                    buffer.clear(); // Drop old frames
                }
                // Reset fade to 0 so audio fades in smoothly on resume
                // instead of popping at full volume
                fade_level.store(0f32.to_bits(), Ordering::SeqCst);
                equalizer.reset_filters();
                // Break to reopen FIFO and drop OS kernel buffer
                break;
            }

            match reader.read(&mut read_buffer) {
                Ok(0) => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Ok(bytes_read) => {
                    // Frame alignment: prepend leftover bytes from previous read.
                    // FIFO read() returns arbitrary byte counts that may not be
                    // frame-aligned. Without this, remainder bytes are silently
                    // dropped, causing sustained sample misalignment → distortion.
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

                    let total_bytes = work_buf.len();
                    let aligned_bytes = (total_bytes / frame_size) * frame_size;
                    let remainder = total_bytes - aligned_bytes;

                    // Save unaligned trailing bytes for the next iteration
                    if remainder > 0 {
                        leftover.extend_from_slice(&work_buf[aligned_bytes..]);
                    }

                    let frames = aligned_bytes / frame_size;

                    float_buffer.clear();

                    for frame in 0..frames {
                        for ch in 0..current_channels as usize {
                            let offset = frame * frame_size + ch * bytes_per_sample_val;

                            let sample_f32 = match current_bits_per_sample {
                                16 => {
                                    let s = i16::from_le_bytes([
                                        work_buf[offset],
                                        work_buf[offset + 1],
                                    ]);
                                    s as f32 / 32768.0
                                }
                                24 => {
                                    let b0 = work_buf[offset] as i32;
                                    let b1 = work_buf[offset + 1] as i32;
                                    let b2 = work_buf[offset + 2] as i32;
                                    let s = (b2 << 24) | (b1 << 16) | (b0 << 8);
                                    (s >> 8) as f32 / 8388608.0
                                }
                                32 => {
                                    let s = i32::from_le_bytes([
                                        work_buf[offset],
                                        work_buf[offset + 1],
                                        work_buf[offset + 2],
                                        work_buf[offset + 3],
                                    ]);
                                    s as f32 / 2147483648.0
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
                        if flush_signal.load(Ordering::SeqCst) {
                            break; // Abort wait immediately
                        }
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
