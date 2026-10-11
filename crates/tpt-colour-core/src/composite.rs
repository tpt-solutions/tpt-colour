//! Alpha compositing: premultiplied pixels, Porter-Duff operators and separable blend modes.
//!
//! Colour channels are premultiplied by alpha unless a function says otherwise.
//! Premultiplied values are used for arithmetic so that alpha 0 produces no
//! dark fringes when interpolating.

/// An RGBA pixel with colour channels premultiplied by alpha, all in `[0, 1]`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PremultipliedRgba {
    /// Premultiplied red.
    pub r: f32,
    /// Premultiplied green.
    pub g: f32,
    /// Premultiplied blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

impl PremultipliedRgba {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Builds a pixel from premultiplied channels, clamping alpha to `[0, 1]` and each colour
    /// channel to `[0, alpha]`, so the invariant holds. NaN becomes 0.
    ///
    /// The fields are public, so a struct literal can still break the invariant. Use this
    /// constructor when the input is untrusted.
    #[must_use]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        let a = unit_or_zero(a);
        let channel = |v: f32| unit_or_zero(v).min(a);
        Self {
            r: channel(r),
            g: channel(g),
            b: channel(b),
            a,
        }
    }

    /// Converts straight (non-premultiplied) RGBA to premultiplied form.
    #[must_use]
    pub fn from_straight([r, g, b, a]: [f32; 4]) -> Self {
        Self {
            r: r * a,
            g: g * a,
            b: b * a,
            a,
        }
    }

    /// Converts back to straight RGBA. Fully transparent pixels return black.
    #[must_use]
    pub fn to_straight(self) -> [f32; 4] {
        if self.a <= 0.0 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        [self.r / self.a, self.g / self.a, self.b / self.a, self.a]
    }

    fn as_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    fn from_array([r, g, b, a]: [f32; 4]) -> Self {
        Self { r, g, b, a }
    }
}

/// Porter-Duff compositing operators (Porter & Duff, 1984).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PorterDuff {
    /// Result is transparent.
    Clear,
    /// Source only.
    Source,
    /// Destination only.
    Destination,
    /// Source composited over destination. The standard "over".
    SourceOver,
    /// Destination composited over source.
    DestinationOver,
    /// Source where destination is opaque.
    SourceIn,
    /// Destination where source is opaque.
    DestinationIn,
    /// Source where destination is transparent.
    SourceOut,
    /// Destination where source is transparent.
    DestinationOut,
    /// Source over destination, only where destination is present.
    SourceAtop,
    /// Destination over source, only where source is present.
    DestinationAtop,
    /// Source or destination, but not both.
    Xor,
}

impl PorterDuff {
    /// Returns the `(Fa, Fb)` coefficients for source and destination alpha.
    fn factors(self, sa: f32, da: f32) -> (f32, f32) {
        match self {
            Self::Clear => (0.0, 0.0),
            Self::Source => (1.0, 0.0),
            Self::Destination => (0.0, 1.0),
            Self::SourceOver => (1.0, 1.0 - sa),
            Self::DestinationOver => (1.0 - da, 1.0),
            Self::SourceIn => (da, 0.0),
            Self::DestinationIn => (0.0, sa),
            Self::SourceOut => (1.0 - da, 0.0),
            Self::DestinationOut => (0.0, 1.0 - sa),
            Self::SourceAtop => (da, 1.0 - sa),
            Self::DestinationAtop => (1.0 - da, sa),
            Self::Xor => (1.0 - da, 1.0 - sa),
        }
    }

    /// Composites `src` onto `dst` with this operator. Both are premultiplied.
    #[must_use]
    pub fn compose(self, src: PremultipliedRgba, dst: PremultipliedRgba) -> PremultipliedRgba {
        let (fa, fb) = self.factors(src.a, dst.a);
        let s = src.as_array();
        let d = dst.as_array();
        PremultipliedRgba::from_array([
            s[0] * fa + d[0] * fb,
            s[1] * fa + d[1] * fb,
            s[2] * fa + d[2] * fb,
            s[3] * fa + d[3] * fb,
        ])
    }
}

/// Error from parsing a Porter-Duff or blend mode name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseNameError;

impl core::fmt::Display for ParseNameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("unknown compositing or blend mode name")
    }
}

impl core::error::Error for ParseNameError {}

