# Changelog

All notable changes to `tpt-colour-manage` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate uses
[Semantic Versioning](https://semver.org/). It is pre-1.0, so breaking changes can appear in minor
releases until 1.0.0.

The workspace-wide log is in the [root changelog](../../CHANGELOG.md).

## [Unreleased]

### Added

- `SoftProof`: proofs a working-space colour through a proof profile, with up to four channels, and
  flags out-of-gamut colours by ΔE₀₀. `with_threshold` sets the threshold, which is 2.0 by default.
- `ColourManager::with_capacity`: a cache cap with least-recently-used eviction.
- `ManageError::source` returns the underlying ICC error.
- HLG OETF and inverse (`tone::hlg_oetf` and `tone::hlg_eotf`).
- Optional `serde` feature, which forwards to the core and ICC crates, and a `simd` feature that
  forwards to the ICC crate. Both are off by default.

### Changed (breaking)

- `Gamut::from_space` returns `Result`. The built-in `srgb()`, `display_p3()` and `rec2020()`
  constructors are unchanged.
- `ColourManager::transform` uses `ColourTransform::new` for black point compensation, as LittleCMS
  does. Relative colorimetric transforms no longer compensate unless requested.
- `ManageError` is `#[non_exhaustive]`.
- `tone_map_xyz` limits only `Y` to `[0, 1]`. `X` and `Z` keep their chromaticity.

### Changed (behaviour)

- The `ColourManager` cache is keyed by `&str` lookups, so a cache hit allocates nothing.

### Fixed

- `map_to_gamut` returned the last clip it tried, even if that clip was rejected. It now returns the
  clip at the last accepted chroma.
- NaN inputs no longer leak through the tone mapper or the gamut mapper.

## [0.1.0]

Initial release of the workspace. The crate was unpublished at this point.
