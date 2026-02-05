//! Built-in DSP 10-Band Parametric Equalizer
//!
//! Uses biquad peaking EQ filters for each frequency band.
//! Provides real-time adjustable gain for each band.

#[cfg(feature = "eq")]
use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Type, Q_BUTTERWORTH_F32};

use std::sync::{Arc, RwLock};

/// 10-band EQ center frequencies in Hz
pub const EQ_FREQUENCIES: [f32; 10] = [
    32.0,    // Sub-bass
    64.0,    // Bass
    125.0,   // Low-mid
    250.0,   // Mid
    500.0,   // Mid
    1000.0,  // Upper-mid
    2000.0,  // Presence
    4000.0,  // Brilliance
    8000.0,  // Air
    16000.0, // Ultra-high
];

/// Convert app EQ value (0.0-1.0) to dB gain (-12 to +12)
pub fn value_to_db(value: f32) -> f32 {
    (value - 0.5) * 24.0
}

/// Convert dB gain (-12 to +12) to app EQ value (0.0-1.0)
#[allow(dead_code)]
pub fn db_to_value(db: f32) -> f32 {
    (db / 24.0) + 0.5
}

/// Shared EQ gains that can be updated from the UI
#[derive(Clone)]
pub struct EqGains {
    /// Gains in dB for each band (-12 to +12)
    gains: Arc<RwLock<[f32; 10]>>,
    /// Whether EQ is enabled
    enabled: Arc<RwLock<bool>>,
    /// User-adjustable preamp in dB (-12 to +12)
    preamp_db: Arc<RwLock<f32>>,
    /// Stereo balance (-1.0 = full left, 0.0 = center, +1.0 = full right)
    balance: Arc<RwLock<f32>>,
}

impl Default for EqGains {
    fn default() -> Self {
        Self::new()
    }
}

impl EqGains {
    pub fn new() -> Self {
        Self {
            gains: Arc::new(RwLock::new([0.0; 10])), // All bands at 0dB
            enabled: Arc::new(RwLock::new(true)),
            preamp_db: Arc::new(RwLock::new(0.0)), // No preamp adjustment
            balance: Arc::new(RwLock::new(0.0)),   // Center
        }
    }

    /// Set gain for a specific band (in dB)
    pub fn set_gain(&self, band: usize, db: f32) {
        if band < 10 {
            if let Ok(mut gains) = self.gains.write() {
                gains[band] = db.clamp(-12.0, 12.0);
            }
        }
    }

    /// Set gain from app value (0.0-1.0)
    pub fn set_gain_from_value(&self, band: usize, value: f32) {
        self.set_gain(band, value_to_db(value));
    }

    /// Get all gains in dB
    pub fn get_gains(&self) -> [f32; 10] {
        self.gains.read().map(|g| *g).unwrap_or([0.0; 10])
    }

    /// Set all gains from app values (0.0-1.0)
    pub fn set_all_from_values(&self, values: &[f32; 10]) {
        if let Ok(mut gains) = self.gains.write() {
            for (i, v) in values.iter().enumerate() {
                gains[i] = value_to_db(*v);
            }
        }
    }

    /// Set enabled state
    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut e) = self.enabled.write() {
            *e = enabled;
        }
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.read().map(|e| *e).unwrap_or(true)
    }

    /// Reset all bands to 0dB
    pub fn reset(&self) {
        if let Ok(mut gains) = self.gains.write() {
            *gains = [0.0; 10];
        }
    }

    /// Reset all bands to 0dB (Explicit alias)
    pub fn reset_bands(&self) {
        self.reset();
    }

    /// Set preamp gain in dB (-12 to +12)
    pub fn set_preamp_db(&self, db: f32) {
        if let Ok(mut preamp) = self.preamp_db.write() {
            *preamp = db.clamp(-12.0, 12.0);
        }
    }

    /// Get preamp gain in dB
    pub fn get_preamp_db(&self) -> f32 {
        self.preamp_db.read().map(|p| *p).unwrap_or(0.0)
    }

    /// Set stereo balance (-1.0 = left, 0.0 = center, +1.0 = right)
    pub fn set_balance(&self, balance: f32) {
        if let Ok(mut bal) = self.balance.write() {
            *bal = balance.clamp(-1.0, 1.0);
        }
    }

    /// Get stereo balance
    pub fn get_balance(&self) -> f32 {
        self.balance.read().map(|b| *b).unwrap_or(0.0)
    }
}

