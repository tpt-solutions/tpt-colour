//! Registers profiles by name, then converts colours between them with cached transforms.
//!
//! Run with `cargo run -p tpt-colour-manage --example colour_manager`.

use tpt_colour_icc::{IccProfile, RenderingIntent};
use tpt_colour_manage::ColourManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `new` registers "srgb" and "display-p3" already. Add a profile from its ICC bytes,
    // as you would for a file read from disk. Here the bytes come from tpt-colour itself.
    let mut colour = ColourManager::with_capacity(16);
    let bytes = IccProfile::display_p3().to_bytes()?;
    colour.register_icc("p3-from-bytes", &bytes)?;

    // Each (source, destination, intent) triple is built once, then reused.
    let to_p3 = colour.transform(
        "srgb",
        "p3-from-bytes",
        RenderingIntent::RelativeColorimetric,
    )?;
    let first = to_p3.transform_rgb([0.8, 0.2, 0.1])?;
    let second = colour
        .transform(
            "srgb",
            "p3-from-bytes",
            RenderingIntent::RelativeColorimetric,
        )?
        .transform_rgb([0.8, 0.2, 0.1])?;
    println!("sRGB (0.8, 0.2, 0.1) -> P3 {first:.4?}");
    // The second call reuses the cached transform, so the result matches the first.
    assert!(first.iter().zip(second).all(|(a, b)| (a - b).abs() < 1e-6));
    println!("cached transforms: {}", colour.cached_transforms());

    // Unknown names are reported, not silently ignored.
    match colour.transform("srgb", "fogra39", RenderingIntent::Perceptual) {
        Ok(_) => println!("unexpectedly found fogra39"),
        Err(e) => println!("expected error: {e}"),
    }
    Ok(())
}
