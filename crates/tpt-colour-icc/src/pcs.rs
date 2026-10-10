//! Profile connection space (PCS): D50 Lab and XYZ, and the 16-bit encodings used by LUT tags.
//!
//! Internally the PCS is CIE XYZ relative to D50. Lab is a D50 presentation of the same
//! data. LUT tables work on normalised values in `[0, 1]`, so these functions map between
//! that normalised form and physical values.

use tpt_colour_core::space::D50;

use crate::profile::ProfileConnectionSpace;

const LAB_EPSILON: f32 = 216.0 / 24_389.0;
const LAB_KAPPA: f32 = 24_389.0 / 27.0;
/// Full-scale XYZ in the 1.15 fixed-point encoding used by ICC (`0x8000` is 1.0).
const XYZ_FULL_SCALE: f32 = 65_535.0 / 32_768.0;

/// Encoding of Lab values in 16-bit LUT tables. ICC v2 and v4 differ in their scaling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabEncoding {
    /// Used by `mft2` (and by `mft1` with 8-bit scaling). `L` spans `0..=0xFF00`, `a` and `b` use `0x8000` as zero.
    V2,
    /// Used by `mAB ` and `mBA `. The full 16-bit range maps onto `L` `0..=100`, `a` and `b` `-128..=127`.
    V4,
}

/// Converts XYZ (D50) to Lab (D50).
#[must_use]
pub fn xyz_to_lab_d50(xyz: [f32; 3]) -> [f32; 3] {
    let f = |t: f32| {
        if t > LAB_EPSILON {
            libm::cbrtf(t)
        } else {
            (LAB_KAPPA * t + 16.0) / 116.0
        }
    };
    let fx = f(xyz[0] / D50[0]);
    let fy = f(xyz[1] / D50[1]);
    let fz = f(xyz[2] / D50[2]);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// Converts Lab (D50) to XYZ (D50).
#[must_use]
pub fn lab_to_xyz_d50(lab: [f32; 3]) -> [f32; 3] {
    let f_inv = |t: f32| {
        if t > 6.0 / 29.0 {
            t * t * t
        } else {
            3.0 * (6.0 / 29.0) * (6.0 / 29.0) * (t - 4.0 / 29.0)
        }
    };
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    [D50[0] * f_inv(fx), D50[1] * f_inv(fy), D50[2] * f_inv(fz)]
}

/// Maps a normalised PCS triple from a LUT output to physical XYZ (D50).
pub(crate) fn decode(pcs: ProfileConnectionSpace, encoding: LabEncoding, n: [f32; 3]) -> [f32; 3] {
    match pcs {
        ProfileConnectionSpace::Xyz => n.map(|v| v * XYZ_FULL_SCALE),
        ProfileConnectionSpace::Lab => {
            let lab = match encoding {
                LabEncoding::V4 => [n[0] * 100.0, n[1] * 255.0 - 128.0, n[2] * 255.0 - 128.0],
                LabEncoding::V2 => [
                    n[0] * 65_535.0 / 65_280.0 * 100.0,
                    n[1] * 65_535.0 / 256.0 - 128.0,
                    n[2] * 65_535.0 / 256.0 - 128.0,
                ],
            };
            lab_to_xyz_d50(lab)
        }
    }
}

/// Maps physical XYZ (D50) to a normalised PCS triple for a LUT input.
pub(crate) fn encode(
    pcs: ProfileConnectionSpace,
    encoding: LabEncoding,
    xyz: [f32; 3],
) -> [f32; 3] {
    match pcs {
        ProfileConnectionSpace::Xyz => xyz.map(|v| v / XYZ_FULL_SCALE),
        ProfileConnectionSpace::Lab => {
            let lab = xyz_to_lab_d50(xyz);
            match encoding {
                LabEncoding::V4 => [
                    lab[0] / 100.0,
                    (lab[1] + 128.0) / 255.0,
                    (lab[2] + 128.0) / 255.0,
                ],
                LabEncoding::V2 => [
                    lab[0] * 65_280.0 / (65_535.0 * 100.0),
                    (lab[1] + 128.0) * 256.0 / 65_535.0,
                    (lab[2] + 128.0) * 256.0 / 65_535.0,
                ],
            }
        }
    }
}
