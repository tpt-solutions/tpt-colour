//! RGB gamut boundaries in CIE XYZ (D65), for containment tests and clipping.

use tpt_colour_core::space::{ColourSpace, D50, DisplayP3, Rec2020, SRgb, d50_to_d65};
use tpt_colour_icc::{IccError, IccProfile, ProfileModel};

/// Tolerance on linear RGB when testing containment, so boundary colours count as inside.
const CONTAINMENT_EPSILON: f32 = 1e-5;

type Mat3 = [[f32; 3]; 3];

/// A linear-light RGB gamut, expressed as its colorant matrix and inverse.
///
/// `to_xyz` maps linear RGB to XYZ (D65). `from_xyz` is its inverse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gamut {
    to_xyz: Mat3,
    from_xyz: Mat3,
}

impl Gamut {
    /// Builds a gamut from a space's primaries. The space's transfer function fixes 0 and 1,
    /// so decoding its unit primaries gives the linear colorants.
    #[must_use]
    pub fn from_space<S: ColourSpace<3>>(space: S) -> Self {
        let column = |e: [f32; 3]| space.to_xyz(e);
        let (r, g, b) = (
            column([1.0, 0.0, 0.0]),
            column([0.0, 1.0, 0.0]),
            column([0.0, 0.0, 1.0]),
        );
        let to_xyz = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
        // Built from valid primaries, so the matrix is never singular.
        Self::from_colorants(to_xyz).unwrap_or(Self {
            to_xyz,
            from_xyz: to_xyz,
        })
    }

    /// sRGB (IEC 61966-2.1 primaries).
    #[must_use]
    pub fn srgb() -> Self {
        Self::from_space(SRgb)
    }

    /// Display P3.
    #[must_use]
    pub fn display_p3() -> Self {
        Self::from_space(DisplayP3)
    }

    /// Rec. 2020.
    #[must_use]
    pub fn rec2020() -> Self {
        Self::from_space(Rec2020)
    }

    /// Builds a gamut from a linear-RGB → XYZ (D65) matrix.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::Singular`] if the colorants are linearly dependent.
    pub fn from_colorants(to_xyz: Mat3) -> Result<Self, IccError> {
        let from_xyz = invert(&to_xyz)?;
        Ok(Self { to_xyz, from_xyz })
    }

    /// The gamut of a matrix/TRC ICC profile, or `None` for LUT profiles.
    ///
    /// LUT profiles have no simple boundary, so they are not gamut-mapped in this version.
    /// Colorants are D50-relative in ICC profiles, so they are adapted to D65 here.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // Colorant columns are named after the RGB channels.
    pub fn from_profile(profile: &IccProfile) -> Option<Self> {
        match &profile.model {
            ProfileModel::Matrix(m) => {
                let c = m.colorants;
                let column = |j: usize| d50_to_d65([c[0][j], c[1][j], c[2][j]]);
                let (r, g, b) = (column(0), column(1), column(2));
                let adapted = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
                Self::from_colorants(adapted).ok()
            }
            ProfileModel::Lut(_) => None,
        }
    }

    /// Linear RGB of an XYZ (D65) colour.
    #[must_use]
    pub fn to_linear(&self, xyz: [f32; 3]) -> [f32; 3] {
        mul_vec(&self.from_xyz, xyz)
    }

    /// Whether an XYZ (D65) colour lies inside the gamut.
    #[must_use]
    pub fn contains(&self, xyz: [f32; 3]) -> bool {
        Self::linear_in_gamut(self.to_linear(xyz))
    }

    /// Whether linear RGB, from [`Gamut::to_linear`], lies inside `[0, 1]` (with tolerance).
    #[must_use]
    pub(crate) fn linear_in_gamut(lin: [f32; 3]) -> bool {
        lin.iter()
            .all(|&v| (-CONTAINMENT_EPSILON..=1.0 + CONTAINMENT_EPSILON).contains(&v))
    }

    /// Clips linear RGB to `[0, 1]` and returns the clipped colour as XYZ (D65).
    #[must_use]
    pub(crate) fn clip_linear(&self, lin: [f32; 3]) -> [f32; 3] {
        mul_vec(&self.to_xyz, lin.map(|v| v.clamp(0.0, 1.0)))
    }

    /// Clips an XYZ (D65) colour to the gamut by clamping its linear RGB to `[0, 1]`.
    ///
    /// This is per-channel clipping, which shifts hue and lightness. Use
    /// [`crate::map_to_gamut`] to preserve perceived lightness and hue.
    #[must_use]
    pub fn clip(&self, xyz: [f32; 3]) -> [f32; 3] {
        self.clip_linear(self.to_linear(xyz))
    }

    /// The colorant matrix (linear RGB → XYZ D65).
    #[must_use]
    pub fn colorants(&self) -> Mat3 {
        self.to_xyz
    }
}

/// The D50 white point, re-exported for callers that adapt XYZ between white points.
pub const WHITE_D50: [f32; 3] = D50;

fn mul_vec(m: &Mat3, v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

#[allow(clippy::many_single_char_names)] // Cofactor expansion reads most naturally with a..i.
fn invert(m: &Mat3) -> Result<Mat3, IccError> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-9 {
        return Err(IccError::Singular);
    }
    let k = 1.0 / det;
    Ok([
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
