//! Gamut containment, perceptual mapping, the profile manager, and HDR tone mapping.

use std::sync::Arc;

use tpt_colour_core::space::{
    ColourSpace, D50, DisplayP3, Oklch, Rec2020, SRgb, d50_to_d65, d65_to_d50,
};
use tpt_colour_icc::{IccProfile, RenderingIntent};
use tpt_colour_manage::tone::{pq_eotf, pq_oetf, reinhard_extended, tone_map_xyz};
use tpt_colour_manage::{ColourManager, Gamut, ManageError, map_to_gamut};

fn assert_near(a: [f32; 3], b: [f32; 3], tol: f32) {
    for i in 0..3 {
        assert!(
            (a[i] - b[i]).abs() <= tol,
            "component {i}: {a:?} vs {b:?} (tol {tol})"
        );
    }
}

/// Display P3 green, which lies outside sRGB.
fn p3_green_xyz() -> [f32; 3] {
    DisplayP3.to_xyz([0.0, 1.0, 0.0])
}

#[test]
fn srgb_white_is_inside_srgb() {
    assert!(Gamut::srgb().contains(SRgb.to_xyz([1.0, 1.0, 1.0])));
}

#[test]
fn p3_green_is_outside_srgb_but_inside_p3_and_rec2020() {
    assert!(!Gamut::srgb().contains(p3_green_xyz()));
    assert!(Gamut::display_p3().contains(p3_green_xyz()));
    assert!(Gamut::rec2020().contains(p3_green_xyz()));
}

#[test]
fn rec2020_primaries_are_inside_rec2020_only_for_its_own_primaries() {
    let g = Gamut::rec2020();
    assert!(g.contains(Rec2020.to_xyz([0.0, 1.0, 0.0])));
    assert!(!Gamut::display_p3().contains(Rec2020.to_xyz([0.0, 1.0, 0.0])));
}

#[test]
fn gamut_from_profile_matches_builtin_p3() {
    let from_profile = Gamut::from_profile(&IccProfile::display_p3()).expect("matrix profile");
    let builtin = Gamut::display_p3();
    for (a, b) in from_profile
        .colorants()
        .iter()
        .flatten()
        .zip(builtin.colorants().iter().flatten())
    {
        assert!((a - b).abs() < 1e-3, "{a} vs {b}");
    }
}

#[test]
fn in_gamut_colours_are_unchanged_by_perceptual_mapping() {
    let grey = SRgb.to_xyz([0.5, 0.5, 0.5]);
    assert_near(map_to_gamut(&Gamut::srgb(), grey), grey, 1e-6);
}

/// The CSS Color 4 algorithm keeps lightness and hue while reducing chroma.
#[test]
fn out_of_gamut_colours_map_inside_keeping_lightness_and_hue() {
    let target = Gamut::srgb();
    let source = p3_green_xyz();
    let mapped = map_to_gamut(&target, source);
    assert!(
        target.contains(mapped),
        "mapped colour should be in sRGB: {mapped:?}"
    );

    let before = Oklch.from_xyz(source);
    let after = Oklch.from_xyz(mapped);
    assert!(
        (before[0] - after[0]).abs() < 0.01,
        "lightness {before:?} → {after:?}"
    );
    let hue_error = (before[2] - after[2])
        .abs()
        .min(360.0 - (before[2] - after[2]).abs());
    assert!(hue_error < 3.0, "hue {before:?} → {after:?}");
    assert!(
        after[1] < before[1],
        "chroma should drop: {before:?} → {after:?}"
    );
}

#[test]
fn mapping_clamps_lightness_extremes() {
    let target = Gamut::srgb();
    let bright = Oklch.to_xyz([1.2, 0.1, 30.0]);
    assert_near(
        map_to_gamut(&target, bright),
        Oklch.to_xyz([1.0, 0.0, 0.0]),
        1e-5,
    );
    assert_near(
        map_to_gamut(&target, [0.0, 0.0, 0.0]),
        [0.0, 0.0, 0.0],
        1e-6,
    );
}

#[test]
fn clip_keeps_colours_inside_the_gamut() {
    let target = Gamut::srgb();
    let clipped = target.clip(p3_green_xyz());
    assert!(target.contains(clipped));
}

#[test]
fn manager_registers_builtins_and_rejects_unknown_names() {
    let mut m = ColourManager::new();
    assert!(m.profile("srgb").is_some());
    assert!(m.profile("display-p3").is_some());
    let err = m
        .transform("nope", "srgb", RenderingIntent::Perceptual)
        .unwrap_err();
    assert_eq!(err, ManageError::UnknownProfile("nope".into()));
}

#[test]
fn transforms_are_cached_and_shared() {
    let mut m = ColourManager::new();
    let a = m
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    let b = m
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(m.cached_transforms(), 1);
    m.transform("display-p3", "srgb", RenderingIntent::RelativeColorimetric)
        .unwrap();
    assert_eq!(m.cached_transforms(), 2);
}

