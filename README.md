# tpt-colour

A pure-Rust colour management substrate: colour spaces and maths, alpha
compositing, ICC profile parsing and transforms, and gamut mapping.

It is `no_std` throughout (the ICC crate also needs `alloc`). SIMD acceleration
is optional behind the `simd` feature; the scalar path is the reference.

## Crates

| Crate | Purpose |
| --- | --- |
| `tpt-colour-core` | Colour spaces, chromatic adaptation, YCbCr, and Porter-Duff / blend compositing |
| `tpt-colour-icc` | ICC profiles (v2/v4): matrix/TRC and LUT (`mft1`, `mft2`, `mAB `, `mBA `) transforms, black point compensation |
| `tpt-colour-manage` | Gamut boundaries, CSS Colour 4 OKLCH gamut mapping, `ColourManager` with transform caching, HDR tone mapping |
| `tpt-colour` | Facade crate that re-exports the above |

`tpt-colour-oracle` compares output against LittleCMS. It is excluded from the
workspace and never shipped.

## Quick start

```rust
use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;

let mut colour = ColourManager::new(); // registers "srgb" and "display-p3"
colour.register_icc("fogra39", &std::fs::read("FOGRA39.icc")?)?;

let to_print = colour.transform("srgb", "fogra39", RenderingIntent::Perceptual)?;
let cmyk = to_print.transform_rgb([0.8, 0.2, 0.1])?;
```

Run the example with `cargo run -p tpt-colour --example convert`.

## Principles

- **Pure Rust in shipped crates.** No C-FFI or `-sys` crates. `cargo deny` enforces this.
- **Deterministic maths.** Operations keep a fixed order, avoid FMA, and use `libm`
  rather than platform `std` maths, so results match bit-for-bit across targets.
- **No `unsafe`.** `unsafe_code` is forbidden at the workspace level.
- **Bounded parsing.** Malformed or hostile ICC data returns an error. It never panics or allocates without limit.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace --target thumbv7em-none-eabihf   # no_std check
cargo deny check
```

The design is in [spec.txt](spec.txt) and the work plan in [TODO.md](TODO.md).

## Licence

Placeholder: `MIT OR Apache-2.0`. Pending confirmation; licence files are not yet added.
