//! `ACEScg`: the AP1 primaries with linear encoding (ACES Technical Bulletin TB-2014-004).
//!
//! The ACES white point is D60. The matrices here include a Bradford adaptation to the D65
//! reference white that the rest of this crate uses, so `AcesCg` white maps to D65 XYZ.

use super::ColourSpace;
use crate::math::mat3_mul;

/// Linear `ACEScg` to D65-relative XYZ, with the D60 to D65 Bradford adaptation folded in.
const TO_XYZ: [[f32; 3]; 3] = [
    [0.652_237_53, 0.128_236_14, 0.169_982_25],
    [0.267_672_18, 0.674_34, 0.057_987_83],
    [-0.005_381_82, 0.001_369_06, 1.093_070_5],
];

/// The inverse of [`TO_XYZ`].
const FROM_XYZ: [[f32; 3]; 3] = [
    [1.660_585_3, -0.315_295_55, -0.241_509_33],
    [-0.659_926_06, 1.608_391_5, 0.017_298_594],
    [0.009_002_569, -0.003_566_876, 0.913_643_3],
];

/// `ACEScg`, linear AP1 primaries.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AcesCg;

impl ColourSpace<3> for AcesCg {
    fn to_xyz(&self, rgb: [f32; 3]) -> [f32; 3] {
        mat3_mul(&TO_XYZ, rgb)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        mat3_mul(&FROM_XYZ, xyz)
    }
}
