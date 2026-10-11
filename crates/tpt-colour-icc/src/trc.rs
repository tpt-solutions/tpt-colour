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

/// Samples in a baked curve. Linear interpolation over this grid is accurate to about 1e-5
/// for the curves ICC profiles use, which is well below one 8-bit code.
pub(crate) const BAKED_SAMPLES: usize = 4096;

impl Trc {
    /// Builds a table from profile samples. A dip in the samples is replaced by the running
    /// maximum before it is stored. Otherwise the inverse lookup, which binary-searches the
    /// table, would give undefined results on a non-monotonic curve.
    pub(crate) fn sampled(mut samples: Vec<f32>) -> Trc {
        let mut max = f32::NEG_INFINITY;
        for v in &mut samples {
            max = max.max(*v);
            *v = max;
        }
        Trc::Table(samples)
    }

    /// Samples the forward curve (encoded → linear) into a table, so evaluation needs no `powf`.
    ///
    /// Tables, identity and pure power curves are returned unchanged. A power curve with an
    /// exponent below one has an unbounded slope at zero, so linear interpolation in a table is
    /// off by about a code near black. Keeping the power exact avoids that.
    #[must_use]
    pub(crate) fn bake(&self) -> Trc {
        match self {
            Self::Identity | Self::Table(_) | Self::Gamma(_) => self.clone(),
            Self::Parametric { .. } => Self::Table(
                (0..BAKED_SAMPLES)
                    .map(|i| self.eval(i as f32 / (BAKED_SAMPLES - 1) as f32))
                    .collect(),
            ),
        }
    }

    /// Samples the inverse curve (linear → encoded) into a table over the linear domain.
    ///
    /// Evaluate the result with [`Trc::eval`] to get the encoded value. A pure power curve
    /// inverts exactly to another power, `x^(1/γ)`, so it is returned as that instead of a
    /// table. Sampling `x^(1/γ)` uniformly is about 1.6 codes out near black for γ = 2.2.
    #[must_use]
    pub(crate) fn bake_inverse(&self) -> Trc {
        match self {
            Self::Identity => Self::Identity,
            Self::Gamma(g) if *g > 0.0 && g.is_finite() => Self::Gamma(1.0 / g),
            _ => Self::Table(
                (0..BAKED_SAMPLES)
                    .map(|i| self.eval_inverse(i as f32 / (BAKED_SAMPLES - 1) as f32))
                    .collect(),
            ),
        }
    }

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

/// Double-precision evaluation, for the single-colour path. Uses exact curves, not baked tables.
impl Trc {
    /// Encoded → linear in `f64`. The input is clamped to `[0, 1]`.
    #[must_use]
    pub(crate) fn eval64(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        match self {
            Self::Identity => x,
            Self::Gamma(g) => libm::pow(x, f64::from(*g)),
            Self::Table(t) => table_eval64(t, x),
            Self::Parametric { function, params } => {
                let p = params.map(f64::from);
                parametric_eval64(*function, &p, x)
            }
        }
    }

