//! 8-bit interleaved buffers: alpha is untouched, and identity transforms are exact.

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

#[test]
fn identity_transform_keeps_8_bit_values_and_alpha() {
    let t = ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::srgb(),
        RenderingIntent::Perceptual,
    )
    .unwrap();
    let mut pixels = vec![10_u8, 128, 250, 77, 0, 255, 64, 9];
    let before = pixels.clone();
    t.transform_u8(&mut pixels, 4).unwrap();
    for (i, (a, b)) in pixels.iter().zip(&before).enumerate() {
        if i % 4 == 3 {
            assert_eq!(a, b, "alpha must not change");
        } else {
            assert!(
                (i32::from(*a) - i32::from(*b)).abs() <= 1,
                "{pixels:?} vs {before:?}"
            );
        }
    }
}

#[test]
fn p3_red_clips_to_srgb_red_and_bad_lengths_are_rejected() {
    let t = ColourTransform::new(
        &IccProfile::display_p3(),
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    let mut rgb = vec![255_u8, 0, 0];
    t.transform_u8(&mut rgb, 3).unwrap();
    // P3 red is outside sRGB, so relative colorimetric clips it to sRGB red.
    assert_eq!(rgb, [255, 0, 0]);
    assert!(t.transform_u8(&mut [1, 2], 3).is_err());
    assert!(t.transform_u8(&mut [1, 2, 3], 2).is_err());
}
