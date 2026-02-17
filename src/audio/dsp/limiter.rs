/// Transparent soft-knee limiter
/// Linear up to threshold (0.85), then exponentially saturates to ceiling (0.98).
/// Preserves volume and dynamics better than simple tanh.
pub fn limiter(x: f32) -> f32 {
    let threshold = 0.85; // ~ -1.4 dB
    let ceiling = 0.98;   // ~ -0.2 dB (headroom)

    if x.abs() <= threshold {
        x
    } else {
         // Exponential soft clip
         // y = ceiling - (ceiling - threshold) * exp(- (abs(x) - threshold) / (ceiling - threshold))
         let abs_x = x.abs();
         let diff = ceiling - threshold;
         let y = ceiling - diff * (-(abs_x - threshold) / diff).exp();
         
         if x > 0.0 { y } else { -y }
    }
}
