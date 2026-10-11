# Colour spaces

Most spaces in `tpt_colour::space` take and return CIE XYZ relative to the D65 white with `Y = 1`.
The HDR spaces are the exception: they take XYZ in cd/m², because their encodings depend on
absolute luminance.

## Working and print spaces

- `AcesCg`: the linear AP1 primaries used in ACES. Its white is D60, so the matrix includes a
  Bradford adaptation to D65.
- `Lab50`: CIE L\*a\*b\* with the D50 white that ICC profiles use for their PCS. Conversion
  adapts between D65 and D50 with Bradford.
- `ScRgb`: linear sRGB with values outside `[0, 1]` allowed. It is the same conversion as
  `LinearSRgb`.
- `Luv`: CIE 1976 L\*u\*v\*.

Chromatic adaptation between any two whites is available directly:

```rust
# extern crate tpt_colour;
use tpt_colour::space::{ChromaticAdaptation, D50, D65, adaptation_matrix};
let to_d50 = adaptation_matrix(ChromaticAdaptation::Cat16, D65, D50).unwrap();
```

`ChromaticAdaptation` has Bradford, CAT02 and CAT16.

## HDR spaces

`xyz_to_ictcp` and `xyz_to_jzazbz` take XYZ in cd/m² with D65 chromaticity. ICtCp is
ITU-R BT.2100-2 with PQ, and Jzazbz is Safdar et al. (2017). Both have inverses. Their PQ curve is
normalised to 10 000 cd/m².

Both are checked against `colour-science` 0.4.7 at 100, 50, 1000 and 0.5 cd/m² test points. The
PQ power is evaluated in f64, because Jzazbz's exponent of about 134 magnifies f32 rounding to
about 1e-5.

## Coding

- `ycocg_encode` and `ycocg_decode` are the float YCoCg.
- `ycocg_r_encode` and `ycocg_r_decode` are YCoCg-R, the reversible integer lifting form. Every
  8-bit colour round-trips exactly, which the tests check across the whole cube.
- `YCbCr::encode_limited` and `decode_limited` give studio-swing code values.
