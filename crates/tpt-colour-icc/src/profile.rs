//! ICC profiles: header, matrix/TRC and LUT models, and built-in sRGB and Display P3 profiles.

use alloc::string::String;

use tpt_colour_core::space::{ColourSpace, D50, DisplayP3, SRgb, d65_to_d50};

use crate::bytes::{sig_at, slice, u32_at};
use crate::error::IccError;
use crate::lut::{MAX_PROFILE_CLUT_VALUES, Pipeline};
use crate::lut_tags::{self, Direction};
use crate::matrix::Mat3;
use crate::tag;
use crate::trc::Trc;

/// Smallest possible profile: 128-byte header plus the tag count.
const MIN_LEN: usize = 132;

/// ICC profile version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IccVersion {
    /// Major version (`2` or `4` in practice).
    pub major: u8,
    /// Minor version (the high nibble of the minor byte).
    pub minor: u8,
}

/// Profile class from the header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileClass {
    /// Scanner or camera input (`scnr`).
    Input,
    /// Display (`mntr`).
    Display,
    /// Printer or other output (`prtr`).
    Output,
    /// Device link (`link`).
    DeviceLink,
    /// Colour space conversion (`spac`).
    ColourSpace,
    /// Abstract (`abst`).
    Abstract,
    /// Named colour (`nmcl`).
    NamedColour,
}

impl ProfileClass {
    fn from_sig(sig: [u8; 4]) -> Option<Self> {
        Some(match &sig {
            b"scnr" => Self::Input,
            b"mntr" => Self::Display,
            b"prtr" => Self::Output,
            b"link" => Self::DeviceLink,
            b"spac" => Self::ColourSpace,
            b"abst" => Self::Abstract,
            b"nmcl" => Self::NamedColour,
            _ => return None,
        })
    }
}

/// Profile connection space (PCS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileConnectionSpace {
    /// CIE XYZ, D50-relative.
    Xyz,
    /// CIE L*a*b*, D50-relative.
    Lab,
}

/// Rendering intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderingIntent {
    /// Perceptual (intent 0).
    #[default]
    Perceptual,
    /// Relative colorimetric (intent 1).
    RelativeColorimetric,
    /// Saturation (intent 2).
    Saturation,
    /// Absolute colorimetric (intent 3).
    AbsoluteColorimetric,
}

impl RenderingIntent {
    /// Maps the header value. Unknown values fall back to perceptual, as ICC allows.
    #[must_use]
    pub fn from_u32(value: u32) -> Self {
        match value & 0xFFFF {
            1 => Self::RelativeColorimetric,
            2 => Self::Saturation,
            3 => Self::AbsoluteColorimetric,
            _ => Self::Perceptual,
        }
    }

    /// Index of the tag group used for this intent: `A2B0`/`A2B1`/`A2B2`.
    ///
    /// Absolute colorimetric uses the relative-colorimetric tables, as the spec directs.
    fn tag_index(self) -> usize {
        match self {
            Self::Perceptual => 0,
            Self::RelativeColorimetric | Self::AbsoluteColorimetric => 1,
            Self::Saturation => 2,
        }
    }
}

/// The matrix/TRC model of an RGB profile.
#[derive(Clone, Debug, PartialEq)]
pub struct MatrixTrc {
    /// Row-major matrix mapping linear RGB to D50 XYZ (columns are the colorants).
    pub colorants: Mat3,
    /// Red, green and blue tone curves.
    pub trc: [Trc; 3],
    /// Chromatic adaptation matrix from the `chad` tag, if present.
    ///
    /// Colorants in v2 and v4 profiles are already adapted to D50, so the transform
    /// does not reapply this matrix. It is kept as metadata.
    pub chad: Option<Mat3>,
}

/// LUT pipelines for each rendering intent, in both directions.
#[derive(Clone, Debug, PartialEq)]
pub struct LutTables {
    a2b: [Option<Pipeline>; 3],
    b2a: [Option<Pipeline>; 3],
    /// Whether each intent has a `D2B` (`multiProcessElements`) tag. Those are read in preference
    /// to `A2B`, and they are not supported, so transforms refuse such intents.
    d2b: [bool; 3],
    /// Whether each intent has a `B2D` (`multiProcessElements`) tag. See [`LutTables::d2b`].
    b2d: [bool; 3],
}

impl LutTables {
    /// Device → PCS pipeline for `intent`.
    ///
    /// If the profile has no table for that intent, the perceptual table is used, as the ICC
    /// specification and `LittleCMS` both do.
    #[must_use]
    pub fn a2b(&self, intent: RenderingIntent) -> Option<&Pipeline> {
        self.a2b[intent.tag_index()]
            .as_ref()
            .or(self.a2b[0].as_ref())
    }

    /// PCS → device pipeline for `intent`, with the same perceptual fallback as [`LutTables::a2b`].
    #[must_use]
    pub fn b2a(&self, intent: RenderingIntent) -> Option<&Pipeline> {
        self.b2a[intent.tag_index()]
            .as_ref()
            .or(self.b2a[0].as_ref())
    }

