//! `YCoCg`, the non-reversible form (Malvar, Sullivan & Srinivasan 2003), in floating point.
//!
//! Unlike [`super::YCbCr`], the coefficients are fixed and need no constants: `Y` is luma and
//! `Co`/`Cg` are orange and green chroma. Chroma is centred on zero. Both directions take
//! gamma-encoded RGB and are exact inverses of each other, up to rounding.

/// Encodes gamma-encoded RGB as `[Y, Co, Cg]`.
#[must_use]
pub fn ycocg_encode([r, g, b]: [f32; 3]) -> [f32; 3] {
    [
        0.25 * r + 0.5 * g + 0.25 * b,
        0.5 * r - 0.5 * b,
        -0.25 * r + 0.5 * g - 0.25 * b,
    ]
}

/// Decodes `[Y, Co, Cg]` to gamma-encoded RGB. The inverse of [`ycocg_encode`].
#[must_use]
pub fn ycocg_decode([y, co, cg]: [f32; 3]) -> [f32; 3] {
    let t = y - cg;
    [t + co, y + cg, t - co]
}

#[cfg(test)]
mod tests {
    use super::{ycocg_decode, ycocg_encode};

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn primaries_have_the_documented_values() {
        assert!(close(ycocg_encode([1.0, 0.0, 0.0]), [0.25, 0.5, -0.25]));
        assert!(close(ycocg_encode([0.0, 1.0, 0.0]), [0.5, 0.0, 0.5]));
        assert!(close(ycocg_encode([0.0, 0.0, 1.0]), [0.25, -0.5, -0.25]));
    }

    #[test]
    fn round_trips() {
        for rgb in [
            [0.1, 0.5, 0.9],
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            [0.3, 0.0, 0.7],
        ] {
            let back = ycocg_decode(ycocg_encode(rgb));
            for (a, b) in back.iter().zip(rgb) {
                assert!((a - b).abs() < 1e-6, "{rgb:?} -> {back:?}");
            }
        }
    }
}
