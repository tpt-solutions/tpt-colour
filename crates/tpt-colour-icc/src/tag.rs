//! Tag table and tag-type parsers (`XYZ `, `curv`, `para`, `chad`, `desc`, `text`, `mluc`).

use alloc::string::String;
use alloc::vec::Vec;

use crate::bytes::{s15f16_at, sig_at, slice, u16_at, u32_at};
use crate::error::IccError;
use crate::matrix::Mat3;
use crate::trc::Trc;

/// Offset of the tag count, immediately after the 128-byte header.
const TAG_COUNT_OFFSET: usize = 128;
/// Each tag table entry is `signature, offset, size`, 12 bytes.
const TAG_ENTRY_LEN: usize = 12;
/// Number of `f32` parameters for each parametric function type.
const PARAM_COUNTS: [usize; 5] = [1, 3, 4, 5, 7];

/// Tag table entries as `(signature, body)` pairs.
pub(crate) type Tags<'a> = Vec<([u8; 4], &'a [u8])>;

/// Returns every tag as `(signature, body)`. Each body is bounds-checked against the profile.
pub(crate) fn table(data: &[u8]) -> Result<Tags<'_>, IccError> {
    let count = u32_at(data, TAG_COUNT_OFFSET)? as usize;
    let first = TAG_COUNT_OFFSET + 4;
    // Bound the allocation by what the input can actually hold.
    if count > data.len().saturating_sub(first) / TAG_ENTRY_LEN {
        return Err(IccError::Truncated);
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = first + i * TAG_ENTRY_LEN;
        let sig = sig_at(data, base)?;
        let off = u32_at(data, base + 4)? as usize;
        let size = u32_at(data, base + 8)? as usize;
        let body = slice(data, off, size).map_err(|_| IccError::BadTag)?;
        out.push((sig, body));
    }
    Ok(out)
}

/// Parses an `XYZ ` tag.
pub(crate) fn xyz(body: &[u8]) -> Result<[f32; 3], IccError> {
    if sig_at(body, 0)? != *b"XYZ " {
        return Err(IccError::BadSignature);
    }
    Ok([
        s15f16_at(body, 8)?,
        s15f16_at(body, 12)?,
        s15f16_at(body, 16)?,
    ])
}

/// Parses a `chad` tag (`sf32`, nine s15Fixed16 values, row-major).
pub(crate) fn chad(body: &[u8]) -> Result<Mat3, IccError> {
    if sig_at(body, 0)? != *b"sf32" {
        return Err(IccError::BadSignature);
    }
    let mut m = [[0.0_f32; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = s15f16_at(body, 8 + (i * 3 + j) * 4)?;
        }
    }
    Ok(m)
}

/// Length in bytes of a `curv` or `para` element starting at `body[0]`, before padding.
pub(crate) fn curve_len(body: &[u8]) -> Result<usize, IccError> {
    match sig_at(body, 0)? {
        sig if sig == *b"curv" => {
            let count = u32_at(body, 8)? as usize;
            Ok(12 + count.checked_mul(2).ok_or(IccError::BadTag)?)
        }
        sig if sig == *b"para" => {
            let kind = usize::from(u16_at(body, 8)?);
            let count = *PARAM_COUNTS.get(kind).ok_or(IccError::Unsupported)?;
            Ok(12 + count * 4)
        }
        _ => Err(IccError::BadSignature),
    }
}

/// Parses a `curv` or `para` tag into a [`Trc`].
pub(crate) fn trc(body: &[u8]) -> Result<Trc, IccError> {
    match sig_at(body, 0)? {
        sig if sig == *b"curv" => curv(body),
        sig if sig == *b"para" => para(body),
        _ => Err(IccError::BadSignature),
    }
}

fn curv(body: &[u8]) -> Result<Trc, IccError> {
    let count = u32_at(body, 8)? as usize;
    match count {
        0 => Ok(Trc::Identity),
        1 => {
            // A single entry is a gamma in u8Fixed8 format.
            let g = f32::from(u16_at(body, 12)?) / 256.0;
            if g == 0.0 {
                return Err(IccError::BadTag);
            }
            Ok(Trc::Gamma(g))
        }
        n => {
            if n > (body.len().saturating_sub(12)) / 2 {
                return Err(IccError::BadTag);
            }
            let mut samples = Vec::with_capacity(n);
            for i in 0..n {
                samples.push(f32::from(u16_at(body, 12 + i * 2)?) / 65_535.0);
            }
            Ok(Trc::Table(samples))
        }
    }
}

fn para(body: &[u8]) -> Result<Trc, IccError> {
    let function = u16_at(body, 8)?;
    let kind = usize::from(function);
    let count = *PARAM_COUNTS.get(kind).ok_or(IccError::Unsupported)?;
    let mut params = [0.0_f32; 7];
    for (i, p) in params.iter_mut().take(count).enumerate() {
        *p = s15f16_at(body, 12 + i * 4)?;
    }
    let [g, a, _, c, ..] = params;
    // Reject parameters that would make the curve undefined (division by zero, zero gamma).
    let degenerate = g == 0.0 || (kind >= 1 && a == 0.0) || (kind >= 3 && c == 0.0);
    if degenerate {
        return Err(IccError::BadTag);
    }
    Ok(Trc::Parametric { function, params })
}

/// Extracts a text description from `desc`, `text` or `mluc` (first record).
///
/// Returns `None` if the tag is malformed; a description is optional metadata.
pub(crate) fn text_description(body: &[u8]) -> Option<String> {
    match body.get(0..4)? {
        b"desc" => {
            let count = u32_at(body, 8).ok()? as usize;
            let raw = slice(body, 12, count).ok()?;
            Some(ascii(raw))
        }
        b"text" => Some(ascii(body.get(8..)?)),
        b"mluc" => {
            let len = u32_at(body, 20).ok()? as usize;
            let off = u32_at(body, 24).ok()? as usize;
            let raw = slice(body, off, len).ok()?;
            let units = raw
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]));
            Some(
                char::decode_utf16(units)
                    .filter_map(Result::ok)
                    .collect::<String>()
                    .trim_end_matches('\0')
                    .into(),
            )
        }
        _ => None,
    }
}

fn ascii(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}
