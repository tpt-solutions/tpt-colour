//! Converts colours through the tpt-colour stack and prints the results.
//!
//! Run with `cargo run -p tpt-colour --example convert`.

use tpt_colour::icc::RenderingIntent;
use tpt_colour::space::{ColourSpace, DisplayP3, SRgb};
use tpt_colour::{ColourManager, Gamut, map_to_gamut};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Colour-space maths: sRGB orange as CIE XYZ (D65).
    let xyz = SRgb.to_xyz([1.0, 0.5, 0.0]);
    println!("sRGB orange → XYZ {xyz:.3?}");

    // 2. Profile-managed transforms. Transforms are cached inside the manager.
    let mut colour = ColourManager::new();
    let to_p3 = colour.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
    println!(
        "sRGB (0.8, 0.2, 0.1) → Display P3 {:.4?}",
        to_p3.transform_rgb([0.8, 0.2, 0.1])?
    );

    // 3. Gamut mapping: a Display P3 green that sRGB cannot show.
    let p3_green = DisplayP3.to_xyz([0.0, 1.0, 0.0]);
    let mapped = map_to_gamut(&Gamut::srgb(), p3_green);
    println!("P3 green → sRGB (perceptual) {:.4?}", SRgb.from_xyz(mapped));
    println!(
        "P3 green → sRGB (clipped)    {:.4?}",
        SRgb.from_xyz(Gamut::srgb().clip(p3_green))
    );
    println!("cached transforms: {}", colour.cached_transforms());
    Ok(())
}
