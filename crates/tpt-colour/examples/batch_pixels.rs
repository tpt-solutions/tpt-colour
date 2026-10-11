//! Converts a buffer of RGBA pixels from Display P3 to sRGB in one call.
//!
//! The batch path reads and writes `[f32; 4]` pixels in place, and leaves alpha untouched.
//! Run with `cargo run -p tpt-colour --example batch_pixels`.

use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut colour = ColourManager::new();
    let transform = colour.transform("display-p3", "srgb", RenderingIntent::Perceptual)?;

    // A tiny 4×1 strip: red, green, blue and a half-transparent grey.
    let mut pixels = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.5, 0.5, 0.5, 0.5],
    ];
    transform.inner().transform_pixels(&mut pixels)?;

    for (i, px) in pixels.iter().enumerate() {
        println!("pixel {i}: {px:.4?}");
    }
    Ok(())
}
