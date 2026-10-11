//! Runs under Node or a browser: `wasm-pack test --node crates/tpt-colour-wasm`.

use tpt_colour_wasm::{Manager, apca_contrast, harmony, parse_colour, wcag_contrast};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn parses_css_colours() {
    assert_eq!(
        parse_colour("rebeccapurple").unwrap(),
        vec![102, 51, 153, 255]
    );
    assert!(parse_colour("nope").is_err());
}

#[wasm_bindgen_test]
fn contrast_matches_reference_values() {
    let white = [1.0, 1.0, 1.0];
    let black = [0.0, 0.0, 0.0];
    assert!((wcag_contrast(&black, &white).unwrap() - 21.0).abs() < 0.01);
    assert!((apca_contrast(&black, &white).unwrap() - 106.0).abs() < 0.5);
    assert!(wcag_contrast(&[0.5, 0.5], &white).is_err());
}

#[wasm_bindgen_test]
fn harmonies_have_the_right_lengths() {
    assert_eq!(harmony(&[1.0, 0.0, 0.0], "complement").unwrap().len(), 3);
    assert_eq!(harmony(&[1.0, 0.0, 0.0], "triadic").unwrap().len(), 6);
    assert!(harmony(&[1.0, 0.0, 0.0], "square").is_err());
}

#[wasm_bindgen_test]
fn manager_converts_between_builtin_profiles() {
    let mut m = Manager::new();
    let p3 = m
        .transform_rgb("srgb", "display-p3", "perceptual", &[0.8, 0.2, 0.1])
        .unwrap();
    let back = m
        .transform_rgb("display-p3", "srgb", "perceptual", &p3)
        .unwrap();
    assert!((back[0] - 0.8).abs() < 2e-3);
    assert!(m.transform_rgb("srgb", "srgb", "fancy", &[0.0; 3]).is_err());
    assert!(m.cached_transforms() >= 1);
}
