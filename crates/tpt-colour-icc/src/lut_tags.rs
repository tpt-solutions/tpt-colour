//! Parsers for LUT tags: `mft1` (lut8Type), `mft2` (lut16Type), `mAB ` and `mBA `.
//!
//! Every offset and length is checked against the tag body before reading. Table and CLUT
//! sizes are validated before allocation.

use alloc::vec;
use alloc::vec::Vec;

use crate::bytes::{s15f16_at, sig_at, slice, u16_at, u32_at};
use crate::error::IccError;
use crate::lut::{Clut, Pipeline, Stage};
use crate::matrix::Mat3;
use crate::pcs::LabEncoding;
use crate::tag;
use crate::trc::Trc;

/// Which way a LUT maps: device → PCS (`A2B`) or PCS → device (`B2A`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Device to PCS.
    AtoB,
    /// PCS to device.
    BtoA,
}

/// Parses a LUT tag, dispatching on its type signature.
pub(crate) fn pipeline(body: &[u8], direction: Direction) -> Result<Pipeline, IccError> {
    match sig_at(body, 0)? {
        sig if sig == *b"mft1" => lut8(body),
        sig if sig == *b"mft2" => lut16(body),
        sig if sig == *b"mAB " && direction == Direction::AtoB => lut_ab(body, direction),
        sig if sig == *b"mBA " && direction == Direction::BtoA => lut_ab(body, direction),
        // `mpet` (multiProcessElements) is out of scope for v1.0.
        _ => Err(IccError::Unsupported),
    }
}

/// `lut8Type` (`mft1`): 8-bit curves and CLUT over 256-entry tables.
///
/// The 3×3 matrix is ignored. It only applies to XYZ-encoded data, which this crate does not use.
fn lut8(body: &[u8]) -> Result<Pipeline, IccError> {
    let input = usize::from(body.get(8).copied().ok_or(IccError::Truncated)?);
    let output = usize::from(body.get(9).copied().ok_or(IccError::Truncated)?);
    let grid = usize::from(body.get(10).copied().ok_or(IccError::Truncated)?);
    let grid_points = vec![grid; input];
    let samples = Clut::sample_count(&grid_points, output).map_err(|_| IccError::BadTag)?;

    let mut pos = 48;
    let mut stages = Vec::with_capacity(4);
    stages.push(Stage::Curves(read_u8_tables(body, &mut pos, input)?));
    let raw = slice(body, pos, samples)?;
    pos += samples;
    let values = raw.iter().map(|&b| f32::from(b) / 255.0).collect();
    stages.push(Stage::Clut(Clut::new(grid_points, output, values)?));
    stages.push(Stage::Curves(read_u8_tables(body, &mut pos, output)?));
    // 8-bit Lab maps exactly onto the v4 normalised form (L = v/255·100, a = v − 128).
    Pipeline::new(input, stages, LabEncoding::V4)
}

fn read_u8_tables(body: &[u8], pos: &mut usize, count: usize) -> Result<Vec<Trc>, IccError> {
    let mut curves = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = slice(body, *pos, 256)?;
        curves.push(Trc::sampled(
            raw.iter().map(|&b| f32::from(b) / 255.0).collect(),
        ));
        *pos += 256;
    }
    Ok(curves)
}

/// `lut16Type` (`mft2`): 16-bit curves and CLUT, with entry counts stored in the header.
fn lut16(body: &[u8]) -> Result<Pipeline, IccError> {
    let input = usize::from(body.get(8).copied().ok_or(IccError::Truncated)?);
    let output = usize::from(body.get(9).copied().ok_or(IccError::Truncated)?);
    let grid = usize::from(body.get(10).copied().ok_or(IccError::Truncated)?);
    let in_entries = usize::from(u16_at(body, 48)?);
    let out_entries = usize::from(u16_at(body, 50)?);
    if in_entries < 2 || out_entries < 2 {
        return Err(IccError::BadTag);
    }
    let grid_points = vec![grid; input];
    let samples = Clut::sample_count(&grid_points, output).map_err(|_| IccError::BadTag)?;

    let mut pos = 52;
    let mut stages = Vec::with_capacity(4);
    stages.push(Stage::Curves(read_u16_tables(
        body, &mut pos, input, in_entries,
    )?));
    let raw = slice(body, pos, samples * 2)?;
    pos += samples * 2;
    let values = raw
        .chunks_exact(2)
        .map(|c| f32::from(u16::from_be_bytes([c[0], c[1]])) / 65_535.0)
        .collect();
    stages.push(Stage::Clut(Clut::new(grid_points, output, values)?));
    stages.push(Stage::Curves(read_u16_tables(
        body,
        &mut pos,
        output,
        out_entries,
    )?));
    Pipeline::new(input, stages, LabEncoding::V2)
}

