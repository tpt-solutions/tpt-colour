//! ICC parsing, TRC evaluation and transforms.

use tpt_colour_core::space::{ColourSpace, D50, DisplayP3, SRgb, d65_to_d50};
use tpt_colour_icc::{ColourTransform, IccError, IccProfile, RenderingIntent, Trc};

const SAMPLES: [[f32; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0],
    [0.5, 0.5, 0.5],
    [0.8, 0.2, 0.1],
    [0.2, 0.5, 0.8],
    [0.35, 0.6, 0.12],
];

/// Test-only shorthand: `transform_rgb` for three-channel profiles, which cannot fail here.
trait RgbExt {
    fn rgb(&self, c: [f32; 3]) -> [f32; 3];
}

impl RgbExt for ColourTransform {
    fn rgb(&self, c: [f32; 3]) -> [f32; 3] {
        self.transform_rgb(c).unwrap()
    }
}

fn assert_near(a: [f32; 3], b: [f32; 3], tol: f32) {
    for i in 0..3 {
        assert!(
            (a[i] - b[i]).abs() <= tol,
            "component {i}: {a:?} vs {b:?} (tol {tol})"
        );
    }
}

fn s15(v: f32) -> [u8; 4] {
    ((v * 65_536.0).round() as i32).to_be_bytes()
}

fn xyz_tag(v: [f32; 3]) -> Vec<u8> {
    let mut t = b"XYZ \0\0\0\0".to_vec();
    for x in v {
        t.extend(s15(x));
    }
    t
}

fn para_tag(function: u16, params: &[f32]) -> Vec<u8> {
    let mut t = b"para\0\0\0\0".to_vec();
    t.extend(function.to_be_bytes());
    t.extend([0, 0]);
    for p in params {
        t.extend(s15(*p));
    }
    t
}

fn srgb_para() -> Vec<u8> {
    para_tag(3, &[2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.040_45])
}

/// Builds a minimal, valid matrix/TRC RGB profile with the given colorants (D50, columns)
/// and one TRC tag shared by all three channels.
fn build_profile(colorants: [[f32; 3]; 3], trc: &[u8]) -> Vec<u8> {
    let tags: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"rXYZ", xyz_tag(colorants[0])),
        (*b"gXYZ", xyz_tag(colorants[1])),
        (*b"bXYZ", xyz_tag(colorants[2])),
        (*b"rTRC", trc.to_vec()),
        (*b"gTRC", trc.to_vec()),
        (*b"bTRC", trc.to_vec()),
        (*b"wtpt", xyz_tag(D50)),
    ];
    let table_len = 4 + 12 * tags.len();
    let mut offset = 128 + table_len;
    let mut table: Vec<u8> = Vec::new();
    let mut body: Vec<u8> = Vec::new();
    for (sig, data) in &tags {
        table.extend(sig);
        table.extend((offset as u32).to_be_bytes());
        table.extend((data.len() as u32).to_be_bytes());
        body.extend(data);
        offset += data.len();
    }
    let mut out = vec![0_u8; 128];
    let total = 128 + table_len + body.len();
    out[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    out[8] = 4;
    out[9] = 0x30;
    out[12..16].copy_from_slice(b"mntr");
    out[16..20].copy_from_slice(b"RGB ");
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    out.extend((tags.len() as u32).to_be_bytes());
    out.extend(table);
    out.extend(body);
    out
}

/// The sRGB colorants as an ICC file would store them (D50-adapted).
fn srgb_colorants() -> [[f32; 3]; 3] {
    let col = |rgb| d65_to_d50(SRgb.to_xyz(rgb));
    let (r, g, b) = (
        col([1.0, 0.0, 0.0]),
        col([0.0, 1.0, 0.0]),
        col([0.0, 0.0, 1.0]),
    );
    [r, g, b]
}

#[test]
fn builtin_srgb_to_itself_is_identity() {
    let s = IccProfile::srgb();
    let t = ColourTransform::new(&s, &s, RenderingIntent::Perceptual).unwrap();
    for c in SAMPLES {
        assert_near(t.rgb(c), c, 1e-4);
    }
}

/// Milestone: sRGB ⇄ Display P3 accurate against the D65 reference maths in core.
#[test]
fn milestone_srgb_to_display_p3() {
    let srgb = IccProfile::srgb();
    let p3 = IccProfile::display_p3();
    let fwd = ColourTransform::new(&srgb, &p3, RenderingIntent::Perceptual).unwrap();
    let back = ColourTransform::new(&p3, &srgb, RenderingIntent::Perceptual).unwrap();
    for c in SAMPLES {
        // Device output is clipped to [0, 1], so clip the reference too.
        let expected = DisplayP3
            .from_xyz(SRgb.to_xyz(c))
            .map(|v| v.clamp(0.0, 1.0));
        assert_near(fwd.rgb(c), expected, 2e-5);
        let round = back.rgb(fwd.rgb(c));
        assert_near(round, c, 1e-3);
    }
}

