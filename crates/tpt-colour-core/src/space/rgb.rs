//! RGB spaces with D65 white points: sRGB, linear sRGB, Display P3 and Rec. 2020.

use super::adapt::{d50_to_d65, d65_to_d50};
use super::{ColourSpace, D50, D65};
use crate::math::{mat3_mul, powf};

// Primaries as CIE xy chromaticities (r, g, b). Every matrix below is derived from these and
// the one D65 white point in `super`, so all the spaces share a white exactly.
const SRGB_PRIMARIES: [[f32; 2]; 3] = [[0.640, 0.330], [0.300, 0.600], [0.150, 0.060]];
const P3_PRIMARIES: [[f32; 2]; 3] = [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]];
const REC2020_PRIMARIES: [[f32; 2]; 3] = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]];

/// Linear RGB → XYZ matrix for `primaries` and `white` (XYZ, `Y = 1`), row-major.
///
/// Scales each primary's column so that `[1, 1, 1]` maps to `white`.
#[allow(clippy::many_single_char_names)]
const fn rgb_to_xyz(primaries: [[f32; 2]; 3], white: [f32; 3]) -> [[f32; 3]; 3] {
    let mut p = [[0.0_f32; 3]; 3];
    let mut c = 0;
    while c < 3 {
        let [x, y] = primaries[c];
        p[0][c] = x / y;
        p[1][c] = 1.0;
        p[2][c] = (1.0 - x - y) / y;
        c += 1;
    }
    let s = mul_vec_const(inv3_const(p), white);
    let mut m = [[0.0_f32; 3]; 3];
    let mut r = 0;
    while r < 3 {
        let mut c = 0;
        while c < 3 {
            m[r][c] = p[r][c] * s[c];
            c += 1;
        }
        r += 1;
    }
    m
}

/// Inverse of a 3×3 matrix by the adjugate. Only used on the constant primaries matrix.
#[allow(clippy::many_single_char_names)]
const fn inv3_const(m: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let k = 1.0 / det;
    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * k,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * k,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * k,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * k,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * k,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * k,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * k,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * k,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * k,
        ],
    ]
}

const fn mul_vec_const(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

const SRGB_TO_XYZ: [[f32; 3]; 3] = rgb_to_xyz(SRGB_PRIMARIES, D65);
const XYZ_TO_SRGB: [[f32; 3]; 3] = inv3_const(SRGB_TO_XYZ);
const P3_TO_XYZ: [[f32; 3]; 3] = rgb_to_xyz(P3_PRIMARIES, D65);
const XYZ_TO_P3: [[f32; 3]; 3] = inv3_const(P3_TO_XYZ);
const REC2020_TO_XYZ: [[f32; 3]; 3] = rgb_to_xyz(REC2020_PRIMARIES, D65);
const XYZ_TO_REC2020: [[f32; 3]; 3] = inv3_const(REC2020_TO_XYZ);
const ADOBE_PRIMARIES: [[f32; 2]; 3] = [[0.640, 0.330], [0.210, 0.710], [0.150, 0.060]];
const ADOBE_TO_XYZ: [[f32; 3]; 3] = rgb_to_xyz(ADOBE_PRIMARIES, D65);
const XYZ_TO_ADOBE: [[f32; 3]; 3] = inv3_const(ADOBE_TO_XYZ);
/// Adobe RGB (1998) encoding exponent, 563/256 (about 2.2).
const ADOBE_GAMMA: f32 = 563.0 / 256.0;

/// Adobe RGB encoding, mirrored about zero so out-of-range negative values stay defined.
fn adobe_eotf(v: f32) -> f32 {
    if v < 0.0 {
        -powf(-v, ADOBE_GAMMA)
    } else {
        powf(v, ADOBE_GAMMA)
    }
}

fn adobe_oetf(l: f32) -> f32 {
    if l < 0.0 {
        -powf(-l, 1.0 / ADOBE_GAMMA)
    } else {
        powf(l, 1.0 / ADOBE_GAMMA)
    }
}

/// Extends a transfer curve defined on `[0, ∞)` to negative values by odd symmetry, as scRGB
/// does. Colours outside the gamut then keep their sign, and `f(−x) = −f(x)` everywhere.
fn mirror(curve: fn(f32) -> f32, v: f32) -> f32 {
    if v < 0.0 { -curve(-v) } else { curve(v) }
}

/// sRGB transfer function (IEC 61966-2-1) decoding to linear light.
#[must_use]
pub(crate) fn srgb_eotf(v: f32) -> f32 {
    mirror(srgb_eotf_pos, v)
}

fn srgb_eotf_pos(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        powf((v + 0.055) / 1.055, 2.4)
    }
}

/// sRGB transfer function encoding linear light.
#[must_use]
pub(crate) fn srgb_oetf(l: f32) -> f32 {
    mirror(srgb_oetf_pos, l)
}

fn srgb_oetf_pos(l: f32) -> f32 {
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * powf(l, 1.0 / 2.4) - 0.055
    }
}

/// BT.2020 transfer function decoding to linear light.
#[must_use]
fn rec2020_eotf(v: f32) -> f32 {
    mirror(rec2020_eotf_pos, v)
}

