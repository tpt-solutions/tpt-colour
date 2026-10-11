# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/). The crates are pre-1.0, so breaking changes can
appear in minor releases until 1.0.0.

## [Unreleased]

### Known limitations (v1.0)

- Perceptual device-side (B2A) output for sRGB into the official v4 LUT profile differs from LittleCMS by up to 12 codes at some colours. Relative intent matches within one code.
- The `mft1`/`mft2` 3×3 matrix is ignored. It only applies to XYZ-encoded PCS in B2A, which is rare.
- `multiProcessElements` (`mpet`) tables are not read. Intents that need them report `Unsupported`.
- Gamut mapping for LUT destination profiles is not done. Their own perceptual tables handle gamut compression.

### Changed (breaking)

- `tpt_colour_core::space::YCbCr` fields are private. Use `YCbCr::new` (validated, returns
  `Option`) and the `kr()` and `kb()` getters.
- `tpt_colour_manage::Gamut::from_space` returns `Result`. The built-in `srgb()`,
  `display_p3()` and `rec2020()` constructors are unchanged.
- `ColourManager::transform` uses `ColourTransform::new` for black point compensation, as
  LittleCMS does. Relative colorimetric transforms no longer compensate unless requested.
- Public enums in the core, ICC and manage crates are `#[non_exhaustive]`:
  `PorterDuff`, `BlendMode`, `IccError` and `ManageError`.
- `tone_map_xyz` limits only `Y` to `[0, 1]`. `X` and `Z` keep their chromaticity.
- `tpt-colour-core`, `tpt-colour-icc` and `tpt-colour-manage` no longer declare the `std`
  feature, and the core crate no longer declares `simd`. None of them were used.

### Changed (behaviour)

- CLUTs with more than eight inputs are `Unsupported`, not parsed. Multilinear lookup touches 2^n corners per pixel, so the cap bounds work per pixel. Malformed tables stay `BadTag`.

### Fixed

- `with_white_patched` could panic when a curve output left `[0, 1]`. CLUT inputs are now
  clamped in `Pipeline::eval`, as LittleCMS does.
- Absolute colorimetric intent now scales by the source and destination media white points
  (`wtpt`). It previously equalled relative colorimetric.
- Perceptual transforms to v4 destinations skip black point compensation when the two black
  points are equal, which keeps the fast paths.
- A bad or unused LUT tag, or an unreadable `wtpt` or `chad`, no longer fails the whole profile.
  The affected intent reports `Unsupported`.
- Transfer curves for sRGB, Display P3 and Rec. 2020 are odd-symmetric about zero (scRGB
  convention). Before, negative inputs used a linear line that did not mirror the power
  segment above 0.04. In-range values are unchanged.
- Pure power curves are kept analytic when baked. The table inverse was about 1.6 codes out
  near black for γ = 2.2.
- Sampled curves with dips are levelled to a running maximum, so their inverse is monotonic.
- NaN inputs no longer leak through pipelines, the tone mapper, the matrix inverse or the gamut
  mapper.
- `map_to_gamut` returned the last clip tried, even if it was rejected. It now returns the clip
  at the last accepted chroma.
- Hue from `to_polar` is never `-0.0` or exactly `360.0`.
- The RGB matrices are derived from one D65 white, so sRGB, Display P3 and Rec. 2020 share it
  exactly. The previous constants disagreed in the fourth decimal place of the Z white.
- The ICC description is read from the `desc` tag only. The previous lookups for `mluc` and
  `text` could never match a tag name.

### Added

- Colour models: HSL, HSV, HWB and xyY (`tpt_colour_core::models`).
- Colour difference: ΔE 76, 94, CIEDE2000 and Oklab (`tpt_colour_core::difference`).
- WCAG 2 contrast ratio (`tpt_colour_core::contrast`).
- CSS-style hex parsing (`tpt_colour_core::hex`).
- Oklab and Oklch gradient interpolation (`tpt_colour_core::mix`).
- Adobe RGB (1998) space (`AdobeRgb`) and Rec. 601 Y′CbCr (`YCbCr::REC601`).
- Blend modes: colour dodge, colour burn and soft light, and the non-separable Hue,
  Saturation, Color and Luminosity. `BlendMode::blend_rgb` blends whole colours, and
  `is_separable` tells the two kinds apart.
- Grey (`GRAY`, `kTRC`) ICC profiles, for matrix-free transforms to and from RGB.
- `ColourTransform::bake_clut`, which samples a transform onto a 3-D grid as a `Pipeline`.
  For Display P3 → sRGB, the mean error is 0.3 codes on a 33-point grid.
- `ColourTransform::to_cube`, which writes an Adobe `.cube` 3-D LUT. CLI: `tpt-colour cube`.
- `ColourTransform::transform_u8`, for interleaved 8-bit RGB or RGBA buffers.
- ProPhoto RGB (`ProPhotoRgb`), with its D50 white adapted to D65.
- `mix_premultiplied_oklab`, gradients where a transparent end adds no colour.
- `FromStr` for `RenderingIntent`, with `ParseIntentError`.
- Doctests on the new core functions.
- Fuzz targets `lut_eval` and `transform_full`, with seed corpora. The CI fuzz job runs all three. A replay test runs the same logic over the seeds on every platform.
- `tpt-colour doctor`, which reports multiProcessElements tables, missing descriptions and tags, and sRGB round-trip error.
- LittleCMS fixtures for the device-side (B2A) direction, generated by `tools/oracle/generate_b2a_fixtures.py`. The relative case passes. The perceptual case is ignored, see TODO.md.
- `tpt-colour-cli` (binary `tpt-colour`, not published): `info`, `convert` and `gamut-check`.
- `PremultipliedRgba::new`, which clamps its inputs to keep the premultiplied invariant.
- HLG OETF and inverse (`tpt_colour_manage::tone::hlg_oetf` and `hlg_eotf`).
- `RenderingIntent` and `tone` re-exported from the facade, and a `prelude` module.
- Cookbook examples: `composite_layers`, `hdr_tonemap`, `batch_pixels`, `ycbcr_video`,
  `parse_and_gradient` and `wcag_check`.
- A per-profile CLUT budget (32 MiB of samples), which limits memory for hostile profiles.
- `ManageError::source` returns the underlying ICC error.

### Performance

- The `ColourManager` cache is keyed by `&str` lookups, so a cache hit allocates nothing.
- Black point detection builds its round-trip pipelines once, not once per sample.
- Gamma-only profiles evaluate their power curves directly, not through a table. That is
  slower per pixel, but exact.

## [0.1.0]

Initial workspace: colour spaces, compositing, ICC v2/v4 matrix/TRC and LUT transforms,
gamut mapping, `ColourManager`, and the LittleCMS oracle tests.
