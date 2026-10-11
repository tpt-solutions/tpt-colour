//! Checks text and background pairs against WCAG 2 contrast thresholds.
//!
//! Run with `cargo run -p tpt-colour --example wcag_check`.

use tpt_colour::contrast::wcag_contrast;
use tpt_colour::hex::parse_hex;
use tpt_colour::models::srgb_to_hsl;

fn unit(rgba: [u8; 4]) -> [f32; 3] {
    [rgba[0], rgba[1], rgba[2]].map(|v| f32::from(v) / 255.0)
}

fn main() {
    let pairs = [
        ("#777777", "#ffffff"),
        ("#767676", "#ffffff"),
        ("#1a1a1a", "#f5f0e6"),
    ];
    for (fg, bg) in pairs {
        let (Some(fg_rgba), Some(bg_rgba)) = (parse_hex(fg), parse_hex(bg)) else {
            println!("{fg} on {bg}: bad hex");
            continue;
        };
        let ratio = wcag_contrast(unit(fg_rgba), unit(bg_rgba));
        let verdict = match ratio {
            r if r >= 7.0 => "AAA",
            r if r >= 4.5 => "AA",
            r if r >= 3.0 => "AA large only",
            _ => "fails",
        };
        let [h, s, l] = srgb_to_hsl(unit(fg_rgba));
        println!("{fg} on {bg}: {ratio:.2}:1 ({verdict}), foreground hsl({h:.0}, {s:.2}, {l:.2})");
    }
}
