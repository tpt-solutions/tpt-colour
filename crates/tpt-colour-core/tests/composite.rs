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
