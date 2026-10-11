//! WCAG 2 contrast ratio between two sRGB colours.

use crate::math::powf;
use crate::space::{ColourSpace, SRgb};

/// Relative luminance (WCAG 2.x, the `Y` of the linear sRGB colour) of a gamma-encoded colour.
#[must_use]
pub fn relative_luminance(rgb: [f32; 3]) -> f32 {
    SRgb.to_xyz(rgb)[1]
}

/// WCAG 2 contrast ratio, from 1 (identical) to 21 (black on white).
///
/// The result does not depend on argument order. WCAG AA asks for 4.5 for body text and 3 for
/// large text. AAA asks for 7 and 4.5.
///
/// # Example
///
/// ```
/// use tpt_colour_core::contrast::wcag_contrast;
/// let ratio = wcag_contrast([0.0; 3], [1.0; 3]);
/// assert!((ratio - 21.0).abs() < 0.01);
/// ```
#[must_use]
pub fn wcag_contrast(a: [f32; 3], b: [f32; 3]) -> f32 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// APCA perceptual luminance `Y` with the soft black clamp applied (APCA 0.0.98G-4g).
fn apca_luminance(rgb: [f32; 3]) -> f32 {
    let [r, g, b] = rgb.map(|c| powf(c.clamp(0.0, 1.0), 2.4));
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
    if y < 0.022 {
        y + powf(0.022 - y, 1.414)
    } else {
        y
    }
}

/// APCA contrast, the signed lightness contrast `Lc` (APCA W3 0.0.98G-4g).
///
/// `text` and `background` are gamma-encoded sRGB in `[0, 1]`. The sign gives the polarity:
/// positive for dark text on a light background, negative for light text on a dark one.
/// The magnitude is roughly 0 to 108. A magnitude below 10 returns 0.
///
/// APCA is the candidate contrast method for WCAG 3. It is not a WCAG 2 pass or fail test, so
/// use [`wcag_contrast`] for the WCAG 2 AA and AAA thresholds.
///
/// # Example
///
/// ```
/// use tpt_colour_core::contrast::apca_contrast;
/// let lc = apca_contrast([0.0; 3], [1.0; 3]);
/// assert!((lc - 106.0).abs() < 0.5);
/// ```
#[must_use]
pub fn apca_contrast(text: [f32; 3], background: [f32; 3]) -> f32 {
    let y_text = apca_luminance(text);
    let y_bg = apca_luminance(background);
    if (y_bg - y_text).abs() < 0.000_5 {
        return 0.0;
    }
    let lc = if y_bg > y_text {
        let sapc = (powf(y_bg, 0.56) - powf(y_text, 0.57)) * 1.14;
        if sapc < 0.1 {
            0.0
        } else {
            sapc - 0.027
        }
    } else {
        let sapc = (powf(y_bg, 0.65) - powf(y_text, 0.62)) * 1.14;
        if sapc > -0.1 {
            0.0
        } else {
            sapc + 0.027
        }
    };
    lc * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apca_matches_reference_values() {
        // Published APCA 0.0.98G-4g values for black and white.
        assert!((apca_contrast([0.0; 3], [1.0; 3]) - 106.04).abs() < 0.1);
        assert!((apca_contrast([1.0; 3], [0.0; 3]) + 107.88).abs() < 0.1);
    }

    #[test]
    fn apca_polarity_and_threshold() {
        let dark = [0.1; 3];
        let light = [0.9; 3];
        assert!(apca_contrast(dark, light) > 0.0);
        assert!(apca_contrast(light, dark) < 0.0);
        assert_eq!(apca_contrast(light, light), 0.0);
        // Nearly identical backgrounds are below the visibility floor.
        assert_eq!(apca_contrast([0.5; 3], [0.5, 0.5, 0.501]), 0.0);
    }

    #[test]
    fn black_and_white_give_the_maximum_ratio() {
        let r = wcag_contrast([0.0; 3], [1.0; 3]);
        assert!((r - 21.0).abs() < 1e-3, "{r}");
        assert!((wcag_contrast([0.4, 0.2, 0.9], [0.4, 0.2, 0.9]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn order_does_not_matter() {
        let a = [0.9, 0.1, 0.2];
        let b = [0.1, 0.1, 0.6];
        assert!((wcag_contrast(a, b) - wcag_contrast(b, a)).abs() < 1e-6);
    }
}
