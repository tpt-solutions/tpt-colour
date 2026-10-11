//! Absolute colorimetric scales XYZ by the ratio of the two media white points (`wtpt`).

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

fn pcs_white(src: &IccProfile, dst: &IccProfile, intent: RenderingIntent) -> [f32; 3] {
    ColourTransform::new(src, dst, intent)
        .unwrap()
        .device_to_pcs(&[1.0, 1.0, 1.0])
        .unwrap()
}

#[test]
fn absolute_equals_relative_when_media_whites_match() {
    let src = IccProfile::srgb();
    let dst = IccProfile::display_p3();
    let abs = pcs_white(&src, &dst, RenderingIntent::AbsoluteColorimetric);
    let rel = pcs_white(&src, &dst, RenderingIntent::RelativeColorimetric);
    for c in 0..3 {
        assert!((abs[c] - rel[c]).abs() < 1e-6, "{abs:?} vs {rel:?}");
    }
}

#[test]
fn absolute_scales_by_the_media_white_ratio() {
    let src = IccProfile::srgb();
    let mut dst = IccProfile::srgb();
    // A destination whose paper is 80% as bright as the source's.
    dst.white_point = dst.white_point.map(|v| v * 0.8);

    let abs = pcs_white(&src, &dst, RenderingIntent::AbsoluteColorimetric);
    let rel = pcs_white(&src, &dst, RenderingIntent::RelativeColorimetric);
    for c in 0..3 {
        assert!(
            (abs[c] - rel[c] / 0.8).abs() < 1e-4,
            "axis {c}: absolute {} vs relative {} / 0.8",
            abs[c],
            rel[c]
        );
    }
}

#[test]
fn absolute_scale_is_not_used_for_relative_intent() {
    let src = IccProfile::srgb();
    let mut dst = IccProfile::srgb();
    dst.white_point = dst.white_point.map(|v| v * 0.8);
    let a = pcs_white(
        &src,
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    );
    let b = pcs_white(&src, &dst, RenderingIntent::RelativeColorimetric);
    for c in 0..3 {
        assert!((a[c] - b[c]).abs() < 1e-6);
    }
}

#[test]
fn rendering_intent_parses_case_insensitively() {
    use tpt_colour_icc::RenderingIntent;
    assert_eq!(
        "Perceptual".parse::<RenderingIntent>(),
        Ok(RenderingIntent::Perceptual)
    );
    assert_eq!(
        "ABSOLUTE".parse::<RenderingIntent>(),
        Ok(RenderingIntent::AbsoluteColorimetric)
    );
    assert!("nope".parse::<RenderingIntent>().is_err());
}
