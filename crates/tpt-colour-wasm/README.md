# tpt-colour-wasm

WebAssembly bindings for [tpt-colour](https://github.com/tpt-solutions/tpt-colour), built with
`wasm-bindgen`. Use them from JavaScript in the browser or in Node.

This crate is not published to crates.io. It is the module behind the
[playground](../../playground/index.html) in the repository.

## What's exposed

| JavaScript | Purpose |
| --- | --- |
| `parseColour(text)` | Parses a CSS colour, returning a `Uint8Array` of `[r, g, b, a]` |
| `wcagContrast(a, b)` | WCAG 2 contrast ratio of two `[r, g, b]` colours |
| `apcaContrast(text, background)` | APCA lightness contrast, signed by polarity |
| `simulateDeficiency(rgb, kind)` | Simulates `protanopia`, `deuteranopia` or `tritanopia` |
| `harmony(rgb, kind)` | `complement`, `analogous`, `triadic` or `split-complementary` |
| `profileDescription(bytes)` | The description of an ICC profile |
| `Manager` | A profile registry with a transform cache: `registerIcc`, `transformRgb` and `cachedTransforms` |

Colours cross the boundary as `Float32Array`s of values in `[0, 1]`, and 8-bit colours as
`Uint8Array`s. Failures are thrown as JavaScript `Error`s.

## Build

```sh
wasm-pack build crates/tpt-colour-wasm --target web --release
```

## Usage

```js
import init, { parseColour, wcagContrast, Manager } from "./pkg/tpt_colour_wasm.js";

await init();
const white = parseColour("white"); // Uint8Array [255, 255, 255, 255]
const contrast = wcagContrast(new Float32Array([0, 0, 0]), new Float32Array([1, 1, 1])); // 21

const m = new Manager(); // "srgb" and "display-p3" are registered
const p3 = m.transformRgb("srgb", "display-p3", "perceptual", new Float32Array([0.8, 0.2, 0.1]));
```

To run the playground locally, build into the playground folder and serve it over HTTP. Browsers
do not load modules from `file://`:

```sh
wasm-pack build crates/tpt-colour-wasm --target web --out-dir ../../playground/pkg --release
python -m http.server --directory playground 8000
```

The playground can load an ICC profile. The profile stays in the browser tab and is not uploaded.

## Tests

```sh
wasm-pack test --node crates/tpt-colour-wasm
```

The full binding guide is in [`docs/book/src/bindings.md`](../../docs/book/src/bindings.md).

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
