//! Colour difference formulas: CIE 1976, CIE 1994 (graphic arts), CIEDE2000 and Oklab.
//!
//! Lab inputs use the D65 reference white. The 2000 and Oklab formulas give about 1.0 for a
//! just-noticeable difference.

use libm::{atan2f, cosf, expf, sinf, sqrtf};

const RAD_PER_DEG: f32 = core::f32::consts::PI / 180.0;
const DEG_PER_RAD: f32 = 180.0 / core::f32::consts::PI;
/// 25^7, the constant in the chroma terms of CIEDE2000.
const POW25_7: f32 = 6_103_515_625.0;

fn sq(x: f32) -> f32 {
    x * x
}

fn pow7(x: f32) -> f32 {
    x * x * x * x * x * x * x
}

/// CIE 1976 ΔE*ab: Euclidean distance in Lab.
///
/// # Example
///
/// ```
/// use tpt_colour_core::difference::delta_e_76;
/// assert!((delta_e_76([50.0, 0.0, 0.0], [50.0, 3.0, 4.0]) - 5.0).abs() < 1e-5);
/// ```
#[must_use]
pub fn delta_e_76(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    let d = [lab1[0] - lab2[0], lab1[1] - lab2[1], lab1[2] - lab2[2]];
    sqrtf(d[0] * d[0] + d[1] * d[1] + d[2] * d[2])
}

/// CIE 1994 ΔE*94 with the graphic arts weights (`kL = 1`, `K1 = 0.045`, `K2 = 0.015`).
///
/// The first colour is the reference, and its chroma sets the weights.
#[must_use]
pub fn delta_e_94(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    const K1: f32 = 0.045;
    const K2: f32 = 0.015;
    let dl = lab1[0] - lab2[0];
    let c1 = sqrtf(lab1[1] * lab1[1] + lab1[2] * lab1[2]);
    let c2 = sqrtf(lab2[1] * lab2[1] + lab2[2] * lab2[2]);
    let dc = c1 - c2;
    let da = lab1[1] - lab2[1];
    let db = lab1[2] - lab2[2];
    // ΔH² from the Lab differences, minus the chroma part. Rounding can make it slightly negative.
    let dh2 = (da * da + db * db - dc * dc).max(0.0);
    let sc = 1.0 + K1 * c1;
    let sh = 1.0 + K2 * c1;
    sqrtf(dl * dl + (dc / sc) * (dc / sc) + dh2 / (sh * sh))
}

