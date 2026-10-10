//! Naive CMYK model.
//!
//! This is a fallback only. Real CMYK conversion needs an ICC profile (FOGRA39,
//! SWOP, …) through `tpt-colour-icc`. The model here treats the ink channels
//! as multiplicative attenuation of sRGB white, which is not colorimetrically
//! accurate for print.

use super::{ColourSpace, SRgb};

/// Naive CMYK: channels in `[0, 1]`, where `1` is full ink coverage.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cmyk;

impl ColourSpace<4> for Cmyk {
    fn to_xyz(&self, [c, m, y, k]: [f32; 4]) -> [f32; 3] {
        let key = 1.0 - k;
        let rgb = [(1.0 - c) * key, (1.0 - m) * key, (1.0 - y) * key];
        SRgb.to_xyz(rgb)
    }

    fn from_xyz(&self, xyz: [f32; 3]) -> [f32; 4] {
        let [r, g, b] = SRgb.from_xyz(xyz);
        let (r, g, b) = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
        let k = 1.0 - r.max(g).max(b);
        if k >= 1.0 {
            return [0.0, 0.0, 0.0, 1.0];
        }
        let key = 1.0 - k;
        [
            (1.0 - r - k) / key,
            (1.0 - g - k) / key,
            (1.0 - b - k) / key,
            k,
        ]
    }
}