    /// True if `intent` has a `D2B` tag. Such a profile must not be transformed with `A2B`,
    /// because the reference readers use the `D2B` table instead.
    #[must_use]
    pub fn has_d2b(&self, intent: RenderingIntent) -> bool {
        self.d2b[intent.tag_index()]
    }

    /// True if `intent` has a `B2D` tag. See [`LutTables::has_d2b`].
    #[must_use]
    pub fn has_b2d(&self, intent: RenderingIntent) -> bool {
        self.b2d[intent.tag_index()]
    }
}

/// How a profile maps between its device space and the PCS.
#[derive(Clone, Debug, PartialEq)]
pub enum ProfileModel {
    /// Three matrix/TRC channels (RGB only).
    Matrix(MatrixTrc),
    /// One grey TRC (`kTRC`). The PCS is `D50 · Y`, where `Y` is the TRC output.
    Gray(Trc),
    /// Lookup tables, one set per rendering intent.
    Lut(LutTables),
}

/// A parsed ICC profile.
#[derive(Clone, Debug, PartialEq)]
pub struct IccProfile {
    /// Profile version.
    pub version: IccVersion,
    /// Profile class.
    pub class: ProfileClass,
    /// Number of device channels (`RGB ` → 3, `CMYK` → 4, `GRAY` → 1).
    pub device_channels: usize,
    /// Profile connection space.
    pub pcs: ProfileConnectionSpace,
    /// Rendering intent recorded in the header.
    pub intent: RenderingIntent,
    /// Human-readable description, if the profile has one.
    pub description: Option<String>,
    /// Media white point from `wtpt` (D50 when absent).
    pub white_point: [f32; 3],
    /// Device ⇄ PCS model.
    pub model: ProfileModel,
}

impl IccProfile {
    /// Parses an ICC profile.
    ///
    /// Supports matrix/TRC RGB profiles and LUT profiles (`mft1`, `mft2`, `mAB `, `mBA `)
    /// for RGB, CMYK and grey data. `multiProcessElements` (`mpet`) is not supported.
    /// Every structure is bounds-checked, so malformed input returns an error rather than panicking.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::Truncated`], [`IccError::BadSignature`], [`IccError::BadTag`]
    /// or [`IccError::Unsupported`] as appropriate.
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < MIN_LEN {
            return Err(IccError::Truncated);
        }
        let declared = u32_at(data, 0)? as usize;
        if declared < MIN_LEN || declared > data.len() {
            return Err(IccError::Truncated);
        }
        let data = slice(data, 0, declared)?;
        if sig_at(data, 36)? != *b"acsp" {
            return Err(IccError::BadSignature);
        }

        let major = data[8];
        let minor = data[9] >> 4;
        if !(2..=4).contains(&major) {
            return Err(IccError::Unsupported);
        }
        let class = ProfileClass::from_sig(sig_at(data, 12)?).ok_or(IccError::Unsupported)?;
        let data_space = sig_at(data, 16)?;
        let device_channels = match &data_space {
            b"RGB " => 3,
            b"CMYK" => 4,
            b"GRAY" => 1,
            _ => return Err(IccError::Unsupported),
        };
        let pcs = match &sig_at(data, 20)? {
            b"XYZ " => ProfileConnectionSpace::Xyz,
            b"Lab " => ProfileConnectionSpace::Lab,
            _ => return Err(IccError::Unsupported),
        };
        let intent = RenderingIntent::from_u32(u32_at(data, 64)?);

        let tags = tag::table(data)?;
        let find = |sig: &[u8; 4]| tags.iter().find(|(s, _)| s == sig).map(|(_, body)| *body);
        let need = |sig: &[u8; 4]| find(sig).ok_or(IccError::Unsupported);
        // Metadata never fails the parse: an unreadable `wtpt` falls back to D50.
        let white_point = find(b"wtpt")
            .and_then(|body| tag::xyz(body).ok())
            .unwrap_or(D50);
        // The description is always stored under the `desc` tag name. Its body may be any of the
        // text types, and `text_description` checks the type signature.
        let description = find(b"desc").and_then(tag::text_description);