    /// Linear → encoded in `f64`. The input and result are clamped to `[0, 1]`.
    #[must_use]
    pub(crate) fn eval_inverse64(&self, y: f64) -> f64 {
        let y = y.clamp(0.0, 1.0);
        let x = match self {
            Self::Identity => y,
            Self::Gamma(g) => libm::pow(y, 1.0 / f64::from(*g)),
            Self::Table(t) => table_inverse64(t, y),
            Self::Parametric { function, params } => {
                let p = params.map(f64::from);
                parametric_inverse64(*function, &p, y)
            }
        };
        x.clamp(0.0, 1.0)
    }
}

fn table_eval64(t: &[f32], x: f64) -> f64 {
    if t.len() < 2 {
        return x;
    }
    let pos = x * (t.len() - 1) as f64;
    let i = (pos as usize).min(t.len() - 2);
    let frac = pos - i as f64;
    f64::from(t[i]) + (f64::from(t[i + 1]) - f64::from(t[i])) * frac
}

fn table_inverse64(t: &[f32], y: f64) -> f64 {
    if t.len() < 2 {
        return y;
    }
    let i = t.partition_point(|&v| f64::from(v) < y);
    if i == 0 {
        return 0.0;
    }
    if i >= t.len() {
        return 1.0;
    }
    let (v0, v1) = (f64::from(t[i - 1]), f64::from(t[i]));
    let frac = if v1 > v0 { (y - v0) / (v1 - v0) } else { 0.0 };
    ((i - 1) as f64 + frac) / (t.len() - 1) as f64
}

#[allow(clippy::many_single_char_names)] // Parameters are named after the ICC specification.
fn parametric_eval64(function: u16, p: &[f64; 7], x: f64) -> f64 {
    let [g, a, b, c, d, e, f] = *p;
    let base = |x: f64| (a * x + b).max(0.0);
    match function {
        0 => libm::pow(x, g),
        1 => {
            if x >= -b / a {
                libm::pow(base(x), g)
            } else {
                0.0
            }
        }
        2 => {
            if x >= -b / a {
                libm::pow(base(x), g) + c
            } else {
                c
            }
        }
        3 => {
            if x >= d {
                libm::pow(base(x), g)
            } else {
                c * x
            }
        }
        4 => {
            if x >= d {
                libm::pow(base(x), g) + e
            } else {
                c * x + f
            }
        }
        _ => x,
    }
}

#[allow(clippy::many_single_char_names)] // Parameters are named after the ICC specification.
fn parametric_inverse64(function: u16, p: &[f64; 7], y: f64) -> f64 {
    let [g, a, b, c, d, e, f] = *p;
    let inv_g = 1.0 / g;
    match function {
        0 => libm::pow(y, inv_g),
        1 => (libm::pow(y, inv_g) - b) / a,
        2 => {
            if y >= c {
                (libm::pow((y - c).max(0.0), inv_g) - b) / a
            } else {
                -b / a
            }
        }
        3 => {
            if y >= c * d {
                (libm::pow(y, inv_g) - b) / a
            } else {
                y / c
            }
        }
        4 => {
            if y >= c * d + f {
                (libm::pow((y - e).max(0.0), inv_g) - b) / a
            } else {
                (y - f) / c
            }
        }
        _ => y,
    }
}

#[cfg(test)]
mod bake_tests {
    use super::*;

    /// Worst error of the baked inverse against the exact inverse, in 8-bit codes.
    fn worst_codes(curve: &Trc) -> f32 {
        let baked = curve.bake_inverse();
        let mut worst = 0.0_f32;
        for i in 0..=20_000 {
            let y = i as f32 / 20_000.0;
            let exact = curve.eval_inverse(y);
            worst = worst.max((baked.eval(y) - exact).abs() * 255.0);
        }
        worst
    }

    #[test]
    fn baked_inverse_is_within_a_tenth_of_a_code() {
        let srgb = Trc::Parametric {
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
        };
        assert!(worst_codes(&srgb) < 0.1, "sRGB {}", worst_codes(&srgb));
        // Pure power curves are analytic now, so their inverse is exact.
        for g in [2.2_f32, 1.8, 0.45] {
            assert!(worst_codes(&Trc::Gamma(g)) < 0.01, "gamma {g}");
        }
    }

    #[test]
    fn non_monotonic_samples_give_a_monotonic_inverse() {
        // The dip at index 2 is levelled to the running maximum, so the inverse never falls.
        let curve = Trc::sampled(alloc::vec![0.0, 0.5, 0.3, 0.9, 1.0]);
        let mut last = 0.0_f32;
        for i in 0..=100 {
            let y = i as f32 / 100.0;
            let x = curve.eval_inverse(y);
            assert!(x.is_finite() && x >= last - 1e-6, "y {y}: {x} < {last}");
            last = x;
        }
    }
}
