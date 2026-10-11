# tpt-colour

Pure-Rust colour management: colour spaces and maths, ICC profiles and transforms, gamut mapping,
soft proofing, HDR tone mapping and alpha compositing.

This is the facade crate for [tpt-colour](https://github.com/tpt-solutions/tpt-colour). It
re-exports the workspace crates, so one dependency covers the whole stack. The library is `no_std`.
The ICC and manage crates also need `alloc`.

## Installation

The crates are not yet published to crates.io, so depend on the repository directly:

```toml
[dependencies]
tpt-colour = { git = "https://github.com/tpt-solutions/tpt-colour" }
```

Once published, this becomes `cargo add tpt-colour`.

## Quick start

Convert an sRGB colour to Display P3 with a managed transform, then gamut-map a wide-gamut colour
back into sRGB:

```rust
use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;

let mut colour = ColourManager::new(); // registers "srgb" and "display-p3"
let to_p3 = colour.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
let p3 = to_p3.transform_rgb([0.8, 0.2, 0.1])?;

let to_srgb = colour.transform("display-p3", "srgb", RenderingIntent::Perceptual)?;
let back = to_srgb.transform_rgb(p3)?;
```

`use tpt_colour::prelude::*;` brings in the types most programs need.

## Which crate to depend on

| Crate | Purpose |
| --- | --- |
| `tpt-colour` | This facade. Use it unless you need a single layer. |
| [`tpt-colour-core`](https://github.com/tpt-solutions/tpt-colour/tree/master/crates/tpt-colour-core) | Colour spaces, ΔE, WCAG and APCA contrast, palettes, compositing. `no_std`, no allocation. |
| [`tpt-colour-icc`](https://github.com/tpt-solutions/tpt-colour/tree/master/crates/tpt-colour-icc) | ICC profile parsing, writing and transforms. `no_std` with `alloc`. |
| [`tpt-colour-manage`](https://github.com/tpt-solutions/tpt-colour/tree/master/crates/tpt-colour-manage) | Gamut mapping, the `ColourManager` profile registry, soft proofing and HDR tone mapping. |

Modules are re-exported under the facade: `space`, `composite`, `contrast`, `difference`, `hex`,
`mix`, `models`, `palette`, `parse` and `vision` from the core crate; `icc` and `manage` for the
other two; and `tone`, `ColourManager`, `Gamut` and `map_to_gamut` at the top level.

## Examples

Each example is a short, runnable program. Run one with
`cargo run -p tpt-colour --example <name>`.

| Example | What it shows |
| --- | --- |
| [`convert`](examples/convert.rs) | Colour-space maths, managed transforms, and clipping against gamut mapping |
| [`batch_pixels`](examples/batch_pixels.rs) | Converting a buffer of `[f32; 4]` RGBA pixels in one call |
| [`composite_layers`](examples/composite_layers.rs) | Porter-Duff source-over, and the multiply and colour-dodge blend modes |
| [`hdr_tonemap`](examples/hdr_tonemap.rs) | Tone mapping linear HDR light into display range, and encoding it as PQ and HLG |
| [`parse_and_gradient`](examples/parse_and_gradient.rs) | Parsing hex colours and interpolating between them in Oklab and Oklch |
| [`wcag_check`](examples/wcag_check.rs) | Checking text and background pairs against WCAG 2 thresholds |
| [`ycbcr_video`](examples/ycbcr_video.rs) | Encoding gamma-encoded RGB as Y′CbCr for Rec. 601, 709 and 2020, and decoding it |
| [`write_profile`](examples/write_profile.rs) | Writing ICC profiles, and recording tpt-colour's output so an independent CMM can check it |
| [`bench`](examples/bench.rs) | Throughput of the main transform paths. Run in `--release` only. |

For more depth, the per-crate examples show the individual layers:
[core](https://github.com/tpt-solutions/tpt-colour/tree/master/crates/tpt-colour-core/examples),
[icc](https://github.com/tpt-solutions/tpt-colour/tree/master/crates/tpt-colour-icc/examples) and
[manage](https://github.com/tpt-solutions/tpt-colour/tree/master/crates/tpt-colour-manage/examples).

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `serde` | off | Serde derives on the public types of the core, ICC and manage crates |
| `simd` | off | Batched transforms through [`tpt-simd`](https://github.com/tpt-solutions/tpt-simd), pinned to a commit. Output is bit-identical to the scalar path. Needs Rust 1.95. |

The default build keeps the 1.85 MSRV.

## Principles

- **Pure Rust in shipped crates.** No C-FFI or `-sys` crates. `cargo deny` enforces this.
- **Deterministic maths.** Operations keep a fixed order, avoid FMA, and use `libm` rather than
  platform `std` maths, so results match across targets.
- **Precision.** Bulk transforms use `f32` with curves baked into 4096-point tables, accurate to
  about 2e-5. Pure power curves stay analytic, since a table is about 1.6 codes out near black.
  Single-colour matrix/TRC transforms use exact `f64` curves.
- **No `unsafe`.** Forbidden at the workspace level.
- **Bounded parsing.** Malformed or hostile ICC data returns an error. It never panics or allocates
  without limit.

Known differences from LittleCMS are listed in the [changelog](CHANGELOG.md).

## Development

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace --exclude tpt-colour-cli --target thumbv7em-none-eabihf   # no_std check
cargo deny check
cargo test --workspace --features tpt-colour/simd   # needs Rust 1.95
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
