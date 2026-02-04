use super::dsp::EqGains;
use super::sources::{run_fifo_audio_loop, run_http_audio_loop};
use super::types::{AudioInputFormat, AudioPipelineConfig, AudioSource};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Audio pipeline with Hi-Res support
pub struct AudioPipeline {
    config: AudioPipelineConfig,
    eq_gains: EqGains,
    running: Arc<AtomicBool>,
    pub global_volume: Arc<std::sync::atomic::AtomicU8>,
    thread_handle: Option<thread::JoinHandle<()>>,
    /// Shared buffer for visualizer
    pub vis_buffer: Option<Arc<Mutex<VecDeque<f32>>>>,
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
            vis_buffer: None,
        }
    }

    /// Create pipeline with FIFO source for Hi-Res
    #[allow(dead_code)]
    pub fn with_fifo(eq_gains: EqGains, fifo_path: &str, format: AudioInputFormat) -> Self {
        Self {
            config: AudioPipelineConfig {
                source: AudioSource::Fifo {
                    path: fifo_path.to_string(),
                },
                format,
            },
            eq_gains,
            running: Arc::new(AtomicBool::new(false)),
            global_volume: Arc::new(std::sync::atomic::AtomicU8::new(100)),
            thread_handle: None,
            vis_buffer: None,
        }
    }

    /// Attach visualizer buffer
    pub fn attach_visualizer(&mut self, buffer: Arc<Mutex<VecDeque<f32>>>) {
        self.vis_buffer = Some(buffer);
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
        let vis_buffer = self.vis_buffer.clone();

        running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            let result = match source {
                AudioSource::Http { host, port } => run_http_audio_loop(
                    &host,
                    port,
                    &format,
                    eq_gains,
                    running.clone(),
                    global_volume,
                    vis_buffer,
                ),
                AudioSource::Fifo { path } => run_fifo_audio_loop(
                    &path,
                    &format,
                    eq_gains,
                    running.clone(),
                    global_volume,
                    vis_buffer,
                ),
            };

            if let Err(e) = result {
                eprintln!("Audio pipeline error: {}", e);
            }
            running.store(false, Ordering::SeqCst);
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    // Fallback for non-eq feature
    #[cfg(not(feature = "eq"))]
    pub fn start(&mut self) -> Result<(), String> {
        Err("EQ feature not enabled".to_string())
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
