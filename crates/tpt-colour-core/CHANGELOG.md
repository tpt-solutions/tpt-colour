# Changelog

All notable changes to `tpt-colour-core` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate uses
[Semantic Versioning](https://semver.org/). It is pre-1.0, so breaking changes can appear in minor
releases until 1.0.0.

The workspace-wide log is in the [root changelog](../../CHANGELOG.md).

## [Unreleased]

### Added

- Colour spaces: Adobe RGB (`AdobeRgb`), ProPhoto RGB (`ProPhotoRgb`), ACEScg (`AcesCg`), CIE
  L\*a\*b\* with a D50 white (`Lab50`), CIE L\*u\*v\* (`Luv`) and Rec. 601 Y′CbCr (`YCbCr::REC601`).
- `YCbCr::encode_limited` and `decode_limited` for studio-swing code values (16 to 235, and 16 to
  240 for chroma).
- HDR spaces: `xyz_to_ictcp`, `ictcp_to_xyz`, `xyz_to_jzazbz` and `jzazbz_to_xyz`. They take XYZ in
  cd/m².
- YCoCg (`ycocg_encode`, `ycocg_decode`) and YCoCg-R (`ycocg_r_encode`, `ycocg_r_decode`). YCoCg-R is
  exact on every 8-bit colour.
- `ChromaticAdaptation` and `adaptation_matrix`: Bradford, CAT02 and CAT16 between any two whites.
- Colour models: HSL, HSV, HWB and xyY (`models`).
- Colour difference: ΔE 76, ΔE 94, CIEDE2000 and Oklab (`difference`).
- WCAG 2 contrast ratio (`contrast::wcag_contrast`) and APCA 0.0.98G-4g lightness contrast
  (`contrast::apca_contrast`).
- CSS parsing (`parse`): `parse_colour`, `parse_rgb_function` for `rgb()` and `rgba()`, and
  `parse_oklch`. They cover the 148 CSS named colours and `transparent`.
- CSS-style hex parsing (`hex::parse_hex`).
- Gradients in Oklab and Oklch (`mix::mix_oklab`, `mix::mix_oklch`), and `mix_premultiplied_oklab`,
  where a transparent end adds no colour.
- Palettes (`palette`): `hue_rotate`, `complement`, `analogous`, `triadic` and
  `split_complementary`, in Oklch with a chroma-reducing sRGB fit.
- Colour-vision simulation (`vision::simulate`) for protanopia, deuteranopia and tritanopia (Machado
  et al., 2009).
- Blend modes: colour dodge, colour burn and soft light. The non-separable modes (hue, saturation,
  colour and luminosity) are included. `BlendMode::blend_rgb` blends whole colours, and
  `is_separable` tells the two kinds apart.
- `PremultipliedRgba::new`, which clamps its inputs to keep the premultiplied invariant.
- `Display` and `FromStr` for `PorterDuff` and `BlendMode`. Parsing ignores ASCII case.
- Doctests on the new functions.
- Optional `serde` feature, which derives `Serialize` and `Deserialize` on the compositing and
  vision types. It is off by default.

### Changed (breaking)

- `YCbCr` fields are private. Use `YCbCr::new`, which validates and returns an `Option`, and the
  `kr()` and `kb()` getters.
- `PorterDuff` and `BlendMode` are `#[non_exhaustive]`.
- The crate no longer declares a `std` feature or a `simd` feature. Neither was used.

### Fixed

- The RGB matrices are derived from one D65 white, so sRGB, Display P3 and Rec. 2020 share it
  exactly. The previous constants disagreed in the fourth decimal place of the Z white.
- Transfer curves for sRGB, Display P3 and Rec. 2020 are odd-symmetric about zero (scRGB
  convention). Negative inputs previously used a linear line that did not mirror the power
  segment. In-range values are unchanged.
- Hue from `to_polar` is never `-0.0` or exactly `360.0`.

## [0.1.0]

Initial release of the workspace. The crate was unpublished at this point.
