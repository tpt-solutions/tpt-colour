//! Measures how different two colours look, and whether text is readable on a background.
//!
//! Run with `cargo run -p tpt-colour-core --example difference_and_contrast`.

use tpt_colour_core::contrast::{apca_contrast, wcag_contrast};
use tpt_colour_core::difference::{delta_e_76, delta_e_2000, delta_e_ok};
use tpt_colour_core::hex::parse_hex;
use tpt_colour_core::space::{ColourSpace, Lab, Oklab, SRgb};

/// Parses a `#rrggbb` string into channels in [0, 1].
fn srgb(hex: &str) -> [f32; 3] {
    let [r, g, b, _] = parse_hex(hex).expect("valid hex colour");
    [r, g, b].map(|c| f32::from(c) / 255.0)
}

fn main() {
    let (a, b) = (srgb("#1f6feb"), srgb("#2f81f7"));

    // ΔE*ab and ΔE00 work in CIELAB; ΔEOK works in Oklab. A value near 1 is about
    // one just-noticeable difference for ΔE00, and about 0.02 for ΔEOK.
    let lab_a = Lab.from_xyz(SRgb.to_xyz(a));
    let lab_b = Lab.from_xyz(SRgb.to_xyz(b));
    println!("ΔE76   {:.2}", delta_e_76(lab_a, lab_b));
    println!("ΔE2000 {:.2}", delta_e_2000(lab_a, lab_b));
    let ok_a = Oklab.from_xyz(SRgb.to_xyz(a));
    let ok_b = Oklab.from_xyz(SRgb.to_xyz(b));
    println!("ΔEOK   {:.4}", delta_e_ok(ok_a, ok_b));

    // Text contrast. WCAG 2 needs 4.5:1 for body text and 3:1 for large text.
    let (text, background) = (srgb("#777777"), srgb("#ffffff"));
    let ratio = wcag_contrast(text, background);
    // #777777 on white just misses the 4.5:1 AA threshold for body text.
    println!("WCAG 2 contrast {ratio:.2}:1 (AA body text needs 4.5)");

    // APCA is signed: positive means dark text on a light background.
    println!("APCA Lc {:.1}", apca_contrast(text, background));
}
