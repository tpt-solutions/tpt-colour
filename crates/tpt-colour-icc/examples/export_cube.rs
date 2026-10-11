//! Bakes an sRGB to Display P3 transform into an Adobe `.cube` 3-D LUT, as text.
//!
//! Run with `cargo run -p tpt-colour-icc --example export_cube`. Redirect the output to a
//! file to load it into video or photo tools that read `.cube` files.

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

/// Points per axis. 17 keeps the file small; 33 is the usual choice for production LUTs.
const GRID: usize = 17;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transform = ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::display_p3(),
        RenderingIntent::RelativeColorimetric,
    )?;

    let cube = transform.to_cube(GRID, "sRGB to Display P3")?;

    // Print only the header and the first data row, so the output stays readable.
    for line in cube.lines().take(6) {
        println!("{line}");
    }
    println!("# ... {} data rows in total", GRID * GRID * GRID);
    Ok(())
}
