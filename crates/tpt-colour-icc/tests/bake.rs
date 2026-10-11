//! A baked CLUT stands in for the transform: it must agree within interpolation error.

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

/// Deterministic pseudo-random samples in `[0, 1]³`, so the test is reproducible.
fn samples(n: usize) -> Vec<[f32; 3]> {
    let mut state = 0x2545_f491_u32;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 8) as f32 / (1 << 24) as f32
    };
    (0..n).map(|_| [next(), next(), next()]).collect()
}

#[test]
fn baked_clut_matches_the_transform_within_interpolation_error() {
    let t = ColourTransform::new(
        &IccProfile::display_p3(),
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    let clut = t.bake_clut(33).unwrap();
    assert_eq!(clut.input_channels(), 3);
    assert_eq!(clut.output_channels(), 3);

    // Errors in 8-bit codes. On a 33-point grid, P3 → sRGB measures about 0.3 codes on average,
    // with 99% of samples within about 4.4. The worst case is near black, where the curve is
    // steep, and is not asserted here.
    let mut errors: Vec<f32> = samples(20_000)
        .into_iter()
        .map(|rgb| {
            let mut exact = [0.0_f32; 3];
            t.transform(&rgb, &mut exact).unwrap();
            let mut baked = [0.0_f32; 3];
            clut.eval(&rgb, &mut baked).unwrap();
            (0..3)
                .map(|c| (exact[c] - baked[c]).abs() * 255.0)
                .fold(0.0_f32, f32::max)
        })
        .collect();
    errors.sort_by(f32::total_cmp);
    let mean = errors.iter().sum::<f32>() / errors.len() as f32;
    let p99 = errors[errors.len() * 99 / 100];
    assert!(mean < 0.5, "mean error {mean} codes");
    assert!(p99 < 6.0, "99th percentile error {p99} codes");
}

#[test]
fn bake_rejects_a_grid_under_two_points() {
    let t = ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::srgb(),
        RenderingIntent::Perceptual,
    )
    .unwrap();
    assert!(t.bake_clut(1).is_err());
}
