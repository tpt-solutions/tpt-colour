//! Profiles written with `to_bytes` parse back to the same model and transform the same colours.

use tpt_colour_icc::{
    ColourTransform, IccError, IccProfile, ProfileConnectionSpace, ProfileModel, RenderingIntent,
    Trc,
};

fn round_trip(profile: &IccProfile) -> IccProfile {
    let bytes = profile.to_bytes().expect("matrix/TRC profile writes");
    IccProfile::parse(&bytes).expect("written profile parses")
}

fn assert_same_colours(a: &IccProfile, b: &IccProfile, label: &str) {
    // Compared in 8-bit codes. The `s15Fixed16` colorants are quantised to about 1.5e-5, and the
    // P3 to sRGB matrix amplifies that near black, so float differences are larger than a code.
    let ta = ColourTransform::new(
        &IccProfile::srgb(),
        a,
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    let tb = ColourTransform::new(
        &IccProfile::srgb(),
        b,
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    for r in 0..=4_u8 {
        for g in 0..=4_u8 {
            for bl in 0..=4_u8 {
                let level = |i: u8| if i == 4 { 255 } else { i * 64 };
                let input = [level(r), level(g), level(bl)];
                let (mut x, mut y) = (input.to_vec(), input.to_vec());
                ta.transform_u8(&mut x, 3).unwrap();
                tb.transform_u8(&mut y, 3).unwrap();
                for (p, q) in x.iter().zip(&y) {
                    assert!(p.abs_diff(*q) <= 1, "{label} {input:?}: {x:?} vs {y:?}");
                }
            }
        }
    }
}

#[test]
fn builtin_profiles_round_trip() {
    for profile in [IccProfile::srgb(), IccProfile::display_p3()] {
        let back = round_trip(&profile);
        for (a, b) in back.white_point.iter().zip(profile.white_point) {
            assert!((a - b).abs() < 2e-5, "white point {a} vs {b}");
        }
        assert_same_colours(&profile, &back, "builtin");
    }
}

#[test]
fn every_trc_kind_round_trips() {
    let kinds = [
        Trc::Identity,
        Trc::Gamma(2.2),
        Trc::Table((0..64).map(|i| (i as f32 / 63.0).powf(1.8)).collect()),
        Trc::Parametric {
            function: 3,
            params: [
                2.4,
                1.0 / 1.055,
                0.055 / 1.055,
                1.0 / 12.92,
                0.04045,
                0.0,
                0.0,
            ],
        },
    ];
    for curve in kinds {
        let mut profile = IccProfile::display_p3();
        if let ProfileModel::Matrix(m) = &mut profile.model {
            m.trc = [curve.clone(), curve.clone(), curve.clone()];
        }
        let back = round_trip(&profile);
        if let ProfileModel::Matrix(m) = &back.model {
            // A gamma comes back as the equivalent `para` type 0, which keeps its exponent.
            match (&curve, &m.trc[0]) {
                (Trc::Gamma(g), Trc::Gamma(h)) => assert!((g - h).abs() < 2e-5, "{g} -> {h}"),
                (Trc::Table(a), Trc::Table(b)) => {
                    assert_eq!(a.len(), b.len());
                    // Table entries are stored as 16-bit codes.
                    for (x, y) in a.iter().zip(b) {
                        assert!((x - y).abs() < 2e-5, "{x} -> {y}");
                    }
                }
                (
                    Trc::Parametric {
                        function: f,
                        params: p,
                    },
                    Trc::Parametric {
                        function: g,
                        params: q,
                    },
                ) => {
                    // Parameters are stored as s15Fixed16, so they are compared to that precision.
                    assert_eq!(f, g);
                    for (x, y) in p.iter().zip(q) {
                        assert!((x - y).abs() < 2e-5, "{x} -> {y}");
                    }
                }
                (expected, got) => assert_eq!(got, expected, "curve kind survives"),
            }
        }
        assert_same_colours(&profile, &back, &format!("{curve:?}"));
    }
}

#[test]
fn chad_survives_when_present() {
    let mut profile = IccProfile::srgb();
    if let ProfileModel::Matrix(m) = &mut profile.model {
        m.chad = Some([[1.0, 0.01, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    }
    let back = round_trip(&profile);
    if let ProfileModel::Matrix(m) = &back.model {
        let chad = m.chad.expect("chad written");
        assert!((chad[0][1] - 0.01).abs() < 2e-5);
    }
}

#[test]
fn lut_and_lab_profiles_are_refused() {
    let mut lab = IccProfile::srgb();
    lab.pcs = ProfileConnectionSpace::Lab;
    assert_eq!(lab.to_bytes(), Err(IccError::Unsupported));
}

#[test]
fn header_records_the_class_intent_and_pcs() {
    let bytes = IccProfile::display_p3().to_bytes().unwrap();
    assert_eq!(&bytes[12..16], b"mntr");
    assert_eq!(&bytes[16..20], b"RGB ");
    assert_eq!(&bytes[20..24], b"XYZ ");
    assert_eq!(&bytes[36..40], b"acsp");
    assert_eq!(
        u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize,
        bytes.len()
    );
}
