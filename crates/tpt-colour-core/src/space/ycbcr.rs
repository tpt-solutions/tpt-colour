//! Y′CbCr ⇄ RGB matrices, full range (Cb and Cr centred on zero).

/// Luma coefficients for a Y′CbCr system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YCbCr {
    /// Red luma weight `K_R`.
    pub kr: f32,
    /// Blue luma weight `K_B`.
    pub kb: f32,
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
}
