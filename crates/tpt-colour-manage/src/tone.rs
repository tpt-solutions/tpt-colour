//! HDR helpers for tpt-kinetix: the PQ transfer function (SMPTE ST 2084) and luminance tone mapping.
//!
//! Luminance is relative: `1.0` is the peak of the display or reference the caller chooses.
//! PQ is absolute, with `1.0` meaning 10 000 cd/m².

use libm::{expf, logf, powf, sqrtf};

const HLG_A: f32 = 0.178_832_77;
const HLG_B: f32 = 0.284_668_9;
const HLG_C: f32 = 0.559_910_7;

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
///
/// This is the inverse EOTF of ST 2084: the function that the standard's encoder applies to
/// light. It applies no OOTF, so it is not the "OETF" of BT.2100 for scene light. The name is
/// kept for compatibility, and [`pq_eotf`] is its exact inverse.
#[must_use]
pub fn pq_oetf(luminance: f32) -> f32 {
    let y = luminance.clamp(0.0, 1.0);
    let yp = powf(y, PQ_M1);
    powf((PQ_C1 + PQ_C2 * yp) / (1.0 + PQ_C3 * yp), PQ_M2)
}

/// HLG OETF (ITU-R BT.2100): scene linear light in `[0, 1]` to the HLG signal in `[0, 1]`.
///
/// This is the OETF alone, with no system gamma (OOTF). Values below zero clamp to zero.
#[must_use]
pub fn hlg_oetf(scene: f32) -> f32 {
    let e = scene.clamp(0.0, 1.0);
    if e <= 1.0 / 12.0 {
        sqrtf(3.0 * e)
    } else {
        HLG_A * logf(12.0 * e - HLG_B) + HLG_C
    }
}

/// HLG inverse OETF: HLG signal in `[0, 1]` back to scene linear light in `[0, 1]`.
#[must_use]
pub fn hlg_eotf(signal: f32) -> f32 {
    let e = signal.clamp(0.0, 1.0);
    if e <= 0.5 {
        e * e / 3.0
    } else {
        (expf((e - HLG_C) / HLG_A) + HLG_B) / 12.0
    }
}

/// Extended Reinhard operator on luminance: `L·(1 + L/W²) / (1 + L)`.
///
/// Values at `white` map to 1. Values above `white` are clipped by the caller. A `white` that is
/// not a positive finite number leaves the plain Reinhard curve `L / (1 + L)`. NaN luminance
/// maps to 0.
#[must_use]
pub fn reinhard_extended(luminance: f32, white: f32) -> f32 {
    let l = if luminance.is_nan() {
        0.0
    } else {
        luminance.max(0.0)
    };
    if l.is_infinite() {
        return f32::INFINITY;
    }
    if !white.is_finite() || white <= 0.0 {
        return l / (1.0 + l);
    }
    let w2 = white * white;
    l * (1.0 + l / w2) / (1.0 + l)
}

/// Tone maps a colour by its luminance (XYZ `Y`), keeping chromaticity.
///
/// Only `Y` is limited to `[0, 1]`. `X` and `Z` are scaled by the same factor as `Y`, so the
/// chromaticity is kept, even where `Y` is clipped. Clamping `X` and `Z` directly would shift the
/// chromaticity: D65 white has `Z ≈ 1.089`, for example. Colours at or above `white` reach `Y = 1`.
#[must_use]
pub fn tone_map_xyz(xyz: [f32; 3], white: f32) -> [f32; 3] {
    let y = xyz[1];
    // Non-finite input has no defined chromaticity, so it maps to black. This also catches NaN.
    if !xyz.iter().all(|v| v.is_finite()) || y <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let y_out = reinhard_extended(y, white).min(1.0);
    let scale = y_out / y;
    xyz.map(|v| v * scale)
}
