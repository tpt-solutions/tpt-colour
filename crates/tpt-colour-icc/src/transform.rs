//! Colour transforms between ICC profiles, routed through the D50 profile connection space.
//!
//! Each side is either a matrix/TRC model or a LUT pipeline. The source maps device values
//! to XYZ (D50). The destination maps XYZ back to device values. Optional black point
//! compensation sits between the two.

use tpt_colour_core::space::D50;

use crate::error::IccError;
use crate::lut::{MAX_CHANNELS, Pipeline};
use crate::matrix::{Mat3, inverse, mul_vec};
use crate::pcs;
use crate::profile::{IccProfile, ProfileConnectionSpace, ProfileModel, RenderingIntent};
use crate::trc::Trc;

/// Device → XYZ (D50).
#[derive(Clone, Debug, PartialEq)]
enum Forward {
    Matrix {
        colorants: Mat3,
        trc: [Trc; 3],
    },
    Lut {
        pipeline: Pipeline,
        pcs: ProfileConnectionSpace,
    },
}

/// XYZ (D50) → device.
#[derive(Clone, Debug, PartialEq)]
enum Backward {
    Matrix {
        inverse: Mat3,
        trc: [Trc; 3],
    },
    Lut {
        pipeline: Pipeline,
        pcs: ProfileConnectionSpace,
    },
}

impl Forward {
    fn channels(&self) -> usize {
        match self {
            Self::Matrix { .. } => 3,
            Self::Lut { pipeline, .. } => pipeline.input_channels(),
        }
    }

    fn to_xyz(&self, device: &[f32]) -> Result<[f32; 3], IccError> {
        match self {
            Self::Matrix { colorants, trc } => {
                let lin = [
                    trc[0].eval(device[0]),
                    trc[1].eval(device[1]),
                    trc[2].eval(device[2]),
                ];
                Ok(mul_vec(colorants, lin))
            }
            Self::Lut { pipeline, pcs } => {
                let mut n = [0.0_f32; 3];
                pipeline.eval(device, &mut n)?;
                Ok(pcs::decode(*pcs, pipeline.encoding(), n))
            }
        }
    }
}

#[allow(clippy::wrong_self_convention)] // `from_xyz` mirrors the colour-space method name.
impl Backward {
    fn channels(&self) -> usize {
        match self {
            Self::Matrix { .. } => 3,
            Self::Lut { pipeline, .. } => pipeline.output_channels(),
        }
    }

    fn from_xyz(&self, xyz: [f32; 3], device: &mut [f32]) -> Result<(), IccError> {
        match self {
            Self::Matrix { inverse, trc } => {
                let lin = mul_vec(inverse, xyz);
                for ((out, t), v) in device.iter_mut().zip(trc).zip(lin) {
                    *out = t.eval_inverse(v);
                }
                Ok(())
            }
            Self::Lut { pipeline, pcs } => {
                let n = pcs::encode(*pcs, pipeline.encoding(), xyz);
                pipeline.eval(&n, device)
            }
        }
    }
}

/// Black points (XYZ, D50) of the source and destination, for black point compensation.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BlackPoints {
    src: [f32; 3],
    dst: [f32; 3],
}

/// A precomputed transform from one ICC profile to another.
///
/// Build it once and reuse it. Each call is then a few table lookups and a small amount of
/// arithmetic per channel.
#[derive(Clone, Debug, PartialEq)]
pub struct ColourTransform {
    forward: Forward,
    backward: Backward,
    black: Option<BlackPoints>,
    intent: RenderingIntent,
}

impl ColourTransform {
    /// Builds a transform from `src` to `dst` for `intent`, without black point compensation.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::Unsupported`] if a profile has no table for `intent`, or
    /// [`IccError::Singular`] if a matrix cannot be inverted.
    pub fn new(
        src: &IccProfile,
        dst: &IccProfile,
        intent: RenderingIntent,
    ) -> Result<Self, IccError> {
        Ok(Self {
            forward: forward(src, intent)?,
            backward: backward(dst, intent)?,
            black: None,
            intent,
        })
    }

    /// Builds a transform with black point compensation.
    ///
    /// Compensation maps the source black point onto the destination black point, keeping
    /// the white point fixed. It applies to perceptual and relative intents. Absolute
    /// colorimetric transforms are returned without it.
    ///
    /// # Errors
    ///
    /// As for [`ColourTransform::new`].
    pub fn with_black_point_compensation(
        src: &IccProfile,
        dst: &IccProfile,
        intent: RenderingIntent,
    ) -> Result<Self, IccError> {
        let mut t = Self::new(src, dst, intent)?;
        if intent != RenderingIntent::AbsoluteColorimetric {
            let src_black = black_point(&t.forward)?;
            let dst_black = black_point(&forward(dst, intent)?)?;
            t.black = Some(BlackPoints {
                src: src_black,
                dst: dst_black,
            });
        }
        Ok(t)
    }

    /// The rendering intent this transform was built with.
    #[must_use]
    pub fn intent(&self) -> RenderingIntent {
        self.intent
    }

