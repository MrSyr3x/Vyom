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
pub fn build_audio_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    fade_level: Arc<AtomicU32>,
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
