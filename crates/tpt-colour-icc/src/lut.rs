//! LUT pipelines: the stage evaluator behind `mft1`, `mft2`, `mAB ` and `mBA ` tags.
//!
//! A pipeline is a short sequence of curves, matrices and n-dimensional CLUTs that operates
//! on normalised values in `[0, 1]`. Its size is bounded at parse time so that malicious
//! profiles cannot force large allocations or unbounded work.

use alloc::vec::Vec;

use crate::error::IccError;
use crate::matrix::{Mat3, mul_vec};
use crate::pcs::LabEncoding;
use crate::trc::Trc;

/// Largest channel count the ICC format allows.
pub(crate) const MAX_CHANNELS: usize = 15;
/// Upper bound on stages in one pipeline. Real profiles use at most five.
const MAX_STAGES: usize = 8;
/// Upper bound on CLUT samples (grid points × output channels).
pub(crate) const MAX_CLUT_VALUES: usize = 1 << 22;
/// Most CLUT inputs supported. Multilinear lookup touches 2^n corners per pixel, so n is capped
/// to keep the work per pixel bounded. Three inputs use tetrahedral interpolation.
pub(crate) const MAX_CLUT_INPUTS: usize = 8;
/// Upper bound on CLUT samples across all tables in one profile (32 MiB of `f32`).
pub(crate) const MAX_PROFILE_CLUT_VALUES: usize = 1 << 23;

/// One step in a pipeline.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Stage {
    /// One curve per channel.
    Curves(Vec<Trc>),
    /// `y = m·x + offset` over three channels.
    Matrix {
        /// Row-major 3×3 matrix.
        m: Mat3,
        /// Additive offset.
        offset: [f32; 3],
    },
    /// n-dimensional lookup table.
    Clut(Clut),
}

/// A validated LUT pipeline on normalised values.
#[derive(Clone, Debug, PartialEq)]
pub struct Pipeline {
    input: usize,
    output: usize,
    stages: Vec<Stage>,
    encoding: LabEncoding,
}

impl Pipeline {
    /// Builds a pipeline, checking that channel counts agree between stages.
    pub(crate) fn new(
        input: usize,
        stages: Vec<Stage>,
        encoding: LabEncoding,
    ) -> Result<Self, IccError> {
        if input == 0 || input > MAX_CHANNELS || stages.len() > MAX_STAGES {
            return Err(IccError::BadTag);
        }
        let mut channels = input;
        for stage in &stages {
            channels = match stage {
                Stage::Curves(curves) if curves.len() == channels => channels,
                Stage::Matrix { .. } if channels == 3 => 3,
                Stage::Clut(clut) if clut.in_ch == channels => clut.out_ch,
                _ => return Err(IccError::BadTag),
            };
        }
        Ok(Self {
            input,
            output: channels,
            stages,
            encoding,
        })
    }

    /// Number of input channels.
    #[must_use]
    pub fn input_channels(&self) -> usize {
        self.input
    }

    /// Number of output channels.
    #[must_use]
    pub fn output_channels(&self) -> usize {
        self.output
    }

    /// Total CLUT samples held by this pipeline, used to enforce the per-profile budget.
    pub(crate) fn clut_values(&self) -> usize {
        self.stages
            .iter()
            .map(|stage| match stage {
                Stage::Clut(clut) => clut.values.len(),
                _ => 0,
            })
            .sum()
    }

    /// Lab encoding used when this pipeline's output is in a Lab PCS.
    pub(crate) fn encoding(&self) -> LabEncoding {
        self.encoding
    }

