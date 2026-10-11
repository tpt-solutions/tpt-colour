//! CIE `Lab` and `LCh` (D65 reference white), and Oklab / Oklch.

use super::{ColourSpace, D50, D65, d50_to_d65, d65_to_d50};
use crate::math::{atan2, cbrt, cos, mat3_mul, sin, sqrt};

const LAB_EPSILON: f32 = 216.0 / 24_389.0; // (6/29)³
const LAB_KAPPA: f32 = 24_389.0 / 27.0;

fn lab_f(t: f32) -> f32 {
    if t > LAB_EPSILON {
        cbrt(t)
    } else {
        (LAB_KAPPA * t + 16.0) / 116.0
    }
}

fn lab_f_inv(t: f32) -> f32 {
    if t > 6.0 / 29.0 {
        t * t * t
    } else {
        3.0 * (6.0 / 29.0) * (6.0 / 29.0) * (t - 4.0 / 29.0)
    }
}

fn xyz_to_lab(xyz: [f32; 3]) -> [f32; 3] {
    xyz_to_lab_white(xyz, D65)
}

/// XYZ to CIE L*a*b* relative to `white` (XYZ with `Y = 1`).
fn xyz_to_lab_white(xyz: [f32; 3], white: [f32; 3]) -> [f32; 3] {
    let fx = lab_f(xyz[0] / white[0]);
    let fy = lab_f(xyz[1] / white[1]);
    let fz = lab_f(xyz[2] / white[2]);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_to_xyz(lab: [f32; 3]) -> [f32; 3] {
    lab_to_xyz_white(lab, D65)
}

/// CIE L*a*b* relative to `white` to XYZ with `Y = 1` for white.
fn lab_to_xyz_white(lab: [f32; 3], white: [f32; 3]) -> [f32; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    [
        white[0] * lab_f_inv(fx),
        white[1] * lab_f_inv(fy),
        white[2] * lab_f_inv(fz),
    ]
}

/// Converts rectangular `(L, a, b)` to polar `(L, C, h)` with `h` in degrees in `[0, 360)`.
///
/// The hue of an achromatic colour (chroma near zero) is the angle of its rounding noise. It is
/// kept, not zeroed, so that converting back to `(a, b)` is exact. Test the chroma to find
/// greys.
fn to_polar(lab: [f32; 3]) -> [f32; 3] {
    let c = sqrt(lab[1] * lab[1] + lab[2] * lab[2]);
    let mut h = atan2(lab[2], lab[1]).to_degrees();
    if h < 0.0 {
        h += 360.0;
    }
    // A tiny negative angle rounds up to exactly 360, and `atan2(-0.0, x)` gives -0.0.
    if h >= 360.0 {
        h -= 360.0;
    }
    if h == 0.0 {
        h = 0.0;
    }
    [lab[0], c, h]
}

/// Converts polar `(L, C, h)` (hue in degrees) to rectangular `(L, a, b)`.
fn to_rect(lch: [f32; 3]) -> [f32; 3] {
    let h = lch[2].to_radians();
    [lch[0], lch[1] * cos(h), lch[1] * sin(h)]
}

/// CIE L*a*b* (D65 reference white).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lab;

impl ColourSpace<3> for Lab {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        lab_to_xyz(c)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        xyz_to_lab(xyz)
    }
}

/// CIE LCh(ab): polar form of [`Lab`], hue in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lch;

impl ColourSpace<3> for Lch {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        lab_to_xyz(to_rect(c))
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        to_polar(xyz_to_lab(xyz))
    }
}

/// Björn Ottosson's Oklab, computed from D65-relative XYZ.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Oklab;

const XYZ_TO_LMS: [[f32; 3]; 3] = [
    [0.818_933, 0.361_866_74, -0.128_859_71],
    [0.032_984_544, 0.929_311_9, 0.036_145_64],
    [0.048_200_3, 0.264_366_27, 0.633_851_7],
];
const LMS_TO_XYZ: [[f32; 3]; 3] = [
    [1.227_014, -0.557_8, 0.281_256_1],
    [-0.040_580_18, 1.112_256_9, -0.071_676_68],
    [-0.076_381_28, -0.421_481_98, 1.586_163_2],
];
const LMS_CBRT_TO_LAB: [[f32; 3]; 3] = [
    [0.210_454_26, 0.793_617_8, -0.004_072_05],
    [1.977_998_5, -2.428_592_2, 0.450_593_7],
    [0.025_904_04, 0.782_771_77, -0.808_675_77],
];
const LAB_TO_LMS_CBRT: [[f32; 3]; 3] = [
    [1.0, 0.396_337_78, 0.215_803_76],
    [1.0, -0.105_561_35, -0.063_854_17],
    [1.0, -0.089_484_18, -1.291_485_5],
];

impl ColourSpace<3> for Oklab {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        let l = mat3_mul(&LAB_TO_LMS_CBRT, c);
        let lms = [l[0] * l[0] * l[0], l[1] * l[1] * l[1], l[2] * l[2] * l[2]];
        mat3_mul(&LMS_TO_XYZ, lms)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        let lms = mat3_mul(&XYZ_TO_LMS, xyz);
        let lms_c = [cbrt(lms[0]), cbrt(lms[1]), cbrt(lms[2])];
        mat3_mul(&LMS_CBRT_TO_LAB, lms_c)
    }
}

/// Oklch: polar form of [`Oklab`], hue in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Oklch;

impl ColourSpace<3> for Oklch {
    fn to_xyz(&self, c: [f32; 3]) -> [f32; 3] {
        Oklab.to_xyz(to_rect(c))
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        to_polar(Oklab.from_xyz(xyz))
    }
}

#[cfg(test)]
mod hue_tests {
    use super::to_polar;

    #[test]
    fn hue_is_in_range_and_never_negative_zero() {
        // Negative zero hue, and a tiny negative angle that would round to 360.
        for lab in [
            [50.0, 10.0, -0.0],
            [50.0, -10.0, -0.0],
            [50.0, 10.0, 0.0],
            [50.0, 1.0, -1e-30],
        ] {
            let h = to_polar(lab)[2];
            assert!(
                (0.0..360.0).contains(&h),
                "hue {h} out of range for {lab:?}"
            );
            assert!(h.is_sign_positive(), "hue {h} is negative zero for {lab:?}");
        }
    }
}

/// CIE L*a*b* with the D50 reference white, the white ICC profiles use for their PCS.
///
/// XYZ is D65-relative at this trait's boundary, as everywhere else in the crate, so `from_xyz`
/// and `to_xyz` adapt between the two whites with Bradford.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lab50;

impl ColourSpace<3> for Lab50 {
    fn to_xyz(&self, lab: [f32; 3]) -> [f32; 3] {
        d50_to_d65(lab_to_xyz_white(lab, D50))
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        xyz_to_lab_white(d65_to_d50(xyz), D50)
    }
}
