//! HDR helpers for tpt-kinetix: the PQ transfer function (SMPTE ST 2084) and luminance tone mapping.
//!
//! Luminance is relative: `1.0` is the peak of the display or reference the caller chooses.
//! PQ is absolute, with `1.0` meaning 10 000 cd/m².

use libm::powf;

const PQ_M1: f32 = 0.159_301_76;
const PQ_M2: f32 = 78.843_75;
const PQ_C1: f32 = 0.835_937_5;
const PQ_C2: f32 = 18.851_562;
const PQ_C3: f32 = 18.6875;

/// PQ encoded signal in `[0, 1]` to linear luminance in `[0, 1]` (1.0 = 10 000 cd/m²).
#[must_use]
pub fn pq_eotf(signal: f32) -> f32 {
    let e = signal.clamp(0.0, 1.0);
    let np = powf(e, 1.0 / PQ_M2);
    let num = (np - PQ_C1).max(0.0);
    let den = PQ_C2 - PQ_C3 * np;
    powf(num / den, 1.0 / PQ_M1)
}

/// Linear luminance in `[0, 1]` (1.0 = 10 000 cd/m²) to PQ encoded signal in `[0, 1]`.
#[must_use]
pub fn pq_oetf(luminance: f32) -> f32 {
    let y = luminance.clamp(0.0, 1.0);
    let yp = powf(y, PQ_M1);
    powf((PQ_C1 + PQ_C2 * yp) / (1.0 + PQ_C3 * yp), PQ_M2)
}

/// Extended Reinhard operator on luminance: `L·(1 + L/W²) / (1 + L)`.
///
/// Values at `white` map to 1. Values above `white` are clipped by the caller.
#[must_use]
pub fn reinhard_extended(luminance: f32, white: f32) -> f32 {
    let l = luminance.max(0.0);
    let w2 = (white * white).max(f32::MIN_POSITIVE);
    l * (1.0 + l / w2) / (1.0 + l)
}

/// Tone maps a colour by its luminance (XYZ `Y`), keeping chromaticity.
///
/// The output is clipped to `[0, 1]`. Colours at or above `white` clip to 1.
#[must_use]
pub fn tone_map_xyz(xyz: [f32; 3], white: f32) -> [f32; 3] {
    let y = xyz[1];
    if y <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let scale = reinhard_extended(y, white).min(1.0) / y;
    xyz.map(|v| (v * scale).clamp(0.0, 1.0))
}
