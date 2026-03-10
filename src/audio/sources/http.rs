use super::common::build_audio_stream;
use crate::audio::dsp::{DspEqualizer, EqGains};
use crate::audio::types::AudioInputFormat;
use cpal::traits::HostTrait;
use cpal::StreamConfig;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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

    // EQ instance for processing loop (needs to match sample rate too!)
    // We'll recreate it if rate changes.
    let mut processing_eq = DspEqualizer::new(current_sample_rate as f32, eq_gains.clone());

    while running.load(Ordering::SeqCst) {
        let reader = match connect_to_http_stream(host, port) {
            Ok(r) => r,
            Err(_) => {
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let mss = MediaSourceStream::new(
            Box::new(ReadOnlySource::new(reader)) as Box<dyn MediaSource>,
            Default::default()
        );
        
        // Wait for buffer flush signal before starting decode
        if flush_signal.load(Ordering::SeqCst) {
            flush_signal.store(false, Ordering::SeqCst);
            if let Ok(mut buffer) = ring_buffer.lock() {
                buffer.clear();
            }
            fade_level.store(0f32.to_bits(), Ordering::SeqCst);
            processing_eq.reset_filters();
        }

        let hint = Hint::new();
        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();

        let probed = match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!("Symphonia probe failed (maybe end of stream?): {:?}", e);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        let mut format = probed.format;
        
        // Find the first audio track
        let track = match format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL) {
            Some(t) => t.clone(),
            None => {
                tracing::error!("No audio track found in stream!");
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let track_id = track.id;
        let p_sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let p_channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        let dec_opts = DecoderOptions::default();
        let mut decoder = match symphonia::default::get_codecs().make(&track.codec_params, &dec_opts) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to create decoder: {:?}", e);
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        // Stream Reconfiguration
        if p_sample_rate != current_sample_rate || p_channels != current_channels {
            tracing::info!("⟳ Audio Format Changed: {}Hz / {}ch", p_sample_rate, p_channels);
            current_sample_rate = p_sample_rate;
            current_channels = p_channels;

            processing_eq = DspEqualizer::new(current_sample_rate as f32, eq_gains.clone());

            _current_stream = match build_stream(current_sample_rate, current_channels) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::error!("Failed to rebuild cpal stream: {}", e);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
        }

        if let Ok(mut buffer) = ring_buffer.lock() {
            buffer.clear();
        }

        let mut sample_buf: Option<symphonia::core::audio::SampleBuffer<f32>> = None;

        // Packet decode loop
        while running.load(Ordering::SeqCst) {
            if flush_signal.load(Ordering::SeqCst) {
                flush_signal.store(false, Ordering::SeqCst);
                if let Ok(mut buffer) = ring_buffer.lock() {
                    buffer.clear();
                }
                fade_level.store(0f32.to_bits(), Ordering::SeqCst);
                processing_eq.reset_filters();
                break; // Break the internal decode loop to reconnect the HTTP socket
            }

            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => {
                    tracing::debug!("Stream ended or Error: {:?} - Reconnecting...", e);
                    processing_eq.reset_filters();
                    break;
                }
            };

            if packet.track_id() != track_id {
                continue; // Not our audio track
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Convert decoded audio to f32 samples
                    if sample_buf.is_none() || sample_buf.as_ref().unwrap().capacity() < decoded.capacity() {
                        let spec = *decoded.spec();
                        let duration = decoded.capacity() as u64;
                        sample_buf = Some(symphonia::core::audio::SampleBuffer::<f32>::new(duration, spec));
                    }

                    if let Some(buf) = &mut sample_buf {
                        buf.copy_interleaved_ref(decoded);
                        let samples = buf.samples();

                        let mut float_buffer = samples.to_vec();

                        processing_eq.process_buffer(&mut float_buffer);

                        // Backpressure: Wait for space 🛑
                        let max_buffer_size = 32768;
                        loop {
                            if flush_signal.load(Ordering::SeqCst) {
                                break; 
                            }
                            let len = ring_buffer.lock().map(|b| b.len()).unwrap_or(0);
                            if len < max_buffer_size {
                                break;
                            }
                            thread::sleep(Duration::from_millis(5));
                            if !running.load(Ordering::SeqCst) {
                                processing_eq.reset_filters();
                                return Ok(());
                            }
                        }

                        if let Ok(mut buffer) = ring_buffer.lock() {
                            buffer.extend(float_buffer);
                            while buffer.len() > max_buffer_size * 2 {
                                buffer.pop_front();
                            }
                        }
                    }
                }
                Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    tracing::debug!("Decoder hit EOF. MPD stream wrapped.");
                    break; // Reconnect
                }
                Err(SymphoniaError::DecodeError(e)) => {
                    tracing::debug!("Decode Error: {:?}. Ignoring packet.", e);
                    continue;
                }
                Err(e) => {
                    tracing::error!("Symphonia decode error: {:?}", e);
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}
