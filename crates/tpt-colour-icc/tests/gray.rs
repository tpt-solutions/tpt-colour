//! Grey (`GRAY`, `kTRC`) profiles: the TRC gives relative luminance, and PCS is D50 · Y.

use tpt_colour_core::space::{ColourSpace, SRgb};
use tpt_colour_icc::{ColourTransform, IccError, IccProfile, RenderingIntent};

/// A `curv` tag holding one gamma value (u8Fixed8).
fn gamma_curve(gamma: f32) -> Vec<u8> {
    let mut t = b"curv".to_vec();
    t.extend([0_u8; 4]);
    t.extend(1_u32.to_be_bytes());
    t.extend(((gamma * 256.0).round() as u16).to_be_bytes());
    t
}

/// A grey profile with a single `kTRC` and XYZ PCS.
fn grey_profile(gamma: f32) -> Vec<u8> {
    let tags = [(*b"kTRC", gamma_curve(gamma))];
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
    let total = 128 + table_len + body.len();
    let mut out = vec![0_u8; 128];
    out[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    out[8] = 2;
    out[12..16].copy_from_slice(b"mntr");
    out[16..20].copy_from_slice(b"GRAY");
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    out.extend((tags.len() as u32).to_be_bytes());
    out.extend(table);
    out.extend(body);
    out
}

#[test]
fn grey_profile_parses_with_one_channel() {
    let profile = IccProfile::parse(&grey_profile(2.2)).unwrap();
    assert_eq!(profile.device_channels, 1);
}

#[test]
fn grey_to_srgb_follows_the_curve_and_is_neutral() {
    let profile = IccProfile::parse(&grey_profile(2.2)).unwrap();
    let t = ColourTransform::new(
        &profile,
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    assert_eq!(t.src_channels(), 1);
    for g in [0.0_f32, 0.25, 0.5, 1.0] {
        let mut out = [0.0_f32; 3];
        t.transform(&[g], &mut out).unwrap();
        // Relative luminance is g^γ, and sRGB encodes that linear value.
        let expected = SRgb.from_xyz(tpt_colour_core::space::D65.map(|w| w * g.powf(2.2)));
        for c in 0..3 {
            assert!(
                (out[c] - expected[c]).abs() < 1e-3,
                "g {g}: {out:?} vs {expected:?}"
            );
        }
        assert!(
            (out[0] - out[2]).abs() < 1e-4,
            "grey must stay neutral: {out:?}"
        );
    }
}

#[test]
fn srgb_to_grey_round_trips() {
    let grey = IccProfile::parse(&grey_profile(2.2)).unwrap();
    let to_grey = ColourTransform::new(
        &IccProfile::srgb(),
        &grey,
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    assert_eq!(to_grey.dst_channels(), 1);
    let mut g = [0.0_f32; 1];
    to_grey.transform(&[0.5, 0.5, 0.5], &mut g).unwrap();
    // sRGB mid-grey is about 21% linear luminance, which the 2.2 curve encodes near 0.5.
    assert!((g[0] - 0.5).abs() < 0.02, "{g:?}");
}

#[test]
fn grey_profile_rejects_wrong_channel_counts() {
    let profile = IccProfile::parse(&grey_profile(2.2)).unwrap();
    let t = ColourTransform::new(
        &profile,
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    assert_eq!(
        t.transform(&[0.5, 0.5], &mut [0.0; 3]),
        Err(IccError::ChannelCount)
    );
}
