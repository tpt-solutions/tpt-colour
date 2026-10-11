//! `ICtCp`, `Jzazbz`, `ACEScg`, Lab with a D50 white, generic chromatic adaptation and scRGB,
//! checked against `colour-science` 0.4.7. The reference values are in comments with the call that
//! produced them.

// The reference values are colour-science's output at full precision, so they are longer than f32.
#![allow(clippy::excessive_precision)]

use tpt_colour_core::space::{
    AcesCg, ChromaticAdaptation, ColourSpace, D50, D65, Lab50, LinearSRgb, ScRgb,
    adaptation_matrix, ictcp_to_xyz, jzazbz_to_xyz, xyz_to_ictcp, xyz_to_jzazbz,
};

fn close(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

fn apply(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}

#[test]
fn ictcp_matches_colour_science() {
    // colour.XYZ_to_ICtCp(XYZ, method='ITU-R BT.2100-2 PQ')
    let cases = [
        (
            [100.0, 100.0, 100.0],
            [0.508_301_264, -0.004_474_046, 0.017_136_198],
        ),
        (
            [50.0, 30.0, 10.0],
            [0.394_674_634, -0.044_680_972, 0.193_841_734],
        ),
        (
            [1000.0, 400.0, 20.0],
            [0.652_639_711, -0.102_820_289, 0.460_483_561],
        ),
        (
            [0.5, 0.2, 0.1],
            [0.084_423_793, 0.035_156_165, 0.127_980_651],
        ),
    ];
    for (xyz, expected) in cases {
        let got = xyz_to_ictcp(xyz);
        assert!(
            close(got, expected, 2e-5),
            "{xyz:?}: {got:?} vs {expected:?}"
        );
    }
}

#[test]
fn ictcp_round_trips() {
    for xyz in [
        [100.0, 100.0, 100.0],
        [50.0, 30.0, 10.0],
        [1000.0, 400.0, 20.0],
    ] {
        let back = ictcp_to_xyz(xyz_to_ictcp(xyz));
        assert!(
            close(back, xyz, 0.01 * xyz[1].max(1.0)),
            "{xyz:?} -> {back:?}"
        );
    }
}

#[test]
fn jzazbz_matches_colour_science() {
    // colour.XYZ_to_Jzazbz(XYZ) with absolute XYZ in cd/m².
    let cases = [
        (
            [100.0, 100.0, 100.0],
            [0.168_477_754, 0.010_674_053, 0.008_795_54],
        ),
        (
            [50.0, 30.0, 10.0],
            [0.111_400_391, 0.087_102_598, 0.070_596_674],
        ),
        (
            [1000.0, 400.0, 20.0],
            [0.343_415_668, 0.236_939_133, 0.192_121_329],
        ),
        (
            [0.5, 0.2, 0.1],
            [0.008_693_146, 0.022_697_123, 0.007_591_759],
        ),
    ];
    for (xyz, expected) in cases {
        let got = xyz_to_jzazbz(xyz);
        assert!(
            close(got, expected, 1e-5),
            "{xyz:?}: {got:?} vs {expected:?}"
        );
    }
}

#[test]
fn jzazbz_round_trips() {
    for xyz in [
        [100.0, 100.0, 100.0],
        [50.0, 30.0, 10.0],
        [1000.0, 400.0, 20.0],
    ] {
        let back = jzazbz_to_xyz(xyz_to_jzazbz(xyz));
        assert!(
            close(back, xyz, 0.01 * xyz[1].max(1.0)),
            "{xyz:?} -> {back:?}"
        );
    }
}

#[test]
fn chromatic_adaptation_matches_colour_science() {
    // colour.adaptation.chromatic_adaptation_VonKries(XYZ, D65, D50, transform=...), with
    // D65 and D50 from xy chromaticities. Our whites differ from those by about 2e-4 in Z.
    let cases = [
        (
            ChromaticAdaptation::Cat02,
            [0.2, 0.3, 0.1],
            [0.212_500_844, 0.302_888_305, 0.074_919_773],
        ),
        (
            ChromaticAdaptation::Cat16,
            [0.2, 0.3, 0.1],
            [0.210_968_991, 0.299_275_798, 0.073_399_076],
        ),
        (
            ChromaticAdaptation::Bradford,
            [0.2, 0.3, 0.1],
            [0.211_450_793, 0.301_348_51, 0.077_855_377],
        ),
        (
            ChromaticAdaptation::Cat02,
            [0.9, 0.5, 0.7],
            [0.916_793_245, 0.506_150_798, 0.530_493_697],
        ),
        (
            ChromaticAdaptation::Cat16,
            [0.9, 0.5, 0.7],
            [0.906_221_993, 0.502_485_435, 0.532_055_175],
        ),
        (
            ChromaticAdaptation::Bradford,
            [0.9, 0.5, 0.7],
            [0.919_475_662, 0.509_930_582, 0.525_520_856],
        ),
    ];
    for (method, xyz, expected) in cases {
        let m = adaptation_matrix(method, D65, D50).expect("non-singular");
        let got = apply(&m, xyz);
        assert!(
            close(got, expected, 2e-3),
            "{method:?} {xyz:?}: {got:?} vs {expected:?}"
        );
    }
}

#[test]
fn adaptation_maps_the_source_white_onto_the_destination_white() {
    for method in [
        ChromaticAdaptation::Bradford,
        ChromaticAdaptation::Cat02,
        ChromaticAdaptation::Cat16,
    ] {
        let m = adaptation_matrix(method, D65, D50).unwrap();
        assert!(close(apply(&m, D65), D50, 1e-4), "{method:?}");
    }
}

#[test]
fn lab50_matches_colour_science() {
    // colour.adaptation.chromatic_adaptation_VonKries(XYZ, D65, D50, 'Bradford') followed by
    // colour.XYZ_to_Lab(XYZ, illuminant=D50).
    let cases = [
        ([0.2, 0.3, 0.1], [61.770_401, -33.706_332, 43.034_82]),
        ([0.9, 0.5, 0.7], [76.674_804, 92.669_832, -12.293_18]),
        ([0.5, 0.5, 0.5], [76.160_884, 7.194_489, 4.604_86]),
    ];
    for (xyz, expected) in cases {
        let got = Lab50.from_xyz(xyz);
        // The D65 white here is the rounded constant, and colour-science derives it from xy. The
        // difference of about 2e-4 in Z moves a* and b* by up to about 0.02.
        assert!(
            close(got, expected, 0.05),
            "{xyz:?}: {got:?} vs {expected:?}"
        );
    }
}

#[test]
fn lab50_white_is_neutral_and_round_trips() {
    let white = Lab50.from_xyz(D65);
    assert!((white[0] - 100.0).abs() < 0.01, "{white:?}");
    assert!(white[1].abs() < 0.01 && white[2].abs() < 0.01, "{white:?}");
    let xyz = [0.2, 0.3, 0.1];
    let back = Lab50.to_xyz(Lab50.from_xyz(xyz));
    assert!(close(back, xyz, 1e-4), "{back:?}");
}

#[test]
fn acescg_white_maps_to_d65_and_round_trips() {
    // ACES white is D60. The matrix includes Bradford D60 -> D65, so the white lands on D65.
    let white = AcesCg.to_xyz([1.0, 1.0, 1.0]);
    assert!(close(white, D65, 3e-4), "{white:?}");
    let rgb = [0.5, 0.2, 0.1];
    let back = AcesCg.from_xyz(AcesCg.to_xyz(rgb));
    assert!(close(back, rgb, 1e-5), "{back:?}");
}

#[test]
fn acescg_primaries_follow_the_matrix_columns() {
    // The red primary's XYZ is the first column of the D60-to-D65 matrix from colour-science.
    let red = AcesCg.to_xyz([1.0, 0.0, 0.0]);
    assert!(
        close(red, [0.652_237_5, 0.267_672_2, -0.005_381_8], 1e-5),
        "{red:?}"
    );
}

#[test]
fn scrgb_keeps_values_outside_the_unit_range() {
    let scrgb: ScRgb = LinearSRgb;
    let extended = [2.5, -0.5, 0.1];
    // The same arithmetic, so the results are identical bit for bit.
    assert!(close(
        scrgb.to_xyz(extended),
        LinearSRgb.to_xyz(extended),
        0.0
    ));
    assert!(close(
        scrgb.from_xyz(scrgb.to_xyz(extended)),
        extended,
        1e-5
    ));
}
