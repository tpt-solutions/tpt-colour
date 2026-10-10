//! LUT profiles: mAB/mBA (CMYK), mft2 (RGB), mft1 (RGB), intents, black point compensation,
//! and bounded parsing. The profiles are built in the test from analytic models in core.
//!
// Byte signatures are passed by reference throughout so call sites read as `b"mAB "`.
#![allow(clippy::trivially_copy_pass_by_ref)]

use tpt_colour_core::space::{Cmyk, ColourSpace, SRgb, d50_to_d65, d65_to_d50};
use tpt_colour_icc::pcs::{lab_to_xyz_d50, xyz_to_lab_d50};
use tpt_colour_icc::{ColourTransform, IccError, IccProfile, RenderingIntent};

const CMYK_SAMPLES: [[f32; 4]; 4] = [
    [0.13, 0.42, 0.77, 0.31],
    [0.9, 0.1, 0.05, 0.0],
    [0.0, 0.0, 0.0, 0.0],
    [0.4, 0.4, 0.4, 0.2],
];

/// Builds a profile from a header and tags.
fn assemble(data: &[u8; 4], pcs: &[u8; 4], major: u8, tags: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let table_len = 4 + 12 * tags.len();
    let mut offset = 128 + table_len;
    let mut table: Vec<u8> = Vec::new();
    let mut body: Vec<u8> = Vec::new();
    for (sig, data) in tags {
        table.extend(sig);
        table.extend((offset as u32).to_be_bytes());
        table.extend((data.len() as u32).to_be_bytes());
        body.extend(data);
        offset += data.len();
    }
    let total = 128 + table_len + body.len();
    let mut out = vec![0_u8; 128];
    out[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    out[8] = major;
    out[12..16].copy_from_slice(b"prtr");
    out[16..20].copy_from_slice(data);
    out[20..24].copy_from_slice(pcs);
    out[36..40].copy_from_slice(b"acsp");
    out.extend((tags.len() as u32).to_be_bytes());
    out.extend(table);
    out.extend(body);
    out
}

/// Visits every grid point of a `dims`-dimensional `g`-point grid, first dimension slowest.
fn sample_grid(dims: usize, g: usize, f: impl Fn(&[f32]) -> Vec<f32>) -> Vec<f32> {
    let mut out = Vec::new();
    let mut point = vec![0.0_f32; dims];
    for idx in 0..g.pow(dims as u32) {
        let mut rem = idx;
        for d in (0..dims).rev() {
            point[d] = (rem % g) as f32 / (g - 1) as f32;
            rem /= g;
        }
        out.extend(f(&point));
    }
    out
}

fn u16_be(v: f32) -> [u8; 2] {
    ((v.clamp(0.0, 1.0) * 65_535.0).round() as u16).to_be_bytes()
}

/// `mAB ` (A2B) or `mBA ` (B2A) with curves on the input and output sides and a 16-bit CLUT.
fn mab_tag(sig: &[u8; 4], input: usize, output: usize, grid: usize, samples: &[f32]) -> Vec<u8> {
    let curves_n = if sig == b"mAB " { input } else { output };
    let a_at = 32;
    let clut_at = a_at + 12 * curves_n;
    let mut t = vec![0_u8; clut_at];
    t[0..4].copy_from_slice(sig);
    t[8] = input as u8;
    t[9] = output as u8;
    t[24..28].copy_from_slice(&(clut_at as u32).to_be_bytes());
    t[28..32].copy_from_slice(&(a_at as u32).to_be_bytes());
    for i in 0..curves_n {
        let at = a_at + 12 * i;
        t[at..at + 4].copy_from_slice(b"curv");
    }
    let mut grid_bytes = [0_u8; 16];
    for g in grid_bytes.iter_mut().take(input) {
        *g = grid as u8;
    }
    t.extend(grid_bytes);
    t.push(2);
    t.extend([0, 0, 0]);
    for &v in samples {
        t.extend(u16_be(v));
    }
    t
}

/// `mft2` (lut16Type) with identity input and output tables.
fn mft2_tag(input: usize, output: usize, grid: usize, samples: &[f32]) -> Vec<u8> {
    let entries = 256_usize;
    let mut t = vec![0_u8; 52];
    t[0..4].copy_from_slice(b"mft2");
    t[8] = input as u8;
    t[9] = output as u8;
    t[10] = grid as u8;
    write_identity_matrix(&mut t);
    t[48..50].copy_from_slice(&(entries as u16).to_be_bytes());
    t[50..52].copy_from_slice(&(entries as u16).to_be_bytes());
    let identity: Vec<u16> = (0..entries)
        .map(|i| (i as f32 / (entries - 1) as f32 * 65_535.0).round() as u16)
        .collect();
    for _ in 0..input {
        for v in &identity {
            t.extend(v.to_be_bytes());
        }
    }
    for &v in samples {
        t.extend(u16_be(v));
    }
    for _ in 0..output {
        for v in &identity {
            t.extend(v.to_be_bytes());
        }
    }
    t
}

/// `mft1` (lut8Type) with identity 256-entry tables.
fn mft1_tag(input: usize, output: usize, grid: usize, samples: &[f32]) -> Vec<u8> {
    let mut t = vec![0_u8; 48];
    t[0..4].copy_from_slice(b"mft1");
    t[8] = input as u8;
    t[9] = output as u8;
    t[10] = grid as u8;
    write_identity_matrix(&mut t);
    let identity: Vec<u8> = (0..=255_u8).collect();
    for _ in 0..input {
        t.extend(&identity);
    }
    for &v in samples {
        t.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
    }
    for _ in 0..output {
        t.extend(&identity);
    }
    t
}

fn write_identity_matrix(t: &mut [u8]) {
    for (i, v) in [1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        .iter()
        .enumerate()
    {
        t[12 + i * 4..16 + i * 4].copy_from_slice(&((v * 65_536.0) as i32).to_be_bytes());
    }
}

/// Normalised ICC v4 Lab from physical Lab (D50).
fn lab_v4_norm(lab: [f32; 3]) -> [f32; 3] {
    [
        lab[0] / 100.0,
        (lab[1] + 128.0) / 255.0,
        (lab[2] + 128.0) / 255.0,
    ]
}

/// Physical Lab (D50) from normalised ICC v4 Lab.
fn lab_v4_phys(n: [f32; 3]) -> [f32; 3] {
    [n[0] * 100.0, n[1] * 255.0 - 128.0, n[2] * 255.0 - 128.0]
}

/// Normalised ICC v2 16-bit Lab from physical Lab (D50).
fn lab_v2_norm(lab: [f32; 3]) -> [f32; 3] {
    [
        lab[0] * 65_280.0 / (65_535.0 * 100.0),
        (lab[1] + 128.0) * 256.0 / 65_535.0,
        (lab[2] + 128.0) * 256.0 / 65_535.0,
    ]
}

/// Physical Lab (D50) from normalised ICC v2 16-bit Lab.
fn lab_v2_phys(n: [f32; 3]) -> [f32; 3] {
    [
        n[0] * 65_535.0 / 65_280.0 * 100.0,
        n[1] * 65_535.0 / 256.0 - 128.0,
        n[2] * 65_535.0 / 256.0 - 128.0,
    ]
}

/// Analytic CMYK → normalised v4 Lab, using the naive model in core.
fn cmyk_to_lab_norm(cmyk: &[f32]) -> Vec<f32> {
    let xyz_d65 = Cmyk.to_xyz([cmyk[0], cmyk[1], cmyk[2], cmyk[3]]);
    let lab = xyz_to_lab_d50(d65_to_d50(xyz_d65));
    lab_v4_norm(lab).to_vec()
}

/// Analytic normalised v4 Lab → CMYK, using the naive model's inverse.
fn lab_norm_to_cmyk(n: &[f32]) -> Vec<f32> {
    let lab = lab_v4_phys([n[0], n[1], n[2]]);
    let xyz_d65 = d50_to_d65(lab_to_xyz_d50(lab));
    Cmyk.from_xyz(xyz_d65).to_vec()
}

/// A CMYK profile with a mAB A2B0 and mBA B2A0 table, both on a `grid`-point grid.
fn cmyk_profile(grid: usize) -> IccProfile {
    let a2b = sample_grid(4, grid, cmyk_to_lab_norm);
    let b2a = sample_grid(3, grid, lab_norm_to_cmyk);
    let bytes = assemble(
        b"CMYK",
        b"Lab ",
        4,
        &[
            (*b"A2B0", mab_tag(b"mAB ", 4, 3, grid, &a2b)),
            (*b"B2A0", mab_tag(b"mBA ", 3, 4, grid, &b2a)),
        ],
    );
    IccProfile::parse(&bytes).expect("CMYK profile parses")
}

fn max_component_error(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).abs()).fold(0.0, f32::max)
}

