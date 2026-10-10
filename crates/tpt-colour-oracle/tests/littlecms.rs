//! Parity with LittleCMS 2.17 on sampled device colours.
//!
//! The reference outputs live in `tests/fixtures/` and were produced by
//! `tools/oracle/generate_littlecms.py`. The profiles themselves are not in the repository.
//! They are read from the directory in `TPT_ICC_DIR`, or from the Windows colour directory
//! if that exists. Without either, the tests are skipped and say so.

use std::path::{Path, PathBuf};

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

/// Largest allowed difference, in 8-bit device codes, between tpt-colour and LittleCMS.
///
/// LittleCMS works in 8-bit here, so the reference itself is quantised. A difference of
/// one code is rounding; anything larger is a real disagreement. Measured worst case: 1.
const MAX_CODE_ERROR: f32 = 1.0;

fn profile_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TPT_ICC_DIR") {
        return Some(PathBuf::from(dir));
    }
    let windows = PathBuf::from("C:/Windows/System32/spool/drivers/color");
    windows.is_dir().then_some(windows)
}

struct Sample {
    input: Vec<f32>,
    expected: Vec<f32>,
}

fn fixture(name: &str) -> Vec<Sample> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|line| {
            let (lhs, rhs) = line.split_once("->").expect("fixture line has '->'");
            let parse = |s: &str| -> Vec<f32> {
                s.split_whitespace()
                    .map(|t| t.parse::<f32>().expect("integer code") / 255.0)
                    .collect()
            };
            Sample {
                input: parse(lhs),
                expected: parse(rhs).iter().map(|v| (v * 255.0).round()).collect(),
            }
        })
        .collect()
}

/// Runs every sample through `t` and returns the worst error in 8-bit codes.
fn worst_error(t: &ColourTransform, samples: &[Sample], name: &str) -> f32 {
    let mut worst = 0.0_f32;
    let mut worst_at = String::new();
    for s in samples {
        let mut out = vec![0.0_f32; t.dst_channels()];
        t.transform(&s.input, &mut out).expect("channel counts match");
        for (o, e) in out.iter().zip(&s.expected) {
            let code = (o * 255.0).round();
            let err = (code - e).abs();
            if err > worst {
                worst = err;
                worst_at = format!("input {:?}: got {code}, LittleCMS {e}", s.input);
            }
        }
    }
    println!("{name}: worst error {worst} codes ({worst_at})");
    worst
}

#[test]
fn rswop_cmyk_to_srgb_matches_littlecms() {
    let Some(dir) = profile_dir() else {
        eprintln!("skipped: set TPT_ICC_DIR to a directory containing RSWOP.icm");
        return;
    };
    let bytes = std::fs::read(dir.join("RSWOP.icm")).expect("RSWOP.icm");
    let profile = IccProfile::parse(&bytes).expect("RSWOP parses");
    let t = ColourTransform::new(&profile, &IccProfile::srgb(), RenderingIntent::Perceptual).unwrap();
    let worst = worst_error(&t, &fixture("rswop_cmyk_to_srgb.txt"), "RSWOP CMYK→sRGB");
    assert!(worst <= MAX_CODE_ERROR, "worst error {worst} codes");
}

#[test]
fn gamma18_rgb_to_srgb_matches_littlecms() {
    let Some(dir) = profile_dir() else {
        eprintln!("skipped: set TPT_ICC_DIR to a directory containing ewrgb18.icm");
        return;
    };
    let bytes = std::fs::read(dir.join("ewrgb18.icm")).expect("ewrgb18.icm");
    let profile = IccProfile::parse(&bytes).expect("ewrgb18 parses");
    let t = ColourTransform::new(&profile, &IccProfile::srgb(), RenderingIntent::Perceptual).unwrap();
    let worst = worst_error(&t, &fixture("ewrgb18_to_srgb.txt"), "gamma 1.8 RGB→sRGB");
    assert!(worst <= MAX_CODE_ERROR, "worst error {worst} codes");
}
