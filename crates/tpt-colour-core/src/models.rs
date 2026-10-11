//! Cylindrical and chromaticity models of sRGB and XYZ: HSL, HSV, HWB and xyY.
//!
//! The RGB side is gamma-encoded sRGB in `[0, 1]`, and hue is in degrees in `[0, 360)`.

#![allow(clippy::many_single_char_names)] // The model formulas use r, g, b, h, s, l, v and so on.

/// `x` modulo `m`, in `[0, m)`, for `m > 0`.
fn wrap(x: f32, m: f32) -> f32 {
    x - m * libm::floorf(x / m)
}

/// Hue in degrees from the max and the channel differences, given `d = max - min > 0`.
fn hue(rgb: [f32; 3], d: f32) -> f32 {
    let [r, g, b] = rgb;
    let sector = if r >= g && r >= b {
        wrap((g - b) / d, 6.0)
    } else if g >= b {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    sector * 60.0
}

/// The 60° sector of a hue, and the second-largest channel as a fraction of chroma.
///
/// The fraction is `1 − |h′ mod 2 − 1|`, which rises through even sectors and falls through odd
/// ones, as the standard HSL and HSV formulas require.
fn sector(h: f32) -> (u32, f32) {
    let h6 = wrap(h, 360.0) / 60.0;
    let i = libm::floorf(h6);
    let x = 1.0 - libm::fabsf(wrap(h6, 2.0) - 1.0);
    (i as u32 % 6, x)
}

/// Chroma-based RGB from `(hue, chroma, m)`, shared by HSL and HSV.
fn from_chroma(h: f32, c: f32, m: f32) -> [f32; 3] {
    let (i, f) = sector(h);
    let x = c * f;
    let (r, g, b) = match i {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

/// sRGB → `[hue, saturation, lightness]`.
///
/// # Example
///
/// ```
/// use tpt_colour_core::models::srgb_to_hsl;
/// let [h, s, l] = srgb_to_hsl([1.0, 0.0, 0.0]);
/// assert!(h.abs() < 1e-4 && (s - 1.0).abs() < 1e-6 && (l - 0.5).abs() < 1e-6);
/// ```
#[must_use]
pub fn srgb_to_hsl(rgb: [f32; 3]) -> [f32; 3] {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    let l = f32::midpoint(max, min);
    let d = max - min;
    if d <= 0.0 {
        return [0.0, 0.0, l];
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    [hue(rgb, d), s, l]
}

/// `[hue, saturation, lightness]` → sRGB.
#[must_use]
pub fn hsl_to_srgb([h, s, l]: [f32; 3]) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    from_chroma(h, c, l - c / 2.0)
}

/// sRGB → `[hue, saturation, value]`.
#[must_use]
pub fn srgb_to_hsv(rgb: [f32; 3]) -> [f32; 3] {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    let d = max - min;
    if d <= 0.0 {
        return [0.0, 0.0, max];
    }
    [hue(rgb, d), d / max, max]
}

/// `[hue, saturation, value]` → sRGB.
#[must_use]
pub fn hsv_to_srgb([h, s, v]: [f32; 3]) -> [f32; 3] {
    let c = v * s;
    from_chroma(h, c, v - c)
}

/// sRGB → `[hue, whiteness, blackness]`, the CSS HWB model.
#[must_use]
pub fn srgb_to_hwb(rgb: [f32; 3]) -> [f32; 3] {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    let hsv = srgb_to_hsv(rgb);
    [hsv[0], min, 1.0 - max]
}

/// `[hue, whiteness, blackness]` → sRGB. When whiteness and blackness sum to one or more, the
/// result is grey at `whiteness / (whiteness + blackness)`, as CSS specifies.
#[must_use]
pub fn hwb_to_srgb([h, w, b]: [f32; 3]) -> [f32; 3] {
    if w + b >= 1.0 {
        let grey = w / (w + b);
        return [grey, grey, grey];
    }
    let v = 1.0 - b;
    let s = 1.0 - w / v;
    hsv_to_srgb([h, s, v])
}

/// XYZ → CIE xyY. Black, with no defined chromaticity, maps to `x = y = 1/3`.
#[must_use]
pub fn xyz_to_xyy(xyz: [f32; 3]) -> [f32; 3] {
    let sum = xyz[0] + xyz[1] + xyz[2];
    if sum <= 0.0 {
        return [1.0 / 3.0, 1.0 / 3.0, 0.0];
    }
    [xyz[0] / sum, xyz[1] / sum, xyz[1]]
}

/// CIE xyY → XYZ. A zero `y` gives black.
#[must_use]
pub fn xyy_to_xyz([x, y, big_y]: [f32; 3]) -> [f32; 3] {
    if y <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    [x * big_y / y, big_y, (1.0 - x - y) * big_y / y]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    #[test]
    fn primaries_have_known_hues() {
        assert!(near(srgb_to_hsl([1.0, 0.0, 0.0]), [0.0, 1.0, 0.5]));
        assert!(near(srgb_to_hsl([0.0, 1.0, 0.0]), [120.0, 1.0, 0.5]));
        assert!(near(srgb_to_hsv([0.0, 0.0, 1.0]), [240.0, 1.0, 1.0]));
        // Magenta-ish hue wraps past 360 to a positive angle.
        assert!((srgb_to_hsl([1.0, 0.0, 0.5])[0] - 330.0).abs() < 1e-4);
    }

    #[test]
    fn round_trips() {
        for rgb in [
            [0.2, 0.7, 0.9],
            [0.5, 0.5, 0.5],
            [0.9, 0.1, 0.3],
            [0.0, 0.0, 0.0],
        ] {
            assert!(near(hsl_to_srgb(srgb_to_hsl(rgb)), rgb), "hsl {rgb:?}");
            assert!(near(hsv_to_srgb(srgb_to_hsv(rgb)), rgb), "hsv {rgb:?}");
            assert!(near(hwb_to_srgb(srgb_to_hwb(rgb)), rgb), "hwb {rgb:?}");
        }
    }

    #[test]
    fn hwb_grey_when_whiteness_and_blackness_fill_the_range() {
        assert!(near(hwb_to_srgb([0.0, 0.6, 0.6]), [0.5, 0.5, 0.5]));
    }

    #[test]
    fn xyy_round_trips_and_black_is_defined() {
        let xyz = [0.4, 0.3, 0.2];
        assert!(near(xyy_to_xyz(xyz_to_xyy(xyz)), xyz));
        assert!(xyz_to_xyy([0.0; 3])[2].abs() < f32::EPSILON);
    }
}
