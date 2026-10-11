# Gamut mapping

Gamut mapping decides what a colour outside the destination's gamut becomes. tpt-colour has
two tools.

## Perceptual intent mapping

When a `ColourManager` transform uses the perceptual intent and the destination has a gamut
boundary, out-of-gamut colours are mapped with the CSS Color 4 method: chroma is reduced in OKLCH
by a binary search on ΔEOK, keeping lightness and hue. The function is `map_to_gamut`.

```rust
# extern crate tpt_colour;
use tpt_colour::{Gamut, map_to_gamut};
use tpt_colour::space::{ColourSpace, SRgb};

let srgb = Gamut::srgb();
let wide = SRgb.to_xyz([0.0, 1.0, 0.0]); // saturated green, outside sRGB as a P3 colour
let mapped = map_to_gamut(&srgb, wide);
```

`Gamut` describes a boundary in XYZ. It has constructors for sRGB, Display P3 and Rec. 2020, and
`Gamut::from_profile` reads one from an ICC profile.

## Destinations with a LUT

Profiles with LUT tables (`A2B`/`B2A`) already contain their own compression for the perceptual
intent, so tpt-colour does not add a second boundary on top. Mapping twice would compress
colours that did not need it.

## Tone mapping is separate

HDR tone mapping (PQ, HLG and extended Reinhard on luminance) lives in `tpt_colour::tone`. It is
not gamut mapping: it reduces luminance range, not chroma.