    /// Evaluates the pipeline on normalised inputs, writing normalised outputs.
    ///
    /// Inputs are clamped to `[0, 1]`, and so are outputs.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if either slice has the wrong length.
    pub fn eval(&self, input: &[f32], output: &mut [f32]) -> Result<(), IccError> {
        if input.len() != self.input || output.len() != self.output {
            return Err(IccError::ChannelCount);
        }
        let mut buf = [0.0_f32; MAX_CHANNELS];
        for (dst, &src) in buf.iter_mut().zip(input) {
            *dst = unit(src);
        }
        for stage in &self.stages {
            match stage {
                Stage::Curves(curves) => {
                    for (v, curve) in buf.iter_mut().zip(curves) {
                        *v = not_nan(curve.eval(*v));
                    }
                }
                Stage::Matrix { m, offset } => {
                    let v = mul_vec(m, [buf[0], buf[1], buf[2]]);
                    buf[0] = not_nan(v[0] + offset[0]);
                    buf[1] = not_nan(v[1] + offset[1]);
                    buf[2] = not_nan(v[2] + offset[2]);
                }
                Stage::Clut(clut) => {
                    // Curve outputs can leave [0, 1]. Clamp them as LittleCMS does, rather than
                    // extrapolating past the grid.
                    for v in &mut buf[..clut.in_ch] {
                        *v = unit(*v);
                    }
                    let mut tmp = [0.0_f32; MAX_CHANNELS];
                    clut.eval(&buf[..clut.in_ch], &mut tmp[..clut.out_ch]);
                    buf[..clut.out_ch].copy_from_slice(&tmp[..clut.out_ch]);
                }
            }
        }
        for (dst, &src) in output.iter_mut().zip(&buf) {
            *dst = unit(src);
        }
        Ok(())
    }
}

impl Pipeline {
    /// Makes device white map exactly to PCS white, as `LittleCMS` does for Lab-PCS LUTs.
    ///
    /// The device white is `1` for RGB and grey and `0` for CMYK, which are the same values
    /// `LittleCMS` uses. If the pipeline maps it to something else, the single CLUT node at that
    /// point is overwritten with the PCS white, after undoing any output curves. Returns `None`
    /// when nothing needs changing or when the pipeline's shape does not allow a patch.
    pub(crate) fn with_white_patched(
        &self,
        device_white: &[f32],
        pcs_white: [f32; 3],
    ) -> Option<Pipeline> {
        let mut obtained = [0.0_f32; 3];
        self.eval(device_white, &mut obtained).ok()?;
        let tolerance = 2.0 / 65_535.0;
        if (0..3).all(|i| (obtained[i] - pcs_white[i]).abs() <= tolerance) {
            return None;
        }
        let k = self
            .stages
            .iter()
            .position(|s| matches!(s, Stage::Clut(_)))?;
        // Input side: only curves, and the device white must land exactly on a grid node.
        let mut coords = [0.0_f32; MAX_CHANNELS];
        coords[..device_white.len()].copy_from_slice(device_white);
        for stage in &self.stages[..k] {
            match stage {
                Stage::Curves(curves) => {
                    for (v, c) in coords.iter_mut().zip(curves) {
                        *v = c.eval(*v);
                    }
                }
                _ => return None,
            }
        }
        let Stage::Clut(clut) = &self.stages[k] else {
            return None;
        };
        let mut index = 0_usize;
        let mut point = [0_usize; MAX_CHANNELS];
        for d in 0..clut.in_ch {
            // Curve outputs are not guaranteed to stay in [0, 1], and the node must be in the grid.
            let pos = coords[d].clamp(0.0, 1.0) * (clut.grid[d] - 1) as f32;
            let node = (pos + 0.5) as usize;
            if node >= clut.grid[d] || (pos - node as f32).abs() > 1e-4 {
                return None;
            }
            point[d] = node;
            index = index * clut.grid[d] + point[d];
        }
        // Output side: only curves, inverted, so the stored node value is the pre-curve value.
        let mut target = pcs_white;
        for stage in &self.stages[k + 1..] {
            match stage {
                Stage::Curves(curves) => {
                    for (v, c) in target.iter_mut().zip(curves) {
                        *v = c.eval_inverse(*v);
                    }
                }
                _ => return None,
            }
        }
        let mut patched = self.clone();
        let Stage::Clut(clut) = &mut patched.stages[k] else {
            return None;
        };
        let start = index * clut.out_ch;
        clut.values
            .get_mut(start..start + 3)?
            .copy_from_slice(&target);
        Some(patched)
    }
}

