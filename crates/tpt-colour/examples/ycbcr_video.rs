//! Encodes gamma-encoded RGB as Y′CbCr for video, and decodes it again.
//!
//! Run with `cargo run -p tpt-colour --example ycbcr_video`.

use tpt_colour::space::YCbCr;

fn main() {
    let rgb = [0.8_f32, 0.4, 0.1];
    for (name, matrix) in [
        ("Rec. 601", YCbCr::REC601),
        ("Rec. 709", YCbCr::REC709),
        ("Rec. 2020", YCbCr::REC2020),
    ] {
        let ycc = matrix.encode(rgb);
        let back = matrix.decode(ycc);
        println!("{name}: Y′CbCr {ycc:.4?} → RGB {back:.4?}");
    }

    // Coefficients can be supplied directly. They must describe a valid RGB system.
    match YCbCr::new(0.2126, 0.0722) {
        Some(custom) => println!("custom K_R {} K_B {}", custom.kr(), custom.kb()),
        None => println!("invalid coefficients"),
    }
}