#[test]
fn re_registering_a_profile_invalidates_its_transforms() {
    let mut m = ColourManager::new();
    m.transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    m.transform("srgb", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    assert_eq!(m.cached_transforms(), 2);
    m.register("srgb", IccProfile::srgb());
    // Only the transform that does not involve "srgb" would survive, and none do here.
    assert_eq!(m.cached_transforms(), 0);
}

#[test]
fn invalid_icc_bytes_are_reported() {
    let mut m = ColourManager::new();
    assert!(m.register_icc("bad", &[0_u8; 16]).is_err());
    assert!(m.profile("bad").is_none());
}

#[test]
fn perceptual_p3_to_srgb_is_gamut_mapped_and_relative_is_clipped() {
    let mut m = ColourManager::new();
    let perceptual = m
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    let relative = m
        .transform("display-p3", "srgb", RenderingIntent::RelativeColorimetric)
        .unwrap();
    assert!(perceptual.gamut().is_some());
    assert!(relative.gamut().is_none());

    let green = [0.0_f32, 1.0, 0.0];
    let p = perceptual.transform_rgb(green).unwrap();
    let r = relative.transform_rgb(green).unwrap();
    for v in p.iter().chain(&r) {
        assert!(
            (0.0..=1.0).contains(v),
            "device value out of range: {p:?} {r:?}"
        );
    }
    assert!((p[0] - r[0]).abs() + (p[1] - r[1]).abs() + (p[2] - r[2]).abs() > 1e-3);
}

#[test]
fn srgb_round_trips_through_p3_within_tolerance() {
    let mut m = ColourManager::new();
    let to_p3 = m
        .transform("srgb", "display-p3", RenderingIntent::Perceptual)
        .unwrap();
    let back = m
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    for c in [[0.2_f32, 0.5, 0.8], [0.5, 0.5, 0.5], [0.9, 0.3, 0.1]] {
        let round = back.transform_rgb(to_p3.transform_rgb(c).unwrap()).unwrap();
        assert_near(round, c, 2e-3);
    }
}

#[test]
fn pq_round_trips() {
    for i in 0..=10 {
        let e = i as f32 / 10.0;
        assert!((pq_oetf(pq_eotf(e)) - e).abs() < 1e-4, "e = {e}");
    }
    assert!(pq_eotf(0.0).abs() < 1e-7);
    assert!((pq_eotf(1.0) - 1.0).abs() < 1e-4);
}

#[test]
fn reinhard_is_monotonic_and_maps_white_to_one() {
    let white = 4.0;
    assert!((reinhard_extended(white, white) - 1.0).abs() < 1e-6);
    let mut prev = -1.0_f32;
    for i in 0..100 {
        let v = reinhard_extended(i as f32 * 0.05, white);
        assert!(v >= prev);
        prev = v;
    }
}

#[test]
fn tone_map_keeps_chromaticity_and_stays_in_range() {
    let xyz = [0.2_f32, 0.3, 0.1];
    let out = tone_map_xyz(xyz, 4.0);
    let chroma = |c: [f32; 3]| {
        let s = c[0] + c[1] + c[2];
        (c[0] / s, c[1] / s)
    };
    let (x0, y0) = chroma(xyz);
    let (x1, y1) = chroma(out);
    assert!((x0 - x1).abs() < 1e-5 && (y0 - y1).abs() < 1e-5);
    assert!((0.0..=1.0).contains(&out[1]));
    assert_near(tone_map_xyz([0.0; 3], 4.0), [0.0; 3], 0.0);
    assert!((tone_map_xyz([50.0, 100.0, 40.0], 4.0)[1] - 1.0).abs() < 1e-6);
}

#[test]
fn tone_map_clip_keeps_chromaticity_of_white() {
    // D65 white at a luminance far above `white`. Y clips to 1, and X:Z must not move.
    let white_d65 = [0.950_47_f32, 1.0, 1.088_83].map(|v| v * 50.0);
    let out = tone_map_xyz(white_d65, 4.0);
    assert!((out[1] - 1.0).abs() < 1e-6, "{out:?}");
    assert!(
        (out[0] / out[2] - 0.950_47 / 1.088_83).abs() < 1e-5,
        "{out:?}"
    );
}

#[test]
fn white_and_d50_are_consistent_across_adaptation() {
    assert_near(d65_to_d50(d50_to_d65(D50)), D50, 1e-4);
}

#[test]
fn tone_mapping_handles_nan_and_degenerate_white() {
    assert!(reinhard_extended(f32::NAN, 4.0).abs() < 1e-9);
    assert!(reinhard_extended(1.0, f32::NAN).is_finite());
    assert!(
        (reinhard_extended(1.0, f32::NAN) - 0.5).abs() < 1e-6,
        "plain Reinhard"
    );
    assert!(
        (reinhard_extended(1.0, 0.0) - 0.5).abs() < 1e-6,
        "plain Reinhard"
    );
    assert_near(tone_map_xyz([f32::NAN, 1.0, 1.0], 4.0), [0.0; 3], 1e-9);
    assert_near(tone_map_xyz([f32::INFINITY, 1.0, 1.0], 4.0), [0.0; 3], 1e-9);
    assert!(
        tone_map_xyz([0.2, 0.3, 0.1], f32::NAN)
            .iter()
            .all(|v| v.is_finite())
    );
}

#[test]
fn map_to_gamut_maps_nan_to_black_and_stays_in_gamut() {
    let target = Gamut::srgb();
    assert_near(map_to_gamut(&target, [f32::NAN, 0.2, 0.3]), [0.0; 3], 1e-9);
    // Out-of-gamut P3 green, mapped perceptually, must land inside the sRGB boundary.
    let mapped = map_to_gamut(&target, p3_green_xyz());
    assert!(target.contains(mapped), "{mapped:?}");
}

#[test]
fn hlg_round_trips_and_hits_its_knee() {
    use tpt_colour_manage::tone::{hlg_eotf, hlg_oetf};
    // The OETF meets 0.5 at scene 1/12, where the two branches join.
    assert!((hlg_oetf(1.0 / 12.0) - 0.5).abs() < 1e-5);
    assert!((hlg_oetf(1.0) - 1.0).abs() < 1e-4);
    for i in 0..=20 {
        let e = i as f32 / 20.0;
        assert!((hlg_eotf(hlg_oetf(e)) - e).abs() < 1e-4, "scene {e}");
    }
}