/// Clamps to `[0, 1]`, with NaN mapped to 0 so it cannot leak into the output.
///
/// `f32::clamp` passes NaN through, which would otherwise reach the caller.
fn unit(v: f32) -> f32 {
    not_nan(v).clamp(0.0, 1.0)
}

/// Replaces NaN with 0 and leaves every other value unchanged.
fn not_nan(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v }
}

/// An n-dimensional lookup table. The first input varies slowest, and outputs are contiguous.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Clut {
    grid: Vec<usize>,
    in_ch: usize,
    out_ch: usize,
    values: Vec<f32>,
}

impl Clut {
    /// Number of samples a CLUT with this grid and output count holds.
    ///
    /// Malformed grids (empty, a side under two points) are `BadTag`. Valid grids beyond the
    /// input or sample limits are `Unsupported`, so callers can tell the two apart.
    ///
    /// Checked before any table data is read, so a hostile grid size cannot cause a large
    /// allocation or an overflow.
    pub(crate) fn sample_count(grid: &[usize], out_ch: usize) -> Result<usize, IccError> {
        if grid.is_empty() || grid.len() > MAX_CHANNELS || out_ch == 0 || out_ch > MAX_CHANNELS {
            return Err(IccError::BadTag);
        }
        if grid.len() > MAX_CLUT_INPUTS {
            return Err(IccError::Unsupported);
        }
        let mut points: usize = 1;
        for &g in grid {
            if g < 2 {
                return Err(IccError::BadTag);
            }
            points = points.checked_mul(g).ok_or(IccError::BadTag)?;
        }
        let total = points.checked_mul(out_ch).ok_or(IccError::BadTag)?;
        if total > MAX_CLUT_VALUES {
            return Err(IccError::Unsupported);
        }
        Ok(total)
    }

    /// Builds a CLUT from its grid, output count and samples.
    pub(crate) fn new(grid: Vec<usize>, out_ch: usize, values: Vec<f32>) -> Result<Self, IccError> {
        if Self::sample_count(&grid, out_ch)? != values.len() {
            return Err(IccError::BadTag);
        }
        Ok(Self {
            in_ch: grid.len(),
            grid,
            out_ch,
            values,
        })
    }

    /// Interpolates at `input` (normalised, clamped) into `out`.
    ///
    /// Three inputs use tetrahedral interpolation. Other sizes use multilinear interpolation.
    fn eval(&self, input: &[f32], out: &mut [f32]) {
        for v in out.iter_mut() {
            *v = 0.0;
        }
        if self.in_ch == 3 {
            self.tetrahedral(input, out);
        } else {
            self.multilinear(input, out);
        }
    }

    /// Splits each input into a base grid index and a fractional offset.
    fn locate(&self, input: &[f32]) -> ([usize; MAX_CHANNELS], [f32; MAX_CHANNELS]) {
        let mut base = [0_usize; MAX_CHANNELS];
        let mut frac = [0.0_f32; MAX_CHANNELS];
        for d in 0..self.in_ch {
            let g = self.grid[d];
            let pos = input[d] * (g - 1) as f32;
            let b = (pos as usize).min(g - 2);
            base[d] = b;
            frac[d] = pos - b as f32;
        }
        (base, frac)
    }

    /// Row-major index of a grid point, with the first dimension slowest.
    fn index(&self, coords: &[usize]) -> usize {
        coords
            .iter()
            .zip(&self.grid)
            .fold(0, |acc, (&c, &g)| acc * g + c)
    }

    /// Adds `weight` times the output vector at grid point `idx` into `out`.
    fn accumulate(&self, idx: usize, weight: f32, out: &mut [f32]) {
        let start = idx * self.out_ch;
        for (o, &v) in out.iter_mut().zip(&self.values[start..start + self.out_ch]) {
            *o += weight * v;
        }
    }

