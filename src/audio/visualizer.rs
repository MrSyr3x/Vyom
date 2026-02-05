use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, FrequencyLimit};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Native FFT Visualizer State
pub struct Visualizer {
    /// Lock-protected buffer of incoming audio samples
    /// We keep a rolling window of 2048 or 4096 samples for FFT
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,

    /// Sample rate (typically 44100 or 48000)
    sample_rate: u32,

    /// FFT Size (must be power of 2, e.g., 2048)
    fft_size: usize,

    /// Previous bars for smoothing (gravity effect) - now acts as "Display Bars"
    prev_bars: Vec<f32>,

    /// Velocities for physics gravity
    velocities: Vec<f32>,

    /// Rolling Maximum Value for Auto-Gain Control (AGC)
    max_val: f32,

    /// Last update timestamp for Framerate-Independent Physics ⏱️
    last_update: Option<std::time::Instant>,
}

impl Visualizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            audio_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(8192))),
            sample_rate,
            fft_size: 4096, // High Res FFT for stable bass
            prev_bars: vec![0.0; 200],
            velocities: vec![0.0; 200],
            max_val: 0.001,
            last_update: None,
        }
    }

    /// Get a cloneable handle to push samples safely from the audio thread
    pub fn get_audio_buffer(&self) -> Arc<Mutex<VecDeque<f32>>> {
        self.audio_buffer.clone()
    }

    /// Push raw audio samples into the buffer via shared handle
    pub fn push_samples(buffer: &Arc<Mutex<VecDeque<f32>>>, new_samples: &[f32], channels: usize) {
        if let Ok(mut buf) = buffer.lock() {
            // Downmix to mono if stereo for visualization
            if channels == 2 {
                for chunk in new_samples.chunks(2) {
                    if chunk.len() == 2 {
                        buf.push_back((chunk[0] + chunk[1]) / 2.0);
                    }
                }
            } else {
                for &s in new_samples {
                    buf.push_back(s);
                }
            }
            // Keep enough for overlap
            while buf.len() > 8192 {
                buf.pop_front();
            }
        }
    }

    /// Process FFT and return normalized bar heights (0.0 - 1.0)
    pub fn get_bars(&mut self, count: usize) -> Vec<f32> {
        let samples = {
            if let Ok(buf) = self.audio_buffer.lock() {
                if buf.len() < self.fft_size {
                    return vec![0.0; count];
                }
                buf.iter()
                    .rev()
                    .take(self.fft_size)
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                return vec![0.0; count];
            }
        };

        let input: Vec<f32> = samples.into_iter().rev().collect();
        let windowed_input = hann_window(&input);

        // FFT
        let spectrum = samples_fft_to_spectrum(
            &windowed_input,
            self.sample_rate,
            FrequencyLimit::Range(20.0, 20_000.0),
            Some(&divide_by_N_sqrt),
        )
        .unwrap_or_default();

        let mut bars = vec![0.0f32; count];
        let raw_data = spectrum.data();
        if raw_data.is_empty() {
            return bars;
        }

        // --- 1. Processing: Logarithmic Gaussian Integration ---
        // Instead of binary "In Bin / Out of Bin", we use overlapping Gaussian windows.

        let min_freq = 40.0f32;
        let max_freq = 12_000.0f32; // Cutoff at 12kHz (User setting) to reduce high-end jitter
        let log_min = min_freq.ln();
        let log_max = max_freq.ln();

        // ... (Freq Calc omitted)

        // Calculate Bar Frequencies and Bandwidths
        let mut bar_freqs = Vec::with_capacity(count);
        for i in 0..count {
            let p = i as f32 / (count as f32 - 1.0).max(1.0);
            let f = (log_min + p * (log_max - log_min)).exp();
            bar_freqs.push(f);
        }

        let mut current_frame_max = 0.0f32;

        // AGC Decay
        self.max_val *= 0.995;
        if self.max_val < 0.1 {
            self.max_val = 0.1;
        }

        let fft_res = self.sample_rate as f32 / self.fft_size as f32;

        for i in 0..count {
            let center_freq = bar_freqs[i];

            // ... (Bandwidth Calc omitted)
            let width = if i < count - 1 {
                bar_freqs[i + 1] - center_freq
            } else {
                center_freq - bar_freqs[i - 1]
            };

            let sigma = width.max(fft_res) * 0.5;
            let scan_range = sigma * 3.0;
            let range_min = center_freq - scan_range;
            let range_max = center_freq + scan_range;

            let mut weighted_sum = 0.0;
            let mut total_weight = 0.0;

            for (fr, val) in raw_data.iter() {
                let f = fr.val();
                if f < range_min {
                    continue;
                }
                if f > range_max {
                    break;
                }

                let diff = f - center_freq;
                let weight = (-(diff * diff) / (2.0 * sigma * sigma)).exp();

                weighted_sum += val.val() * weight;
                total_weight += weight;
            }

            let avg_val = if total_weight > 0.0001 {
                weighted_sum / total_weight
            } else {
                0.0
            };

            // Pink Noise Compensation - Standard
            // 2.0x boost
            let correction = 1.0 + (i as f32 / count as f32) * 2.0;
            let corrected_val = avg_val * correction;

            if corrected_val > current_frame_max {
                current_frame_max = corrected_val;
            }
            bars[i] = corrected_val;
        }

        // update AGC (Stabilized)
        if current_frame_max > self.max_val {
            self.max_val = self.max_val * 0.9 + current_frame_max * 0.1;
        }

        // --- 1. Delta-Time Calculation ⏱️ ---
        let now = std::time::Instant::now();
        let mut dt = if let Some(last) = self.last_update {
            now.duration_since(last).as_secs_f32()
        } else {
            0.016
        };
        // SAFETY CLAMP: Detect Pause/Resume
        // If dt > 100ms, we likely paused. Reset timing to prevent jerky catch-up.
        if dt > 0.1 {
            dt = 0.016;
            // Optional: Soft reset to prevent visual jump?
            // self.velocities = vec![0.0; count];
        }
        self.last_update = Some(now);

        // Normalize first
        for val in bars.iter_mut().take(count) {
            let norm = if self.max_val > 0.0 {
                *val / self.max_val
            } else {
                0.0
            };
            // SAFETY CLIP: Prevent "Explosion" when resuming from silence.
            // If max_val is tiny, norm could be 100.0. Clamp it to 1.0.
            let norm = norm.min(1.0);

            *val = norm.powf(0.85);
        }

        // --- 2. Integral Smoothing ---
        // Rise Speed: 15.0 units/sec exponential approach
        let rise_speed = 15.0;

        // --- 3. Monstercat Spatial ---
        let monster_decay = 0.80;
        for i in 1..count {
            if bars[i] < bars[i - 1] * monster_decay {
                bars[i] = bars[i - 1] * monster_decay;
            }
        }
        for i in (0..count - 1).rev() {
            if bars[i] < bars[i + 1] * monster_decay {
                bars[i] = bars[i + 1] * monster_decay;
            }
        }

        // --- 4. Delta-Time Gravity 🪐 ---
        if self.prev_bars.len() != count {
            self.prev_bars = vec![0.0; count];
        }
        if self.velocities.len() != count {
            self.velocities = vec![0.0; count];
        }

        // Pure Physics: Units per Second Squared
        // Range 0.0-1.0.
        // g=2.0 means it takes ~0.7s to fall from 1.0 to 0.0 from rest.
        // g=3.0 gives a nice snappy bounce.
        let gravity = 3.0;
        let _hysteresis = 0.005;

        for (i, bar_val) in bars.iter_mut().enumerate().take(count) {
            let mut input_val = *bar_val;

            // Silence Gate
            if input_val < 0.005 {
                input_val = 0.0;
            }

            let mut display_val = self.prev_bars[i];
            let mut vel = self.velocities[i];

            if input_val > display_val {
                // Framerate-independent Lerp
                let t = 1.0 - (-rise_speed * dt).exp();
                display_val = display_val + (input_val - display_val) * t;

                // If rising, reset downward velocity
                vel = 0.0;
            } else {
                // Falling: Kinematics
                // v = v + a*t
                vel += gravity * dt;
                // d = v*t
                display_val -= vel * dt;

                if display_val < 0.0 {
                    display_val = 0.0;
                    vel = 0.0;
                }
            }

            self.prev_bars[i] = display_val;
            self.velocities[i] = vel;
            *bar_val = display_val;
        }

        bars
    }
}
