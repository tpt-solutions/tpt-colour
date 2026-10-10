//! Throughput baseline for the colour pipeline. Run in release mode, since debug timings
//! are meaningless:
//!
//!     cargo run --release -p tpt-colour --example bench
//!     cargo run --release -p tpt-colour --features simd --example bench
//!
//! Each measurement is the fastest of several runs, which damps noise from other load on
//! the machine. Compare the scalar and `simd` builds on the same machine.

use std::hint::black_box;
use std::time::{Duration, Instant};

use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;
use tpt_colour::manage::{Gamut, map_to_gamut};
use tpt_colour::space::ColourSpace;

const PIXELS: usize = 1_000_000;
const RUNS: usize = 5;

#[cfg(feature = "simd")]
const MATRIX_LABEL: &str = "sRGB → Display P3 (matrix/TRC, simd batches)";
#[cfg(not(feature = "simd"))]
const MATRIX_LABEL: &str = "sRGB → Display P3 (matrix/TRC, scalar)";

/// Synthetic RGBA image: a diagonal ramp, so every channel takes many values.
fn image() -> Vec<[f32; 4]> {
    (0..PIXELS)
        .map(|i| {
            let t = i as f32 / PIXELS as f32;
            [t, 1.0 - t, (t * 7.0).fract(), 1.0]
        })
        .collect()
}

/// Times `work` on a fresh copy of `input`, `RUNS` times, and returns the fastest run.
fn best_of(input: &[[f32; 4]], mut work: impl FnMut(&mut [[f32; 4]])) -> Duration {
    (0..RUNS)
        .map(|_| {
            let mut pixels = input.to_vec();
            let start = Instant::now();
            work(black_box(&mut pixels));
            black_box(&pixels);
            start.elapsed()
        })
        .min()
        .expect("RUNS > 0")
}

fn report(name: &str, elapsed: Duration) {
    let secs = elapsed.as_secs_f64();
    println!(
        "{name:<46} {:>8.1} Mpx/s  ({:>6.1} ns/px)",
        PIXELS as f64 / secs / 1e6,
        secs * 1e9 / PIXELS as f64
    );
}

fn main() {
    let mut colour = ColourManager::new();
    let input = image();

    let matrix = colour
        .transform("srgb", "display-p3", RenderingIntent::RelativeColorimetric)
        .unwrap();
    let t = matrix.inner();
    report(
        MATRIX_LABEL,
        best_of(&input, |px| t.transform_pixels(px).unwrap()),
    );

    let perceptual = colour
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    report(
        "Display P3 → sRGB (perceptual, gamut-mapped)",
        best_of(&input, |px| {
            for p in px {
                let rgb = perceptual.transform_rgb([p[0], p[1], p[2]]).unwrap();
                p[..3].copy_from_slice(&rgb);
            }
        }),
    );

    // Out-of-gamut for sRGB, so every call runs the chroma search rather than returning early.
    let target = Gamut::srgb();
    let out_of_gamut = ColourSpace::to_xyz(&tpt_colour::space::DisplayP3, [0.0, 1.0, 0.0]);
    let elapsed = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for i in 0..PIXELS {
                let v = out_of_gamut[0] + i as f32 * 1e-9;
                black_box(map_to_gamut(&target, [v, out_of_gamut[1], out_of_gamut[2]]));
            }
            start.elapsed()
        })
        .min()
        .expect("RUNS > 0");
    report("OKLCH gamut mapping, out-of-gamut input", elapsed);
}
