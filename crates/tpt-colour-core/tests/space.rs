//! Colour-space conversions: known reference values, round trips and parity with palette.

use palette::{FromColor, Lab as PLab, Oklab as POklab, Srgb as PSrgb, Xyz as PXyz};
use tpt_colour_core::space::{
    Cmyk, ColourSpace, D50, D65, DisplayP3, Lab, Lch, LinearSRgb, Oklab, Oklch, Rec2020, SRgb,
    YCbCr, d50_to_d65, d65_to_d50,
};

fn assert_near(a: [f32; 3], b: [f32; 3], tol: f32) {
    for i in 0..3 {
        assert!(
            (a[i] - b[i]).abs() <= tol,
            "component {i}: {a:?} vs {b:?} (tol {tol})"
        );
    }
}

const SAMPLES: [[f32; 3]; 5] = [
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0],
    [0.8, 0.2, 0.1],
    [0.2, 0.5, 0.8],
    [0.35, 0.6, 0.12],
];

#[test]
fn srgb_white_maps_to_d65() {
    assert_near(SRgb.to_xyz([1.0, 1.0, 1.0]), D65, 1e-5);
    assert_near(SRgb.to_xyz([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0], 1e-6);
}

#[test]
fn every_rgb_space_shares_the_d65_white() {
    // All the RGB matrices are derived from one white point, so they agree exactly.
    assert_near(DisplayP3.to_xyz([1.0, 1.0, 1.0]), D65, 1e-5);
    assert_near(Rec2020.to_xyz([1.0, 1.0, 1.0]), D65, 1e-5);
}

#[test]
fn srgb_matches_palette() {
    for c in SAMPLES {
        let ours = SRgb.to_xyz(c);
        let p: PXyz = PXyz::from_color(PSrgb::new(c[0], c[1], c[2]));
        assert_near(ours, [p.x, p.y, p.z], 2e-3);
    }
}

#[test]
fn round_trips_are_stable() {
    for c in SAMPLES {
        assert_near(SRgb.from_xyz(SRgb.to_xyz(c)), c, 1e-4);
        assert_near(DisplayP3.from_xyz(DisplayP3.to_xyz(c)), c, 1e-4);
        assert_near(Rec2020.from_xyz(Rec2020.to_xyz(c)), c, 1e-4);
        assert_near(LinearSRgb.from_xyz(LinearSRgb.to_xyz(c)), c, 1e-4);
    }
}

#[test]
fn wide_gamut_white_is_d65() {
    assert_near(DisplayP3.to_xyz([1.0, 1.0, 1.0]), D65, 1e-3);
    assert_near(Rec2020.to_xyz([1.0, 1.0, 1.0]), D65, 1e-3);
}

#[test]
fn lab_white_and_known_value() {
    assert_near(Lab.from_xyz(D65), [100.0, 0.0, 0.0], 1e-3);
    // Lab of sRGB pure red (D65), from the published reference table.
    let red = Lab.from_xyz(SRgb.to_xyz([1.0, 0.0, 0.0]));
    assert_near(red, [53.24, 80.09, 67.20], 0.05);
}

#[test]
fn lab_matches_palette() {
    for c in SAMPLES {
        let ours = Lab.from_xyz(SRgb.to_xyz(c));
        let p: PLab = PLab::from_color(PSrgb::new(c[0], c[1], c[2]));
        assert_near(ours, [p.l, p.a, p.b], 0.02);
    }
}

#[test]
fn lab_round_trip() {
    for c in SAMPLES {
        let xyz = SRgb.to_xyz(c);
        assert_near(Lab.to_xyz(Lab.from_xyz(xyz)), xyz, 1e-4);
    }
}

#[test]
fn lch_is_polar_lab() {
    for c in SAMPLES {
        let lab = Lab.from_xyz(SRgb.to_xyz(c));
        let lch = Lch.from_xyz(SRgb.to_xyz(c));
        let c_chroma = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt();
        assert!((lch[0] - lab[0]).abs() < 1e-3);
        assert!((lch[1] - c_chroma).abs() < 1e-3);
        assert!((0.0..360.0).contains(&lch[2]));
        assert_near(Lch.to_xyz(lch), SRgb.to_xyz(c), 1e-4);
    }
}

#[test]
fn oklab_white_and_parity() {
    assert_near(Oklab.from_xyz(D65), [1.0, 0.0, 0.0], 1e-3);
    for c in SAMPLES {
        let ours = Oklab.from_xyz(SRgb.to_xyz(c));
        let p: POklab = POklab::from_color(PSrgb::new(c[0], c[1], c[2]));
        assert_near(ours, [p.l, p.a, p.b], 2e-3);
        assert_near(Oklab.to_xyz(ours), SRgb.to_xyz(c), 1e-4);
    }
}

#[test]
fn oklch_round_trip() {
    for c in SAMPLES {
        let xyz = SRgb.to_xyz(c);
        assert_near(Oklch.to_xyz(Oklch.from_xyz(xyz)), xyz, 1e-4);
    }
}

#[test]
fn cmyk_fallback_behaves() {
    assert_near(Cmyk.to_xyz([0.0, 0.0, 0.0, 0.0]), D65, 1e-3);
    assert_near(Cmyk.to_xyz([0.0, 0.0, 0.0, 1.0]), [0.0, 0.0, 0.0], 1e-6);
    // Pure key ink round-trips.
    let k = Cmyk.from_xyz(Cmyk.to_xyz([0.0, 0.0, 0.0, 0.5]));
    assert_near([k[0], k[1], k[2]], [0.0, 0.0, 0.0], 1e-4);
    assert!((k[3] - 0.5).abs() < 1e-4);
}

#[test]
fn bradford_maps_white_points() {
    assert_near(d65_to_d50(D65), D50, 1e-3);
    assert_near(d50_to_d65(D50), D65, 1e-3);
    let xyz = SRgb.to_xyz([0.3, 0.6, 0.9]);
    assert_near(d50_to_d65(d65_to_d50(xyz)), xyz, 1e-4);
}

#[test]
fn ycbcr_round_trip_and_white() {
    let white = YCbCr::REC709.encode([1.0, 1.0, 1.0]);
    assert_near(white, [1.0, 0.0, 0.0], 1e-6);
    for m in [YCbCr::REC709, YCbCr::REC2020] {
        for c in SAMPLES {
            assert_near(m.decode(m.encode(c)), c, 1e-5);
        }
    }
}

#[test]
fn adobe_rgb_white_is_d65_and_round_trips() {
    use tpt_colour_core::space::AdobeRgb;
    assert_near(AdobeRgb.to_xyz([1.0, 1.0, 1.0]), D65, 1e-5);
    for c in SAMPLES {
        assert_near(AdobeRgb.from_xyz(AdobeRgb.to_xyz(c)), c, 1e-4);
    }
}

#[test]
fn ycbcr_rec601_coefficients() {
    assert!((YCbCr::REC601.kr() - 0.299).abs() < 1e-6);
    assert!((YCbCr::REC601.kb() - 0.114).abs() < 1e-6);
    let m = YCbCr::REC601;
    assert_near(m.decode(m.encode([0.3, 0.6, 0.9])), [0.3, 0.6, 0.9], 1e-5);
}

#[test]
fn transfer_curves_are_odd_for_negative_channels() {
    // Out-of-range negatives use the linear toe, so f(-x) = -f(x) for each curve.
    for x in [0.02_f32, 0.3, 0.8] {
        assert_near(
            SRgb.to_xyz([-x, 0.0, 0.0]),
            neg(SRgb.to_xyz([x, 0.0, 0.0])),
            1e-6,
        );
        assert_near(
            DisplayP3.to_xyz([-x, 0.0, 0.0]),
            neg(DisplayP3.to_xyz([x, 0.0, 0.0])),
            1e-6,
        );
        assert_near(
            Rec2020.to_xyz([-x, 0.0, 0.0]),
            neg(Rec2020.to_xyz([x, 0.0, 0.0])),
            1e-6,
        );
    }
}

fn neg(v: [f32; 3]) -> [f32; 3] {
    v.map(|c| -c)
}

#[test]
fn prophoto_white_is_d65_after_adaptation_and_round_trips() {
    use tpt_colour_core::space::ProPhotoRgb;
    // The D50 white of ProPhoto adapts to the D65 white of the interface.
    assert_near(ProPhotoRgb.to_xyz([1.0, 1.0, 1.0]), D65, 1e-3);
    for c in SAMPLES {
        assert_near(ProPhotoRgb.from_xyz(ProPhotoRgb.to_xyz(c)), c, 1e-4);
    }
}