/// 10-Band Parametric Equalizer using biquad filters
#[cfg(feature = "eq")]
pub struct DspEqualizer {
    /// Biquad filters for each band (stereo: left + right)
    filters_left: Vec<DirectForm1<f32>>,
    filters_right: Vec<DirectForm1<f32>>,
    /// Sample rate for filter coefficient calculation
    sample_rate: f32,
    /// Shared gains (can be updated from UI thread)
    gains: EqGains,
    /// Last applied gains (to detect changes)
    last_gains: [f32; 10],
    /// Preamp (negative gain to prevent clipping)
    preamp: f32,
    /// Crossfade mix (0.0 = dry, 1.0 = wet/EQ) for smooth transitions
    mix: f32,
    /// Target mix based on enabled state
    target_mix: f32,
    /// Crossfade speed (samples to transition)
    crossfade_speed: f32,
}

#[cfg(feature = "eq")]
impl DspEqualizer {
    /// Create a new 10-band equalizer
    pub fn new(sample_rate: f32, gains: EqGains) -> Self {
        let mut filters_left = Vec::with_capacity(10);
        let mut filters_right = Vec::with_capacity(10);

        // Initialize filters for each band
        for freq in EQ_FREQUENCIES.iter() {
            let coeffs = Self::make_peaking_coeffs(sample_rate, *freq, 0.0);
            filters_left.push(DirectForm1::<f32>::new(coeffs));
            filters_right.push(DirectForm1::<f32>::new(coeffs));
        }

        // Crossfade speed: ~10ms at sample rate
        let crossfade_speed = 1.0 / (sample_rate * 0.010);

        Self {
            filters_left,
            filters_right,
            sample_rate,
            gains,
            last_gains: [0.0; 10],
            preamp: 1.0, // No reduction initially (flat EQ)
            mix: 1.0,    // Start with EQ active
            target_mix: 1.0,
            crossfade_speed,
        }
    }

