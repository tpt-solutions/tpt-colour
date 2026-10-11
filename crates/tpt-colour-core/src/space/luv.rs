//! CIE `Luv` (1976 L*u*v*) with the D65 reference white.

use super::{ColourSpace, D65};
use crate::math::cbrt;

/// CIE 1976 L*u*v*. Same lightness as [`super::Lab`], with chromaticity from the CIE 1976 `u'v'`
/// diagram. Use it for additive light and for the `u'v'` chromaticity, as in display work.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Luv;

const EPSILON: f32 = 216.0 / 24_389.0; // (6/29)³
const KAPPA: f32 = 24_389.0 / 27.0; // (29/3)³

/// CIE `u'` and `v'` for XYZ. Both are zero when the colour is black (`X + 15Y + 3Z = 0`).
fn uv_prime([x, y, z]: [f32; 3]) -> [f32; 2] {
    let denom = x + 15.0 * y + 3.0 * z;
    if denom <= 0.0 {
        return [0.0, 0.0];
    }
    [4.0 * x / denom, 9.0 * y / denom]
}

impl ColourSpace<3> for Luv {
    fn to_xyz(&self, [lightness, u_star, v_star]: [f32; 3]) -> [f32; 3] {
        if lightness <= 0.0 {
            return [0.0; 3];
        }
        let [un, vn] = uv_prime(D65);
        let y = if lightness > KAPPA * EPSILON {
            let f = (lightness + 16.0) / 116.0;
            D65[1] * f * f * f
        } else {
            D65[1] * lightness / KAPPA
        };
        let up = u_star / (13.0 * lightness) + un;
        let vp = v_star / (13.0 * lightness) + vn;
        if vp == 0.0 {
            return [0.0; 3];
        }
        let x = y * 9.0 * up / (4.0 * vp);
        let z = y * (12.0 - 3.0 * up - 20.0 * vp) / (4.0 * vp);
        [x, y, z]
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let yr = xyz[1] / D65[1];
        let l = if yr > EPSILON {
            116.0 * cbrt(yr) - 16.0
        } else {
            KAPPA * yr
        };
        if l == 0.0 {
            return [0.0; 3];
        }
        let [un, vn] = uv_prime(D65);
        let [up, vp] = uv_prime(xyz);
        [l, 13.0 * l * (up - un), 13.0 * l * (vp - vn)]
    }
}

#[cfg(test)]
mod tests {
    use super::Luv;
    use crate::space::{ColourSpace, D65, SRgb};

    #[test]
    fn white_has_full_lightness_and_no_chroma() {
        let lab = Luv.from_xyz(D65);
        assert!((lab[0] - 100.0).abs() < 1e-3, "{lab:?}");
        assert!(lab[1].abs() < 1e-3 && lab[2].abs() < 1e-3, "{lab:?}");
    }

    #[test]
    fn srgb_red_matches_reference_values() {
        // sRGB red in CIE 1976 L*u*v* (D65): L* ≈ 53.24, u* ≈ 175.0, v* ≈ 37.8.
        let luv = Luv.from_xyz(SRgb.to_xyz([1.0, 0.0, 0.0]));
        assert!((luv[0] - 53.24).abs() < 0.05, "{luv:?}");
        assert!((luv[1] - 175.0).abs() < 0.2, "{luv:?}");
        assert!((luv[2] - 37.8).abs() < 0.2, "{luv:?}");
    }

    #[test]
    fn round_trips_through_xyz() {
        for xyz in [[0.2, 0.3, 0.1], [0.9, 0.5, 0.7], [0.01, 0.005, 0.02]] {
            let back = Luv.to_xyz(Luv.from_xyz(xyz));
            for (a, b) in back.iter().zip(xyz) {
                assert!((a - b).abs() < 1e-4, "{xyz:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn black_maps_to_zero() {
        assert!(Luv.from_xyz([0.0; 3]).iter().all(|c| c.abs() < 1e-9));
        assert!(Luv.to_xyz([0.0; 3]).iter().all(|c| c.abs() < 1e-9));
    }
}
