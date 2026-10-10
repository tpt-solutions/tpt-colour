//! ICC profile parsing and transforms (`no_std` + `alloc`).
//!
//! Supports matrix/TRC RGB profiles and LUT profiles (`mft1`, `mft2`, `mAB `, `mBA `)
//! for RGB, CMYK and grey data, in ICC v2 and v4. `multiProcessElements` (`mpet`) is out
//! of scope for v1.0. Parsing is bounds-checked throughout: malformed input returns an
//! error and never panics.
#![no_std]
extern crate alloc;

mod bytes;
mod error;
mod lut;
mod lut_tags;
mod math;
mod matrix;
pub mod pcs;
mod profile;
mod tag;
mod transform;
mod trc;

pub use error::IccError;
pub use lut::Pipeline;
pub use profile::{
    IccProfile, IccVersion, LutTables, MatrixTrc, ProfileClass, ProfileConnectionSpace,
    ProfileModel, RenderingIntent,
};
pub use transform::ColourTransform;
pub use trc::Trc;

/// Parses an ICC profile. Equivalent to [`IccProfile::parse`].
///
/// # Errors
///
/// Returns an [`IccError`] if the data is truncated, malformed or unsupported.
pub fn parse(data: &[u8]) -> Result<IccProfile, IccError> {
    IccProfile::parse(data)
}
