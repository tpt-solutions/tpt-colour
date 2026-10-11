//! Parsing of CSS colour syntax: any of the forms below, through [`parse_colour`].
//!
//! Hex colours are parsed by [`crate::hex`]. Named colours are in [`crate::named`].

use crate::hex::parse_hex;
use crate::space::{ColourSpace, Oklch, SRgb};

pub use crate::named::named_colour;

/// Parses any supported CSS colour into 8-bit `[r, g, b, a]`, ignoring ASCII case.
///
/// Accepted forms: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `oklch()`, the 148
/// named colours, and `transparent`. Out-of-gamut `oklch()` colours are clipped to sRGB.
///
/// # Example
///
/// ```
/// use tpt_colour_core::parse::parse_colour;
/// assert_eq!(parse_colour("rebeccapurple"), Some([102, 51, 153, 255]));
/// assert_eq!(parse_colour("oklch(100% 0 0)"), Some([255, 255, 255, 255]));
/// ```
#[must_use]
pub fn parse_colour(text: &str) -> Option<[u8; 4]> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("transparent") {
        return Some([0, 0, 0, 0]);
    }
    if text.starts_with('#') {
        return parse_hex(text);
    }
    if is_function(text, "rgb") {
        return parse_rgb_function(text);
    }
    if is_function(text, "oklch") {
        return parse_oklch(text);
    }
    let [r, g, b] = named_colour(text)?;
    Some([r, g, b, 255])
}

/// Whether `text` is a call to the colour function `name`, such as `rgb(` or `rgba(`.
fn is_function(text: &str, name: &str) -> bool {
    strip_function(text, name).is_some()
        || (name == "rgb" && strip_function(text, "rgba").is_some())
}

/// Splits the arguments of a colour function. Commas, spaces and `/` all separate values.
/// Returns the values and their count, which must be 3 or 4.
fn arguments(inner: &str) -> Option<([&str; 4], usize)> {
    let mut values = [""; 4];
    let mut count = 0;
    for part in inner
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
    {
        if count == values.len() {
            return None;
        }
        values[count] = part;
        count += 1;
    }
    (count == 3 || count == 4).then_some((values, count))
}

/// Parses an alpha value in `[0, 1]` into 8 bits, rounding to the nearest value.
fn alpha_byte(text: &str) -> Option<u8> {
    let a: f32 = text.parse().ok()?;
    if !(0.0..=1.0).contains(&a) {
        return None;
    }
    // The cast saturates, and `a` is already in range.
    Some((a * 255.0 + 0.5) as u8)
}

/// Converts a gamma-encoded component in `[0, 1]` to 8 bits, clipping first.
fn to_byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Parses `rgb(r, g, b)` or `rgba(r, g, b, a)` into 8-bit `[r, g, b, a]`, ignoring ASCII case.
///
/// Channels are integers from 0 to 255. Alpha is a number from 0 to 1 and is rounded to the
/// nearest 8-bit value, or 255 when omitted. Commas, spaces and `/` all separate the values, so
/// both the legacy comma syntax and the space syntax with `/ alpha` are accepted. Returns `None`
/// for anything else, including percentages, out-of-range values and a wrong number of values.
///
/// # Example
///
/// ```
/// use tpt_colour_core::parse::parse_rgb_function;
/// assert_eq!(parse_rgb_function("rgb(255, 128, 0)"), Some([255, 128, 0, 255]));
/// assert_eq!(parse_rgb_function("rgb(0 0 0 / 0.5)"), Some([0, 0, 0, 128]));
/// ```
#[must_use]
pub fn parse_rgb_function(text: &str) -> Option<[u8; 4]> {
    let text = text.trim();
    let inner = strip_function(text, "rgba").or_else(|| strip_function(text, "rgb"))?;
    let (values, count) = arguments(inner)?;
    let channel = |s: &str| s.parse::<u8>().ok();
    let alpha = if count == 4 {
        alpha_byte(values[3])?
    } else {
        255
    };
    Some([
        channel(values[0])?,
        channel(values[1])?,
        channel(values[2])?,
        alpha,
    ])
}

/// Parses `oklch(L C H)` or `oklch(L C H / A)` into 8-bit sRGB `[r, g, b, a]`.
///
/// `L` is a number from 0 to 1 or a percentage. `C` is a non-negative number. `H` is an angle in
/// degrees, with an optional `deg` suffix. The colour is converted to sRGB and clipped to
/// `[0, 1]`, so out-of-gamut colours lose chroma in the clip rather than in a gamut mapping.
#[must_use]
pub fn parse_oklch(text: &str) -> Option<[u8; 4]> {
    let text = text.trim();
    let inner = strip_function(text, "oklch")?;
    let (values, count) = arguments(inner)?;

    let lightness = match values[0].strip_suffix('%') {
        Some(percent) => percent.parse::<f32>().ok()? / 100.0,
        None => values[0].parse::<f32>().ok()?,
    };
    let chroma: f32 = values[1].parse().ok()?;
    let hue: f32 = values[2]
        .strip_suffix("deg")
        .unwrap_or(values[2])
        .parse()
        .ok()?;
    if !lightness.is_finite() || !chroma.is_finite() || chroma < 0.0 || !hue.is_finite() {
        return None;
    }
    let alpha = if count == 4 {
        alpha_byte(values[3])?
    } else {
        255
    };

    let [r, g, b] = SRgb.from_xyz(Oklch.to_xyz([lightness, chroma, hue]));
    Some([to_byte(r), to_byte(g), to_byte(b), alpha])
}