#[test]
fn cmyk_profile_parses_with_expected_channels() {
    let p = cmyk_profile(5);
    assert_eq!(p.device_channels, 4);
    assert_eq!(p.pcs, tpt_colour_icc::ProfileConnectionSpace::Lab);
}

/// Milestone: CMYK → Lab (through the sRGB display path) matches the analytic model.
#[test]
fn milestone_cmyk_to_srgb_matches_model() {
    let profile = cmyk_profile(17);
    let t =
        ColourTransform::new(&profile, &IccProfile::srgb(), RenderingIntent::Perceptual).unwrap();
    for c in CMYK_SAMPLES {
        let mut out = [0.0_f32; 3];
        t.transform(&c, &mut out).unwrap();
        let expected = SRgb.from_xyz(Cmyk.to_xyz(c));
        let err = max_component_error(out, expected);
        assert!(
            err < 0.02,
            "{c:?}: got {out:?}, expected {expected:?}, err {err}"
        );
    }
}

/// Milestone: sRGB → CMYK → sRGB round trip.
///
/// The tolerance is wider than for the forward direction. The naive CMYK model used here has
/// gradient discontinuities where its black generation switches channels. Grid interpolation
/// is first-order across those surfaces, and the error does not fall monotonically with grid
/// size (measured at cyan `[0.1, 0.9, 0.95]`: 0.07 on a 33-point grid, 0.02 on 65). Smoother
/// production profiles do not show this.
#[test]
fn milestone_cmyk_round_trip_through_lab() {
    let profile = cmyk_profile(33);
    let to_cmyk =
        ColourTransform::new(&IccProfile::srgb(), &profile, RenderingIntent::Perceptual).unwrap();
    let to_srgb =
        ColourTransform::new(&profile, &IccProfile::srgb(), RenderingIntent::Perceptual).unwrap();
    for c in CMYK_SAMPLES {
        let rgb = SRgb.from_xyz(Cmyk.to_xyz(c));
        let mut cmyk = [0.0_f32; 4];
        to_cmyk.transform(&rgb, &mut cmyk).unwrap();
        let mut back = [0.0_f32; 3];
        to_srgb.transform(&cmyk, &mut back).unwrap();
        let err = max_component_error(back, rgb);
        assert!(
            err < 0.08,
            "{c:?}: sRGB {rgb:?} → CMYK {cmyk:?} → {back:?}, err {err}"
        );
    }
}

