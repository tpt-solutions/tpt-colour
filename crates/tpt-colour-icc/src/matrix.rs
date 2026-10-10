//! 3×3 matrix helpers with a fixed operation order.

use crate::error::IccError;

/// A row-major 3×3 matrix.
pub(crate) type Mat3 = [[f32; 3]; 3];

/// Matrix–vector product `m × v`.
#[must_use]
pub(crate) fn mul_vec(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Inverse via the adjugate.
///
/// # Errors
///
/// Returns [`IccError::Singular`] if the determinant is effectively zero.
#[allow(clippy::many_single_char_names)] // Cofactor expansion reads most naturally with a..i.
pub(crate) fn inverse(m: &Mat3) -> Result<Mat3, IccError> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-9 {
        return Err(IccError::Singular);
    }
    let k = 1.0 / det;
    Ok([
        [
            (e * i - f * h) * k,
            (c * h - b * i) * k,
            (b * f - c * e) * k,
        ],
        [
            (f * g - d * i) * k,
            (a * i - c * g) * k,
            (c * d - a * f) * k,
        ],
        [
            (d * h - e * g) * k,
            (b * g - a * h) * k,
            (a * e - b * d) * k,
        ],
    ])
}
