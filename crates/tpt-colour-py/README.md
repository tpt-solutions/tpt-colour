# tpt-colour-py

Python bindings for [tpt-colour](https://github.com/tpt-solutions/tpt-colour), built with PyO3 and
maturin. The module is `tpt_colour`. It needs Python 3.9 or later, and one `abi3` wheel covers
those versions.

This crate is not published to PyPI or crates.io. It is built from the repository.

## Build a wheel

```sh
cd crates/tpt-colour-py
pip install maturin
maturin build --release -o dist
pip install dist/tpt_colour-*.whl
```

To develop against a virtual environment, `maturin develop --release` installs the module in
place.

## Usage

```python
import tpt_colour as tc

tc.parse_colour("rebeccapurple")          # [102, 51, 153, 255]
tc.wcag_contrast([0, 0, 0], [1, 1, 1])    # 21.0
tc.apca_contrast([0.4, 0.4, 0.4], [1, 1, 1])
tc.simulate_deficiency([0.8, 0.2, 0.1], "deuteranopia")
tc.harmony([0.8, 0.2, 0.1], "triadic")    # two colours, each a list of three floats

m = tc.Manager()                          # "srgb" and "display-p3" are registered
with open("FOGRA39.icc", "rb") as f:
    m.register_icc("fogra39", f.read())
m.transform_rgb("srgb", "fogra39", "perceptual", [0.8, 0.2, 0.1])
m.cached_transforms                       # number of cached transforms
```

Colours are sequences of floats in `[0, 1]`, and 8-bit colours are sequences of integers. Intents
are `"perceptual"`, `"relative"`, `"saturation"` and `"absolute"`. Errors raise `ValueError`.

## Tests

```sh
python -m unittest discover -s tests -v
```

The tests run against the installed wheel. The full binding guide is in
[`docs/book/src/bindings.md`](../../docs/book/src/bindings.md).

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
