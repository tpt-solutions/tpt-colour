//! Converts one colour through several spaces, all via CIE XYZ.
//!
//! Run with `cargo run -p tpt-colour-core --example spaces`.

use tpt_colour_core::space::{ColourSpace, DisplayP3, Lab, Oklab, Oklch, SRgb};

fn main() {
    // An sRGB orange, with channels in [0, 1].
    let srgb = [1.0, 0.5, 0.0];

    // Every space converts to and from CIE XYZ (D65), so any pair can be chained.
    let xyz = SRgb.to_xyz(srgb);
    println!("XYZ        {xyz:.4?}");

    println!("CIELAB     {:.4?}", Lab.from_xyz(xyz));
    println!("Oklab      {:.4?}", Oklab.from_xyz(xyz));
    println!("Oklch      {:.4?}", Oklch.from_xyz(xyz));

    // The same XYZ value is a different sRGB-encoded triple in Display P3,
    // because P3 has a wider gamut. Values stay unclamped, so check the range.
    let p3 = DisplayP3.from_xyz(xyz);
    println!("Display P3 {p3:.4?}");
    let in_gamut = p3.iter().all(|c| (0.0..=1.0).contains(c));
    println!("inside the Display P3 gamut: {in_gamut}");
}
