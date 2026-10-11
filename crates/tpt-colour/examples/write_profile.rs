//! Writes ICC profiles with tpt-colour, and records what tpt-colour makes of them, so an
//! independent CMM can check the files. The Python check is `tools/icc/check_written_profiles.py`.
//!
//! Run with `cargo run -p tpt-colour --example write_profile -- <output-dir>`.

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use tpt_colour::icc::{ColourTransform, IccProfile, ProfileModel, RenderingIntent, Trc};

/// 8-bit test colours, covering the corners, greys and a few saturated mid-tones.
const COLOURS: [[u8; 3]; 9] = [
    [0, 0, 0],
    [255, 255, 255],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
    [200, 60, 30],
    [30, 180, 90],
    [90, 40, 220],
];

/// The sRGB decoding curve: device value to linear light.
fn srgb_decode(x: f32) -> f32 {
    if x <= 0.040_45 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// A Display P3 profile with its curves replaced, keeping its primaries and white.
fn with_curves(curve: Trc) -> IccProfile {
    let mut profile = IccProfile::display_p3();
    if let ProfileModel::Matrix(m) = &mut profile.model {
        m.trc = [curve.clone(), curve.clone(), curve];
    }
    profile
}

fn main() {
    let dir = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: write_profile <output-dir>"),
    );
    fs::create_dir_all(&dir).expect("output directory");

    // A TRC maps device value to linear light, so a sampled sRGB curve holds linear values.
    let table: Vec<f32> = (0..256).map(|i| srgb_decode(i as f32 / 255.0)).collect();
    let profiles = [
        ("srgb_builtin", IccProfile::srgb()),
        ("p3_gamma22", with_curves(Trc::Gamma(2.2))),
        ("p3_table", with_curves(Trc::Table(table))),
    ];

    let mut report = String::new();
    for (name, profile) in profiles {
        let bytes = profile.to_bytes().expect("matrix/TRC profile writes");
        fs::write(dir.join(format!("{name}.icc")), &bytes).expect("write profile");

        // What tpt-colour does with the bytes it wrote, read back through the parser.
        let parsed = IccProfile::parse(&bytes).expect("written profile parses");
        let srgb = IccProfile::srgb();
        let transform = ColourTransform::new(&srgb, &parsed, RenderingIntent::RelativeColorimetric)
            .expect("transform builds");
        for colour in COLOURS {
            let mut pixel = colour.to_vec();
            transform
                .transform_u8(&mut pixel, 3)
                .expect("three channels");
            let _ = writeln!(
                report,
                "{name} {} {} {} {} {} {}",
                colour[0], colour[1], colour[2], pixel[0], pixel[1], pixel[2]
            );
        }
    }
    fs::write(dir.join("tpt_expected.txt"), report).expect("write expectations");
    println!("wrote {} profiles to {}", 3, dir.display());
}