/// Returns the arguments between `name(` and the closing `)`, ignoring ASCII case in `name`.
fn strip_function<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let rest = text.get(..name.len())?;
    if !rest.eq_ignore_ascii_case(name) {
        return None;
    }
    text[name.len()..].strip_prefix('(')?.strip_suffix(')')
}

#[cfg(test)]
mod tests {
    use super::{named_colour, parse_colour, parse_oklch, parse_rgb_function};

    #[test]
    fn parses_legacy_and_modern_syntax() {
        assert_eq!(
            parse_rgb_function("rgb(255, 128, 0)"),
            Some([255, 128, 0, 255])
        );
        assert_eq!(parse_rgb_function("RGBA(1,2,3,1)"), Some([1, 2, 3, 255]));
        assert_eq!(
            parse_rgb_function("rgb(16 32 48 / 0.25)"),
            Some([16, 32, 48, 64])
        );
        assert_eq!(parse_rgb_function("  rgb(0,0,0)  "), Some([0, 0, 0, 255]));
    }

    #[test]
    fn rejects_bad_input() {
        for bad in [
            "",
            "rgb(1, 2)",
            "rgb(1, 2, 3, 4, 5)",
            "rgb(256, 0, 0)",
            "rgb(-1, 0, 0)",
            "rgb(50%, 0, 0)",
            "rgb(1.5, 0, 0)",
            "rgb(1, 2, 3, 1.5)",
            "rgb(1, 2, 3",
            "hsl(1, 2, 3)",
        ] {
            assert_eq!(parse_rgb_function(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn parses_oklch_against_css_reference_colours() {
        // CSS Color 4 gives this as sRGB red.
        let red = parse_oklch("oklch(62.8% 0.2577 29.23)").unwrap();
        assert!(red[0] >= 254 && red[1] <= 1 && red[2] <= 1, "{red:?}");
        assert_eq!(parse_oklch("oklch(100% 0 0)"), Some([255, 255, 255, 255]));
        assert_eq!(parse_oklch("oklch(0 0 0)"), Some([0, 0, 0, 255]));
        let grey = parse_oklch("oklch(50% 0 90deg)").unwrap();
        assert!(grey[0] == grey[1] && grey[1] == grey[2], "{grey:?}");
        assert_eq!(parse_oklch("oklch(0.5 0.1 200 / 0.5)").unwrap()[3], 128);
    }

    #[test]
    fn rejects_bad_oklch() {
        for bad in [
            "oklch(0.5 -0.1 0)",
            "oklch(0.5 0.1)",
            "oklch(x 0.1 0)",
            "oklch(0.5 0.1 0",
            "lch(1 2 3)",
        ] {
            assert_eq!(parse_oklch(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn named_colours_are_case_insensitive() {
        assert_eq!(named_colour("rebeccapurple"), Some([102, 51, 153]));
        assert_eq!(named_colour("RebeccaPurple"), Some([102, 51, 153]));
        assert_eq!(named_colour("Grey"), named_colour("gray"));
        assert_eq!(named_colour("transparent"), None, "handled by parse_colour");
        assert_eq!(named_colour("bluish"), None);
    }

    #[test]
    fn parse_colour_dispatches_every_form() {
        assert_eq!(parse_colour("#f00"), Some([255, 0, 0, 255]));
        assert_eq!(parse_colour("  rgb(1, 2, 3)  "), Some([1, 2, 3, 255]));
        assert_eq!(parse_colour("RGBA(1 2 3 / 1)"), Some([1, 2, 3, 255]));
        assert_eq!(parse_colour("white"), Some([255, 255, 255, 255]));
        assert_eq!(parse_colour("TRANSPARENT"), Some([0, 0, 0, 0]));
        assert!(parse_colour("oklch(1 0 0)").is_some());
        assert_eq!(parse_colour("notacolour"), None);
        assert_eq!(parse_colour("rgb(1, 2, 3"), None);
    }

    #[test]
    fn non_ascii_input_is_rejected_without_panicking() {
        for text in ["é", "rgbé(1,2,3)", "oklchä(0 0 0)", "#é", "ééééééé"] {
            assert_eq!(parse_colour(text), None, "{text:?}");
        }
    }
}
