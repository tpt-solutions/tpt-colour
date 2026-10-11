//! HDR colour spaces: `ICtCp` (ITU-R BT.2100) and `Jzazbz` (Safdar et al., 2017).
//!
//! Both take CIE XYZ **in cd/m²** (absolute luminance, D65), not the relative XYZ of the other
//! spaces, because their perceptual encodings depend on absolute luminance. Their PQ curve is
//! normalised to 10 000 cd/m². The matrices are the ones `colour-science` uses, so the results
//! can be compared with it directly.

use crate::math::mat3_mul;

/// ST 2084 constants: `m1 = 2610/16384`, `m2 = 2523/4096 · 128`, `c1 = 3424/4096`,
/// `c2 = 2413/128`, `c3 = 2392/128`.
const PQ_M1: f32 = 0.159_301_76;
const PQ_M2: f32 = 78.843_75;
const PQ_C1: f32 = 0.835_937_5;
const PQ_C2: f32 = 18.851_562;
const PQ_C3: f32 = 18.687_5;

/// `Jzazbz` uses the same PQ curve with its own exponent, `1.7 · 2523/4096 · 128`.
const JZ_PQ_M2: f32 = 134.034_38;

/// Absolute luminance (cd/m²) to the PQ signal in `[0, 1]`, for the given exponent .
///
/// The powers are evaluated in f64. With an exponent near 134 (`Jzazbz`), f32 rounding in the base
/// is amplified to about 1e-5 in the result, which is larger than a colour code.
fn pq_encode_with(nits: f32, m2: f32) -> f32 {
    let (c1, c2, c3) = (f64::from(PQ_C1), f64::from(PQ_C2), f64::from(PQ_C3));
    let y = libm::pow((f64::from(nits) / 10_000.0).max(0.0), f64::from(PQ_M1));
    libm::pow((c1 + c2 * y) / (1.0 + c3 * y), f64::from(m2)) as f32
}

/// PQ signal to absolute luminance (cd/m²), for the given exponent .
fn pq_decode_with(signal: f32, m2: f32) -> f32 {
    let (c1, c2, c3) = (f64::from(PQ_C1), f64::from(PQ_C2), f64::from(PQ_C3));
    let p = libm::pow(f64::from(signal.max(0.0)), 1.0 / f64::from(m2));
    let numerator = (p - c1).max(0.0);
    let denominator = c2 - c3 * p;
    (10_000.0 * libm::pow(numerator / denominator, 1.0 / f64::from(PQ_M1))) as f32
}

/// ST 2084 PQ encoding, the `ICtCp` curve.
fn pq_encode(nits: f32) -> f32 {
    pq_encode_with(nits, PQ_M2)
}

/// ST 2084 PQ decoding, the `ICtCp` curve.
fn pq_decode(signal: f32) -> f32 {
    pq_decode_with(signal, PQ_M2)
}

/// CIE XYZ (D65, cd/m²) to linear Rec. 2020 RGB.
const XYZ_TO_REC2020: [[f32; 3]; 3] = [
    [1.716_651_2, -0.355_670_8, -0.253_366_3],
    [-0.666_684_3, 1.616_481_2, 0.015_768_547],
    [0.017_639_857, -0.042_770_613, 0.942_103_1],
];

/// Linear Rec. 2020 RGB to CIE XYZ (D65, cd/m²). The inverse of [`XYZ_TO_REC2020`].
const REC2020_TO_XYZ: [[f32; 3]; 3] = [
    [0.636_958_06, 0.144_616_9, 0.168_880_98],
    [0.262_700_2, 0.677_998_07, 0.059_301_715],
    [0.0, 0.028_072_692, 1.060_985_1],
];

/// Linear Rec. 2020 RGB to the cone-like LMS of `ICtCp`.
const REC2020_TO_LMS: [[f32; 3]; 3] = [
    [0.412_109_38, 0.523_925_8, 0.063_964_844],
    [0.166_748_05, 0.720_459, 0.112_792_97],
    [0.024_169_922, 0.075_439_45, 0.900_390_6],
];

/// The inverse of [`REC2020_TO_LMS`].
const LMS_TO_REC2020: [[f32; 3]; 3] = [
    [3.436_606_6, -2.506_452, 0.069_845_42],
    [-0.791_329_56, 1.983_600_5, -0.192_270_9],
    [-0.025_949_9, -0.098_913_71, 1.124_863_6],
];

/// PQ-coded LMS to `ICtCp`.
const LMS_P_TO_ICTCP: [[f32; 3]; 3] = [
    [0.5, 0.5, 0.0],
    [1.613_769_5, -3.323_486_3, 1.709_716_8],
    [4.378_174, -4.245_605_5, -0.132_568_36],
];

/// The inverse of [`LMS_P_TO_ICTCP`].
const ICTCP_TO_LMS_P: [[f32; 3]; 3] = [
    [1.0, 0.008_609_037, 0.111_029_625],
    [1.0, -0.008_609_037, -0.111_029_625],
    [1.0, 0.560_031_3, -0.320_627_18],
];

