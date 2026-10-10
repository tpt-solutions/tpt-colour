//! Bit-exact determinism. These constants were recorded on x86-64 and must match on every
//! target (ARM, WASM, and so on). A mismatch means an operation order or a libm function changed,
//! or the target's floating-point behaviour differs. Comparing bits, not tolerances, is the point.

use tpt_colour_icc::RenderingIntent;
use tpt_colour_manage::ColourManager;

/// Inputs and the expected `f32` bit patterns of the outputs, for Display P3 → sRGB (perceptual).
const CASES: [([f32; 3], [u32; 3]); 4] = [
    ([1.0, 0.0, 0.0], [0x3F7F_FFFD, 0x3D2D_5A27, 0x3D36_B58C]),
    ([0.0, 1.0, 0.0], [0x0000_0000, 0x3F7C_5B57, 0x3E23_5E31]),
    ([0.5, 0.5, 0.5], [0x3EFF_FB2C, 0x3F00_005A, 0x3F00_03A4]),
    ([0.2, 0.7, 0.9], [0x0000_0000, 0x3F36_0337, 0x3F6A_20E2]),
];

#[test]
fn transforms_are_bit_identical_to_recorded_values() {
    let mut colour = ColourManager::new();
    let t = colour
        .transform("display-p3", "srgb", RenderingIntent::Perceptual)
        .unwrap();
    for (input, expected) in CASES {
        let out = t.transform_rgb(input).unwrap();
        let bits = out.map(f32::to_bits);
        println!("{input:?} → {out:?} bits {:08X?}", bits);
        assert_eq!(bits, expected, "input {input:?}: {out:?}");
    }
}
