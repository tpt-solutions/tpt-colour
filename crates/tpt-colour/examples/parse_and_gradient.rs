//! Parses hex colours and builds a gradient between them in Oklab and Oklch.
//!
//! Run with `cargo run -p tpt-colour --example parse_and_gradient`.

use tpt_colour::hex::parse_hex;
use tpt_colour::mix::{mix_oklab, mix_oklch};

/// Scales 8-bit channels to the `[0, 1]` range the library uses.
fn unit(rgba: [u8; 4]) -> [f32; 3] {
    [rgba[0], rgba[1], rgba[2]].map(|v| f32::from(v) / 255.0)
}

fn main() {
    let start = parse_hex("#ff7a00").expect("valid hex");
    let end = parse_hex("#1f6feb").expect("valid hex");
    let (a, b) = (unit(start), unit(end));

    for step in 0..=4 {
        let t = step as f32 / 4.0;
        let lab = mix_oklab(a, b, t);
        let lch = mix_oklch(a, b, t);
        println!("t = {t:.2}  via oklab {lab:.3?}  via oklch {lch:.3?}");
    }
}
