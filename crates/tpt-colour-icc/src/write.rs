//! Writes matrix/TRC RGB and grey ICC profiles (version 2.1, XYZ PCS).
//!
//! The output is the inverse of [`IccProfile::parse`] for these profile models. LUT profiles and
//! profiles with a Lab PCS are not written, and return [`IccError::Unsupported`].

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use tpt_colour_core::space::D50;

use crate::error::IccError;
use crate::profile::{
    IccProfile, ProfileClass, ProfileConnectionSpace, ProfileModel, RenderingIntent,
};
use crate::trc::Trc;

/// A tag signature and its body, in table order.
type TagList = Vec<([u8; 4], Vec<u8>)>;

/// Number of parameters for each `para` function type.
const PARA_PARAMS: [usize; 5] = [1, 3, 4, 5, 7];

impl IccProfile {
    /// Serialises the profile as an ICC version 2.1 file.
    ///
    /// Supports matrix/TRC RGB profiles and grey (`kTRC`) profiles with an XYZ PCS. The output
    /// parses back to the same model. Colorants and white points are stored as `s15Fixed16`, so
    /// they are quantised to about 1.5e-5, and table entries to 16 bits. The transform then agrees
    /// with the original to within one 8-bit code. Gamma curves are written as `para` function type
    /// 0, which reads back as `Trc::Gamma`.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::Unsupported`] for LUT profiles, or for a Lab PCS, which this writer does
    /// not produce.
    ///
    /// # Example
    ///
    /// ```
    /// use tpt_colour_icc::{IccProfile, RenderingIntent};
    /// let bytes = IccProfile::srgb().to_bytes()?;
    /// let back = IccProfile::parse(&bytes)?;
    /// assert_eq!(back.intent, RenderingIntent::Perceptual);
    /// # Ok::<(), tpt_colour_icc::IccError>(())
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, IccError> {
        if self.pcs != ProfileConnectionSpace::Xyz {
            return Err(IccError::Unsupported);
        }
        let (space, mut tags): (&[u8; 4], TagList) = match &self.model {
            ProfileModel::Matrix(m) => {
                let column = |c: usize| [m.colorants[0][c], m.colorants[1][c], m.colorants[2][c]];
                let mut tags = vec![
                    (*b"rXYZ", xyz_tag(column(0))),
                    (*b"gXYZ", xyz_tag(column(1))),
                    (*b"bXYZ", xyz_tag(column(2))),
                    (*b"rTRC", trc_tag(&m.trc[0])),
                    (*b"gTRC", trc_tag(&m.trc[1])),
                    (*b"bTRC", trc_tag(&m.trc[2])),
                ];
                if let Some(chad) = m.chad {
                    tags.push((*b"chad", chad_tag(chad)));
                }
                (b"RGB ", tags)
            }
            ProfileModel::Gray(trc) => (b"GRAY", vec![(*b"kTRC", trc_tag(trc))]),
            ProfileModel::Lut(_) => return Err(IccError::Unsupported),
        };
        tags.insert(
            0,
            (
                *b"desc",
                desc_tag(self.description.as_deref().unwrap_or("")),
            ),
        );
        tags.insert(1, (*b"wtpt", xyz_tag(self.white_point)));

        Ok(assemble(self, *space, &tags))
    }
}

/// Lays out the header, the tag table and the tag bodies, each body on a 4-byte boundary.
fn assemble(profile: &IccProfile, space: [u8; 4], tags: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let table_len = 4 + 12 * tags.len();
    let mut out = vec![0_u8; 128 + table_len];
    out[8] = 2;
    out[9] = 0x10;
    out[12..16].copy_from_slice(&class_signature(profile.class));
    out[16..20].copy_from_slice(&space);
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    out[64..68].copy_from_slice(&intent_code(profile.intent).to_be_bytes());
    out[68..80].copy_from_slice(&xyz_bytes(D50));
    out[128..132].copy_from_slice(&(tags.len() as u32).to_be_bytes());

    for (i, (sig, body)) in tags.iter().enumerate() {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        let entry = 132 + 12 * i;
        let offset = out.len() as u32;
        out[entry..entry + 4].copy_from_slice(sig);
        out[entry + 4..entry + 8].copy_from_slice(&offset.to_be_bytes());
        out[entry + 8..entry + 12].copy_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
    }
    let total = out.len() as u32;
    out[0..4].copy_from_slice(&total.to_be_bytes());
    out
}

