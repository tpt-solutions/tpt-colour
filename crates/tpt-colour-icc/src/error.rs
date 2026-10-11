//! Errors from parsing ICC profiles and building transforms.

use core::fmt;

/// Errors returned when parsing an ICC profile or building a transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IccError {
    /// The data ended before a structure it declares.
    Truncated,
    /// A required signature (the `acsp` magic, a tag type) did not match.
    BadSignature,
    /// A tag points outside the profile or holds invalid parameters.
    BadTag,
    /// Valid ICC, but uses a feature not yet supported: `multiProcessElements` tables, a grey or
    /// non-RGB matrix profile, an unknown curve type, or a LUT that exceeds the profile budget.
    Unsupported,
    /// A colorant matrix is singular and cannot be inverted.
    Singular,
    /// A slice has the wrong number of channels for the profile or transform.
    ChannelCount,
    /// An image buffer is too short for its width, height and stride, or the stride is shorter
    /// than one row of pixels.
    BufferLayout,
}

impl fmt::Display for IccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Truncated => "ICC data is truncated",
            Self::BadSignature => "ICC signature mismatch",
            Self::BadTag => "ICC tag is out of bounds or invalid",
            Self::Unsupported => "ICC profile uses an unsupported feature",
            Self::Singular => "ICC colorant matrix is singular",
            Self::ChannelCount => "channel count does not match the profile",
            Self::BufferLayout => "image buffer is too small for its width, height and stride",
        };
        f.write_str(msg)
    }
}

impl core::error::Error for IccError {}
