# Changelog

All notable changes to the `tpt-colour` facade are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate uses
[Semantic Versioning](https://semver.org/). It is pre-1.0, so breaking changes can appear in minor
releases until 1.0.0.

The facade re-exports the other crates. Changes to their behaviour are recorded in each crate's
own changelog, and the [root changelog](../../CHANGELOG.md) covers the whole workspace.

## [Unreleased]

### Added

- `tpt_colour::prelude`, which brings in the types most programs need.
- `RenderingIntent`, `tone`, `ColourTransform` and `IccProfile` are re-exported at the top level.
- Cookbook examples: `composite_layers`, `hdr_tonemap`, `batch_pixels`, `ycbcr_video`,
  `parse_and_gradient` and `wcag_check`. Also `convert`, `write_profile` and `bench`.
- Optional `serde` feature, forwarded to the core, ICC and manage crates. Off by default.
- The `simd` feature, which forwards to the manage crate and then the ICC crate. Off by default.

## [0.1.0]

Initial release of the workspace. The crate was unpublished at this point.
