//! Black point detection, following `LittleCMS` (`cmssamp.c`).
//!
//! The source black point is the profile's own black, found with the darker-colorant method for
//! v2 profiles. The destination black point uses Adobe's round-trip algorithm for LUT-based
//! profiles. v4 perceptual and saturation intents use the fixed perceptual black. All of these
//! return `None` where `LittleCMS` returns `FALSE`. The caller then uses zero.

use alloc::vec::Vec;

use crate::lut::MAX_CHANNELS;
use crate::pcs::{lab_to_xyz_d50, xyz_to_lab_d50};
use crate::profile::{IccProfile, ProfileClass, ProfileModel, RenderingIntent};
use crate::transform::{Backward, Forward, backward, forward};

/// The fixed v4 perceptual black point (XYZ, D50), from `cmsPERCEPTUAL_BLACK_*` in `lcms2.h`.
const PERCEPTUAL_BLACK: [f32; 3] = [0.003_36, 0.003_473_1, 0.002_87];

/// Source black point (XYZ, D50), as `cmsDetectBlackPoint`.
pub(crate) fn source(profile: &IccProfile, intent: RenderingIntent) -> Option<[f32; 3]> {
    if !detectable(profile, intent) {
        return None;
    }
    if is_v4_fixed(profile, intent) {
        return v4_black(profile);
    }
    if intent == RenderingIntent::RelativeColorimetric
        && profile.class == ProfileClass::Output
        && is_ink(profile)
    {
        return perceptual_round_trip_black(profile);
    }
    darker_colorant(profile, intent)
}

/// Destination black point (XYZ, D50), as `cmsDetectDestinationBlackPoint`.
pub(crate) fn destination(profile: &IccProfile, intent: RenderingIntent) -> Option<[f32; 3]> {
    if !detectable(profile, intent) {
        return None;
    }
    if is_v4_fixed(profile, intent) {
        return v4_black(profile);
    }
    let lut_based = matches!(&profile.model, ProfileModel::Lut(t) if t.b2a(intent).is_some());
    if !lut_based || !(is_grey(profile) || is_rgb(profile) || is_ink(profile)) {
        return source(profile, intent);
    }
    adobe_black(profile, intent)
}

/// Link, abstract and named-colour profiles have no black point, and neither does absolute intent.
fn detectable(profile: &IccProfile, intent: RenderingIntent) -> bool {
    intent != RenderingIntent::AbsoluteColorimetric
        && !matches!(
            profile.class,
            ProfileClass::DeviceLink | ProfileClass::Abstract | ProfileClass::NamedColour
        )
}

/// v4 perceptual and saturation black points are fixed by the specification.
fn is_v4_fixed(profile: &IccProfile, intent: RenderingIntent) -> bool {
    profile.version.major >= 4
        && matches!(
            intent,
            RenderingIntent::Perceptual | RenderingIntent::Saturation
        )
}

fn v4_black(profile: &IccProfile) -> Option<[f32; 3]> {
    match &profile.model {
        ProfileModel::Matrix(_) | ProfileModel::Gray(_) => {
            darker_colorant(profile, RenderingIntent::RelativeColorimetric)
        }
        ProfileModel::Lut(_) => Some(PERCEPTUAL_BLACK),
    }
}

fn is_grey(profile: &IccProfile) -> bool {
    profile.device_channels == 1
}

/// Three device channels are taken to be RGB. The header's data colour space is not kept.
fn is_rgb(profile: &IccProfile) -> bool {
    profile.device_channels == 3
}

fn is_ink(profile: &IccProfile) -> bool {
    profile.device_channels == 4
}

/// The darkest device colour, mapped to Lab through `intent`. Its lightness is clipped, and the
/// colour is made neutral. The result is returned as XYZ (D50).
fn darker_colorant(profile: &IccProfile, intent: RenderingIntent) -> Option<[f32; 3]> {
    let fwd = forward(profile, intent).ok()?;
    let n = fwd.channels();
    // The darkest CMYK colorant is every ink at 100%. Other spaces use a zero device black.
    let mut device = [0.0_f32; MAX_CHANNELS];
    if n == 4 {
        device = [1.0; MAX_CHANNELS];
    }
    let lab = xyz_to_lab_d50(fwd.to_xyz(&device[..n]).ok()?);
    let l = if lab[0] > 95.0 || lab[0] < 0.0 {
        0.0
    } else {
        lab[0].min(50.0)
    };
    Some(lab_to_xyz_d50([l, 0.0, 0.0]))
}

/// Black of a v2 output profile with ink limiting removed. It uses the perceptual intent
/// round-tripped through the device, as `BlackPointUsingPerceptualBlack`.
fn perceptual_round_trip_black(profile: &IccProfile) -> Option<[f32; 3]> {
    let path = RoundTrip::new(profile, RenderingIntent::Perceptual)?;
    let lab_out = path.apply([0.0; 3])?;
    let l = lab_out[0].min(50.0);
    Some(lab_to_xyz_d50([l, 0.0, 0.0]))
}

/// The two halves of a round trip, built once and reused for every sample.
struct RoundTrip {
    back: Backward,
    fwd: Forward,
}

impl RoundTrip {
    /// Device side for `intent`, and the PCS side always relative colorimetric.
    fn new(profile: &IccProfile, intent: RenderingIntent) -> Option<Self> {
        Some(Self {
            back: backward(profile, intent).ok()?,
            fwd: forward(profile, RenderingIntent::RelativeColorimetric).ok()?,
        })
    }