/// Converts XYZ in cd/m² (D65) to `ICtCp`, as ITU-R BT.2100-2 defines it for PQ.
///
/// `I` is intensity, and `Ct` and `Cp` are the blue-yellow and red-green chroma. `I` is 0 for
/// black and about 0.508 for D65 white at 100 cd/m².
///
/// # Example
///
/// ```
/// use tpt_colour_core::space::{ictcp_to_xyz, xyz_to_ictcp};
/// let ictcp = xyz_to_ictcp([100.0, 100.0, 100.0]);
/// assert!((ictcp[0] - 0.508).abs() < 1e-3);
/// let back = ictcp_to_xyz(ictcp);
/// assert!((back[1] - 100.0).abs() < 0.05);
/// ```
#[must_use]
pub fn xyz_to_ictcp(xyz: [f32; 3]) -> [f32; 3] {
    let rgb = mat3_mul(&XYZ_TO_REC2020, xyz);
    let lms = mat3_mul(&REC2020_TO_LMS, rgb);
    mat3_mul(&LMS_P_TO_ICTCP, lms.map(pq_encode))
}

/// Converts `ICtCp` back to XYZ in cd/m² (D65). The inverse of [`xyz_to_ictcp`].
#[must_use]
pub fn ictcp_to_xyz(ictcp: [f32; 3]) -> [f32; 3] {
    let lms = mat3_mul(&ICTCP_TO_LMS_P, ictcp).map(pq_decode);
    let rgb = mat3_mul(&LMS_TO_REC2020, lms);
    mat3_mul(&REC2020_TO_XYZ, rgb)
}

/// `Jzazbz` parameters (Safdar et al., 2017).
const JZ_B: f32 = 1.15;
const JZ_G: f32 = 0.66;
const JZ_D: f32 = -0.56;
const JZ_D0: f32 = 0.000_000_000_016_295_5;

/// `Jzazbz`: XYZ to the LMS of the `Jzazbz` model, before the PQ curve.
const XYZ_TO_JZ_LMS: [[f32; 3]; 3] = [
    [0.414_789_7, 0.579_999, 0.014_648],
    [-0.201_51, 1.120_649, 0.053_100_8],
    [-0.016_600_8, 0.264_8, 0.668_479_9],
];

/// The inverse of [`XYZ_TO_JZ_LMS`].
const JZ_LMS_TO_XYZ: [[f32; 3]; 3] = [
    [1.924_226_4, -1.004_792_3, 0.037_651_405],
    [0.350_316_76, 0.726_481_2, -0.065_384_42],
    [-0.090_982_81, -0.312_728_3, 1.522_766_6],
];

/// PQ-coded LMS to Izazbz.
const LMS_P_TO_IZAZBZ: [[f32; 3]; 3] = [
    [0.5, 0.5, 0.0],
    [3.524, -4.066_708, 0.542_708],
    [0.199_076, 1.096_799, -1.295_875],
];

/// The inverse of [`LMS_P_TO_IZAZBZ`].
const IZAZBZ_TO_LMS_P: [[f32; 3]; 3] = [
    [1.0, 0.138_605_04, 0.058_047_317],
    [1.0, -0.138_605_04, -0.058_047_317],
    [1.0, -0.096_019_24, -0.811_891_9],
];

/// Converts XYZ in cd/m² (D65) to `Jzazbz` `[Jz, az, bz]`, after Safdar et al. (2017).
///
/// `Jz` is lightness, and `az` and `bz` are the red-green and yellow-blue opponent axes. `Jz` is
/// about 0.168 for D65 white at 100 cd/m².
///
/// # Example
///
/// ```
/// use tpt_colour_core::space::{jzazbz_to_xyz, xyz_to_jzazbz};
/// let jab = xyz_to_jzazbz([100.0, 100.0, 100.0]);
/// assert!((jab[0] - 0.168_48).abs() < 1e-4);
/// let back = jzazbz_to_xyz(jab);
/// assert!((back[1] - 100.0).abs() < 0.01);
/// ```
#[must_use]
pub fn xyz_to_jzazbz(xyz: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = xyz;
    let adjusted = [JZ_B * x - (JZ_B - 1.0) * z, JZ_G * y - (JZ_G - 1.0) * x, z];
    let lms = mat3_mul(&XYZ_TO_JZ_LMS, adjusted).map(|v| pq_encode_with(v, JZ_PQ_M2));
    let izazbz = mat3_mul(&LMS_P_TO_IZAZBZ, lms);
    let jz = ((1.0 + JZ_D) * izazbz[0]) / (1.0 + JZ_D * izazbz[0]) - JZ_D0;
    [jz, izazbz[1], izazbz[2]]
}

/// Converts `Jzazbz` `[Jz, az, bz]` back to XYZ in cd/m² (D65). The inverse of [`xyz_to_jzazbz`].
#[must_use]
pub fn jzazbz_to_xyz(jab: [f32; 3]) -> [f32; 3] {
    let [jz, az, bz] = jab;
    let iz = (jz + JZ_D0) / (1.0 + JZ_D - JZ_D * (jz + JZ_D0));
    let lms = mat3_mul(&IZAZBZ_TO_LMS_P, [iz, az, bz]).map(|v| pq_decode_with(v, JZ_PQ_M2));
    let [xm, ym, zm] = mat3_mul(&JZ_LMS_TO_XYZ, lms);
    let z = zm;
    let x = (xm + (JZ_B - 1.0) * z) / JZ_B;
    let y = (ym + (JZ_G - 1.0) * x) / JZ_G;
    [x, y, z]
}
