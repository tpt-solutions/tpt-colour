//! Compositing: premultiplied maths, Porter-Duff operators, blend modes and edge alphas.

use tpt_colour_core::composite::{BlendMode, PorterDuff, PremultipliedRgba, blend, blend_over};

const RED: PremultipliedRgba = PremultipliedRgba {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
const BLUE_HALF: PremultipliedRgba = PremultipliedRgba {
    r: 0.0,
    g: 0.0,
    b: 0.5,
    a: 0.5,
};
const WHITE: PremultipliedRgba = PremultipliedRgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
const GREY: PremultipliedRgba = PremultipliedRgba {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};

fn assert_px(a: PremultipliedRgba, b: PremultipliedRgba) {
    let (a, b) = ([a.r, a.g, a.b, a.a], [b.r, b.g, b.b, b.a]);
    for i in 0..4 {
        assert!((a[i] - b[i]).abs() < 1e-5, "{a:?} vs {b:?}");
    }
}

#[test]
fn straight_premultiplied_round_trip() {
    let straight = [0.2, 0.4, 0.6, 0.5];
    let back = PremultipliedRgba::from_straight(straight).to_straight();
    for i in 0..4 {
        assert!((back[i] - straight[i]).abs() < 1e-6);
    }
}

#[test]
fn alpha_zero_has_no_colour() {
    // Transparent pixels carry no colour, so they cannot cause dark fringes.
    let p = PremultipliedRgba::from_straight([1.0, 1.0, 1.0, 0.0]);
    assert!(p.to_straight().iter().all(|c| c.abs() < 1e-9));
}

#[test]
fn interpolation_has_no_dark_fringe() {
    // Halfway between opaque red and transparent: straight colour stays red.
    let transparent = PremultipliedRgba::TRANSPARENT;
    let mid = PremultipliedRgba {
        r: (RED.r + transparent.r) * 0.5,
        g: (RED.g + transparent.g) * 0.5,
        b: (RED.b + transparent.b) * 0.5,
        a: (RED.a + transparent.a) * 0.5,
    };
    let straight = mid.to_straight();
    assert!((straight[0] - 1.0).abs() < 1e-6);
    assert!(straight[1].abs() < 1e-6 && straight[2].abs() < 1e-6);
    assert!((straight[3] - 0.5).abs() < 1e-6);
}

#[test]
fn source_over_opaque_source_replaces_destination() {
    assert_px(blend_over(RED, WHITE), RED);
    assert_px(PorterDuff::SourceOver.compose(RED, BLUE_HALF), RED);
}

#[test]
fn source_over_transparent_source_keeps_destination() {
    assert_px(blend_over(PremultipliedRgba::TRANSPARENT, GREY), GREY);
}

#[test]
fn porter_duff_edge_alphas() {
    let t = PremultipliedRgba::TRANSPARENT;
    assert_px(PorterDuff::Clear.compose(RED, WHITE), t);
    assert_px(PorterDuff::Source.compose(RED, WHITE), RED);
    assert_px(PorterDuff::Destination.compose(RED, WHITE), WHITE);
    assert_px(PorterDuff::SourceIn.compose(RED, t), t);
    assert_px(PorterDuff::SourceOut.compose(RED, t), RED);
    assert_px(PorterDuff::Xor.compose(RED, WHITE), t);
    assert_px(PorterDuff::SourceAtop.compose(RED, t), t);
    assert_px(PorterDuff::DestinationAtop.compose(RED, t), RED);
}

#[test]
fn multiply_with_white_is_identity() {
    assert_px(blend(BlendMode::Multiply, WHITE, GREY), GREY);
}

#[test]
fn multiply_with_black_is_black() {
    let black = PremultipliedRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    assert_px(blend(BlendMode::Multiply, black, GREY), black);
}

#[test]
fn blend_over_transparent_destination_is_source() {
    for mode in [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::HardLight,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::Difference,
        BlendMode::Exclusion,
    ] {
        assert_px(
            blend(mode, BLUE_HALF, PremultipliedRgba::TRANSPARENT),
            BLUE_HALF,
        );
    }
}

#[test]
fn blend_of_two_transparent_is_transparent() {
    let t = PremultipliedRgba::TRANSPARENT;
    assert_px(blend(BlendMode::Multiply, t, t), t);
}

#[test]
fn screen_and_overlay_on_opaque_grey() {
    // Screen(0.5, 0.5) = 0.75.
    let out = blend(BlendMode::Screen, GREY, GREY);
    assert!((out.r - 0.75).abs() < 1e-6);
    // Overlay(0.5, 0.5) = Multiply at the boundary = 0.5.
    let out = blend(BlendMode::Overlay, GREY, GREY);
    assert!((out.r - 0.5).abs() < 1e-6);
}

#[test]
fn normal_blend_matches_source_over() {
    let a = blend(BlendMode::Normal, BLUE_HALF, GREY);
    let b = blend_over(BLUE_HALF, GREY);
    assert_px(a, b);
}

#[test]
fn extra_blend_modes_match_the_w3c_formulas() {
    use tpt_colour_core::composite::BlendMode;
    // Colour dodge saturates at 1 when the source is 1, and colour burn at 0 when it is 0.
    assert!((BlendMode::ColorDodge.blend_channel(0.5, 0.5) - 1.0).abs() < 1e-6);
    assert!(BlendMode::ColorDodge.blend_channel(0.0, 0.7).abs() < 1e-6);
    assert!(BlendMode::ColorBurn.blend_channel(0.5, 0.5).abs() < 1e-6);
    assert!((BlendMode::ColorBurn.blend_channel(1.0, 0.2) - 1.0).abs() < 1e-6);
    // Soft light with a mid-grey source leaves the backdrop unchanged.
    assert!((BlendMode::SoftLight.blend_channel(0.3, 0.5) - 0.3).abs() < 1e-6);
    // Soft light, source above one half, against a dark backdrop: D(0.2) = 0.448 by the spec.
    let expected = 0.2 + (2.0 * 0.8 - 1.0) * (0.448 - 0.2);
    assert!((BlendMode::SoftLight.blend_channel(0.2, 0.8) - expected).abs() < 1e-4);
}

#[test]
fn premultiplied_constructor_enforces_its_invariant() {
    use tpt_colour_core::composite::PremultipliedRgba;
    // Colour above alpha is clipped to alpha, and out-of-range alpha is clamped first.
    let p = PremultipliedRgba::new(0.9, 0.2, f32::NAN, 0.5);
    assert_eq!((p.r, p.g, p.b, p.a), (0.5, 0.2, 0.0, 0.5));
    let q = PremultipliedRgba::new(2.0, -1.0, 0.3, 3.0);
    assert_eq!((q.r, q.g, q.b, q.a), (1.0, 0.0, 0.3, 1.0));
}

#[test]
fn non_separable_modes_follow_the_w3c_examples() {
    use tpt_colour_core::composite::BlendMode;
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
    let red = [1.0, 0.0, 0.0];
    let blue = [0.0, 0.0, 1.0];
    let grey = [0.5, 0.5, 0.5];

    // Luminosity of a source on a dark grey backdrop: lum(red) = 0.3, so the grey is 0.3.
    assert!(near(
        BlendMode::Luminosity.blend_rgb([0.2; 3], red),
        [0.3; 3]
    ));
    // Colour takes the source's hue and saturation, here blue lifted to lum(grey) = 0.5.
    let colour = BlendMode::Color.blend_rgb(grey, blue);
    assert!(
        colour[2] > colour[0] && (lum3(colour) - 0.5).abs() < 1e-5,
        "{colour:?}"
    );
    // A grey backdrop has no saturation or hue, so Hue and Saturation return grey.
    assert!(near(BlendMode::Hue.blend_rgb(grey, red), [0.5; 3]));
    assert!(near(BlendMode::Saturation.blend_rgb(grey, red), grey));
    // Hue of blue on a red backdrop: blue at luminosity 0.3, clipped back into gamut.
    let v = 0.3 - 0.11 * 0.7 / 0.89;
    assert!(near(BlendMode::Hue.blend_rgb(red, blue), [v, v, 1.0]));
    // Non-separable modes are flagged, and separable ones are not.
    assert!(!BlendMode::Hue.is_separable());
    assert!(BlendMode::Multiply.is_separable());
}

fn lum3(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}
