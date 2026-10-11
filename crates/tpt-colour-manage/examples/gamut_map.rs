//! Maps Display P3 colours that sRGB cannot show into the sRGB gamut.
//!
//! Run with `cargo run -p tpt-colour-manage --example gamut_map`.

use tpt_colour_core::space::{ColourSpace, DisplayP3, SRgb};
use tpt_colour_manage::{Gamut, map_to_gamut};

fn main() {
    let srgb = Gamut::srgb();
    let p3 = Gamut::display_p3();

    // A saturated Display P3 green lies outside sRGB.
    let green = DisplayP3.to_xyz([0.0, 1.0, 0.0]);
    println!("inside sRGB before mapping: {}", srgb.contains(green));

    // Clipping each channel is fast, but it shifts hue. Gamut mapping keeps lightness
    // and hue, and reduces chroma until the colour fits.
    let clipped = srgb.clip(green);
    let mapped = map_to_gamut(&srgb, green);
    println!("clipped  -> sRGB {:.4?}", SRgb.from_xyz(clipped));
    println!("mapped   -> sRGB {:.4?}", SRgb.from_xyz(mapped));
    println!("inside sRGB after mapping:  {}", srgb.contains(mapped));

    // Colours already in gamut pass through unchanged.
    let grey = SRgb.to_xyz([0.5, 0.5, 0.5]);
    let unchanged = map_to_gamut(&srgb, grey);
    assert!(
        unchanged
            .iter()
            .zip(grey)
            .all(|(a, b)| (a - b).abs() < 1e-6)
    );
    println!("in-gamut grey is unchanged: {}", p3.contains(grey));
}
