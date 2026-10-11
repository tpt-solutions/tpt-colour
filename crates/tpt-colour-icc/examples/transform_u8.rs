//! Converts an interleaved 8-bit RGBA buffer from sRGB to Display P3.
//!
//! Run with `cargo run -p tpt-colour-icc --example transform_u8`.

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let srgb = IccProfile::srgb();
    let display_p3 = IccProfile::display_p3();

    // Relative colorimetric keeps in-gamut colours where they are, which suits this direction.
    let transform =
        ColourTransform::new(&srgb, &display_p3, RenderingIntent::RelativeColorimetric)?;

    // Four pixels: red, green, blue and mid grey. `transform_u8` takes the channel count, so
    // a 4-channel buffer is RGBA; the alpha byte is left as it is.
    let mut pixels: Vec<u8> = vec![
        255, 0, 0, 255, //
        0, 255, 0, 255, //
        0, 0, 255, 255, //
        128, 128, 128, 128,
    ];
    transform.transform_u8(&mut pixels, 4)?;

    for px in pixels.chunks_exact(4) {
        println!("rgba {px:?}");
    }
    Ok(())
}