/// Implements `Display` (the lowercase kebab-case name) and `FromStr` (ignoring ASCII case) from
/// one table, so the two cannot drift apart. Uses no `alloc`.
macro_rules! named_enum {
    ($ty:ident { $($variant:ident => $name:literal),+ $(,)? }) => {
        impl core::fmt::Display for $ty {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let name = match self {
                    $(Self::$variant => $name,)+
                };
                f.write_str(name)
            }
        }

        impl core::str::FromStr for $ty {
            type Err = ParseNameError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $(
                    if s.eq_ignore_ascii_case($name) {
                        return Ok(Self::$variant);
                    }
                )+
                Err(ParseNameError)
            }
        }
    };
}

named_enum!(PorterDuff {
    Clear => "clear",
    Source => "source",
    Destination => "destination",
    SourceOver => "source-over",
    DestinationOver => "destination-over",
    SourceIn => "source-in",
    DestinationIn => "destination-in",
    SourceOut => "source-out",
    DestinationOut => "destination-out",
    SourceAtop => "source-atop",
    DestinationAtop => "destination-atop",
    Xor => "xor",
});

named_enum!(BlendMode {
    Normal => "normal",
    Multiply => "multiply",
    Screen => "screen",
    Overlay => "overlay",
    HardLight => "hard-light",
    Darken => "darken",
    Lighten => "lighten",
    Difference => "difference",
    Exclusion => "exclusion",
    ColorDodge => "color-dodge",
    ColorBurn => "color-burn",
    SoftLight => "soft-light",
    Hue => "hue",
    Saturation => "saturation",
    Color => "color",
    Luminosity => "luminosity",
});

/// Source-over compositing of premultiplied `src` onto `dst`.
#[must_use]
pub fn blend_over(src: PremultipliedRgba, dst: PremultipliedRgba) -> PremultipliedRgba {
    PorterDuff::SourceOver.compose(src, dst)
}

/// Separable blend modes (W3C Compositing and Blending Level 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendMode {
    /// Source replaces destination (no blending). Equivalent to source-over.
    Normal,
    /// `cb × cs`.
    Multiply,
    /// `cb + cs − cb × cs`.
    Screen,
    /// Multiply or screen depending on the destination.
    Overlay,
    /// Multiply or screen depending on the source.
    HardLight,
    /// `min(cb, cs)`.
    Darken,
    /// `max(cb, cs)`.
    Lighten,
    /// `|cb − cs|`.
    Difference,
    /// `cb + cs − 2 × cb × cs`.
    Exclusion,
    /// Brightens the backdrop to reflect the source. `cb` if `cb` is 0, `1` if `cs` is 1.
    ColorDodge,
    /// Darkens the backdrop to reflect the source. `1 − (1 − cb) / cs`, clamped.
    ColorBurn,
    /// Pegtop's soft light, as in the W3C specification.
    SoftLight,
    /// Hue of the source with the saturation and luminosity of the backdrop. Non-separable.
    Hue,
    /// Saturation of the source with the hue and luminosity of the backdrop. Non-separable.
    Saturation,
    /// Hue and saturation of the source with the luminosity of the backdrop. Non-separable.
    Color,
    /// Luminosity of the source with the hue and saturation of the backdrop. Non-separable.
    Luminosity,
}

impl BlendMode {
    /// Whether the mode acts on each channel on its own. The non-separable modes (`Hue`,
    /// `Saturation`, `Color`, `Luminosity`) mix the three channels together.
    #[must_use]
    pub fn is_separable(self) -> bool {
        !matches!(
            self,
            Self::Hue | Self::Saturation | Self::Color | Self::Luminosity
        )
    }

    /// Blends one straight colour channel. `cb` is the backdrop and `cs` the source.
    ///
    /// The result is in `[0, 1]` for inputs in `[0, 1]`. For the non-separable modes there is no
    /// per-channel result, so this returns the source channel. Use [`BlendMode::blend_rgb`].
    #[must_use]
    pub fn blend_channel(self, cb: f32, cs: f32) -> f32 {
        match self {
            Self::Normal | Self::Hue | Self::Saturation | Self::Color | Self::Luminosity => cs,
            Self::Multiply => cb * cs,
            Self::Screen => cb + cs - cb * cs,
            Self::Overlay => hard_light(cs, cb),
            Self::HardLight => hard_light(cb, cs),
            Self::Darken => cb.min(cs),
            Self::Lighten => cb.max(cs),
            Self::Difference => (cb - cs).abs(),
            Self::Exclusion => cb + cs - 2.0 * cb * cs,
            Self::ColorDodge => color_dodge(cb, cs),
            Self::ColorBurn => color_burn(cb, cs),
            Self::SoftLight => soft_light(cb, cs),
        }
    }

