use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vyom::audio::{DspEqualizer, EqGains};

fn bench_process_sample(c: &mut Criterion) {
    // Setup a 10-band equalizer at 44.1kHz with some active bands
    let gains = EqGains::new();
    gains.set_gain(0, 5.0); // Boost Sub-bass
    gains.set_gain(5, -3.0); // Cut 1kHz

    let mut eq = DspEqualizer::new(44100.0, gains);

    // Warm up the filters
    eq.process_sample(0.0, 0.0);

    // Benchmark the tight inner DSP loop mathematical throughput
    c.bench_function("dsp_eq_process_sample", |b| {
        b.iter(|| {
            // Processing a synthetic sine/square wave generic audio signal pair
            eq.process_sample(black_box(0.5), black_box(-0.5))
        })
    });
}

// Group the benchmarks and expose to cargo
criterion_group!(benches, bench_process_sample);
criterion_main!(benches);