fn class_signature(class: ProfileClass) -> [u8; 4] {
    match class {
        ProfileClass::Input => *b"scnr",
        ProfileClass::Display => *b"mntr",
        ProfileClass::Output => *b"prtr",
        ProfileClass::DeviceLink => *b"link",
        ProfileClass::ColourSpace => *b"spac",
        ProfileClass::Abstract => *b"abst",
        ProfileClass::NamedColour => *b"nmcl",
    }
}

fn intent_code(intent: RenderingIntent) -> u32 {
    match intent {
        RenderingIntent::Perceptual => 0,
        RenderingIntent::RelativeColorimetric => 1,
        RenderingIntent::Saturation => 2,
        RenderingIntent::AbsoluteColorimetric => 3,
    }
}

/// An `s15Fixed16Number` in big-endian form.
fn s15_fixed16(value: f32) -> [u8; 4] {
    // The cast saturates at the i32 range, which an XYZ value in profiles never reaches.
    (libm::roundf(value * 65_536.0) as i32).to_be_bytes()
}

fn xyz_bytes(v: [f32; 3]) -> [u8; 12] {
    let mut out = [0_u8; 12];
    for (i, value) in v.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&s15_fixed16(*value));
    }
    out
}

fn xyz_tag(v: [f32; 3]) -> Vec<u8> {
    let mut body = b"XYZ \0\0\0\0".to_vec();
    body.extend_from_slice(&xyz_bytes(v));
    body
}

fn chad_tag(m: [[f32; 3]; 3]) -> Vec<u8> {
    let mut body = b"sf32\0\0\0\0".to_vec();
    for row in m {
        for value in row {
            body.extend_from_slice(&s15_fixed16(value));
        }
    }
    body
}

fn trc_tag(trc: &Trc) -> Vec<u8> {
    match trc {
        Trc::Identity => b"curv\0\0\0\0\0\0\0\0".to_vec(),
        Trc::Gamma(g) => {
            // Function type 0 keeps the exponent at s15Fixed16 precision.
            para_tag(0, &[*g, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        }
        Trc::Parametric { function, params } => para_tag(*function, params),
        Trc::Table(values) => {
            let mut body = b"curv\0\0\0\0".to_vec();
            body.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                let code = libm::roundf(value.clamp(0.0, 1.0) * 65_535.0) as u16;
                body.extend_from_slice(&code.to_be_bytes());
            }
            body
        }
    }
}

fn para_tag(function: u16, params: &[f32; 7]) -> Vec<u8> {
    let count = PARA_PARAMS[usize::from(function.min(4))];
    let mut body = b"para\0\0\0\0".to_vec();
    body.extend_from_slice(&function.to_be_bytes());
    body.extend_from_slice(&[0, 0]);
    for value in &params[..count] {
        body.extend_from_slice(&s15_fixed16(*value));
    }
    body
}

/// A `desc` tag holding `text` as ASCII. Non-ASCII characters become `?`.
fn desc_tag(text: &str) -> Vec<u8> {
    let ascii: String = text
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect();
    let mut body = b"desc\0\0\0\0".to_vec();
    body.extend_from_slice(&((ascii.len() + 1) as u32).to_be_bytes());
    body.extend_from_slice(ascii.as_bytes());
    body.push(0);
    // Empty Unicode and ScriptCode sections, as the `desc` type defines them.
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&[0; 2]);
    body.push(0);
    body.extend_from_slice(&[0; 67]);
    body
}
