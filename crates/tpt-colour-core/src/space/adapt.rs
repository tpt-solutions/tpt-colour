//! Bradford chromatic adaptation between the D50 and D65 white points.

use crate::math::mat3_mul;

// Bradford-cone adaptation matrices, D65 ⇄ D50, row-major.
const D65_TO_D50: [[f32; 3]; 3] = [
    [1.047_811_2, 0.022_886_6, -0.050_127_0],
    [0.029_542_4, 0.990_484_4, -0.017_049_1],
    [-0.009_234_5, 0.015_043_6, 0.752_131_6],
];
const D50_TO_D65: [[f32; 3]; 3] = [
    [0.955_576_6, -0.023_039_3, 0.063_163_6],
    [-0.028_289_5, 1.009_941_6, 0.021_007_7],
    [0.012_298_2, -0.020_483_0, 1.329_909_8],
];

/// Adapts XYZ relative to D65 to be relative to D50 (Bradford).
#[must_use]
pub fn d65_to_d50(xyz: [f32; 3]) -> [f32; 3] {
    mat3_mul(&D65_TO_D50, xyz)
}

/// Adapts XYZ relative to D50 to be relative to D65 (Bradford).
#[must_use]
pub fn d50_to_d65(xyz: [f32; 3]) -> [f32; 3] {
    mat3_mul(&D50_TO_D65, xyz)
}
