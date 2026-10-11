# tpt-colour-core

Colour spaces, chromatic adaptation, colour difference and alpha compositing. This crate is
`no_std` and has no dependencies beyond `libm`.

- Spaces: sRGB, linear sRGB, Display P3, Rec. 2020, Adobe RGB (1998), XYZ, Lab, LCh, Oklab,
  Oklch, CMYK (a naive fallback, see the ICC crate for real CMYK), and Y′CbCr (Rec. 601, 709, 2020).
- Models: HSL, HSV, HWB and xyY.
- Colour difference: ΔE 76, 94, 2000 and Oklab (ΔEOK).
- Contrast: WCAG 2 relative luminance and contrast ratio.
- Parsing: CSS-style hex colours.
- Interpolation: gradients in Oklab and Oklch.
- Compositing: premultiplied RGBA, Porter-Duff operators and separable blend modes.

Part of the [tpt-colour](https://github.com/tpt-solutions/tpt-colour) workspace. Most users
should depend on the `tpt-colour` facade instead.
