//! Perceptual gamut mapping by chroma reduction in OKLCH, following CSS Color Level 4 §13.2.

use tpt_colour_core::space::{ColourSpace, Oklab};

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
    if gamut.contains(xyz) {
        return xyz;
    }
    let origin = Oklab.from_xyz(xyz);
    let lightness = origin[0];
    if lightness >= 1.0 {
        return Oklab.to_xyz([1.0, 0.0, 0.0]);
    }
    if lightness <= 0.0 {
        return [0.0, 0.0, 0.0];
    }

    // Hue is fixed during the chroma search, so its cosine and sine are computed once.
    let (a0, b0) = (origin[1], origin[2]);
    let chroma0 = libm::sqrtf(a0 * a0 + b0 * b0);
    let (cos_h, sin_h) = if chroma0 > 0.0 {
        (a0 / chroma0, b0 / chroma0)
    } else {
        (1.0, 0.0)
    };
    let at = |chroma: f32| [lightness, chroma * cos_h, chroma * sin_h];

    let lab0 = at(chroma0);
    let (mut clipped, mut delta) = clip_candidate(gamut, lab0, gamut.to_linear(Oklab.to_xyz(lab0)));
    if delta < JND {
        return clipped;
    }

    let (mut min, mut max) = (0.0_f32, chroma0);
    let mut min_in_gamut = true;
    while max - min > EPSILON {
        let chroma = f32::midpoint(min, max);
        let lab = at(chroma);
        let xyz = Oklab.to_xyz(lab);
        let lin = gamut.to_linear(xyz);
        if min_in_gamut && Gamut::linear_in_gamut(lin) {
            min = chroma;
            continue;
        }
        (clipped, delta) = clip_candidate(gamut, lab, lin);
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

/// Clips an Oklab candidate and reports the ΔEOK between the clipped and unclipped colours.
///
/// `lin` is the candidate's linear RGB, which the caller has already computed.
fn clip_candidate(gamut: &Gamut, lab: [f32; 3], lin: [f32; 3]) -> ([f32; 3], f32) {
    let clipped = gamut.clip_linear(lin);
    let c = Oklab.from_xyz(clipped);
    let (dl, da, db) = (c[0] - lab[0], c[1] - lab[1], c[2] - lab[2]);
    let delta = libm::sqrtf(dl * dl + da * da + db * db);
    (clipped, delta)
}
