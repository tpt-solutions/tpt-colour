# Bindings: WebAssembly, Python and C

The core crates are Rust. Three bindings expose the same operations to other languages. Each one
is a thin wrapper, so behaviour matches the Rust API.

| Binding | Crate | Built with | Tested with |
|---|---|---|---|
| WebAssembly | `crates/tpt-colour-wasm` | `wasm-pack build --target web` | `wasm-pack test --node` and a Node smoke test |
| Python | `crates/tpt-colour-py` | `maturin build --release` | `unittest`, against the installed wheel |
| C | `crates/tpt-colour-capi` | `cargo build --release`, plus `cbindgen` for the header | `unittest` through `ctypes`, against the built library |

The Python and C bindings are outside the default workspace. PyO3 links against the interpreter,
and the C ABI needs `unsafe`, which the workspace forbids.

## WebAssembly

Colours cross the boundary as `Float32Array`s of values in `[0, 1]`, and 8-bit colours as
`Uint8Array`s. Errors are thrown as JavaScript `Error`s.

```js
import init, { parseColour, wcagContrast, Manager } from "./pkg/tpt_colour_wasm.js";
await init();
const white = parseColour("white"); // Uint8Array [255, 255, 255, 255]
const m = new Manager();
const p3 = m.transformRgb("srgb", "display-p3", "perceptual", [0.8, 0.2, 0.1]);
```

The [playground](../../../playground/index.html) is a single page that runs the module. Serve the
`playground` directory over HTTP, because browsers do not load modules from `file://`:

```sh
wasm-pack build crates/tpt-colour-wasm --target web --out-dir ../../playground/pkg --release
python -m http.server --directory playground 8000
```

The page lets you load an ICC profile. The profile stays in that tab and is not uploaded.

## Python

```python
import tpt_colour as tc

tc.parse_colour("rebeccapurple")          # [102, 51, 153, 255]
tc.wcag_contrast([0, 0, 0], [1, 1, 1])    # 21.0
m = tc.Manager()
m.transform_rgb("srgb", "display-p3", "perceptual", [0.8, 0.2, 0.1])
```

Install a wheel with `pip install` after building it with `maturin build`. Errors raise
`ValueError`.

## C

The header is `crates/tpt-colour-capi/include/tpt_colour.h`. Every function that can fail returns
a status code: `0` is success, and the negative codes are listed in the header. Strings are UTF-8
with an explicit length. The only allocation is `tpt_manager_new`, which must be paired with
`tpt_manager_free`.

```c
#include "tpt_colour.h"
uint8_t rgba[4];
int32_t status = tpt_parse_colour("rebeccapurple", 13, rgba);
```

Regenerate the header after changing the C API:

```sh
cd crates/tpt-colour-capi
cbindgen --config cbindgen.toml --crate tpt-colour-capi --output include/tpt_colour.h
```
