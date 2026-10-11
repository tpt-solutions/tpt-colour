//! Colour transforms between ICC profiles, routed through the D50 profile connection space.
//!
//! Each side is either a matrix/TRC model or a LUT pipeline. The source maps device values
//! to XYZ (D50). The destination maps XYZ back to device values. Optional black point
//! compensation sits between the two.

use tpt_colour_core::space::D50;

use crate::black;
use crate::error::IccError;
use crate::lut::{Clut, MAX_CHANNELS, Pipeline, Stage};
use crate::matrix::{Mat3, inverse, mul_vec};
use crate::pcs;
use crate::pcs::LabEncoding;
use crate::profile::{IccProfile, ProfileConnectionSpace, ProfileModel, RenderingIntent};
use crate::trc::Trc;

/// Device → XYZ (D50).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Forward {
    Matrix {
        colorants: Mat3,
        trc: [Trc; 3],
    },
    /// Grey: `Y` from the TRC, and XYZ is `D50 · Y`.
    Gray {
        trc: Trc,
    },
    Lut {
        pipeline: Pipeline,
        pcs: ProfileConnectionSpace,
    },
}

/// XYZ (D50) → device.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Backward {
    Matrix {
        inverse: Mat3,
        trc: [Trc; 3],
    },
    /// Grey: the device value is the inverse TRC applied to `Y`. Holds the baked inverse.
    Gray {
        trc: Trc,
    },
    Lut {
        pipeline: Pipeline,
        pcs: ProfileConnectionSpace,
    },
}

impl Forward {
    pub(crate) fn channels(&self) -> usize {
        match self {
            Self::Matrix { .. } => 3,
            Self::Gray { .. } => 1,
            Self::Lut { pipeline, .. } => pipeline.input_channels(),
        }
    }

