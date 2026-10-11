# tpt-colour-core

Colour spaces, chromatic adaptation, colour difference, contrast, palettes, colour-vision
simulation and alpha compositing, in `no_std` Rust.

This is the maths layer of [tpt-colour](https://github.com/tpt-solutions/tpt-colour). It has no
allocation, no I/O, and no dependencies beyond [`libm`](https://crates.io/crates/libm), apart from
an optional `serde` feature. Most applications should depend on the
[`tpt-colour`](https://crates.io/crates/tpt-colour) facade instead, which re-exports this crate
alongside ICC profile support and gamut mapping.

## What's inside

| Module | Contents |
| --- | --- |
| `space` | sRGB, linear sRGB, Display P3, Rec. 2020, Adobe RGB, ProPhoto RGB, ACEScg, CMYK, CIE XYZ, CIE L\*a\*b\* (D65 and D50), LCh, L\*u\*v\*, Oklab and Oklch. Y′CbCr for Rec. 601, 709 and 2020 (limited and full range), YCoCg and YCoCg-R, ICtCp and Jzazbz for HDR. Bradford, CAT02 and CAT16 chromatic adaptation. |
| `difference` | ΔE 76, ΔE 94, CIEDE2000 and ΔEOK |
| `contrast` | WCAG 2 contrast ratio and relative luminance, and APCA 0.0.98G-4g lightness contrast |
| `composite` | Premultiplied RGBA, the Porter-Duff operators, and separable and non-separable blend modes |
| `mix` | Gradient interpolation in Oklab and Oklch, including premultiplied forms |
| `palette` | Hue rotation, complementary, analogous, triadic and split-complementary harmonies |
| `parse` | CSS colour strings: hex, `rgb()`, `oklch()`, the 148 CSS named colours and `transparent` |
| `hex` | Hex colours with an optional `#`, returned as 8-bit `[r, g, b, a]` |
| `models` | HSL, HSV, HWB and xyY |
| `vision` | Protanopia, deuteranopia and tritanopia simulation (Machado et al., 2009) |

## Installation

```sh
cargo add tpt-colour-core
```

Enable `serde` to derive `Serialize` and `Deserialize` on the compositing and vision types:

```sh
cargo add tpt-colour-core --features serde
```

## Quick start

Convert an sRGB colour through CIE XYZ into Oklab, and measure text contrast:

```rust
use tpt_colour_core::contrast::wcag_contrast;
use tpt_colour_core::space::{ColourSpace, Oklab, SRgb};

let orange = [1.0, 0.5, 0.0];
let [l, a, b] = Oklab.from_xyz(SRgb.to_xyz(orange));

// Black text on white is the highest possible contrast, 21:1.
let ratio = wcag_contrast([0.0; 3], [1.0; 3]);
```

Every space converts to and from CIE XYZ, so any two spaces can be chained through it.

Runnable versions of these ideas are in [`examples/`](examples):

- [`spaces`](examples/spaces.rs): one colour through XYZ, Lab, Oklab, Oklch and Display P3.
- [`difference_and_contrast`](examples/difference_and_contrast.rs): ΔE between two colours, and WCAG and APCA contrast.
- [`palette_and_vision`](examples/palette_and_vision.rs): CSS colour parsing, harmonies, Oklab mixing and colour-vision simulation.

Run one with `cargo run -p tpt-colour-core --example spaces`.

## Design

- **`no_std`.** The crate builds for bare-metal targets. CI checks it against
  `thumbv7em-none-eabihf`.
- **Reproducible maths.** Operations use `f32` with a fixed evaluation order and no fused
  multiply-add, so results match across platforms.
- **No `unsafe`.** The workspace forbids it.

The crate needs Rust 1.85 or later, and uses edition 2024.

## Where this fits

| Crate | Purpose |
| --- | --- |
| `tpt-colour-core` | This crate: colour spaces, difference, contrast, compositing |
| [`tpt-colour-icc`](https://crates.io/crates/tpt-colour-icc) | ICC profile parsing and transforms |
| [`tpt-colour-manage`](https://crates.io/crates/tpt-colour-manage) | Gamut mapping, profile management, soft proofing and HDR tone mapping |
| [`tpt-colour`](https://crates.io/crates/tpt-colour) | Facade that re-exports the crates above |

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
