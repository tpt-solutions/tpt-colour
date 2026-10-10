//! Tone reproduction curves (TRCs): encoded ⇄ linear, for gamma, sampled and parametric curves.

use alloc::vec::Vec;

use crate::math::{powf, valid_domain};

/// A tone reproduction curve.
///
/// [`Trc::eval`] maps encoded device values to linear light. [`Trc::eval_inverse`]
/// maps linear light back to encoded values.
#[derive(Clone, Debug, PartialEq)]
pub enum Trc {
    /// Identity: output equals input.
    Identity,
    /// Pure power curve `x^γ`.
    Gamma(f32),
    /// Sampled curve over `[0, 1]`, linearly interpolated.
    Table(Vec<f32>),
    /// ICC parametric curve (`para`), function type `0..=4`.
    ///
    /// `params` holds `g, a, b, c, d, e, f` in that order; unused entries are zero.
    Parametric {
        /// ICC function type (`0..=4`).
        function: u16,
        /// Parameters `[g, a, b, c, d, e, f]`.
        params: [f32; 7],
    },
}

impl Trc {
    /// Encoded → linear. The input is clamped to `[0, 1]`.
    #[must_use]
    pub fn eval(&self, x: f32) -> f32 {
        let x = valid_domain(x);
        match self {
            Self::Identity => x,
            Self::Gamma(g) => powf(x, *g),
            Self::Table(t) => table_eval(t, x),
            Self::Parametric { function, params } => parametric_eval(*function, params, x),
        }
    }

    /// Linear → encoded. The input is clamped to `[0, 1]`, and so is the result.
    ///
    /// Tables assume a non-decreasing sequence of samples.
    #[must_use]
    pub fn eval_inverse(&self, y: f32) -> f32 {
        let y = valid_domain(y);
        let x = match self {
            Self::Identity => y,
            Self::Gamma(g) => powf(y, 1.0 / *g),
            Self::Table(t) => table_inverse(t, y),
            Self::Parametric { function, params } => parametric_inverse(*function, params, y),
        };
        valid_domain(x)
    }
}

fn table_eval(t: &[f32], x: f32) -> f32 {
    if t.len() < 2 {
        return x;
    }
    let pos = x * (t.len() - 1) as f32;
    let i = (pos as usize).min(t.len() - 2);
    let frac = pos - i as f32;
    t[i] + (t[i + 1] - t[i]) * frac
}

fn table_inverse(t: &[f32], y: f32) -> f32 {
    if t.len() < 2 {
        return y;
    }
    let i = t.partition_point(|&v| v < y);
    if i == 0 {
        return 0.0;
    }
    if i >= t.len() {
        return 1.0;
    }
    let (v0, v1) = (t[i - 1], t[i]);
    let frac = if v1 > v0 { (y - v0) / (v1 - v0) } else { 0.0 };
    ((i - 1) as f32 + frac) / (t.len() - 1) as f32
}

#[allow(clippy::many_single_char_names)] // Parameters are named after the ICC specification.
fn parametric_eval(function: u16, p: &[f32; 7], x: f32) -> f32 {
    let [g, a, b, c, d, e, f] = *p;
    match function {
        0 => powf(x, g),
        1 => {
            if x >= -b / a {
                powf((a * x + b).max(0.0), g)
            } else {
                0.0
            }
        }
        2 => {
            if x >= -b / a {
                powf((a * x + b).max(0.0), g) + c
            } else {
                c
            }
        }
        3 => {
            if x >= d {
                powf((a * x + b).max(0.0), g)
            } else {
                c * x
            }
        }
        4 => {
            if x >= d {
                powf((a * x + b).max(0.0), g) + e
            } else {
                c * x + f
            }
        }
        _ => x,
    }
}

#[allow(clippy::many_single_char_names)] // Parameters are named after the ICC specification.
fn parametric_inverse(function: u16, p: &[f32; 7], y: f32) -> f32 {
    let [g, a, b, c, d, e, f] = *p;
    let inv_g = 1.0 / g;
    match function {
        0 => powf(y, inv_g),
        1 => (powf(y, inv_g) - b) / a,
        2 => {
            if y >= c {
                (powf((y - c).max(0.0), inv_g) - b) / a
            } else {
                -b / a
            }
        }
        3 => {
            if y >= c * d {
                (powf(y, inv_g) - b) / a
            } else {
                y / c
            }
        }
        4 => {
            if y >= c * d + f {
                (powf((y - e).max(0.0), inv_g) - b) / a
            } else {
                (y - f) / c
            }
        }
        _ => y,
    }
}
