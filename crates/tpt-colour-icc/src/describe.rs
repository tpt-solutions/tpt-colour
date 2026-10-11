//! A description of a transform's stage graph, for debugging and for tools that explain what a
//! profile does.

use alloc::vec;
use alloc::vec::Vec;

use crate::lut::{Pipeline, Stage};
use crate::profile::ProfileConnectionSpace;
use crate::transform::{Backward, ColourTransform, Forward};

/// One step of a transform, in the order the data passes through it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Step {
    /// Tone curves, one per channel. Forward it is the TRC, backward the inverse TRC.
    Curves {
        /// Number of channels the curves act on.
        channels: usize,
    },
    /// A single grey TRC, giving `Y` (device to PCS) or the device value (PCS to device).
    GreyTrc,
    /// A 3×3 colorant matrix, with an offset, applied to three channels.
    Matrix,
    /// An n-dimensional lookup table.
    Clut {
        /// Grid points along each input axis.
        grid: Vec<usize>,
        /// Number of output channels.
        outputs: usize,
    },
    /// The connection to the profile connection space (PCS), in the given encoding.
    Pcs(ProfileConnectionSpace),
    /// Black point compensation between the source and destination black points.
    BlackPointCompensation,
    /// Absolute colorimetric scaling by the ratio of the two media whites.
    AbsoluteScaling,
}

impl ColourTransform {
    /// The stage graph of this transform, in application order: the source device to the PCS,
    /// the optional black point and absolute scaling steps, then the PCS to the destination
    /// device.
    ///
    /// Each step is one operation the transform really runs. The fast matrix path and the
    /// baked-table path both appear as the steps they stand for, so the list matches what the
    /// transform computes. Single-colour `transform_rgb` uses the same stages in f64.
    ///
    /// # Example
    ///
    /// ```
    /// use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent, Step};
    /// let t = ColourTransform::new(
    ///     &IccProfile::srgb(),
    ///     &IccProfile::display_p3(),
    ///     RenderingIntent::RelativeColorimetric,
    /// )
    /// .unwrap();
    /// let steps = t.describe();
    /// assert!(matches!(steps.first(), Some(Step::Curves { channels: 3 })));
    /// assert!(steps.iter().any(|s| matches!(s, Step::Pcs(_))));
    /// ```
    #[must_use]
    pub fn describe(&self) -> Vec<Step> {
        let mut steps = forward_steps(&self.forward);
        steps.push(Step::Pcs(forward_pcs(&self.forward)));
        if self.black.is_some() {
            steps.push(Step::BlackPointCompensation);
        }
        if self.abs_scale.is_some() {
            steps.push(Step::AbsoluteScaling);
        }
        steps.extend(backward_steps(&self.backward));
        steps
    }
}

fn forward_pcs(forward: &Forward) -> ProfileConnectionSpace {
    match forward {
        Forward::Lut { pcs, .. } => *pcs,
        Forward::Matrix { .. } | Forward::Gray { .. } => ProfileConnectionSpace::Xyz,
    }
}

fn forward_steps(forward: &Forward) -> Vec<Step> {
    match forward {
        Forward::Matrix { .. } => vec![Step::Curves { channels: 3 }, Step::Matrix],
        Forward::Gray { .. } => vec![Step::GreyTrc],
        Forward::Lut { pipeline, .. } => pipeline_steps(pipeline),
    }
}

fn backward_steps(backward: &Backward) -> Vec<Step> {
    match backward {
        Backward::Matrix { .. } => vec![Step::Matrix, Step::Curves { channels: 3 }],
        Backward::Gray { .. } => vec![Step::GreyTrc],
        Backward::Lut { pipeline, .. } => pipeline_steps(pipeline),
    }
}

fn pipeline_steps(pipeline: &Pipeline) -> Vec<Step> {
    pipeline
        .stages()
        .iter()
        .map(|stage| match stage {
            Stage::Curves(curves) => Step::Curves {
                channels: curves.len(),
            },
            Stage::Matrix { .. } => Step::Matrix,
            Stage::Clut(clut) => Step::Clut {
                grid: clut.grid().to_vec(),
                outputs: clut.outputs(),
            },
        })
        .collect()
}
