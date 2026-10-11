//! Colour spaces and conversions to and from CIE XYZ.
//!
//! All XYZ values are relative to the D65 white point with `Y = 1` for white,
//! unless a type documents otherwise.

mod adapt;
mod cmyk;
mod lab;
mod luv;
mod rgb;
mod ycbcr;
mod ycocg;

pub use adapt::{d50_to_d65, d65_to_d50};
pub use cmyk::Cmyk;
pub use lab::{Lab, Lch, Oklab, Oklch};
pub use luv::Luv;
pub use rgb::{AdobeRgb, DisplayP3, LinearSRgb, ProPhotoRgb, Rec2020, SRgb};
pub(crate) use rgb::{srgb_eotf, srgb_oetf};
pub use ycbcr::YCbCr;
pub use ycocg::{ycocg_decode, ycocg_encode};

/// D65 reference white (relative XYZ, `Y = 1`).
pub const D65: [f32; 3] = [0.950_47, 1.0, 1.088_83];

/// D50 reference white (relative XYZ, `Y = 1`), used by the ICC profile connection space.
pub const D50: [f32; 3] = [0.964_22, 1.0, 0.825_21];

/// A colour space with `N` channels that converts to and from CIE XYZ.
#[allow(clippy::wrong_self_convention)] // `from_xyz` is the spec's name for this method.
pub trait ColourSpace<const N: usize>: Copy + Clone {
    /// Converts a colour in this space to XYZ.
    #[must_use]
    fn to_xyz(&self, color: [f32; N]) -> [f32; 3];

    /// Converts XYZ to this space. Out-of-gamut results are returned unclamped.
    #[must_use]
    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; N];
}

/// CIE XYZ itself (D65-relative).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Xyz;

impl ColourSpace<3> for Xyz {
    fn to_xyz(&self, color: [f32; 3]) -> [f32; 3] {
        color
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 3] {
        xyz
    }
}