/// CIEDE2000 ΔE00 with unit weights (`kL = kC = kH = 1`), following Sharma, Wu and Dalal (2005).
///
/// # Example
///
/// ```
/// use tpt_colour_core::difference::delta_e_2000;
/// let de = delta_e_2000([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485]);
/// assert!((de - 2.0425).abs() < 1e-3);
/// ```
#[must_use]
pub fn delta_e_2000(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    let [l1, a1, b1] = lab1;
    let [l2, a2, b2] = lab2;
    let c1 = sqrtf(a1 * a1 + b1 * b1);
    let c2 = sqrtf(a2 * a2 + b2 * b2);
    let cbar = f32::midpoint(c1, c2);
    let c7 = pow7(cbar);
    let g = 0.5 * (1.0 - sqrtf(c7 / (c7 + POW25_7)));

    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = sqrtf(a1p * a1p + b1 * b1);
    let c2p = sqrtf(a2p * a2p + b2 * b2);
    let h1p = hue_degrees(b1, a1p, c1p);
    let h2p = hue_degrees(b2, a2p, c2p);

    let dlp = l2 - l1;
    let dcp = c2p - c1p;
    let dhp = if c1p * c2p == 0.0 {
        0.0
    } else {
        let d = h2p - h1p;
        if d > 180.0 {
            d - 360.0
        } else if d < -180.0 {
            d + 360.0
        } else {
            d
        }
    };
    let dhp_term = 2.0 * sqrtf(c1p * c2p) * sinf((dhp / 2.0) * RAD_PER_DEG);

    let lbarp = f32::midpoint(l1, l2);
    let cbarp = f32::midpoint(c1p, c2p);
    let hbarp = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        f32::midpoint(h1p, h2p)
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * cosf((hbarp - 30.0) * RAD_PER_DEG)
        + 0.24 * cosf((2.0 * hbarp) * RAD_PER_DEG)
        + 0.32 * cosf((3.0 * hbarp + 6.0) * RAD_PER_DEG)
        - 0.20 * cosf((4.0 * hbarp - 63.0) * RAD_PER_DEG);
    let d_theta = 30.0 * expf(-sq((hbarp - 275.0) / 25.0));
    let cbarp7 = pow7(cbarp);
    let rc = 2.0 * sqrtf(cbarp7 / (cbarp7 + POW25_7));
    let lbar50 = (lbarp - 50.0) * (lbarp - 50.0);
    let sl = 1.0 + 0.015 * lbar50 / sqrtf(20.0 + lbar50);
    let sc = 1.0 + 0.045 * cbarp;
    let sh = 1.0 + 0.015 * cbarp * t;
    let rt = -sinf((2.0 * d_theta) * RAD_PER_DEG) * rc;

    let tl = dlp / sl;
    let tc = dcp / sc;
    let th = dhp_term / sh;
    sqrtf((tl * tl + tc * tc + th * th + rt * tc * th).max(0.0))
}

/// Euclidean distance in Oklab. This is the ΔEOK that CSS Color 4 uses for gamut mapping.
#[must_use]
pub fn delta_e_ok(oklab1: [f32; 3], oklab2: [f32; 3]) -> f32 {
    let d = [
        oklab1[0] - oklab2[0],
        oklab1[1] - oklab2[1],
        oklab1[2] - oklab2[2],
    ];
    sqrtf(d[0] * d[0] + d[1] * d[1] + d[2] * d[2])
}

/// Hue angle in degrees in `[0, 360)`, or 0 for a colour with no chroma.
fn hue_degrees(b: f32, a: f32, chroma: f32) -> f32 {
    if chroma == 0.0 {
        return 0.0;
    }
    let h = atan2f(b, a) * DEG_PER_RAD;
    if h < 0.0 { h + 360.0 } else { h }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference pairs from Sharma, Wu and Dalal (2005), Table 1 (CIEDE2000), pairs 1-4.
    const PAIRS: [([f32; 3], [f32; 3], f32); 4] = [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
        ([50.0, 2.8361, -74.0200], [50.0, 0.0, -82.7485], 3.4412),
        ([50.0, -1.3802, -84.2814], [50.0, 0.0, -82.7485], 1.0000),
    ];

    #[test]
    fn ciede2000_matches_published_reference_values() {
        for (a, b, want) in PAIRS {
            let got = delta_e_2000(a, b);
            assert!(
                (got - want).abs() < 2e-3,
                "{a:?} vs {b:?}: {got} want {want}"
            );
        }
    }

    #[test]
    fn metrics_are_zero_on_identical_colours_and_symmetric() {
        let a = [60.0, 20.0, -30.0];
        let b = [55.0, 10.0, -10.0];
        for f in [delta_e_76, delta_e_94, delta_e_2000, delta_e_ok] {
            assert!(f(a, a).abs() < 1e-6);
        }
        assert!((delta_e_2000(a, b) - delta_e_2000(b, a)).abs() < 1e-4);
        assert!((delta_e_76(a, b) - 22.912_878).abs() < 1e-4);
    }

    #[test]
    fn delta_e_94_is_no_larger_than_76_for_chromatic_pairs() {
        let a = [60.0, 40.0, 30.0];
        let b = [60.0, 30.0, 40.0];
        assert!(delta_e_94(a, b) <= delta_e_76(a, b));
    }
}