    fn multilinear(&self, input: &[f32], out: &mut [f32]) {
        let (base, frac) = self.locate(input);
        let n = self.in_ch;
        let mut coords = [0_usize; MAX_CHANNELS];
        for mask in 0..(1_usize << n) {
            let mut weight = 1.0_f32;
            for d in 0..n {
                let upper = (mask >> d) & 1 == 1;
                weight *= if upper { frac[d] } else { 1.0 - frac[d] };
                coords[d] = base[d] + usize::from(upper);
            }
            if weight != 0.0 {
                self.accumulate(self.index(&coords[..n]), weight, out);
            }
        }
    }

    fn tetrahedral(&self, input: &[f32], out: &mut [f32]) {
        let (base, frac) = self.locate(input);
        // Order the three axes by descending fraction.
        let mut order = [0_usize, 1, 2];
        for i in 0..3 {
            for j in 0..2 - i {
                if frac[order[j]] < frac[order[j + 1]] {
                    order.swap(j, j + 1);
                }
            }
        }
        let [p0, p1, p2] = order;
        let weights = [
            1.0 - frac[p0],
            frac[p0] - frac[p1],
            frac[p1] - frac[p2],
            frac[p2],
        ];
        // The four vertices of the enclosing tetrahedron.
        let mut offsets = [[0_usize; 3]; 4];
        offsets[1][p0] = 1;
        offsets[2][p0] = 1;
        offsets[2][p1] = 1;
        offsets[3] = [1, 1, 1];
        for (off, &w) in offsets.iter().zip(&weights) {
            if w == 0.0 {
                continue;
            }
            let coords = [base[0] + off[0], base[1] + off[1], base[2] + off[2]];
            self.accumulate(self.index(&coords), w, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1-in, 3-out pipeline whose input curve overshoots `[0, 1]` (`y = 1.5x`), so device white
    /// maps to 1.5. Before the clamp, the node index was 3 on a grid of 3 and panicked.
    fn overshooting_pipeline() -> Pipeline {
        let overshoot = Trc::Parametric {
            function: 1,
            params: [1.0, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let stages = alloc::vec![
            Stage::Curves(alloc::vec![overshoot]),
            Stage::Clut(Clut::new(alloc::vec![3], 3, alloc::vec![0.0; 9]).unwrap()),
            Stage::Curves(alloc::vec![Trc::Identity; 3]),
        ];
        Pipeline::new(1, stages, LabEncoding::V4).unwrap()
    }

    #[test]
    fn white_patch_survives_curve_output_outside_unit_range() {
        let pipeline = overshooting_pipeline();
        let pcs_white = [0.5, 0.25, 0.75];
        let patched = pipeline
            .with_white_patched(&[1.0], pcs_white)
            .expect("white is off target, so a patch is needed");
        let mut out = [0.0_f32; 3];
        patched.eval(&[1.0], &mut out).unwrap();
        for (got, want) in out.iter().zip(pcs_white) {
            assert!((got - want).abs() <= 1e-4, "got {got}, want {want}");
        }
    }

    #[test]
    fn white_patch_is_skipped_when_white_already_matches() {
        let pipeline = overshooting_pipeline();
        assert!(
            pipeline
                .with_white_patched(&[1.0], [0.0, 0.0, 0.0])
                .is_none()
        );
    }
}

#[cfg(test)]
mod cap_tests {
    use super::*;

    #[test]
    fn too_many_inputs_is_unsupported_not_malformed() {
        assert_eq!(Clut::sample_count(&[2; 9], 3), Err(IccError::Unsupported));
        assert_eq!(Clut::sample_count(&[2; 8], 3), Ok(256 * 3));
        assert_eq!(Clut::sample_count(&[1, 2], 3), Err(IccError::BadTag));
    }
}
