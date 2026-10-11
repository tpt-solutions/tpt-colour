//! Bradford chromatic adaptation between the D50 and D65 white points.

use crate::math::{mat3_inverse, mat3_matmul, mat3_mul};

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

/// A von Kries chromatic adaptation method, named by its cone-response matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChromaticAdaptation {
    /// Bradford, as ICC profiles use for their D50 PCS.
    Bradford,
    /// CIECAM02 (CAT02).
    Cat02,
    /// CAM16 (CAT16).
    Cat16,
}

impl ChromaticAdaptation {
    /// The response matrix that maps XYZ to cone-like `RGB`.
    fn response(self) -> [[f32; 3]; 3] {
        match self {
            Self::Bradford => [
                [0.895_1, 0.266_4, -0.161_4],
                [-0.750_2, 1.713_5, 0.036_7],
                [0.038_9, -0.068_5, 1.029_6],
            ],
            Self::Cat02 => [
                [0.732_8, 0.429_6, -0.162_4],
                [-0.703_6, 1.697_5, 0.006_1],
                [0.003, 0.013_6, 0.983_4],
            ],
            Self::Cat16 => [
                [0.401_288, 0.650_173, -0.051_461],
                [-0.250_268, 1.204_414, 0.045_854],
                [-0.002_079, 0.048_952, 0.953_127],
            ],
        }
    }
}

/// The 3×3 matrix that adapts XYZ from white `src_white` to white `dst_white` (both XYZ with
/// `Y = 1`), by a von Kries scaling in the cone space of `method`.
///
/// Returns `None` if the response matrix is singular for the given whites, which happens only
/// for a white with a zero cone response.
///
/// # Example
///
/// ```
/// use tpt_colour_core::space::{ChromaticAdaptation, D50, D65, adaptation_matrix};
/// let to_d50 = adaptation_matrix(ChromaticAdaptation::Bradford, D65, D50).unwrap();
/// let white = [0.950_47, 1.0, 1.088_83];
/// let adapted: [f32; 3] = core::array::from_fn(|i| {
///     to_d50[i][0] * white[0] + to_d50[i][1] * white[1] + to_d50[i][2] * white[2]
/// });
/// assert!((adapted[0] - D50[0]).abs() < 1e-3);
/// ```
#[must_use]
pub fn adaptation_matrix(
    method: ChromaticAdaptation,
    src_white: [f32; 3],
    dst_white: [f32; 3],
) -> Option<[[f32; 3]; 3]> {
    let m = method.response();
    let src_cone = mat3_mul(&m, src_white);
    let dst_cone = mat3_mul(&m, dst_white);
    let ratio = [0, 1, 2].map(|i| dst_cone[i] / src_cone[i]);
    let scaled = core::array::from_fn(|i| core::array::from_fn(|j| ratio[i] * m[i][j]));
    let inverse = mat3_inverse(&m)?;
    Some(mat3_matmul(&inverse, &scaled))
}
