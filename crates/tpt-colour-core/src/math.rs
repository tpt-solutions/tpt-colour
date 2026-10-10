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
