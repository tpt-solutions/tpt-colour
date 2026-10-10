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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Source-over compositing of premultiplied `src` onto `dst`.
#[must_use]
pub fn blend_over(src: PremultipliedRgba, dst: PremultipliedRgba) -> PremultipliedRgba {
    PorterDuff::SourceOver.compose(src, dst)
}

/// Separable blend modes (W3C Compositing and Blending Level 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl BlendMode {
    /// Blends one straight colour channel. `cb` is the backdrop and `cs` the source.
    #[must_use]
    pub fn blend_channel(self, cb: f32, cs: f32) -> f32 {
        match self {
            Self::Normal => cs,
            Self::Multiply => cb * cs,
            Self::Screen => cb + cs - cb * cs,
            Self::Overlay => hard_light(cs, cb),
            Self::HardLight => hard_light(cb, cs),
            Self::Darken => cb.min(cs),
            Self::Lighten => cb.max(cs),
            Self::Difference => (cb - cs).abs(),
            Self::Exclusion => cb + cs - 2.0 * cb * cs,
        }
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

/// Composites `src` onto `dst` using a separable blend mode, then source-over alpha.
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
    let mix = |ps: f32, pb: f32, cs: f32, cb: f32| {
        (1.0 - dst.a) * ps + (1.0 - src.a) * pb + src.a * dst.a * mode.blend_channel(cb, cs)
    };
    PremultipliedRgba {
        r: mix(src.r, dst.r, s[0], d[0]),
        g: mix(src.g, dst.g, s[1], d[1]),
        b: mix(src.b, dst.b, s[2], d[2]),
        a: ao,
    }
}
