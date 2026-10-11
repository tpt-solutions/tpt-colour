//! Colour interpolation for gradients, in Oklab and Oklch.
//!
//! The inputs and outputs are gamma-encoded sRGB in `[0, 1]`. Interpolation runs in the
//! perceptual space, so the midpoint of two colours keeps its lightness. Oklch hue takes the
//! shorter arc, as CSS Color 4 does by default.

use crate::space::{ColourSpace, Oklab, Oklch, SRgb};

/// Linear interpolation in Oklab at `t` (0 gives `a`, 1 gives `b`). `t` is not clamped.
///
/// # Example
///
/// ```
/// use tpt_colour_core::mix::mix_oklab;
/// let end = mix_oklab([0.2, 0.4, 0.6], [0.8, 0.2, 0.1], 1.0);
/// assert!((end[0] - 0.8).abs() < 1e-4);
/// ```
#[must_use]
pub fn mix_oklab(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let la = Oklab.from_xyz(SRgb.to_xyz(a));
    let lb = Oklab.from_xyz(SRgb.to_xyz(b));
    let m = [0, 1, 2].map(|i| la[i] + (lb[i] - la[i]) * t);
    SRgb.from_xyz(Oklab.to_xyz(m))
}

/// Interpolation in Oklch at `t`, with hue along the shorter arc.
///
/// An achromatic end takes the other end's hue, so a grey does not pull the gradient through
/// an arbitrary hue. The result may be outside the sRGB gamut, and is returned unclamped.
#[must_use]
pub fn mix_oklch(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let la = Oklch.from_xyz(SRgb.to_xyz(a));
    let lb = Oklch.from_xyz(SRgb.to_xyz(b));
    let (ha, hb) = (la[2], lb[2]);
    let hue_a = if la[1] <= 1e-4 { hb } else { ha };
    let hue_b = if lb[1] <= 1e-4 { ha } else { hb };
    // Signed difference wrapped into [-180, 180), so the path is the shorter arc.
    let delta = wrap(hue_b - hue_a + 180.0, 360.0) - 180.0;
    let m = [
        la[0] + (lb[0] - la[0]) * t,
        la[1] + (lb[1] - la[1]) * t,
        hue_a + delta * t,
    ];
    SRgb.from_xyz(Oklch.to_xyz(m))
}

/// `x` modulo `m`, in `[0, m)`.
fn wrap(x: f32, m: f32) -> f32 {
    x - m * libm::floorf(x / m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
    }

    #[test]
    fn endpoints_are_preserved() {
        let a = [0.9, 0.2, 0.1];
        let b = [0.1, 0.4, 0.8];
        assert!(near(mix_oklab(a, b, 0.0), a, 1e-4));
        assert!(near(mix_oklab(a, b, 1.0), b, 1e-4));
        assert!(near(mix_oklch(a, b, 0.0), a, 1e-4));
        assert!(near(mix_oklch(a, b, 1.0), b, 1e-4));
    }

    #[test]
    fn black_and_white_midpoint_is_perceptual_grey() {
        // Oklab lightness at the midpoint of black and white is 0.5, which is not sRGB 0.5.
        let mid = mix_oklab([0.0; 3], [1.0; 3], 0.5);
        assert!(mid[0] > 0.3 && mid[0] < 0.5, "{mid:?}");
        assert!((mid[0] - mid[1]).abs() < 1e-4 && (mid[1] - mid[2]).abs() < 1e-4);
    }

    #[test]
    fn grey_takes_the_hue_of_the_other_end() {
        let grey = [0.5, 0.5, 0.5];
        let red = [1.0, 0.0, 0.0];
        // A quarter of the way from grey to red should still be warm, not tinted green or blue.
        let q = mix_oklch(grey, red, 0.5);
        assert!(q[0] > q[2], "{q:?}");
    }
}

/// Interpolation of straight RGBA colours with premultiplied alpha, in Oklab.
///
/// Each colour is premultiplied by its alpha before the Oklab components are mixed, then divided
/// by the mixed alpha. A transparent end therefore contributes no colour, so a gradient from a
/// transparent red to an opaque blue stays blue at the midpoint instead of turning purple. The
/// result is straight RGBA, with colour in gamma-encoded sRGB.
#[must_use]
pub fn mix_premultiplied_oklab(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let premultiplied = |c: [f32; 4]| {
        let lab = Oklab.from_xyz(SRgb.to_xyz([c[0], c[1], c[2]]));
        ([0, 1, 2].map(|i| lab[i] * c[3]), c[3])
    };
    let (la, aa) = premultiplied(a);
    let (lb, ab) = premultiplied(b);
    let alpha = aa + (ab - aa) * t;
    if alpha <= 0.0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let lab = [0, 1, 2].map(|i| (la[i] + (lb[i] - la[i]) * t) / alpha);
    let rgb = SRgb.from_xyz(Oklab.to_xyz(lab));
    [rgb[0], rgb[1], rgb[2], alpha]
}

#[cfg(test)]
mod premultiplied_tests {
    use super::*;

    #[test]
    fn transparent_end_adds_no_colour() {
        let transparent_red = [1.0, 0.0, 0.0, 0.0];
        let opaque_blue = [0.0, 0.0, 1.0, 1.0];
        let mid = mix_premultiplied_oklab(transparent_red, opaque_blue, 0.5);
        assert!((mid[3] - 0.5).abs() < 1e-6);
        // Blue at half alpha, with no red from the transparent end.
        assert!(mid[2] > mid[0] + 0.3, "{mid:?}");
        // Plain mixing gives a visibly different colour at the same point.
        let plain = mix_oklab([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.5);
        assert!(plain[0] > mid[0] + 0.1, "{plain:?} vs {mid:?}");
    }

    #[test]
    fn opaque_ends_match_plain_mixing() {
        let a = [0.9, 0.2, 0.1, 1.0];
        let b = [0.1, 0.4, 0.8, 1.0];
        let m = mix_premultiplied_oklab(a, b, 0.3);
        let p = mix_oklab([a[0], a[1], a[2]], [b[0], b[1], b[2]], 0.3);
        for c in 0..3 {
            assert!((m[c] - p[c]).abs() < 1e-4, "{m:?} vs {p:?}");
        }
        assert!((m[3] - 1.0).abs() < 1e-6);
    }
}
