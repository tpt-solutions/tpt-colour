//! SIMD batching must be bit-identical to the scalar path. Run with `--features simd`.
#![cfg(feature = "simd")]

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

/// Deterministic pixels in `[0, 1]`, with alpha, so the batch path and remainder both run.
fn pixels(n: usize) -> Vec<[f32; 4]> {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 40) as f32 / (1_u64 << 24) as f32
    };
    (0..n).map(|_| [next(), next(), next(), next()]).collect()
}

fn check(src: &IccProfile, dst: &IccProfile) {
    let t = ColourTransform::new(src, dst, RenderingIntent::RelativeColorimetric).unwrap();
    let input = pixels(1003);

    let mut batched = input.clone();
    t.transform_pixels(&mut batched).unwrap();

    for (px, original) in batched.iter().zip(&input) {
        let mut scalar = [0.0_f32; 3];
        t.transform(&original[..3], &mut scalar).unwrap();
        for c in 0..3 {
            assert_eq!(
                px[c].to_bits(),
                scalar[c].to_bits(),
                "{original:?}: {px:?} vs {scalar:?}"
            );
        }
        assert_eq!(
            px[3].to_bits(),
            original[3].to_bits(),
            "alpha must be untouched"
        );
    }
}

#[test]
fn srgb_to_display_p3_batches_match_scalar() {
    check(&IccProfile::srgb(), &IccProfile::display_p3());
}

#[test]
fn display_p3_to_srgb_batches_match_scalar() {
    check(&IccProfile::display_p3(), &IccProfile::srgb());
}
