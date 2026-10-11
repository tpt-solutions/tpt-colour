//! Y′CbCr ⇄ RGB matrices. The default `encode`/`decode` are full range (Cb and Cr centred on
//! zero). `encode_limited`/`decode_limited` give the studio-swing (limited range) code values.

/// Luma coefficients for a Y′CbCr system.
///
/// The coefficients are validated on construction: both must be positive and their sum must be
/// below one, so that the green weight `K_G = 1 - K_R - K_B` is positive and the chroma scales
/// are finite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YCbCr {
    kr: f32,
    kb: f32,
}

impl YCbCr {
    /// Rec. 709 (`K_R = 0.2126`, `K_B = 0.0722`).
    pub const REC709: Self = Self {
        kr: 0.2126,
        kb: 0.0722,
    };

    /// Rec. 2020 (`K_R = 0.2627`, `K_B = 0.0593`).
    pub const REC2020: Self = Self {
        kr: 0.2627,
        kb: 0.0593,
    };

    /// Rec. 601 (`K_R = 0.299`, `K_B = 0.114`). Full range, as the rest of this type.
    pub const REC601: Self = Self {
        kr: 0.299,
        kb: 0.114,
    };

    /// Creates luma coefficients, or `None` if they cannot describe an RGB system.
    ///
    /// Requires `0 < kr`, `0 < kb` and `kr + kb < 1`.
    #[must_use]
    pub fn new(kr: f32, kb: f32) -> Option<Self> {
        let valid = kr > 0.0 && kb > 0.0 && kr + kb < 1.0;
        valid.then_some(Self { kr, kb })
    }

    /// Red luma weight `K_R`.
    #[must_use]
    pub fn kr(&self) -> f32 {
        self.kr
    }

    /// Blue luma weight `K_B`.
    #[must_use]
    pub fn kb(&self) -> f32 {
        self.kb
    }

    /// Encodes gamma-encoded RGB in `[0, 1]` to `[Y′, Cb, Cr]`.
    #[must_use]
    pub fn encode(&self, [r, g, b]: [f32; 3]) -> [f32; 3] {
        let kg = 1.0 - self.kr - self.kb;
        let y = self.kr * r + kg * g + self.kb * b;
        let cb = (b - y) / (2.0 * (1.0 - self.kb));
        let cr = (r - y) / (2.0 * (1.0 - self.kr));
        [y, cb, cr]
    }

    /// Decodes `[Y′, Cb, Cr]` to gamma-encoded RGB.
    #[must_use]
    pub fn decode(&self, [y, cb, cr]: [f32; 3]) -> [f32; 3] {
        let kg = 1.0 - self.kr - self.kb;
        let r = y + 2.0 * (1.0 - self.kr) * cr;
        let b = y + 2.0 * (1.0 - self.kb) * cb;
        let g = (y - self.kr * r - self.kb * b) / kg;
        [r, g, b]
    }

    /// Encodes gamma-encoded RGB in `[0, 1]` to limited-range (studio swing) `[Y′, Cb, Cr]`, each
    /// normalised to `[0, 1]`.
    ///
    /// For 8-bit video, `Y′` spans codes 16 to 235 and `Cb`/`Cr` span 16 to 240 with 128 at zero
    /// chroma. Multiply the result by 255 to get those codes.
    #[must_use]
    pub fn encode_limited(&self, rgb: [f32; 3]) -> [f32; 3] {
        let [y, cb, cr] = self.encode(rgb);
        [
            LIMITED_Y_OFFSET + LIMITED_Y_SCALE * y,
            LIMITED_C_OFFSET + LIMITED_C_SCALE * cb,
            LIMITED_C_OFFSET + LIMITED_C_SCALE * cr,
        ]
    }

    /// Decodes limited-range `[Y′, Cb, Cr]` (each normalised to `[0, 1]`) to gamma-encoded RGB.
    /// This is the inverse of [`YCbCr::encode_limited`].
    #[must_use]
    pub fn decode_limited(&self, [y, cb, cr]: [f32; 3]) -> [f32; 3] {
        self.decode([
            (y - LIMITED_Y_OFFSET) / LIMITED_Y_SCALE,
            (cb - LIMITED_C_OFFSET) / LIMITED_C_SCALE,
            (cr - LIMITED_C_OFFSET) / LIMITED_C_SCALE,
        ])
    }
}

/// `16 / 255`: the code for black luma.
const LIMITED_Y_OFFSET: f32 = 16.0 / 255.0;
/// `219 / 255`: the luma code span from black (16) to white (235).
const LIMITED_Y_SCALE: f32 = 219.0 / 255.0;
/// `128 / 255`: the code for zero chroma.
const LIMITED_C_OFFSET: f32 = 128.0 / 255.0;
/// `224 / 255`: the chroma code span, 16 to 240.
const LIMITED_C_SCALE: f32 = 224.0 / 255.0;

#[cfg(test)]
mod tests {
    use super::YCbCr;

    #[test]
    fn rejects_coefficients_that_divide_by_zero() {
        assert!(YCbCr::new(1.0, 0.0).is_none());
        assert!(YCbCr::new(0.0, 0.5).is_none());
        assert!(YCbCr::new(0.5, 0.5).is_none(), "K_G would be zero");
        assert!(YCbCr::new(f32::NAN, 0.1).is_none());
    }

    #[test]
    fn limited_range_uses_studio_code_values() {
        let ycc = YCbCr::REC709;
        let white = ycc.encode_limited([1.0; 3]);
        assert!((white[0] * 255.0 - 235.0).abs() < 1e-3, "{white:?}");
        assert!((white[1] * 255.0 - 128.0).abs() < 1e-3, "{white:?}");
        let black = ycc.encode_limited([0.0; 3]);
        assert!((black[0] * 255.0 - 16.0).abs() < 1e-3, "{black:?}");
    }

    #[test]
    fn limited_range_round_trips() {
        for ycc in [YCbCr::REC709, YCbCr::REC2020, YCbCr::REC601] {
            for rgb in [[0.1, 0.5, 0.9], [1.0, 0.0, 0.25], [0.0, 0.0, 0.0]] {
                let back = ycc.decode_limited(ycc.encode_limited(rgb));
                for (a, b) in back.iter().zip(rgb) {
                    assert!((a - b).abs() < 1e-5, "{rgb:?} -> {back:?}");
                }
            }
        }
    }

    #[test]
    fn accepts_standard_coefficients() {
        let rec709 = YCbCr::new(0.2126, 0.0722).unwrap();
        assert_eq!(rec709, YCbCr::REC709);
        assert!((YCbCr::REC2020.kr() - 0.2627).abs() < 1e-6);
    }
}