        let a2b = intent_tags(&tags, *b"A2B", Direction::AtoB);
        let b2a = intent_tags(&tags, *b"B2A", Direction::BtoA);
        // Each table is capped on its own, but six of them could still hold a lot of memory. The
        // profile as a whole is capped here, so a hostile file cannot retain gigabytes of CLUTs.
        let clut_values: usize = a2b
            .iter()
            .chain(&b2a)
            .flatten()
            .map(Pipeline::clut_values)
            .sum();
        if clut_values > MAX_PROFILE_CLUT_VALUES {
            return Err(IccError::Unsupported);
        }
        // A profile is LUT-based when it has the tags, even if every one is unreadable. Those
        // intents then fail as `Unsupported`, rather than the profile being read as matrix/TRC.
        let has_lut = [*b"A2B", *b"B2A"].iter().any(|prefix| {
            (0..3).any(|i| find(&[prefix[0], prefix[1], prefix[2], b'0' + i]).is_some())
        });
        let model = if has_lut {
            let d2b = [0_u8, 1, 2].map(|i| find(&[b'D', b'2', b'B', b'0' + i]).is_some());
            let b2d = [0_u8, 1, 2].map(|i| find(&[b'B', b'2', b'D', b'0' + i]).is_some());
            ProfileModel::Lut(LutTables { a2b, b2a, d2b, b2d })
        } else {
            if data_space == *b"GRAY" {
                // A grey profile has one TRC, which maps device value to relative luminance.
                return Ok(Self {
                    version: IccVersion { major, minor },
                    class,
                    device_channels,
                    pcs,
                    intent,
                    description,
                    white_point,
                    model: ProfileModel::Gray(tag::trc(need(b"kTRC")?)?),
                });
            }
            if data_space != *b"RGB " {
                return Err(IccError::Unsupported);
            }
            let xyz_of = |sig: &[u8; 4]| tag::xyz(need(sig)?);
            let [r, g, b] = [xyz_of(b"rXYZ")?, xyz_of(b"gXYZ")?, xyz_of(b"bXYZ")?];
            let colorants = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
            let trc = [
                tag::trc(need(b"rTRC")?)?,
                tag::trc(need(b"gTRC")?)?,
                tag::trc(need(b"bTRC")?)?,
            ];
            let chad = find(b"chad").and_then(|body| tag::chad(body).ok());
            ProfileModel::Matrix(MatrixTrc {
                colorants,
                trc,
                chad,
            })
        };

        Ok(Self {
            version: IccVersion { major, minor },
            class,
            device_channels,
            pcs,
            intent,
            description,
            white_point,
            model,
        })
    }

    /// Built-in sRGB profile (IEC 61966-2.1). Needs no file.
    #[must_use]
    pub fn srgb() -> Self {
        builtin(SRgb, "sRGB IEC61966-2.1")
    }

    /// Built-in Display P3 profile. Needs no file.
    #[must_use]
    pub fn display_p3() -> Self {
        builtin(DisplayP3, "Display P3")
    }
}

/// Parses the three intent variants of one LUT tag family (`A2B0..2` or `B2A0..2`).
///
/// A table that fails to parse counts as absent. Only the intents that use it are affected, and
/// they then report `Unsupported`, so one bad unused tag cannot fail the whole profile.
fn intent_tags(
    tags: &[([u8; 4], &[u8])],
    prefix: [u8; 3],
    direction: Direction,
) -> [Option<Pipeline>; 3] {
    let mut out: [Option<Pipeline>; 3] = [None, None, None];
    for (i, slot) in out.iter_mut().enumerate() {
        let sig = [prefix[0], prefix[1], prefix[2], b'0' + i as u8];
        let body = tags.iter().find(|(s, _)| *s == sig).map(|(_, body)| *body);
        *slot = body.and_then(|body| lut_tags::pipeline(body, direction).ok());
    }
    out
}

/// The sRGB transfer function as an ICC parametric curve (type 3).
fn srgb_trc() -> Trc {
    Trc::Parametric {
        function: 3,
        params: [
            2.4,
            1.0 / 1.055,
            0.055 / 1.055,
            1.0 / 12.92,
            0.040_45,
            0.0,
            0.0,
        ],
    }
}

/// Builds a built-in profile from a D65 colour space's primaries, adapted to D50.
fn builtin<S: ColourSpace<3>>(space: S, description: &str) -> IccProfile {
    let column = |rgb: [f32; 3]| d65_to_d50(space.to_xyz(rgb));
    let (r, g, b) = (
        column([1.0, 0.0, 0.0]),
        column([0.0, 1.0, 0.0]),
        column([0.0, 0.0, 1.0]),
    );
    let colorants = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
    IccProfile {
        version: IccVersion { major: 4, minor: 3 },
        class: ProfileClass::Display,
        device_channels: 3,
        pcs: ProfileConnectionSpace::Xyz,
        intent: RenderingIntent::Perceptual,
        description: Some(String::from(description)),
        white_point: D50,
        model: ProfileModel::Matrix(MatrixTrc {
            colorants,
            trc: [srgb_trc(), srgb_trc(), srgb_trc()],
            chad: None,
        }),
    }
}

/// Error from parsing a rendering intent name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseIntentError;

impl core::fmt::Display for ParseIntentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("expected perceptual, relative, saturation or absolute")
    }
}

impl core::error::Error for ParseIntentError {}

impl core::str::FromStr for RenderingIntent {
    type Err = ParseIntentError;

    /// Parses `perceptual`, `relative`, `saturation` or `absolute`, ignoring ASCII case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "perceptual" => Ok(Self::Perceptual),
            "relative" => Ok(Self::RelativeColorimetric),
            "saturation" => Ok(Self::Saturation),
            "absolute" => Ok(Self::AbsoluteColorimetric),
            _ => Err(ParseIntentError),
        }
    }
}
