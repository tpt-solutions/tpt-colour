//! Scalar helpers used by the ICC engine. Transcendentals go through `libm`.

/// Power function via `libm`.
#[inline]
#[must_use]
pub(crate) fn powf(x: f32, y: f32) -> f32 {
    libm::powf(x, y)
}

/// Clamps to the normalised `[0, 1]` domain used by ICC curves and device values. NaN maps to 0,
/// because `f32::clamp` would pass it through.
#[inline]
#[must_use]
pub(crate) fn valid_domain(x: f32) -> f32 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}
