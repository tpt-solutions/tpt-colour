//! `.cube` export: header, point order (red fastest), and end points.

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

fn parse_values(text: &str) -> Vec<[f32; 3]> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace().map(str::parse::<f32>);
            match (parts.next(), parts.next(), parts.next()) {
                (Some(Ok(r)), Some(Ok(g)), Some(Ok(b))) => Some([r, g, b]),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn cube_has_a_header_and_red_fastest_points() {
    let t = ColourTransform::new(
        &IccProfile::display_p3(),
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    )
    .unwrap();
    let text = t.to_cube(5, "P3 \"to\" sRGB").unwrap();
    assert!(text.starts_with("TITLE \"P3 to sRGB\"\n"), "{text}");
    assert!(text.contains("LUT_3D_SIZE 5\n"));
    assert!(text.contains("DOMAIN_MAX 1.0 1.0 1.0\n"));

    let values = parse_values(&text);
    assert_eq!(values.len(), 5 * 5 * 5);

    // Index 1 is the next red step: red varies fastest, so it is the second point.
    let mut expected = [0.0_f32; 3];
    t.transform(&[0.25, 0.0, 0.0], &mut expected).unwrap();
    for c in 0..3 {
        assert!(
            (values[1][c] - expected[c]).abs() < 1e-5,
            "{:?} vs {:?}",
            values[1],
            expected
        );
    }
    // Last point is white.
    let mut white = [0.0_f32; 3];
    t.transform(&[1.0, 1.0, 1.0], &mut white).unwrap();
    assert!((values[124][0] - white[0]).abs() < 1e-5);
}

#[test]
fn cube_rejects_a_size_under_two() {
    let t = ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::srgb(),
        RenderingIntent::Perceptual,
    )
    .unwrap();
    assert!(t.to_cube(1, "x").is_err());
}
