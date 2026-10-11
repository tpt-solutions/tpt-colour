//! Parses CSS colour strings, builds a palette, and simulates colour blindness.
//!
//! Run with `cargo run -p tpt-colour-core --example palette_and_vision`.

use tpt_colour_core::mix::mix_oklab;
use tpt_colour_core::palette::{complement, triadic};
use tpt_colour_core::parse::parse_colour;
use tpt_colour_core::vision::{Deficiency, simulate};

/// Scales 8-bit channels to the [0, 1] range the library uses.
fn unit(rgba: [u8; 4]) -> [f32; 3] {
    [rgba[0], rgba[1], rgba[2]].map(|v| f32::from(v) / 255.0)
}

fn main() {
    // Accepts hex, rgb(), oklch(), CSS named colours and `transparent`.
    let base = unit(parse_colour("rebeccapurple").expect("named colour"));
    println!("base          {base:.3?}");

    let opposite = complement(base);
    println!("complement    {opposite:.3?}");

    for (i, colour) in triadic(base).iter().enumerate() {
        println!("triadic {}    {colour:.3?}", i + 1);
    }

    // Blend halfway in Oklab, which keeps lightness steady across the mix.
    let mid = mix_oklab(base, opposite, 0.5);
    println!("midpoint      {mid:.3?}");

    // How the midpoint looks to a viewer with each form of colour-vision deficiency.
    for kind in [
        Deficiency::Protanopia,
        Deficiency::Deuteranopia,
        Deficiency::Tritanopia,
    ] {
        println!("{kind:?}: {:.3?}", simulate(mid, kind));
    }
}
