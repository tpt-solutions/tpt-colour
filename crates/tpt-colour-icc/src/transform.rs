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
                // `trc` holds baked inverse curves, so `eval` gives the encoded value.
                for ((out, t), v) in device.iter_mut().zip(trc).zip(lin) {
                    *out = t.eval(v);
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
    exact: Option<ExactMatrix>,
}

/// Double-precision matrix/TRC path, used by [`ColourTransform::transform_rgb`].
///
/// It keeps the profiles' own curves, not the baked tables, so a single colour is evaluated
/// to f64 precision.
#[derive(Clone, Debug, PartialEq)]
struct ExactMatrix {
    src: [[f64; 3]; 3],
    src_trc: [Trc; 3],
    dst_inv: [[f64; 3]; 3],
    dst_trc: [Trc; 3],
}

impl ExactMatrix {
    fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let lin = [0, 1, 2].map(|c| self.src_trc[c].eval64(f64::from(rgb[c])));
        let xyz = mul64(&self.src, lin);
        let dst_lin = mul64(&self.dst_inv, xyz);
        [0, 1, 2].map(|c| self.dst_trc[c].eval_inverse64(dst_lin[c]) as f32)
    }
}

fn mul64(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

/// Builds the f64 path when both profiles are matrix/TRC.
fn exact_matrix(src: &IccProfile, dst: &IccProfile) -> Option<ExactMatrix> {
    let (ProfileModel::Matrix(s), ProfileModel::Matrix(d)) = (&src.model, &dst.model) else {
        return None;
    };
    let dst_m = d.colorants.map(|r| r.map(f64::from));
    let dst_inv = invert64(&dst_m)?;
    Some(ExactMatrix {
        src: s.colorants.map(|r| r.map(f64::from)),
        src_trc: s.trc.clone(),
        dst_inv,
        dst_trc: d.trc.clone(),
    })
}

#[allow(clippy::many_single_char_names)] // Cofactor expansion reads most naturally with a..i.
fn invert64(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-12 {
        return None;
    }
    let k = 1.0 / det;
    Some([
        [
            (e * i - f * h) * k,
            (c * h - b * i) * k,
            (b * f - c * e) * k,
        ],
        [
            (f * g - d * i) * k,
            (a * i - c * g) * k,
            (c * d - a * f) * k,
        ],
        [
            (d * h - e * g) * k,
            (b * g - a * h) * k,
            (a * e - b * d) * k,
        ],
    ])
}

impl ColourTransform {
    /// Builds a transform from `src` to `dst` for `intent`.
    ///
    /// Perceptual transforms from an ICC v4 source apply black point compensation implicitly,
    /// as the v4 specification and `LittleCMS` do. Everything else uses no compensation.
    /// Use [`ColourTransform::with_black_point_compensation`] to force it for other cases.
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
        let implicit = intent == RenderingIntent::Perceptual && src.version.major >= 4;
        Self::build(src, dst, intent, implicit)
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
        Self::build(src, dst, intent, true)
    }

    fn build(
        src: &IccProfile,
        dst: &IccProfile,
        intent: RenderingIntent,
        compensate: bool,
    ) -> Result<Self, IccError> {
        let mut t = Self {
            forward: forward(src, intent)?,
            backward: backward(dst, intent)?,
            black: None,
            intent,
            exact: exact_matrix(src, dst),
        };
        if compensate && intent != RenderingIntent::AbsoluteColorimetric {
            let src_black = black_point(&t.forward)?;
            let dst_black = black_point(&forward(dst, intent)?)?;
            t.black = Some(BlackPoints {
                src: src_black,
                dst: dst_black,
            });
            // Black point compensation stays in f32, so the exact path no longer applies.
            t.exact = None;
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
    /// For matrix/TRC profiles without black point compensation, this is the single-colour
    /// path: it evaluates the profiles' exact curves in f64. Other profiles use the f32 path.
    /// The two agree to about 2e-5, the error of the baked tables used by [`transform`](Self::transform).
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] unless both profiles are three-channel.
    pub fn transform_rgb(&self, rgb: [f32; 3]) -> Result<[f32; 3], IccError> {
        if let Some(exact) = &self.exact {
            if self.src_channels() == 3 && self.dst_channels() == 3 {
                return Ok(exact.apply(rgb));
            }
        }
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
        #[cfg(feature = "simd")]
        if let Some(parts) = self.matrix_parts() {
            transform_pixels_matrix(parts, pixels);
            return Ok(());
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

#[cfg(feature = "simd")]
impl ColourTransform {
    /// Matrix/TRC parts of a matrix-to-matrix transform without black point compensation.
    fn matrix_parts(&self) -> Option<MatrixParts<'_>> {
        match (&self.forward, &self.backward, &self.black) {
            (
                Forward::Matrix {
                    colorants,
                    trc: src_trc,
                },
                Backward::Matrix {
                    inverse,
                    trc: dst_trc,
                },
                None,
            ) => Some(MatrixParts {
                colorants,
                src_trc,
                inverse,
                dst_trc,
            }),
            _ => None,
        }
    }
}

#[cfg(feature = "simd")]
/// Batched transform for matrix/TRC pairs: eight pixels per matrix product.
///
/// Remainder pixels go through the scalar path, so the result is identical either way.
fn transform_pixels_matrix(p: MatrixParts<'_>, pixels: &mut [[f32; 4]]) {
    use crate::batch::{LANES, Lanes, mat_lanes};

    let mut chunks = pixels.chunks_exact_mut(LANES);
    for chunk in &mut chunks {
        let mut lin = [Lanes::splat(0.0); 3];
        for (c, lane) in lin.iter_mut().enumerate() {
            *lane = Lanes::from_fn(|i| p.src_trc[c].eval(chunk[i][c]));
        }
        let xyz = mat_lanes(p.colorants, lin);
        let dst_lin = mat_lanes(p.inverse, xyz);
        for (c, trc) in p.dst_trc.iter().enumerate() {
            for (i, px) in chunk.iter_mut().enumerate() {
                px[c] = trc.eval(dst_lin[c][i]);
            }
        }
    }
    for px in chunks.into_remainder() {
        let rgb = [
            p.src_trc[0].eval(px[0]),
            p.src_trc[1].eval(px[1]),
            p.src_trc[2].eval(px[2]),
        ];
        let xyz = mul_vec(p.colorants, rgb);
        let lin = mul_vec(p.inverse, xyz);
        for c in 0..3 {
            px[c] = p.dst_trc[c].eval(lin[c]);
        }
    }
}

/// Borrowed matrix/TRC parts of a transform.
#[cfg(feature = "simd")]
#[derive(Clone, Copy)]
struct MatrixParts<'a> {
    colorants: &'a Mat3,
    src_trc: &'a [Trc; 3],
    inverse: &'a Mat3,
    dst_trc: &'a [Trc; 3],
}

/// Source-side pipeline or matrix for `intent`.
fn forward(profile: &IccProfile, intent: RenderingIntent) -> Result<Forward, IccError> {
    match &profile.model {
        ProfileModel::Matrix(m) => Ok(Forward::Matrix {
            colorants: m.colorants,
            trc: m.trc.clone().map(|t| t.bake()),
        }),
        ProfileModel::Lut(tables) => {
            let pipeline = tables.a2b(intent).ok_or(IccError::Unsupported)?.clone();
            if pipeline.output_channels() != 3
                || pipeline.input_channels() != profile.device_channels
            {
                return Err(IccError::Unsupported);
            }
            let pipeline = white_patched(pipeline, profile, intent);
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
            trc: m.trc.clone().map(|t| t.bake_inverse()),
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

/// Applies the `LittleCMS` white-on-white correction to a device → Lab pipeline.
///
/// Like `LittleCMS`, it applies only to Lab-PCS profiles and to non-absolute intents. The device
/// white is `1` for RGB and grey, and `0` for CMYK.
fn white_patched(pipeline: Pipeline, profile: &IccProfile, intent: RenderingIntent) -> Pipeline {
    if profile.pcs != ProfileConnectionSpace::Lab || intent == RenderingIntent::AbsoluteColorimetric
    {
        return pipeline;
    }
    let white_level = if profile.device_channels == 4 {
        0.0
    } else {
        1.0
    };
    let device_white = [white_level; MAX_CHANNELS];
    let pcs_white = pcs::encode(ProfileConnectionSpace::Lab, pipeline.encoding(), D50);
    pipeline
        .with_white_patched(&device_white[..profile.device_channels], pcs_white)
        .unwrap_or(pipeline)
}