/// Returns the A2B0 pipeline of a LUT profile.
fn a2b_pipeline(profile: &IccProfile) -> &tpt_colour_icc::Pipeline {
    match &profile.model {
        tpt_colour_icc::ProfileModel::Lut(t) => t.a2b(RenderingIntent::Perceptual).unwrap(),
        tpt_colour_icc::ProfileModel::Matrix(_) => panic!("expected a LUT profile"),
    }
}

/// Tetrahedral interpolation must reproduce linear functions exactly.
#[test]
fn tetrahedral_reproduces_linear_functions() {
    let linear = |x: &[f32]| {
        vec![
            0.2 * x[0] + 0.3 * x[1] + 0.5 * x[2],
            0.5 * x[0] + 0.25 * x[1] + 0.25 * x[2],
            0.1 + 0.2 * x[0] + 0.3 * x[1] + 0.2 * x[2],
        ]
    };
    let a2b = sample_grid(3, 5, linear);
    let bytes = assemble(
        b"RGB ",
        b"Lab ",
        4,
        &[(*b"A2B0", mab_tag(b"mAB ", 3, 3, 5, &a2b))],
    );
    let profile = IccProfile::parse(&bytes).unwrap();
    let pipe = a2b_pipeline(&profile);
    for p in [
        [0.13_f32, 0.77, 0.42],
        [0.5, 0.5, 0.5],
        [0.99, 0.01, 0.6],
        [0.0, 1.0, 0.25],
    ] {
        let mut out = [0.0_f32; 3];
        pipe.eval(&p, &mut out).unwrap();
        let expected = linear(&p);
        for i in 0..3 {
            assert!(
                (out[i] - expected[i]).abs() < 1e-3,
                "{p:?}: {out:?} vs {expected:?}"
            );
        }
    }
}

