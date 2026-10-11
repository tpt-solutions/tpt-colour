//! Colour-vision deficiency simulation for dichromats.
//!
//! The matrices are Machado, Oliveira & Fernandes (2009), severity 1.0 (full dichromacy),
//! applied to linear sRGB. They are a display approximation for design review. They are not a
//! clinical model, and they do not cover anomalous trichromacy (the milder forms).

use crate::space::{srgb_eotf, srgb_oetf};

/// A dichromatic colour-vision deficiency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Deficiency {
    /// Missing long-wavelength (L) cones.
    Protanopia,
    /// Missing medium-wavelength (M) cones.
    Deuteranopia,
    /// Missing short-wavelength (S) cones.
    Tritanopia,
}

impl Deficiency {
    /// Linear-sRGB simulation matrix, rows applied to `[r, g, b]`.
    const fn matrix(self) -> [[f32; 3]; 3] {
        match self {
            Self::Protanopia => [
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ],
            Self::Deuteranopia => [
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ],
            Self::Tritanopia => [
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ],
        }
    }
}

/// Simulates how a gamma-encoded sRGB colour in `[0, 1]` looks to a viewer with `deficiency`.
///
/// Neutral greys are unchanged. The result is clamped to `[0, 1]`, because the matrices can
/// leave the sRGB gamut.
///
/// # Example
///
/// ```
/// use tpt_colour_core::vision::{simulate, Deficiency};
/// let grey = [0.5, 0.5, 0.5];
/// let seen = simulate(grey, Deficiency::Protanopia);
/// assert!((seen[0] - 0.5).abs() < 1e-3);
/// ```
#[must_use]
pub fn simulate(rgb: [f32; 3], deficiency: Deficiency) -> [f32; 3] {
    let lin = rgb.map(srgb_eotf);
    let m = deficiency.matrix();
    let out = [
        m[0][0] * lin[0] + m[0][1] * lin[1] + m[0][2] * lin[2],
        m[1][0] * lin[0] + m[1][1] * lin[1] + m[1][2] * lin[2],
        m[2][0] * lin[0] + m[2][1] * lin[1] + m[2][2] * lin[2],
    ];
    out.map(|l| srgb_oetf(l.clamp(0.0, 1.0)).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::{Deficiency, simulate};

    const ALL: [Deficiency; 3] = [
        Deficiency::Protanopia,
        Deficiency::Deuteranopia,
        Deficiency::Tritanopia,
    ];

    #[test]
    fn matrix_rows_sum_to_one_so_greys_are_preserved() {
        for kind in ALL {
            for row in kind.matrix() {
                let sum: f32 = row.iter().sum();
                assert!((sum - 1.0).abs() < 1e-5, "{kind:?} {row:?}");
            }
        }
    }

    #[test]
    fn greys_are_unchanged() {
        for kind in ALL {
            for v in [0.0, 0.25, 0.5, 1.0] {
                let seen = simulate([v; 3], kind);
                for c in seen {
                    assert!((c - v).abs() < 1e-4, "{kind:?} {v} -> {seen:?}");
                }
            }
        }
    }

    #[test]
    fn output_stays_in_range_and_changes_saturated_colours() {
        for kind in ALL {
            let seen = simulate([1.0, 0.0, 0.0], kind);
            assert!(seen.iter().all(|c| (0.0..=1.0).contains(c)), "{seen:?}");
            assert!(seen[1] > 0.0 || seen[2] > 0.0, "{kind:?} {seen:?}");
        }
    }
}
