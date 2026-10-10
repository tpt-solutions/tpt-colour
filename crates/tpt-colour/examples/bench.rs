//! Scalar throughput baseline. Run in release mode, since debug timings are meaningless:
//!
//!     cargo run --release -p tpt-colour --example bench
//!
//! There is no `simd` path yet, so this is the reference to compare against once it exists.

use std::hint::black_box;
use std::time::Instant;

use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;
use tpt_colour::manage::{Gamut, map_to_gamut};

const PIXELS: usize = 1_000_000;

fn report(name: &str, pixels: usize, elapsed: std::time::Duration) {
    let secs = elapsed.as_secs_f64();
    println!(
        "{name:<42} {:>8.1} Mpx/s  ({:>6.1} ns/px)",
        pixels as f64 / secs / 1e6,
        secs * 1e9 / pixels as f64
    );
}

fn main() {
    let mut colour = ColourManager::new();

    // Synthetic RGBA image: a diagonal ramp, so every channel takes many values.
    let mut image: Vec<[f32; 4]> = (0..PIXELS)
        .map(|i| {
            let t = i as f32 / PIXELS as f32;
            [t, 1.0 - t, (t * 7.0).fract(), 1.0]
        })
        .collect();

    let matrix = colour
        .transform("srgb", "display-p3", RenderingIntent::RelativeColorimetric)
        .unwrap();
    let start = Instant::now();
    for px in black_box(&mut image) {
        let rgb = matrix.transform_rgb([px[0], px[1], px[2]]).unwrap();
        px[..3].copy_from_slice(&rgb);
    }
    report(
        "sRGB → Display P3 (matrix/TRC, RGBA)",
        PIXELS,
        start.elapsed(),
    );

    let perceptual = colour
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    let start = Instant::now();
    for px in black_box(&mut image) {
        let rgb = perceptual.transform_rgb([px[0], px[1], px[2]]).unwrap();
        px[..3].copy_from_slice(&rgb);
    }
    report(
        "Display P3 → sRGB (perceptual, gamut-mapped)",
        PIXELS,
        start.elapsed(),
    );

    let target = Gamut::srgb();
    let xyz = [0.4, 0.3, 0.2];
    let start = Instant::now();
    for i in 0..PIXELS {
        let v = xyz[0] + i as f32 * 1e-7;
        black_box(map_to_gamut(&target, [v, xyz[1], xyz[2]]));
    }
    report(
        "OKLCH gamut mapping alone (XYZ in/out)",
        PIXELS,
        start.elapsed(),
    );
}
