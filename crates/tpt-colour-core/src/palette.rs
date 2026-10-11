//! Hue-based colour harmonies in Oklch.
//!
//! Each function rotates the hue of a gamma-encoded sRGB colour in Oklch. Lightness is kept.
//! Chroma is lowered only where the rotated colour falls outside sRGB, so the result is always
//! displayable. The fit is a simple bisection on chroma, not a full gamut mapping.

use crate::space::{ColourSpace, Oklch, SRgb};

/// Rotates the Oklch hue of `rgb` by `degrees`, returning gamma-encoded sRGB in `[0, 1]`.
///
/// # Example
///
/// ```
/// use tpt_colour_core::palette::hue_rotate;
/// let cyan = hue_rotate([1.0, 0.0, 0.0], 180.0);
/// assert!(cyan[0] < 0.5 && cyan[1] > 0.5 && cyan[2] > 0.5);
/// ```
#[must_use]
pub fn hue_rotate(rgb: [f32; 3], degrees: f32) -> [f32; 3] {
    let [l, c, h] = Oklch.from_xyz(SRgb.to_xyz(rgb));
    fit_to_srgb([l, c, h + degrees])
}

/// The complement: the hue opposite `rgb`, 180° away.
#[must_use]
pub fn complement(rgb: [f32; 3]) -> [f32; 3] {
    hue_rotate(rgb, 180.0)
}

/// The two colours 30° either side of `rgb`.
#[must_use]
pub fn analogous(rgb: [f32; 3]) -> [[f32; 3]; 2] {
    [hue_rotate(rgb, -30.0), hue_rotate(rgb, 30.0)]
}

/// The two other corners of the triad, 120° and 240° from `rgb`.
#[must_use]
pub fn triadic(rgb: [f32; 3]) -> [[f32; 3]; 2] {
    [hue_rotate(rgb, 120.0), hue_rotate(rgb, 240.0)]
}

/// The two colours either side of the complement, 150° and 210° from `rgb`.
#[must_use]
pub fn split_complementary(rgb: [f32; 3]) -> [[f32; 3]; 2] {
    [hue_rotate(rgb, 150.0), hue_rotate(rgb, 210.0)]
}

/// Returns the Oklch colour as gamma-encoded sRGB in `[0, 1]`, reducing chroma by bisection if
/// it lies outside the sRGB gamut. Lightness and hue are kept.
fn fit_to_srgb([l, c, h]: [f32; 3]) -> [f32; 3] {
    const TOLERANCE: f32 = 1e-5;
    let in_gamut = |rgb: [f32; 3]| {
        rgb.iter()
            .all(|v| (-TOLERANCE..=1.0 + TOLERANCE).contains(v))
    };
    let encode = |chroma: f32| SRgb.from_xyz(Oklch.to_xyz([l, chroma, h]));

    let full = encode(c);
    if in_gamut(full) {
        return full.map(|v| v.clamp(0.0, 1.0));
    }
    let (mut lo, mut hi) = (0.0_f32, c);
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        if in_gamut(encode(mid)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    encode(lo).map(|v| v.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::{analogous, complement, hue_rotate, triadic};
    use crate::space::{ColourSpace, Oklch, SRgb};

    fn hue(rgb: [f32; 3]) -> f32 {
        Oklch.from_xyz(SRgb.to_xyz(rgb))[2]
    }

    fn hue_gap(a: f32, b: f32) -> f32 {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    }

    #[test]
    fn outputs_are_displayable() {
        for rgb in [
            [1.0, 0.0, 0.0],
            [0.2, 0.9, 0.1],
            [0.5, 0.5, 0.5],
            [0.0, 0.0, 1.0],
        ] {
            for angle in [30.0, 120.0, 180.0, 240.0] {
                let out = hue_rotate(rgb, angle);
                assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "{out:?}");
            }
        }
    }

    #[test]
    fn complement_moves_the_hue_by_half_a_turn() {
        // Pure red sits at a hue where the sRGB gamut is narrow, so its chroma is reduced.
        // The hue survives that reduction, so this checks the rotation itself.
        let red = [1.0, 0.0, 0.0];
        let gap = hue_gap(hue(complement(red)), hue(red) + 180.0);
        assert!(gap < 1.5, "gap {gap}");
    }

    #[test]
    fn greys_stay_grey() {
        let grey = [0.4, 0.4, 0.4];
        let out = complement(grey);
        assert!(
            (out[0] - out[1]).abs() < 2e-3 && (out[1] - out[2]).abs() < 2e-3,
            "{out:?}"
        );
    }

    #[test]
    fn harmony_helpers_return_the_expected_offsets() {
        let blue = [0.1, 0.2, 0.9];
        let base = hue(blue);
        for (got, offset) in triadic(blue).iter().zip([120.0, 240.0]) {
            assert!(hue_gap(hue(*got), base + offset) < 1.5);
        }
        for (got, offset) in analogous(blue).iter().zip([-30.0, 30.0]) {
            assert!(hue_gap(hue(*got), base + offset) < 1.5);
        }
    }
}