    /// Blends a whole straight colour: backdrop `cb` and source `cs`, both in `[0, 1]`.
    ///
    /// Separable modes apply [`BlendMode::blend_channel`] to each channel. The non-separable
    /// modes follow W3C Compositing and Blending Level 1, which uses luminosity and saturation.
    /// The arithmetic is in the encoded (gamma) values, as the specification's examples are.
    /// For linear-light blending, convert to linear first.
    #[must_use]
    pub fn blend_rgb(self, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
            Self::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
            Self::Color => set_lum(cs, lum(cb)),
            Self::Luminosity => set_lum(cb, lum(cs)),
            _ => [0, 1, 2].map(|i| self.blend_channel(cb[i], cs[i])),
        }
    }
}

/// Luminosity from the W3C weights.
fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// Saturation: the spread between the largest and smallest channel.
fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// Pulls out-of-range colours back into `[0, 1]` along the line to their luminosity.
fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 && l - n > 0.0 {
        out = out.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 && x - l > 0.0 {
        out = out.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    out
}

/// Sets the luminosity of `c` to `l`, then clips it into gamut.
fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

/// Sets the saturation of `c` to `s`, keeping the hue. A flat colour becomes black.
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let (mut lo, mut hi) = (0_usize, 0_usize);
    for i in 1..3 {
        if c[i] < c[lo] {
            lo = i;
        }
        if c[i] > c[hi] {
            hi = i;
        }
    }
    let mut out = [0.0_f32; 3];
    let (cmin, cmax) = (c[lo], c[hi]);
    if cmax > cmin {
        // With lo != hi, the remaining index is the middle channel.
        let mid = 3 - lo - hi;
        out[mid] = (c[mid] - cmin) * s / (cmax - cmin);
        out[hi] = s;
    }
    out
}

fn color_dodge(cb: f32, cs: f32) -> f32 {
    if cb <= 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

fn color_burn(cb: f32, cs: f32) -> f32 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

fn soft_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
    } else {
        // D(x) from the W3C specification.
        let d = if cb <= 0.25 {
            ((16.0 * cb - 12.0) * cb + 4.0) * cb
        } else {
            crate::math::sqrt(cb)
        };
        cb + (2.0 * cs - 1.0) * (d - cb)
    }
}

fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        cb * 2.0 * cs
    } else {
        let s = 2.0 * cs - 1.0;
        cb + s - cb * s
    }
}

/// Composites `src` onto `dst` using any blend mode, then source-over alpha.
///
/// Implements `Co = (1 − αb)·Cs + (1 − αs)·Cb + αs·αb·B(Cb, Cs)` on premultiplied
/// channels, where `Cs` and `Cb` are the straight colours.
#[must_use]
pub fn blend(mode: BlendMode, src: PremultipliedRgba, dst: PremultipliedRgba) -> PremultipliedRgba {
    let ao = src.a + dst.a - src.a * dst.a;
    if ao <= 0.0 {
        return PremultipliedRgba::TRANSPARENT;
    }
    let s = src.to_straight();
    let d = dst.to_straight();
    let blended = mode.blend_rgb([d[0], d[1], d[2]], [s[0], s[1], s[2]]);
    let mix =
        |ps: f32, pb: f32, b: f32| (1.0 - dst.a) * ps + (1.0 - src.a) * pb + src.a * dst.a * b;
    PremultipliedRgba {
        r: mix(src.r, dst.r, blended[0]),
        g: mix(src.g, dst.g, blended[1]),
        b: mix(src.b, dst.b, blended[2]),
        a: ao,
    }
}

/// `v` clamped to `[0, 1]`, with NaN mapped to 0.
fn unit_or_zero(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

#[cfg(test)]
mod name_tests {
    extern crate std;
    use super::{BlendMode, ParseNameError, PorterDuff};
    use std::string::ToString;

    #[test]
    fn names_round_trip_through_display_and_fromstr() {
        for op in [PorterDuff::Clear, PorterDuff::SourceAtop, PorterDuff::Xor] {
            assert_eq!(op.to_string().parse::<PorterDuff>(), Ok(op));
        }
        for mode in [
            BlendMode::Normal,
            BlendMode::HardLight,
            BlendMode::Luminosity,
        ] {
            assert_eq!(mode.to_string().parse::<BlendMode>(), Ok(mode));
        }
    }

    #[test]
    fn parsing_ignores_ascii_case_and_rejects_unknown_names() {
        assert_eq!(
            "Source-Over".parse::<PorterDuff>(),
            Ok(PorterDuff::SourceOver)
        );
        assert_eq!("MULTIPLY".parse::<BlendMode>(), Ok(BlendMode::Multiply));
        assert_eq!("over".parse::<PorterDuff>(), Err(ParseNameError));
        assert_eq!("multiply".parse::<PorterDuff>(), Err(ParseNameError));
    }
}