/// Multilinear interpolation must reproduce linear functions of four inputs exactly.
#[test]
fn multilinear_reproduces_linear_functions_of_four_inputs() {
    let linear = |x: &[f32]| vec![0.1 + 0.2 * x[0] + 0.3 * x[1] + 0.1 * x[2] + 0.2 * x[3]];
    let a2b = sample_grid(4, 3, linear);
    let bytes = assemble(
        b"CMYK",
        b"Lab ",
        4,
        &[(*b"A2B0", mab_tag(b"mAB ", 4, 1, 3, &a2b))],
    );
    let profile = IccProfile::parse(&bytes).unwrap();
    let pipe = a2b_pipeline(&profile);
    assert_eq!(pipe.input_channels(), 4);
    assert_eq!(pipe.output_channels(), 1);
    for p in [[0.2_f32, 0.9, 0.4, 0.7], [0.05, 0.5, 0.95, 0.3]] {
        let mut out = [0.0_f32; 1];
        pipe.eval(&p, &mut out).unwrap();
        assert!((out[0] - linear(&p)[0]).abs() < 1e-3, "{p:?}: {out:?}");
    }
}

#[test]
fn lut_profile_rejects_wrong_channel_counts() {
    let t = ColourTransform::new(
        &cmyk_profile(5),
        &IccProfile::srgb(),
        RenderingIntent::Perceptual,
    )
    .unwrap();
    let mut out = [0.0_f32; 3];
    assert_eq!(
        t.transform(&[0.1, 0.2, 0.3], &mut out),
        Err(IccError::ChannelCount)
    );
    assert_eq!(t.src_channels(), 4);
    assert_eq!(t.dst_channels(), 3);
}

#[test]
fn missing_intent_table_is_unsupported() {
    let profile = cmyk_profile(5);
    let r = ColourTransform::new(
        &profile,
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    );
    assert_eq!(r.err(), Some(IccError::Unsupported));
}

/// Black point compensation maps the source black onto the destination black.
#[test]
fn black_point_compensation_lifts_black_to_destination_black() {
    // Same as the naive model, but K = 1 has a little light (a non-zero black point).
    let lifted = |c: &[f32]| {
        let mut lab = cmyk_to_lab_norm(c);
        if c[3] > 0.0 {
            // Add a lightness floor proportional to K, as a real press black would have.
            lab[0] += 0.2 * c[3];
        }
        lab
    };
    let a2b = sample_grid(4, 9, lifted);
    let bytes = assemble(
        b"CMYK",
        b"Lab ",
        4,
        &[(*b"A2B0", mab_tag(b"mAB ", 4, 3, 9, &a2b))],
    );
    let profile = IccProfile::parse(&bytes).unwrap();
    let black = [0.0_f32, 0.0, 0.0, 1.0];
    let srgb = IccProfile::srgb();

    let mut plain = [0.0_f32; 3];
    ColourTransform::new(&profile, &srgb, RenderingIntent::Perceptual)
        .unwrap()
        .transform(&black, &mut plain)
        .unwrap();
    let mut compensated = [0.0_f32; 3];
    ColourTransform::with_black_point_compensation(&profile, &srgb, RenderingIntent::Perceptual)
        .unwrap()
        .transform(&black, &mut compensated)
        .unwrap();

    assert!(
        plain[0] > 0.05,
        "uncompensated black should be visibly grey: {plain:?}"
    );
    assert!(
        compensated[0] < 0.01,
        "compensated black should reach black: {compensated:?}"
    );
}

/// A v2 RGB profile whose lut16 (`mft2`) tables hold sRGB in v2 16-bit Lab.
fn rgb_lab_profile_mft2(grid: usize) -> IccProfile {
    let a2b = sample_grid(3, grid, |rgb| {
        let xyz = SRgb.to_xyz([rgb[0], rgb[1], rgb[2]]);
        let lab = xyz_to_lab_d50(d65_to_d50(xyz));
        lab_v2_norm(lab).to_vec()
    });
    let b2a = sample_grid(3, grid, |n| {
        let lab = lab_v2_phys([n[0], n[1], n[2]]);
        let xyz = d50_to_d65(lab_to_xyz_d50(lab));
        SRgb.from_xyz(xyz).to_vec()
    });
    let bytes = assemble(
        b"RGB ",
        b"Lab ",
        2,
        &[
            (*b"A2B0", mft2_tag(3, 3, grid, &a2b)),
            (*b"B2A0", mft2_tag(3, 3, grid, &b2a)),
        ],
    );
    IccProfile::parse(&bytes).expect("mft2 profile parses")
}

