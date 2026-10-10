//! Bounds-checked big-endian readers. Every read returns `Truncated` instead of panicking.

use crate::error::IccError;

/// Returns `len` bytes starting at `off`, or `Truncated`.
pub(crate) fn slice(data: &[u8], off: usize, len: usize) -> Result<&[u8], IccError> {
    let end = off.checked_add(len).ok_or(IccError::Truncated)?;
    data.get(off..end).ok_or(IccError::Truncated)
}

/// Reads a 4-byte signature.
pub(crate) fn sig_at(data: &[u8], off: usize) -> Result<[u8; 4], IccError> {
    let s = slice(data, off, 4)?;
    Ok([s[0], s[1], s[2], s[3]])
}

/// Reads a big-endian `u16`.
pub(crate) fn u16_at(data: &[u8], off: usize) -> Result<u16, IccError> {
    let s = slice(data, off, 2)?;
    Ok(u16::from_be_bytes([s[0], s[1]]))
}

/// Reads a big-endian `u32`.
pub(crate) fn u32_at(data: &[u8], off: usize) -> Result<u32, IccError> {
    let s = slice(data, off, 4)?;
    Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// Reads a signed 15.16 fixed-point number.
pub(crate) fn s15f16_at(data: &[u8], off: usize) -> Result<f32, IccError> {
    let s = slice(data, off, 4)?;
    Ok(i32::from_be_bytes([s[0], s[1], s[2], s[3]]) as f32 / 65_536.0)
}