    pub(crate) fn to_xyz(&self, device: &[f32]) -> Result<[f32; 3], IccError> {
        match self {
            Self::Matrix { colorants, trc } => {
                let lin = [
                    trc[0].eval(device[0]),
                    trc[1].eval(device[1]),
                    trc[2].eval(device[2]),
                ];
                Ok(mul_vec(colorants, lin))
            }
            Self::Gray { trc } => {
                let y = trc.eval(device[0]);
                Ok(D50.map(|w| w * y))
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
    pub(crate) fn channels(&self) -> usize {
        match self {
            Self::Matrix { .. } => 3,
            Self::Gray { .. } => 1,
            Self::Lut { pipeline, .. } => pipeline.output_channels(),
        }
    }

    pub(crate) fn from_xyz(&self, xyz: [f32; 3], device: &mut [f32]) -> Result<(), IccError> {
        match self {
            Self::Matrix { inverse, trc } => {
                let lin = mul_vec(inverse, xyz);
                // `trc` holds baked inverse curves, so `eval` gives the encoded value.
                for ((out, t), v) in device.iter_mut().zip(trc).zip(lin) {
                    *out = t.eval(v);
                }
                Ok(())
            }
            Self::Gray { trc } => {
                // Luminance is the Y of the PCS. Its D50 white is Y = 1 for every profile.
                device[0] = trc.eval(xyz[1]);
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
    /// Per-axis ratio of source to destination media white, for absolute colorimetric only.
    abs_scale: Option<[f32; 3]>,
    intent: RenderingIntent,
    exact: Option<ExactMatrix>,
}

/// Media-white ratio for absolute colorimetric. `None` when the ratio is one on every axis.
fn absolute_scale(src: &IccProfile, dst: &IccProfile) -> Option<[f32; 3]> {
    let ratio = [0, 1, 2].map(|c| {
        let (s, d) = (src.white_point[c], dst.white_point[c]);
        if s.is_finite() && d.is_finite() && d.abs() > 1e-9 {
            s / d
        } else {
            1.0
        }
    });
    let identity = ratio.iter().all(|r| (r - 1.0).abs() <= 1e-6);
    (!identity).then_some(ratio)
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
    if !det.is_finite() || det.abs() < 1e-12 {
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
    /// Perceptual and saturation transforms to an ICC v4 destination apply black point
    /// compensation implicitly, as the v4 specification and `LittleCMS` do. Everything else uses
    /// no compensation. Absolute colorimetric scales XYZ by the ratio of the two media white
    /// points (`wtpt`). Use [`ColourTransform::with_black_point_compensation`] to force
    /// compensation in other cases.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::Unsupported`] if a profile has no table for `intent`, or if it has a
    /// `D2B`/`B2D` (`multiProcessElements`) table for `intent`, which is not supported. Returns
    /// [`IccError::Singular`] if a matrix cannot be inverted.
    pub fn new(
        src: &IccProfile,
        dst: &IccProfile,
        intent: RenderingIntent,
    ) -> Result<Self, IccError> {
        let v4_intent = matches!(
            intent,
            RenderingIntent::Perceptual | RenderingIntent::Saturation
        );
        let implicit = v4_intent && dst.version.major >= 4;
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
            abs_scale: None,
            intent,
            exact: exact_matrix(src, dst),
        };
        if intent == RenderingIntent::AbsoluteColorimetric {
            t.abs_scale = absolute_scale(src, dst);
            if t.abs_scale.is_some() {
                // The exact path does not apply the media-white scale.
                t.exact = None;
            }
        }
        if compensate && intent != RenderingIntent::AbsoluteColorimetric {
            let black = BlackPoints {
                src: black::source(src, intent).unwrap_or_default(),
                dst: black::destination(dst, intent).unwrap_or_default(),
            };
            // Equal black points make compensation the identity, so skip it and keep the fast
            // paths. This is the common case for v4 perceptual with zero black points.
            let same = (0..3).all(|c| (black.src[c] - black.dst[c]).abs() <= 1e-7);
            if !same {
                t.black = Some(black);
                // Black point compensation stays in f32, so the exact path no longer applies.
                t.exact = None;
            }
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
        let mut xyz = self.forward.to_xyz(src)?;
        if let Some(scale) = self.abs_scale {
            for (v, s) in xyz.iter_mut().zip(scale) {
                *v *= s;
            }
        }
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

    /// Samples this transform on a `grid`-point cube and returns it as a 3-in, 3-out pipeline.
    ///
    /// The pipeline evaluates with tetrahedral interpolation, so it is a fast, deterministic
    /// approximation of the transform. It does not include gamut mapping, which `ManagedTransform`
    /// adds.
    ///
    /// Accuracy depends on the grid. For Display P3 → sRGB, the mean error is about 1.0 code at
    /// 17 points, 0.3 at 33 and 0.1 at 65. The 99th percentile is about 9, 4.4 and 1.6 codes. The
    /// worst cases are near black, where the curves are steep. A 33-point grid is a reasonable
    /// default.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] unless both profiles are three-channel, and
    /// [`IccError::BadTag`] for a grid under two points.
    pub fn bake_clut(&self, grid: usize) -> Result<Pipeline, IccError> {
        if self.src_channels() != 3 || self.dst_channels() != 3 {
            return Err(IccError::ChannelCount);
        }
        if grid < 2 {
            return Err(IccError::BadTag);
        }
        let scale = (grid - 1) as f32;
        let mut values = alloc::vec::Vec::with_capacity(grid * grid * grid * 3);
        for r in 0..grid {
            for g in 0..grid {
                for b in 0..grid {
                    let input = [r as f32 / scale, g as f32 / scale, b as f32 / scale];
                    let mut out = [0.0_f32; 3];
                    self.transform(&input, &mut out)?;
                    values.extend_from_slice(&out);
                }
            }
        }
        let clut = Clut::new(alloc::vec![grid; 3], 3, values)?;
        Pipeline::new(3, alloc::vec![Stage::Clut(clut)], LabEncoding::V4)
    }

    /// Writes this transform as an Adobe `.cube` 3-D LUT, with `size` points per side.
    ///
    /// Red varies fastest, as the format requires. The title is cleaned to printable ASCII
    /// without quotes. Values are written with six decimals. The LUT has no gamut mapping, as
    /// for [`ColourTransform::bake_clut`].
    ///
    /// # Errors
    ///
    /// As for [`ColourTransform::bake_clut`].
    pub fn to_cube(&self, size: usize, title: &str) -> Result<alloc::string::String, IccError> {
        use core::fmt::Write;

        if self.src_channels() != 3 || self.dst_channels() != 3 {
            return Err(IccError::ChannelCount);
        }
        if size < 2 {
            return Err(IccError::BadTag);
        }
        let clean: alloc::string::String = title
            .chars()
            .filter(|c| c.is_ascii_graphic() && *c != '"' || *c == ' ')
            .collect();
        let scale = (size - 1) as f32;
        let mut out = alloc::string::String::new();
        let _ = writeln!(out, "TITLE \"{clean}\"");
        let _ = writeln!(out, "LUT_3D_SIZE {size}");
        let _ = writeln!(out, "DOMAIN_MIN 0.0 0.0 0.0");
        let _ = writeln!(out, "DOMAIN_MAX 1.0 1.0 1.0");
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let input = [r as f32 / scale, g as f32 / scale, b as f32 / scale];
                    let mut rgb = [0.0_f32; 3];
                    self.transform(&input, &mut rgb)?;
                    let _ = writeln!(out, "{:.6} {:.6} {:.6}", rgb[0], rgb[1], rgb[2]);
                }
            }
        }
        Ok(out)
    }

    /// Transforms interleaved 8-bit pixels in place: `channels` is 3 (RGB) or 4 (RGBA).
    ///
    /// Each value is scaled to `[0, 1]`, transformed, and rounded back to 8 bits. Alpha, the
    /// fourth channel, is left unchanged. Both profiles must be three-channel.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if `channels` is not 3 or 4, if `pixels` is not a whole
    /// number of pixels, or if either profile is not three-channel.
    pub fn transform_u8(&self, pixels: &mut [u8], channels: usize) -> Result<(), IccError> {
        if !(channels == 3 || channels == 4)
            || pixels.len() % channels != 0
            || self.src_channels() != 3
            || self.dst_channels() != 3
        {
            return Err(IccError::ChannelCount);
        }
        for px in pixels.chunks_exact_mut(channels) {
            let input = [f32::from(px[0]), f32::from(px[1]), f32::from(px[2])].map(|v| v / 255.0);
            let mut out = [0.0_f32; 3];
            self.transform(&input, &mut out)?;
            for (dst, v) in px.iter_mut().zip(out) {
                *dst = libm::roundf(v.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        Ok(())
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
            ) if self.abs_scale.is_none() => Some(MatrixParts {
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
pub(crate) fn forward(profile: &IccProfile, intent: RenderingIntent) -> Result<Forward, IccError> {
    match &profile.model {
        ProfileModel::Matrix(m) => Ok(Forward::Matrix {
            colorants: m.colorants,
            trc: m.trc.clone().map(|t| t.bake()),
        }),
        ProfileModel::Gray(trc) => Ok(Forward::Gray { trc: trc.bake() }),
        ProfileModel::Lut(tables) => {
            if tables.has_d2b(intent) {
                return Err(IccError::Unsupported);
            }
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
pub(crate) fn backward(
    profile: &IccProfile,
    intent: RenderingIntent,
) -> Result<Backward, IccError> {
    match &profile.model {
        ProfileModel::Matrix(m) => Ok(Backward::Matrix {
            inverse: inverse(&m.colorants)?,
            trc: m.trc.clone().map(|t| t.bake_inverse()),
        }),
        ProfileModel::Gray(trc) => Ok(Backward::Gray {
            trc: trc.bake_inverse(),
        }),
        ProfileModel::Lut(tables) => {
            if tables.has_b2d(intent) {
                return Err(IccError::Unsupported);
            }
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
