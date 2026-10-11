# tpt-colour

A pure-Rust colour management substrate: colour spaces and maths, alpha
compositing, ICC profile parsing and transforms, and gamut mapping.

It is `no_std` throughout (the ICC crate also needs `alloc`). The scalar path is the reference.
SIMD batching is optional behind the `simd` feature, which uses
[`tpt-simd`](https://github.com/tpt-solutions/tpt-simd) (pinned to a commit). It produces
bit-identical output and needs Rust 1.95; the default build keeps the 1.85 MSRV.

## Crates

Each crate has its own README, changelog and examples.

| Crate | Purpose | Docs |
| --- | --- | --- |
| [`tpt-colour`](crates/tpt-colour) | Facade crate that re-exports the others. Start here. | [README](crates/tpt-colour/README.md) · [examples](crates/tpt-colour/examples) |
| [`tpt-colour-core`](crates/tpt-colour-core) | Colour spaces (sRGB, P3, Rec. 2020, Adobe RGB, Lab, Oklab, YCbCr), HSL/HSV/HWB/xyY, ΔE formulas, WCAG and APCA contrast, palettes, colour-vision simulation, and Porter-Duff / blend compositing | [README](crates/tpt-colour-core/README.md) · [examples](crates/tpt-colour-core/examples) |
| [`tpt-colour-icc`](crates/tpt-colour-icc) | ICC profiles (v2/v4): matrix/TRC and LUT (`mft1`, `mft2`, `mAB `, `mBA `) transforms, writing, black point compensation, absolute colorimetric | [README](crates/tpt-colour-icc/README.md) · [examples](crates/tpt-colour-icc/examples) |
| [`tpt-colour-manage`](crates/tpt-colour-manage) | Gamut boundaries, CSS Colour 4 OKLCH gamut mapping, `ColourManager` with transform caching, soft proofing, HDR tone mapping (PQ, HLG, Reinhard) | [README](crates/tpt-colour-manage/README.md) · [examples](crates/tpt-colour-manage/examples) |

Tools and bindings, not published: [`tpt-colour-cli`](crates/tpt-colour-cli/README.md) (the
`tpt-colour` command), [`tpt-colour-wasm`](crates/tpt-colour-wasm/README.md),
[`tpt-colour-capi`](crates/tpt-colour-capi/README.md) and
[`tpt-colour-py`](crates/tpt-colour-py/README.md). `tpt-colour-oracle` compares output against
LittleCMS. It is excluded from the workspace and never shipped.

## Quick start

```rust
use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;

let mut colour = ColourManager::new(); // registers "srgb" and "display-p3"
colour.register_icc("fogra39", &std::fs::read("FOGRA39.icc")?)?;

let to_print = colour.transform("srgb", "fogra39", RenderingIntent::Perceptual)?;
let cmyk = to_print.transform_rgb([0.8, 0.2, 0.1])?;
```

Run the example with `cargo run -p tpt-colour --example convert`. The
[crate README](crates/tpt-colour/README.md) lists all the examples, and the
[mdBook guide](docs/book/src/introduction.md) explains the design choices behind them.

## Principles

- **Pure Rust in shipped crates.** No C-FFI or `-sys` crates. `cargo deny` enforces this.
- **Deterministic maths.** Operations keep a fixed order, avoid FMA, and use `libm`
  rather than platform `std` maths, so results match bit-for-bit across targets.
- **Precision.** Bulk transforms use f32 with curves baked into 4096-point tables, which
  is accurate to about 2e-5. Pure power curves stay analytic, because a table is off by about
  1.6 codes near black. Single-colour matrix/TRC transforms use exact f64 curves.
- **No `unsafe`.** `unsafe_code` is forbidden at the workspace level.
- **Bounded parsing.** Malformed or hostile ICC data returns an error. It never panics or allocates without limit.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace --exclude tpt-colour-cli --target thumbv7em-none-eabihf   # no_std check (the CLI is std)
cargo deny check
cargo test --workspace --features tpt-colour/simd   # needs Rust 1.95
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Examples live in each crate's `examples/` folder. List the facade's with `cargo run -p tpt-colour --example`.

Release notes are in [CHANGELOG.md](CHANGELOG.md). Contribution rules are in
[CONTRIBUTING.md](CONTRIBUTING.md), and vulnerability reports are in [SECURITY.md](SECURITY.md).

The design is in [spec.txt](spec.txt) and the work plan in [TODO.md](TODO.md).

## Licence

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
