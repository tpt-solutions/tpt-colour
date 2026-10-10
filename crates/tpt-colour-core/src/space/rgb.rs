//! RGB spaces with D65 white points: sRGB, linear sRGB, Display P3 and Rec. 2020.

use super::ColourSpace;
use crate::math::{mat3_mul, powf};

// Linear RGB → XYZ (D65) matrices, row-major, derived from each space's primaries.
const SRGB_TO_XYZ: [[f32; 3]; 3] = [
    [0.412_456_4, 0.357_576_1, 0.180_437_5],
    [0.212_672_9, 0.715_152_2, 0.072_175],
    [0.019_333_9, 0.119_192, 0.950_304_1],
];
const XYZ_TO_SRGB: [[f32; 3]; 3] = [
    [3.240_454_2, -1.537_138_5, -0.498_531_4],
    [-0.969_266, 1.876_010_8, 0.041_556],
    [0.055_643_4, -0.204_025_9, 1.057_225_2],
];
const P3_TO_XYZ: [[f32; 3]; 3] = [
    [0.486_570_9, 0.265_667_7, 0.198_217_3],
    [0.228_974_6, 0.691_738_5, 0.079_286_9],
    [0.0, 0.045_113_4, 1.043_944_4],
];
const XYZ_TO_P3: [[f32; 3]; 3] = [
    [2.493_497, -0.931_383_6, -0.402_710_8],
    [-0.829_489_5, 1.762_664_1, 0.023_624_7],
    [0.035_845_8, -0.076_172_4, 0.956_884_5],
];
const REC2020_TO_XYZ: [[f32; 3]; 3] = [
    [0.636_958, 0.144_616_9, 0.168_880_9],
    [0.262_700_2, 0.677_998_1, 0.059_301_7],
    [0.0, 0.028_072_7, 1.060_985_1],
];
const XYZ_TO_REC2020: [[f32; 3]; 3] = [
    [1.716_651_2, -0.355_670_8, -0.253_366_3],
    [-0.666_684_4, 1.616_481_2, 0.015_768_5],
    [0.017_639_9, -0.042_770_6, 0.942_103_1],
];

/// sRGB transfer function (IEC 61966-2-1) decoding to linear light.
#[must_use]
pub(crate) fn srgb_eotf(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        powf((v + 0.055) / 1.055, 2.4)
    }
}

/// sRGB transfer function encoding linear light.
#[must_use]
pub(crate) fn srgb_oetf(l: f32) -> f32 {
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * powf(l, 1.0 / 2.4) - 0.055
    }
}

/// BT.2020 transfer function decoding to linear light.
#[must_use]
fn rec2020_eotf(v: f32) -> f32 {
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