    /// Create peaking EQ filter coefficients
    fn make_peaking_coeffs(sample_rate: f32, freq: f32, gain_db: f32) -> Coefficients<f32> {
        // Use peaking EQ filter type
        Coefficients::<f32>::from_params(
            Type::PeakingEQ(gain_db),
            sample_rate.hz(),
            freq.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap_or_else(|_| {
            // Fallback to unity gain if calculation fails
            Coefficients::<f32>::from_params(
                Type::PeakingEQ(0.0),
                44100.0.hz(),
                1000.0.hz(),
                Q_BUTTERWORTH_F32,
            )
            .unwrap()
        })
    }

    /// Update filter coefficients if gains have changed
    fn update_filters_if_needed(&mut self) {
        let current_gains = self.gains.get_gains();

        // Check if any gains changed
        let mut needs_update = false;
        for i in 0..10 {
            if (current_gains[i] - self.last_gains[i]).abs() > 0.01 {
                needs_update = true;
                break;
            }
        }

        if needs_update {
            // Calculate preamp: reduce by the maximum boost to prevent clipping
            let max_boost = current_gains.iter().cloned().fold(0.0f32, f32::max);
            // Convert dB to linear gain reduction
            // If max boost is +6dB, preamp should be -6dB = 10^(-6/20) ≈ 0.5
            self.preamp = if max_boost > 0.0 {
                10.0_f32.powf(-max_boost / 20.0)
            } else {
                1.0
            };

            for (i, gain) in current_gains.iter().enumerate() {
                let coeffs = Self::make_peaking_coeffs(self.sample_rate, EQ_FREQUENCIES[i], *gain);
                self.filters_left[i].update_coefficients(coeffs);
                self.filters_right[i].update_coefficients(coeffs);
            }
            self.last_gains = current_gains;
        }
    }

    /// Process a single stereo sample pair with smooth crossfade
    pub fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Update target mix based on enabled state
        self.target_mix = if self.gains.is_enabled() { 1.0 } else { 0.0 };

        // Smoothly interpolate mix towards target (prevents clicks on toggle)
        if self.mix < self.target_mix {
            self.mix = (self.mix + self.crossfade_speed).min(self.target_mix);
        } else if self.mix > self.target_mix {
            self.mix = (self.mix - self.crossfade_speed).max(self.target_mix);
        }

        // If fully bypassed, return dry signal
        if self.mix <= 0.0001 {
            return (left, right);
        }

        // Update filters if gains changed
        self.update_filters_if_needed();

        // Apply preamp BEFORE EQ to prevent clipping
        let mut l = left * self.preamp;
        let mut r = right * self.preamp;

        // Apply all 10 bands in series
        for i in 0..10 {
            l = self.filters_left[i].run(l);
            r = self.filters_right[i].run(r);
        }

        // Apply limiter
        let wet_l = limiter(l);
        let wet_r = limiter(r);

        // Get user preamp and balance from EqGains
        let user_preamp_db = self.gains.get_preamp_db();
        let user_preamp_linear = if user_preamp_db != 0.0 {
            10.0_f32.powf(user_preamp_db / 20.0)
        } else {
            1.0
        };

        let balance = self.gains.get_balance();
        // Balance: -1 = full left (right = 0), 0 = center, +1 = full right (left = 0)
        let left_gain = if balance > 0.0 { 1.0 - balance } else { 1.0 };
        let right_gain = if balance < 0.0 { 1.0 + balance } else { 1.0 };

        // Apply user preamp and balance
        let final_l = wet_l * user_preamp_linear * left_gain;
        let final_r = wet_r * user_preamp_linear * right_gain;

        // Crossfade between dry and wet (smooth transition on toggle)
        if self.mix >= 0.9999 {
            // Fully wet - no crossfade needed
            (final_l, final_r)
        } else {
            // Blend dry and wet signals
            let dry_mix = 1.0 - self.mix;
            (
                left * dry_mix + final_l * self.mix,
                right * dry_mix + final_r * self.mix,
            )
        }
    }

    /// Process a buffer of interleaved stereo samples
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        // Process interleaved stereo (L, R, L, R, ...)
        // Each sample call handles crossfade and filter updates
        for chunk in buffer.chunks_mut(2) {
            if chunk.len() == 2 {
                let (l, r) = self.process_sample(chunk[0], chunk[1]);
                chunk[0] = l;
                chunk[1] = r;
            }
        }
    }

    /// Reset filter state (call on extended silence to prevent transients)
    pub fn reset_filters(&mut self) {
        for i in 0..10 {
            // Recreate filters with current coefficients to reset internal state
            let coeffs =
                Self::make_peaking_coeffs(self.sample_rate, EQ_FREQUENCIES[i], self.last_gains[i]);
            self.filters_left[i] = biquad::DirectForm1::<f32>::new(coeffs);
            self.filters_right[i] = biquad::DirectForm1::<f32>::new(coeffs);
        }
    }
}

/// Smooth limiter using tanh for transparent limiting
/// Better than hard clipping - preserves dynamics while preventing distortion
fn limiter(x: f32) -> f32 {
    // tanh provides smooth saturation curve
    // Multiply by 0.95 to leave headroom
    x.tanh() * 0.95
}

/// Soft clipping function (kept for reference)
#[allow(dead_code)]
fn soft_clip(x: f32) -> f32 {
    if x > 1.0 {
        1.0 - (-x + 1.0).exp() * 0.5
    } else if x < -1.0 {
        -1.0 + (x + 1.0).exp() * 0.5
    } else {
        x
    }
}

/// Stub equalizer for when eq feature is disabled
#[cfg(not(feature = "eq"))]
pub struct DspEqualizer;

#[cfg(not(feature = "eq"))]
impl DspEqualizer {
    pub fn new(_sample_rate: f32, _gains: EqGains) -> Self {
        Self
    }

    pub fn process_sample(&mut self, left: f32, right: f32) -> (f32, f32) {
        (left, right)
    }

    pub fn process_buffer(&mut self, _buffer: &mut [f32]) {}
}