    /// Lab → device → Lab.
    fn apply(&self, lab: [f32; 3]) -> Option<[f32; 3]> {
        let n = self.back.channels();
        let mut device = [0.0_f32; MAX_CHANNELS];
        self.back
            .from_xyz(lab_to_xyz_d50(lab), &mut device[..n])
            .ok()?;
        let xyz = self.fwd.to_xyz(&device[..n]).ok()?;
        Some(xyz_to_lab_d50(xyz))
    }
}

/// Adobe's round-trip black point search, as `cmsDetectDestinationBlackPoint`.
///
/// The round trip is sampled over 256 lightness steps. A quadratic is fitted to the shadow
/// end of the curve, and its vertex gives the black lightness.
fn adobe_black(profile: &IccProfile, intent: RenderingIntent) -> Option<[f32; 3]> {
    let relative = intent == RenderingIntent::RelativeColorimetric;
    // Relative colorimetric starts from the source black. Perceptual and saturation start neutral.
    let initial = if relative {
        xyz_to_lab_d50(source(profile, intent)?)
    } else {
        [0.0; 3]
    };
    let a = initial[1].clamp(-50.0, 50.0);
    let b = initial[2].clamp(-50.0, 50.0);

    let path = RoundTrip::new(profile, intent)?;
    let mut in_ramp = [0.0_f64; 256];
    let mut out_ramp = [0.0_f64; 256];
    for (l, (inp, out)) in in_ramp.iter_mut().zip(out_ramp.iter_mut()).enumerate() {
        let lightness = l as f32 * 100.0 / 255.0;
        let rt = path.apply([lightness, a, b])?;
        *inp = f64::from(lightness);
        *out = f64::from(rt[0]);
    }

    // Make the round trip monotonic from the top down.
    for l in (0..255).rev() {
        out_ramp[l] = out_ramp[l].min(out_ramp[l + 1]);
    }
    let min_l = out_ramp[0];
    let max_l = out_ramp[255];
    if min_l.partial_cmp(&max_l) != Some(core::cmp::Ordering::Less) {
        return None;
    }

    if relative {
        // A nearly straight midrange means the source black is already the answer.
        let straight = (0..256).all(|l| {
            in_ramp[l] <= min_l + 0.2 * (max_l - min_l) || (in_ramp[l] - out_ramp[l]).abs() < 4.0
        });
        if straight {
            return Some(lab_to_xyz_d50(initial));
        }
    }

    let y_ramp: Vec<f64> = out_ramp
        .iter()
        .map(|v| (v - min_l) / (max_l - min_l))
        .collect();
    let (lo, hi) = if relative { (0.1, 0.5) } else { (0.03, 0.25) };
    let (xs, ys): (Vec<f64>, Vec<f64>) = in_ramp
        .iter()
        .zip(&y_ramp)
        .filter(|(_, y)| **y >= lo && **y < hi)
        .map(|(x, y)| (*x, *y))
        .unzip();
    if xs.len() < 3 {
        return None;
    }

    let l = parabola_vertex(&xs, &ys).max(0.0);
    Some(lab_to_xyz_d50([l as f32, initial[1], initial[2]]))
}

/// Vertex of the least-squares quadratic through `(x, y)`, clamped to `[0, 50]`.
///
/// Returns 0 for fewer than four points, a singular fit, or a fit with no real root,
/// as `LittleCMS` does.
#[allow(clippy::many_single_char_names)] // Least-squares sums read most naturally with short names.
fn parabola_vertex(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n < 4 {
        return 0.0;
    }
    let (mut s1, mut s2, mut s3, mut s4) = (0.0, 0.0, 0.0, 0.0);
    let (mut sy, mut syx, mut syx2) = (0.0, 0.0, 0.0);
    for (&xi, &yi) in x.iter().zip(y) {
        s1 += xi;
        s2 += xi * xi;
        s3 += xi * xi * xi;
        s4 += xi * xi * xi * xi;
        sy += yi;
        syx += yi * xi;
        syx2 += yi * xi * xi;
    }
    let m = [[n as f64, s1, s2], [s1, s2, s3], [s2, s3, s4]];
    let Some([c, b, a]) = solve3(m, [sy, syx, syx2]) else {
        return 0.0;
    };

    let clamp = |v: f64| v.clamp(0.0, 50.0);
    if a.abs() < 1.0e-10 {
        if b.abs() < 1.0e-10 {
            return 0.0;
        }
        return clamp(-c / b);
    }
    let d = b * b - 4.0 * a * c;
    if d <= 0.0 {
        return 0.0;
    }
    clamp((-b + libm::sqrt(d)) / (2.0 * a))
}

/// Solves a 3×3 linear system by Cramer's rule. Returns `None` if the matrix is singular.
#[allow(clippy::many_single_char_names)] // Cramer's rule reads most naturally with short names.
fn solve3(m: [[f64; 3]; 3], v: [f64; 3]) -> Option<[f64; 3]> {
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(&m);
    if d == 0.0 {
        return None;
    }
    let mut out = [0.0; 3];
    for (col, o) in out.iter_mut().enumerate() {
        let mut mc = m;
        for row in 0..3 {
            mc[row][col] = v[row];
        }
        *o = det(&mc) / d;
    }
    Some(out)
}
