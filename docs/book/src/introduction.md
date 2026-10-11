# Introduction

tpt-colour is a pure-Rust colour library. It covers colour spaces and their conversions, alpha
compositing, colour-difference metrics, and ICC profile transforms with a profile manager on top.

The crates fit in layers:

| Crate | Role | `no_std` |
|---|---|---|
| `tpt-colour-core` | Colour spaces, chromatic adaptation, compositing, contrast, colour difference, vision simulation, palettes | yes |
| `tpt-colour-icc` | ICC v2/v4 parsing, matrix/TRC and LUT transforms, black point compensation | yes (with `alloc`) |
| `tpt-colour-manage` | Gamut mapping, the profile manager and its cache, soft-proofing, HDR tone mapping | yes (with `alloc`) |
| `tpt-colour` | The facade: one dependency that re-exports the three above | yes |
| `tpt-colour-wasm`, `tpt-colour-py`, `tpt-colour-capi` | WebAssembly, Python and C bindings | no |

Most programs depend on the facade and use the `prelude`:

```rust
# extern crate tpt_colour;
use tpt_colour::prelude::*;
```

## Principles

- **Pure Rust in shipped crates.** No C-FFI and no `-sys` crates. `cargo deny` checks this.
- **Deterministic maths.** Operations keep a fixed order, avoid fused multiply-add, and use
  `libm`, so results match across targets. See [Determinism](determinism.md).
- **No `unsafe`** in the shipped crates. The C binding is the one place that needs it, and it is
  kept out of the workspace.
- **Bounded parsing.** Malformed or hostile ICC data returns an error. It does not panic, and it
  does not allocate without limit.

The conventions follow British English throughout: `colour`, `grey`, `normalise`.