/// lut16 (`mft2`) with v2 16-bit Lab: a sRGB-equivalent LUT profile should act like sRGB.
#[test]
fn lut16_v2_lab_profile_round_trips_srgb() {
    let profile = rgb_lab_profile_mft2(17);
    let srgb = IccProfile::srgb();
    let to_pcs = ColourTransform::new(&profile, &srgb, RenderingIntent::Perceptual).unwrap();
    let from_pcs = ColourTransform::new(&srgb, &profile, RenderingIntent::Perceptual).unwrap();
    for c in [
        [0.2_f32, 0.5, 0.8],
        [0.7, 0.3, 0.1],
        [0.45, 0.45, 0.45],
        [0.9, 0.9, 0.2],
    ] {
        let back = to_pcs.transform_rgb(c).unwrap();
        assert!(max_component_error(back, c) < 0.02, "{c:?} → {back:?}");
        let device = from_pcs.transform_rgb(c).unwrap();
        assert!(max_component_error(device, c) < 0.02, "{c:?} → {device:?}");
    }
}

/// lut8 (`mft1`) uses exact 8-bit Lab, so the same identity check holds at 8-bit precision.
#[test]
fn lut8_profile_maps_like_srgb() {
    let grid = 9;
    let a2b = sample_grid(3, grid, |rgb| {
        let lab = xyz_to_lab_d50(d65_to_d50(SRgb.to_xyz([rgb[0], rgb[1], rgb[2]])));
        lab_v4_norm(lab).to_vec()
    });
    let bytes = assemble(
        b"RGB ",
        b"Lab ",
        2,
        &[(*b"A2B0", mft1_tag(3, 3, grid, &a2b))],
    );
    let profile = IccProfile::parse(&bytes).expect("mft1 profile parses");
    let t =
        ColourTransform::new(&profile, &IccProfile::srgb(), RenderingIntent::Perceptual).unwrap();
    for c in [[0.2_f32, 0.5, 0.8], [0.7, 0.3, 0.1]] {
        let out = t.transform_rgb(c).unwrap();
        assert!(max_component_error(out, c) < 0.02, "{c:?} → {out:?}");
    }
}

#[test]
fn oversized_clut_is_rejected_before_allocation() {
    // 15 inputs on a 255-point grid is far beyond the sample cap.
    let mut t = vec![0_u8; 64];
    t[0..4].copy_from_slice(b"mAB ");
    t[8] = 15;
    t[9] = 3;
    t[24..28].copy_from_slice(&32_u32.to_be_bytes());
    let mut clut = vec![255_u8; 16];
    clut.push(2);
    clut.extend([0, 0, 0]);
    t.truncate(32);
    t.extend(clut);
    let bytes = assemble(b"CMYK", b"Lab ", 4, &[(*b"A2B0", t)]);
    assert!(IccProfile::parse(&bytes).is_err());
}

#[test]
fn truncated_lut_profiles_are_rejected() {
    let bytes = assemble(
        b"CMYK",
        b"Lab ",
        4,
        &[(*b"A2B0", mab_tag(b"mAB ", 4, 3, 3, &vec![0.5; 81 * 3]))],
    );
    for len in 0..bytes.len() {
        assert!(IccProfile::parse(&bytes[..len]).is_err(), "len {len}");
    }
}

#[test]
fn corrupted_lut_profiles_never_panic() {
    let samples = sample_grid(4, 3, cmyk_to_lab_norm);
    let bytes = assemble(
        b"CMYK",
        b"Lab ",
        4,
        &[(*b"A2B0", mab_tag(b"mAB ", 4, 3, 3, &samples))],
    );
    for i in 0..bytes.len() {
        for v in [0x00_u8, 0xFF, 0x7F] {
            let mut bad = bytes.clone();
            bad[i] = v;
            if let Ok(p) = IccProfile::parse(&bad) {
                let _ = ColourTransform::new(&p, &IccProfile::srgb(), RenderingIntent::Perceptual);
            }
        }
    }
}

#[test]
fn mpet_is_unsupported() {
    let mut t = vec![0_u8; 40];
    t[0..4].copy_from_slice(b"mpet");
    let bytes = assemble(b"CMYK", b"Lab ", 4, &[(*b"A2B0", t)]);
    assert_eq!(IccProfile::parse(&bytes).err(), Some(IccError::Unsupported));
}