    /// Number of source device channels.
    #[must_use]
    pub fn src_channels(&self) -> usize {
        self.forward.channels()
    }

    /// Number of destination device channels.
    #[must_use]
    pub fn dst_channels(&self) -> usize {
        self.backward.channels()
    }

    /// Transforms one device colour. Inputs and outputs are normalised to `[0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if either slice has the wrong length.
    pub fn transform(&self, src: &[f32], dst: &mut [f32]) -> Result<(), IccError> {
        if src.len() != self.src_channels() || dst.len() != self.dst_channels() {
            return Err(IccError::ChannelCount);
        }
        let xyz = self.device_to_pcs(src)?;
        self.pcs_to_device(xyz, dst)
    }

    /// Converts a source device colour to XYZ (D50), including black point compensation if enabled.
    ///
    /// Useful for inserting a step, such as gamut mapping, between the two halves of the transform.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if `src` has the wrong length.
    pub fn device_to_pcs(&self, src: &[f32]) -> Result<[f32; 3], IccError> {
        if src.len() != self.src_channels() {
            return Err(IccError::ChannelCount);
        }
        let xyz = self.forward.to_xyz(src)?;
        Ok(match &self.black {
            Some(black) => compensate(xyz, black),
            None => xyz,
        })
    }

    /// Converts XYZ (D50) to destination device values.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if `dst` has the wrong length.
    pub fn pcs_to_device(&self, xyz: [f32; 3], dst: &mut [f32]) -> Result<(), IccError> {
        if dst.len() != self.dst_channels() {
            return Err(IccError::ChannelCount);
        }
        self.backward.from_xyz(xyz, dst)
    }

    /// Transforms one RGB triple.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] unless both profiles are three-channel.
    pub fn transform_rgb(&self, rgb: [f32; 3]) -> Result<[f32; 3], IccError> {
        let mut out = [0.0_f32; 3];
        self.transform(&rgb, &mut out)?;
        Ok(out)
    }

    /// Transforms RGBA-sized pixels in place, reading the first `src_channels` components and
    /// writing the first `dst_channels`. Components beyond the destination are left unchanged,
    /// so the fourth component stays as alpha for RGB output.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if either side has more than four channels.
    pub fn transform_pixels(&self, pixels: &mut [[f32; 4]]) -> Result<(), IccError> {
        let (src_n, dst_n) = (self.src_channels(), self.dst_channels());
        if src_n > 4 || dst_n > 4 {
            return Err(IccError::ChannelCount);
        }
        for px in pixels {
            let mut input = [0.0_f32; 4];
            input[..src_n].copy_from_slice(&px[..src_n]);
            let mut output = [0.0_f32; 4];
            self.transform(&input[..src_n], &mut output[..dst_n])?;
            px[..dst_n].copy_from_slice(&output[..dst_n]);
        }
        Ok(())
    }
}

/// Source-side pipeline or matrix for `intent`.
fn forward(profile: &IccProfile, intent: RenderingIntent) -> Result<Forward, IccError> {
    match &profile.model {
        ProfileModel::Matrix(m) => Ok(Forward::Matrix {
            colorants: m.colorants,
            trc: m.trc.clone(),
        }),
        ProfileModel::Lut(tables) => {
            let pipeline = tables.a2b(intent).ok_or(IccError::Unsupported)?.clone();
            if pipeline.output_channels() != 3
                || pipeline.input_channels() != profile.device_channels
            {
                return Err(IccError::Unsupported);
            }
            Ok(Forward::Lut {
                pipeline,
                pcs: profile.pcs,
            })
        }
    }
}

/// Destination-side pipeline or inverse matrix for `intent`.
fn backward(profile: &IccProfile, intent: RenderingIntent) -> Result<Backward, IccError> {
    match &profile.model {
        ProfileModel::Matrix(m) => Ok(Backward::Matrix {
            inverse: inverse(&m.colorants)?,
            trc: m.trc.clone(),
        }),
        ProfileModel::Lut(tables) => {
            let pipeline = tables.b2a(intent).ok_or(IccError::Unsupported)?.clone();
            if pipeline.input_channels() != 3
                || pipeline.output_channels() != profile.device_channels
            {
                return Err(IccError::Unsupported);
            }
            Ok(Backward::Lut {
                pipeline,
                pcs: profile.pcs,
            })
        }
    }
}

/// The black point of a source: its device black (all zero, or K = 1 for CMYK) mapped to XYZ.
fn black_point(forward: &Forward) -> Result<[f32; 3], IccError> {
    let channels = forward.channels();
    let mut device = [0.0_f32; MAX_CHANNELS];
    if channels == 4 {
        device[3] = 1.0;
    }
    forward.to_xyz(&device[..channels])
}

/// Linear black point compensation per XYZ channel, anchored on the D50 white point.
fn compensate(xyz: [f32; 3], black: &BlackPoints) -> [f32; 3] {
    let mut out = xyz;
    for c in 0..3 {
        let span = D50[c] - black.src[c];
        if span.abs() > 1e-6 {
            out[c] = (xyz[c] - black.src[c]) * (D50[c] - black.dst[c]) / span + black.dst[c];
        }
    }
    out
}
