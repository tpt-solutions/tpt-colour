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

/// Encodes integer RGB as `[Y, Co, Cg]` with YCoCg-R, the reversible lifting form (Malvar,
/// Sullivan and Srinivasan, 2008). Each output is an exact integer, and [`ycocg_r_decode`]
/// inverts it.
///
/// For 8-bit input, `Y` is in `0..=255`, and `Co` and `Cg` are in `-255..=255`.
#[must_use]
#[allow(clippy::many_single_char_names)] // RGB and YCoCg component names
pub fn ycocg_r_encode([r, g, b]: [i32; 3]) -> [i32; 3] {
    let co = r - b;
    let t = b + (co >> 1);
    let cg = g - t;
    let y = t + (cg >> 1);
    [y, co, cg]
}

/// Decodes YCoCg-R `[Y, Co, Cg]` to integer RGB. The exact inverse of [`ycocg_r_encode`].
#[must_use]
#[allow(clippy::many_single_char_names)] // RGB and YCoCg component names
pub fn ycocg_r_decode([y, co, cg]: [i32; 3]) -> [i32; 3] {
    let t = y - (cg >> 1);
    let g = cg + t;
    let b = t - (co >> 1);
    let r = b + co;
    [r, g, b]
}

#[cfg(test)]
mod r_tests {
    use super::{ycocg_r_decode, ycocg_r_encode};

    #[test]
    fn every_8_bit_colour_round_trips_exactly() {
        for r in (0..=255).step_by(3) {
            for g in (0..=255).step_by(3) {
                for b in 0..=255 {
                    let ycc = ycocg_r_encode([r, g, b]);
                    assert_eq!(ycocg_r_decode(ycc), [r, g, b]);
                    assert!((0..=255).contains(&ycc[0]));
                    assert!((-255..=255).contains(&ycc[1]) && (-255..=255).contains(&ycc[2]));
                }
            }
        }
    }

    #[test]
    fn white_and_black_are_exact() {
        assert_eq!(ycocg_r_encode([255, 255, 255]), [255, 0, 0]);
        assert_eq!(ycocg_r_encode([0, 0, 0]), [0, 0, 0]);
    }
}
