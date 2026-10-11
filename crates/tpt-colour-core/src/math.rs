//! Deterministic maths helpers.
//!
//! Matrix products use a fixed operation order, and transcendental functions
//! go through `libm` rather than platform `std`, so results are identical on
//! every target.

/// Multiplies a 3×3 matrix by a column vector with a fixed summation order.
#[inline]
#[must_use]
pub(crate) fn mat3_mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Power function via `libm`.
#[inline]
#[must_use]
pub(crate) fn powf(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}

/// Cube root via `libm`; defined for negative inputs.
#[inline]
#[must_use]
pub(crate) fn cbrt(x: f32) -> f32 {
    libm::cbrtf(x)
}

/// Square root via `libm`.
#[inline]
#[must_use]
pub(crate) fn sqrt(x: f32) -> f32 {
    libm::sqrtf(x)
}

/// Two-argument arctangent via `libm`, in radians.
#[inline]
#[must_use]
pub(crate) fn atan2(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}

/// Sine via `libm`.
#[inline]
#[must_use]
pub(crate) fn sin(x: f32) -> f32 {
    libm::sinf(x)
}

/// Cosine via `libm`.
#[inline]
#[must_use]
pub(crate) fn cos(x: f32) -> f32 {
    libm::cosf(x)
}

/// Product of two 3×3 matrices, `a · b`, with a fixed summation order.
#[inline]
#[must_use]
pub(crate) fn mat3_matmul(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0_f32; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

#[allow(clippy::many_single_char_names)] // cofactor notation
/// Inverse of a 3×3 matrix from its cofactors, or `None` if it is singular or not finite.
#[must_use]
pub(crate) fn mat3_inverse(m: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let c00 = e * i - f * h;
    let c01 = f * g - d * i;
    let c02 = d * h - e * g;
    let det = a * c00 + b * c01 + c * c02;
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let s = 1.0 / det;
    Some([
        [c00 * s, (c * h - b * i) * s, (b * f - c * e) * s],
        [c01 * s, (a * i - c * g) * s, (c * d - a * f) * s],
        [c02 * s, (b * g - a * h) * s, (a * e - b * d) * s],
    ])
}
