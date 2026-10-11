# tpt-colour-manage

Gamut mapping, profile management, soft proofing and HDR tone mapping, built on
[`tpt-colour-icc`](https://crates.io/crates/tpt-colour-icc). Pure Rust, `no_std` with `alloc`.

Part of [tpt-colour](https://github.com/tpt-solutions/tpt-colour). This crate answers the questions
that come after a single transform: what to do with a colour the destination cannot show, how to
keep track of many named profiles without rebuilding transforms, how to preview a file on a
narrower device, and how to bring HDR content into display range.

## What's inside

| Item | Purpose |
| --- | --- |
| `Gamut` | An RGB gamut as a boundary in XYZ. Build it from a standard space, from colorants or from an ICC profile. Test a colour with `contains`, or clip it with `clip`. |
| `map_to_gamut` | Perceptual gamut mapping by OKLCH chroma reduction, following CSS Color Level 4 §13.2. Lightness and hue are kept. |
| `ColourManager` | A registry of named profiles that caches transforms. An LRU cap is available with `with_capacity`. |
| `ManagedTransform` | A transform with its destination gamut attached, ready to run on one colour or a buffer. |
| `SoftProof` | Previews a working-space colour through a proof profile, and flags colours the proof cannot reproduce by ΔE₀₀. |
| `tone` | PQ and HLG transfer functions, Reinhard extended tone mapping, and `tone_map_xyz`, which rolls off luminance but keeps chromaticity. |

## Installation

```sh
cargo add tpt-colour-manage
```

Enable `serde` for serde derives on the public types, or `simd` for batched transforms (see
[Features](#features)).

## Quick start

Gamut-map a Display P3 green into sRGB, keeping its lightness and hue:

```rust
use tpt_colour_core::space::{ColourSpace, DisplayP3};
use tpt_colour_manage::{Gamut, map_to_gamut};

let srgb = Gamut::srgb();
let green = DisplayP3.to_xyz([0.0, 1.0, 0.0]);
assert!(!srgb.contains(green));
let mapped = map_to_gamut(&srgb, green);
assert!(srgb.contains(mapped));
```

Manage profiles by name, with cached transforms:

```rust
use tpt_colour_icc::RenderingIntent;
use tpt_colour_manage::ColourManager;

let mut colour = ColourManager::new(); // registers "srgb" and "display-p3"
colour.register_icc("fogra39", &std::fs::read("FOGRA39.icc")?)?;

let to_print = colour.transform("srgb", "fogra39", RenderingIntent::Perceptual)?;
let cmyk = to_print.transform_rgb([0.8, 0.2, 0.1])?;
```

Runnable versions are in [`examples/`](examples):

- [`gamut_map`](examples/gamut_map.rs): clipping against gamut mapping, for an out-of-gamut colour.
- [`colour_manager`](examples/colour_manager.rs): registering a profile from bytes, cache reuse, and unknown-name errors.
- [`soft_proof`](examples/soft_proof.rs): previewing a Display P3 image through an sRGB proof, and flagging colours the proof cannot show.

Run one with `cargo run -p tpt-colour-manage --example gamut_map`.

## Choosing between clipping and mapping

`Gamut::clip` clamps each channel in XYZ. It is fast, but it moves hue, so a saturated colour can
shift toward a neighbouring hue. `map_to_gamut` reduces chroma in OKLCH until the colour fits, so
lightness and hue stay the same. It is slower, and it is the better default for photographs and
for design colours that must keep their character. Colours already in gamut are returned unchanged
by either.

## Soft proofing

`SoftProof::new` takes a working space, a proof profile and a display profile, and resolves them
through a `ColourManager`, so the transforms are cached like any others. `proof_pixel` returns the
colour as it would look on the display, and sets `out_of_gamut` when the proof's gamut cannot hold
the colour by more than the ΔE₀₀ threshold (2.0 by default, set with `with_threshold`).

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `serde` | off | Serde derives on public types, and on the `serde` features of the crates below it |
| `simd` | off | Batched transforms through the ICC crate's `simd` feature. Output is bit-identical to the scalar path. Needs Rust 1.95. |

The default build keeps the 1.85 MSRV.

## Errors

`ColourManager::transform` returns `ManageError::UnknownProfile` for a name that was never
registered, and `ManageError::Icc` when a transform cannot be built. `ManageError` implements
`std::error::Error` through `source`, so the underlying ICC error stays reachable.

## Where this fits

| Crate | Purpose |
| --- | --- |
| [`tpt-colour-core`](https://crates.io/crates/tpt-colour-core) | Colour spaces, difference, contrast, compositing |
| [`tpt-colour-icc`](https://crates.io/crates/tpt-colour-icc) | ICC profile parsing and transforms |
| `tpt-colour-manage` | This crate: gamut mapping, profile management, soft proofing, tone mapping |
| [`tpt-colour`](https://crates.io/crates/tpt-colour) | Facade that re-exports the crates above |

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
