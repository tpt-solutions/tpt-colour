# tpt-colour-icc

ICC profile parsing, writing and colour transforms, in pure Rust with `no_std` and `alloc`.

Part of [tpt-colour](https://github.com/tpt-solutions/tpt-colour). It reads ICC v2 and v4 profiles
that use matrix/TRC or lookup-table models, converts colours between them, and writes simple
matrix/TRC profiles back out. Parsing is bounds-checked throughout: malformed data returns an
`IccError`, and never panics or allocates without limit.

## What's supported

| Area | Support |
| --- | --- |
| Profile versions | ICC v2 and v4 |
| Device models | Matrix/TRC RGB, grey `kTRC`, and LUT profiles for RGB, CMYK and grey |
| LUT tags | `mft1`, `mft2`, `mAB ` and `mBA ` (device to PCS and PCS to device) |
| Connection space | XYZ and Lab PCS |
| Rendering intents | Perceptual, relative colorimetric, saturation and absolute colorimetric |
| Black point compensation | Yes, see `ColourTransform::with_black_point_compensation` |
| Curves | Gamma, sampled tables and the parametric `para` types 0 to 4 |
| Writing | Matrix/TRC RGB and grey profiles (ICC v2.1, XYZ PCS). A written profile parses back. |
| Profile IDs | `computed_profile_id` and `check_profile_id`, the MD5 header ID |
| Export | Adobe `.cube` 3-D LUTs (`to_cube`), and baked `Pipeline` CLUTs (`bake_clut`) |

Not supported, and reported as `IccError::Unsupported` rather than parsed:

- `multiProcessElements` (`mpet`) tables.
- LUTs with more than eight input channels. Each profile's CLUTs share a 32 MiB sample budget.

## Installation

```sh
cargo add tpt-colour-icc
```

Enable `serde` for serde derives on the profile types, or `simd` for batched transforms (see
[Features](#features)).

## Quick start

Build a transform between two built-in profiles and convert one colour:

```rust
use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

let srgb = IccProfile::srgb();
let p3 = IccProfile::display_p3();
let transform = ColourTransform::new(&srgb, &p3, RenderingIntent::RelativeColorimetric)?;

let out = transform.transform_rgb([1.0, 0.0, 0.0])?;
```

To use a profile from disk, parse its bytes:

```rust
use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

let cmyk = IccProfile::parse(&std::fs::read("FOGRA39.icc")?)?;
let transform = ColourTransform::new(&IccProfile::srgb(), &cmyk, RenderingIntent::Perceptual)?;
let ink = transform.transform_rgb([0.8, 0.2, 0.1])?;
```

Runnable versions are in [`examples/`](examples):

- [`inspect_profile`](examples/inspect_profile.rs): prints a profile's header and LUT coverage. Takes an optional path.
- [`transform_u8`](examples/transform_u8.rs): converts an interleaved 8-bit RGBA buffer from sRGB to Display P3.
- [`export_cube`](examples/export_cube.rs): bakes a transform into an Adobe `.cube` LUT.

Run one with `cargo run -p tpt-colour-icc --example transform_u8`.

## Pixel buffers

Pick the entry point that matches your buffer:

| Method | Buffer |
| --- | --- |
| `transform_rgb` | One `[f32; 3]` colour |
| `transform_pixels` | `[f32; 4]` pixels, alpha untouched |
| `transform_u8` / `transform_u16` | Interleaved 8-bit or 16-bit channels |
| `transform_u8_image` / `transform_u16_image` | As above, with a row stride; padding is left untouched |

`transform_u8` and `transform_u16` accept only RGB or RGBA buffers, and return
`IccError::ChannelCount` otherwise. The image methods return `IccError::BufferLayout` when the
stride or buffer is too small for the width and height.

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `serde` | off | Serde derives on profile and pipeline types |
| `simd` | off | Batched transforms through [`tpt-simd`](https://github.com/tpt-solutions/tpt-simd). Output is bit-identical to the scalar path. Needs Rust 1.95. |

The default build keeps the 1.85 MSRV.

## Design

- **Scalar path first.** The scalar code is the reference. Bulk transforms use `f32` with curves
  baked into 4096-point tables, accurate to about 2e-5. Pure power curves stay analytic, because a
  table is about 1.6 codes out near black. Single-colour transforms use exact `f64` curves.
- **Checked against LittleCMS.** The workspace's test oracle compares results with LittleCMS 2.19,
  using stored fixtures, so the shipped crates do not depend on C. Known differences are listed in
  the [changelog](CHANGELOG.md).
- **Fuzzed.** Parser and LUT evaluation have fuzz targets, with seed corpora replayed on every
  platform in CI.
- **No `unsafe`.** The workspace forbids it.

## Where this fits

| Crate | Purpose |
| --- | --- |
| [`tpt-colour-core`](https://crates.io/crates/tpt-colour-core) | Colour spaces, difference, contrast, compositing |
| `tpt-colour-icc` | This crate: ICC profiles and transforms |
| [`tpt-colour-manage`](https://crates.io/crates/tpt-colour-manage) | Gamut mapping, profile management, soft proofing and HDR tone mapping |
| [`tpt-colour`](https://crates.io/crates/tpt-colour) | Facade that re-exports the crates above |

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
