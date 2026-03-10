use crate::audio::visualizer::Visualizer;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::StreamConfig;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
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

/// Helper to build audio output stream with consistent volume/fade/visualizer logic
#[allow(clippy::too_many_arguments)]
pub fn build_audio_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    fade_level: Arc<AtomicU32>,
    global_volume: Arc<std::sync::atomic::AtomicU8>,
    vis_buffer: Option<Arc<Mutex<VecDeque<f32>>>>,
    fade_speed: f32,
    flush_signal: Arc<std::sync::atomic::AtomicBool>,
) -> Result<cpal::Stream, String> {
    let rb_clone = ring_buffer.clone();
    let fl_clone = fade_level.clone();
    let gv_clone = global_volume.clone();
    let vb_clone = vis_buffer.clone();
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0;

    // ═══════════════════════════════════════════════════════════════════
    // PRE-FILL: Seed the ring buffer with silence BEFORE the stream
    // starts. This prevents the POP/buzz that occurs when the cpal
    // callback fires with an empty buffer and then data suddenly arrives
    // (step discontinuity from 0 → signal).
    //
    // We fill with ~50ms of silence (2205 samples at 44100Hz stereo).
    // This gives the reader thread time to start pushing real audio
    // data, so the transition is smooth: silence → fade-in → audio.
    // ═══════════════════════════════════════════════════════════════════
    let prefill_samples = (sample_rate as usize * channels) / 20; // ~50ms
    if let Ok(mut buffer) = ring_buffer.lock() {
        for _ in 0..prefill_samples {
            buffer.push_back(0.0);
        }
    }

    // Ensure fade starts at zero for a clean ramp-up
    fade_level.store(0f32.to_bits(), std::sync::atomic::Ordering::SeqCst);

    // Post-flush mute: number of samples remaining to output as silence.
    // When flush fires, this is set to ~100ms worth of samples.
    // The callback drains any ring buffer data as silence until this reaches 0.
    let mute_remaining = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let mute_clone = mute_remaining.clone();
    let flush_clone = flush_signal.clone();

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Audio Thread: Must run fast! ⚡

                // Check if a flush just happened — if so, activate mute window
                if flush_clone.load(Ordering::Relaxed) {
                    // Don't clear the flag here (reader thread does that),
                    // just set the mute window
                    let mute_samples = sample_rate / 10; // ~100ms of silence
                    mute_clone.store(mute_samples, Ordering::Relaxed);
                }

                let mute_left = mute_clone.load(Ordering::Relaxed);

                if mute_left > 0 {
                    // POST-FLUSH MUTE: Output pure silence and drain ring buffer
                    // This eliminates buzz from stale data that was already in the
                    // ring buffer when flush fired
                    if let Ok(mut buffer) = rb_clone.lock() {
                        // Drain stale samples so they don't play after mute ends
                        let drain_count = data.len().min(buffer.len());
                        buffer.drain(..drain_count);
                    }
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    // Reset fade to 0 during mute so we get a clean fade-in after
                    fl_clone.store(0f32.to_bits(), Ordering::Relaxed);
                    let consumed = (data.len() as u32).min(mute_left);
                    mute_clone.store(mute_left - consumed, Ordering::Relaxed);
                    return;
                }

                if let Ok(mut buffer) = rb_clone.lock() {
                    let mut fade = f32::from_bits(fl_clone.load(Ordering::Relaxed));
                    let vol = gv_clone.load(Ordering::Relaxed);
                    let gain = (vol as f32 / 100.0).powf(3.0); // Cubic volume curve

                    for sample in data.iter_mut() {
                        if let Some(s) = buffer.pop_front() {
                            if fade < 1.0 {
                                fade = (fade + fade_speed).min(1.0);
                            }
                            *sample = s * fade * gain;
                        } else {
                            // Buffer underrun: output silence, fade down
                            if fade > 0.0 {
                                fade = (fade - fade_speed).max(0.0);
                            }
                            *sample = 0.0;
                        }
                    }
                    // Save fade state
                    fl_clone.store(fade.to_bits(), Ordering::Relaxed);
                } else {
                    // CRITICAL FIX: If lock fails, output silence instead of garbage/repeat
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                }

                // Visualize (Post-fill)
                if let Some(vis) = &vb_clone {
                    Visualizer::push_samples(vis, data, channels);
                }
            },
            |err| tracing::error!("Audio stream error: {}", err),
            None,
        )
        .map_err(|e| format!("Failed to build output stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to play stream: {}", e))?;
    Ok(stream)
}