fn read_u16_tables(
    body: &[u8],
    pos: &mut usize,
    count: usize,
    entries: usize,
) -> Result<Vec<Trc>, IccError> {
    let mut curves = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = slice(body, *pos, entries * 2)?;
        let samples = raw
            .chunks_exact(2)
            .map(|c| f32::from(u16::from_be_bytes([c[0], c[1]])) / 65_535.0)
            .collect();
        curves.push(Trc::sampled(samples));
        *pos += entries * 2;
    }
    Ok(curves)
}

/// `lutAToBType` (`mAB `) and `lutBToAType` (`mBA `): flexible element chains.
///
/// Element order is fixed by the spec. For `A2B`: A curves, CLUT, M curves, matrix, B curves.
/// For `B2A`: B curves, matrix, M curves, CLUT, A curves.
fn lut_ab(body: &[u8], direction: Direction) -> Result<Pipeline, IccError> {
    let input = usize::from(body.get(8).copied().ok_or(IccError::Truncated)?);
    let output = usize::from(body.get(9).copied().ok_or(IccError::Truncated)?);
    let offset = |at| -> Result<usize, IccError> { Ok(u32_at(body, at)? as usize) };
    let (off_b, off_mat, off_mc, off_clut, off_a) = (
        offset(12)?,
        offset(16)?,
        offset(20)?,
        offset(24)?,
        offset(28)?,
    );

    let stages: Vec<Option<Stage>> = match direction {
        Direction::AtoB => vec![
            curves_at(body, off_a, input)?,
            clut_at(body, off_clut, input, output)?,
            curves_at(body, off_mc, output)?,
            matrix_at(body, off_mat)?,
            curves_at(body, off_b, output)?,
        ],
        Direction::BtoA => vec![
            curves_at(body, off_b, input)?,
            matrix_at(body, off_mat)?,
            curves_at(body, off_mc, input)?,
            clut_at(body, off_clut, input, output)?,
            curves_at(body, off_a, output)?,
        ],
    };
    let stages: Vec<Stage> = stages.into_iter().flatten().collect();
    Pipeline::new(input, stages, LabEncoding::V4)
}

/// Reads a run of curves starting at `offset`. Offset zero means the element is absent.
fn curves_at(body: &[u8], offset: usize, count: usize) -> Result<Option<Stage>, IccError> {
    if offset == 0 {
        return Ok(None);
    }
    let mut pos = offset;
    let mut curves = Vec::with_capacity(count);
    for _ in 0..count {
        let rest = body.get(pos..).ok_or(IccError::BadTag)?;
        let len = tag::curve_len(rest)?;
        curves.push(tag::trc(slice(rest, 0, len)?)?);
        // Each curve is padded to a four-byte boundary.
        pos += len.next_multiple_of(4);
    }
    Ok(Some(Stage::Curves(curves)))
}

/// Reads a CLUT element: a grid per input, a precision byte, then the samples.
fn clut_at(
    body: &[u8],
    offset: usize,
    input: usize,
    output: usize,
) -> Result<Option<Stage>, IccError> {
    if offset == 0 {
        return Ok(None);
    }
    let header = slice(body, offset, 20)?;
    let grid: Vec<usize> = header[..input.min(16)]
        .iter()
        .map(|&g| usize::from(g))
        .collect();
    if grid.len() != input {
        return Err(IccError::BadTag);
    }
    let precision = header[16];
    let samples = Clut::sample_count(&grid, output)?;
    let data = offset + 20;
    let values = match precision {
        1 => slice(body, data, samples)?
            .iter()
            .map(|&b| f32::from(b) / 255.0)
            .collect(),
        2 => slice(body, data, samples * 2)?
            .chunks_exact(2)
            .map(|c| f32::from(u16::from_be_bytes([c[0], c[1]])) / 65_535.0)
            .collect(),
        _ => return Err(IccError::BadTag),
    };
    Ok(Some(Stage::Clut(Clut::new(grid, output, values)?)))
}

/// Reads a 3×4 matrix element: nine coefficients row-major, then three offsets.
fn matrix_at(body: &[u8], offset: usize) -> Result<Option<Stage>, IccError> {
    if offset == 0 {
        return Ok(None);
    }
    let mut e = [0.0_f32; 12];
    for (i, v) in e.iter_mut().enumerate() {
        *v = s15f16_at(body, offset + i * 4)?;
    }
    let m: Mat3 = [[e[0], e[1], e[2]], [e[3], e[4], e[5]], [e[6], e[7], e[8]]];
    Ok(Some(Stage::Matrix {
        m,
        offset: [e[9], e[10], e[11]],
    }))
}