#[test]
fn parsed_profile_matches_builtin() {
    let bytes = build_profile(srgb_colorants(), &srgb_para());
    let parsed = IccProfile::parse(&bytes).unwrap();
    assert_eq!(parsed.version.major, 4);
    assert_eq!(parsed.pcs, tpt_colour_icc::ProfileConnectionSpace::Xyz);
    assert_near(parsed.white_point, D50, 1e-4);

    let builtin = IccProfile::srgb();
    let p3 = IccProfile::display_p3();
    let from_parsed = ColourTransform::new(&parsed, &p3, RenderingIntent::Perceptual).unwrap();
    let from_builtin = ColourTransform::new(&builtin, &p3, RenderingIntent::Perceptual).unwrap();
    for c in SAMPLES {
        assert_near(from_parsed.rgb(c), from_builtin.rgb(c), 1e-4);
    }
}

#[test]
fn pixels_are_transformed_and_alpha_is_kept() {
    let t = ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::display_p3(),
        RenderingIntent::Perceptual,
    )
    .unwrap();
    let mut px = [[0.8, 0.2, 0.1, 0.25]];
    t.transform_pixels(&mut px).unwrap();
    let expected = DisplayP3.from_xyz(SRgb.to_xyz([0.8, 0.2, 0.1]));
    assert_near([px[0][0], px[0][1], px[0][2]], expected, 1e-3);
    assert!((px[0][3] - 0.25).abs() < 1e-9);
}

#[test]
fn truncated_profiles_are_rejected_without_panic() {
    let bytes = build_profile(srgb_colorants(), &srgb_para());
    for len in 0..bytes.len() {
        assert!(IccProfile::parse(&bytes[..len]).is_err(), "len {len}");
    }
}

#[test]
fn corrupted_profiles_never_panic() {
    let bytes = build_profile(srgb_colorants(), &srgb_para());
    for i in 0..bytes.len() {
        for value in [0x00_u8, 0xFF, 0x7F] {
            let mut bad = bytes.clone();
            bad[i] = value;
            // Any outcome is acceptable as long as it is not a panic.
            let _ = IccProfile::parse(&bad);
        }
    }
}

#[test]
fn bad_signature_is_rejected() {
    let mut bytes = build_profile(srgb_colorants(), &srgb_para());
    bytes[36..40].copy_from_slice(b"xxxx");
    assert_eq!(IccProfile::parse(&bytes), Err(IccError::BadSignature));
}

#[test]
fn lut_profiles_are_unsupported() {
    let mut bytes = build_profile(srgb_colorants(), &srgb_para());
    bytes[16..20].copy_from_slice(b"CMYK");
    assert_eq!(IccProfile::parse(&bytes), Err(IccError::Unsupported));
}

#[test]
fn gamma_and_identity_curves() {
    let gamma = Trc::Gamma(2.0);
    assert!((gamma.eval(0.5) - 0.25).abs() < 1e-6);
    assert!((gamma.eval_inverse(0.25) - 0.5).abs() < 1e-6);
    assert!((Trc::Identity.eval(0.3) - 0.3).abs() < 1e-9);
}

#[test]
fn table_curve_interpolates_and_inverts() {
    let t = Trc::Table(vec![0.0, 0.1, 0.4, 1.0]);
    assert!((t.eval(1.0 / 3.0) - 0.1).abs() < 1e-6);
    for i in 0..=10 {
        let x = i as f32 / 10.0;
        assert!((t.eval_inverse(t.eval(x)) - x).abs() < 1e-3, "x = {x}");
    }
}

#[test]
fn parametric_curves_invert() {
    let curves = [
        (0_u16, vec![2.2]),
        (1, vec![2.2, 1.0, 0.0]),
        (2, vec![2.2, 0.9, 0.0, 0.1]),
        (
            3,
            vec![2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.040_45],
        ),
        // Continuous at d = 0.1: (a·d + b)^g + e = c·d + f. Scaled so the curve stays within [0, 1].
        (
            4,
            vec![
                2.2,
                0.9,
                0.0,
                0.1,
                0.1,
                0.1 * 0.1 - (0.9_f32 * 0.1).powf(2.2),
                0.0,
            ],
        ),
    ];
    for (function, raw) in curves {
        let mut params = [0.0_f32; 7];
        params[..raw.len()].copy_from_slice(&raw);
        let trc = Trc::Parametric { function, params };
        for i in 1..=10 {
            let x = i as f32 / 10.0;
            let round = trc.eval_inverse(trc.eval(x));
            assert!(
                (round - x).abs() < 2e-3,
                "type {function}: x = {x}, got {round}"
            );
        }
    }
}

/// The single-colour path evaluates exact curves in f64, so it should match the core reference
/// to f32 rounding, far tighter than the baked-table path.
#[test]
fn single_colour_path_matches_reference_to_f32_precision() {
    let fwd = ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::display_p3(),
        RenderingIntent::Perceptual,
    )
    .unwrap();
    for c in SAMPLES {
        let expected = DisplayP3
            .from_xyz(SRgb.to_xyz(c))
            .map(|v| v.clamp(0.0, 1.0));
        assert_near(fwd.rgb(c), expected, 2e-6);
    }
}