fn rec2020_eotf_pos(v: f32) -> f32 {
    const BETA: f32 = 0.018_053_97;
    if v < 4.5 * BETA {
        v / 4.5
    } else {
        powf((v + 0.099_296_8) / 1.099_296_8, 1.0 / 0.45)
    }
}

/// BT.2020 transfer function encoding linear light.
#[must_use]
fn rec2020_oetf(l: f32) -> f32 {
    mirror(rec2020_oetf_pos, l)
}

fn rec2020_oetf_pos(l: f32) -> f32 {
    const BETA: f32 = 0.018_053_97;
    if l < BETA {
        4.5 * l
    } else {
        1.099_296_8 * powf(l, 0.45) - 0.099_296_8
    }
}

/// sRGB (gamma-encoded, D65 white point).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SRgb;

impl ColourSpace<3> for SRgb {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        let lin = [srgb_eotf(c[0]), srgb_eotf(c[1]), srgb_eotf(c[2])];
        mat3_mul(&SRGB_TO_XYZ, lin)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let lin = mat3_mul(&XYZ_TO_SRGB, xyz);
        [srgb_oetf(lin[0]), srgb_oetf(lin[1]), srgb_oetf(lin[2])]
    }
}

/// Linear-light sRGB primaries (no transfer function).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinearSRgb;

impl ColourSpace<3> for LinearSRgb {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        mat3_mul(&SRGB_TO_XYZ, c)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        mat3_mul(&XYZ_TO_SRGB, xyz)
    }
}

/// Display P3 (sRGB transfer function, P3 primaries, D65 white point).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisplayP3;

impl ColourSpace<3> for DisplayP3 {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        let lin = [srgb_eotf(c[0]), srgb_eotf(c[1]), srgb_eotf(c[2])];
        mat3_mul(&P3_TO_XYZ, lin)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let lin = mat3_mul(&XYZ_TO_P3, xyz);
        [srgb_oetf(lin[0]), srgb_oetf(lin[1]), srgb_oetf(lin[2])]
    }
}

/// Rec. 2020 (BT.2020 transfer function, D65 white point).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rec2020;

impl ColourSpace<3> for Rec2020 {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        let lin = [rec2020_eotf(c[0]), rec2020_eotf(c[1]), rec2020_eotf(c[2])];
        mat3_mul(&REC2020_TO_XYZ, lin)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let lin = mat3_mul(&XYZ_TO_REC2020, xyz);
        [
            rec2020_oetf(lin[0]),
            rec2020_oetf(lin[1]),
            rec2020_oetf(lin[2]),
        ]
    }
}

/// `ProPhoto` RGB (ROMM): primaries and white are D50, so results are adapted to D65 at the edge.
const PROPHOTO_PRIMARIES: [[f32; 2]; 3] = [[0.7347, 0.2653], [0.1596, 0.8404], [0.0366, 0.0001]];
const PROPHOTO_TO_XYZ: [[f32; 3]; 3] = rgb_to_xyz(PROPHOTO_PRIMARIES, D50);
const XYZ_TO_PROPHOTO: [[f32; 3]; 3] = inv3_const(PROPHOTO_TO_XYZ);

/// `ProPhoto` encoding: a linear toe below 16/512, then a gamma of 1.8.
fn prophoto_eotf(v: f32) -> f32 {
    mirror(prophoto_eotf_pos, v)
}

fn prophoto_eotf_pos(v: f32) -> f32 {
    if v < 16.0 / 512.0 {
        v / 16.0
    } else {
        powf(v, 1.8)
    }
}

fn prophoto_oetf(l: f32) -> f32 {
    mirror(prophoto_oetf_pos, l)
}

fn prophoto_oetf_pos(l: f32) -> f32 {
    if l < 1.0 / 512.0 {
        16.0 * l
    } else {
        powf(l, 1.0 / 1.8)
    }
}

/// `ProPhoto` RGB (ROMM), D50 white. Its gamut is wider than Rec. 2020, and it is used for
/// lossless editing. Values in the [`ColourSpace`] interface are D65-relative, as elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProPhotoRgb;

impl ColourSpace<3> for ProPhotoRgb {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        let lin = [
            prophoto_eotf(c[0]),
            prophoto_eotf(c[1]),
            prophoto_eotf(c[2]),
        ];
        d50_to_d65(mat3_mul(&PROPHOTO_TO_XYZ, lin))
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let lin = mat3_mul(&XYZ_TO_PROPHOTO, d65_to_d50(xyz));
        [
            prophoto_oetf(lin[0]),
            prophoto_oetf(lin[1]),
            prophoto_oetf(lin[2]),
        ]
    }
}

/// Adobe RGB (1998): gamma 563/256 (about 2.2), Adobe primaries, D65 white point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdobeRgb;

impl ColourSpace<3> for AdobeRgb {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        let lin = [adobe_eotf(c[0]), adobe_eotf(c[1]), adobe_eotf(c[2])];
        mat3_mul(&ADOBE_TO_XYZ, lin)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let lin = mat3_mul(&XYZ_TO_ADOBE, xyz);
        [adobe_oetf(lin[0]), adobe_oetf(lin[1]), adobe_oetf(lin[2])]
    }
}
