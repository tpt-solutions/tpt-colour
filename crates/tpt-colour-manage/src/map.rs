//! Perceptual gamut mapping by chroma reduction in OKLCH, following CSS Color Level 4 §13.2.

use tpt_colour_core::space::{ColourSpace, Oklab, Oklch};

use crate::gamut::Gamut;

/// Just-noticeable difference in ΔEOK. Clipped colours within this distance are accepted.
const JND: f32 = 0.02;
/// Convergence threshold for the chroma search.
const EPSILON: f32 = 0.0001;

/// Maps an XYZ (D65) colour into `gamut` by reducing OKLCH chroma.
///
/// Lightness and hue are kept. Chroma is reduced by bisection until the colour is inside
/// the gamut or the clipped colour is within [`JND`] of the original, which is what
/// "perceptually close" means here. Colours with `L ≥ 1` map to white and `L ≤ 0` to black.
///
/// Colours already inside the gamut are returned unchanged.
#[must_use]
pub fn map_to_gamut(gamut: &Gamut, xyz: [f32; 3]) -> [f32; 3] {
    let origin = Oklch.from_xyz(xyz);
    let lightness = origin[0];
    if lightness >= 1.0 {
        return Oklch.to_xyz([1.0, 0.0, 0.0]);
    }
    if lightness <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    if gamut.contains(xyz) {
        return xyz;
    }

    let (mut clipped, mut delta) = clip_candidate(gamut, origin);
    if delta < JND {
        return clipped;
    }

    let (mut min, mut max) = (0.0_f32, origin[1]);
    let mut min_in_gamut = true;
    let mut current = origin;
    while max - min > EPSILON {
        let chroma = f32::midpoint(min, max);
        current[1] = chroma;
        if min_in_gamut && gamut.contains(Oklch.to_xyz(current)) {
            min = chroma;
            continue;
        }
        (clipped, delta) = clip_candidate(gamut, current);
        if delta < JND {
            if JND - delta < EPSILON {
                return clipped;
            }
            min_in_gamut = false;
            min = chroma;
        } else {
            max = chroma;
        }
    }
    clipped
}

/// Clips an OKLCH candidate and reports the ΔEOK between the clipped and unclipped colours.
fn clip_candidate(gamut: &Gamut, lch: [f32; 3]) -> ([f32; 3], f32) {
    let xyz = Oklch.to_xyz(lch);
    let clipped = gamut.clip(xyz);
    let a = Oklab.from_xyz(clipped);
    let b = Oklab.from_xyz(xyz);
    let (dl, da, db) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    let delta = libm::sqrtf(dl * dl + da * da + db * db);
    (clipped, delta)
}
