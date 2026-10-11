# Getting started

Add the facade to `Cargo.toml`. The crate is not yet published, so use a path or git dependency:

```toml
[dependencies]
tpt-colour = { path = "../tpt-colour/crates/tpt-colour" }
```

## Convert a colour between two profiles

`ColourManager` holds the built-in `srgb` and `display-p3` profiles, plus any you register. It
builds a transform the first time you ask for one, and caches it after that.

```rust
# extern crate tpt_colour;
use tpt_colour::{ColourManager, RenderingIntent};

let mut colour = ColourManager::new();
let to_p3 = colour.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
let p3 = to_p3.transform_rgb([0.8, 0.2, 0.1])?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Check contrast

```rust
# extern crate tpt_colour;
use tpt_colour::contrast::{apca_contrast, wcag_contrast};

let text = [0.11, 0.11, 0.10];
let background = [0.96, 0.96, 0.96];
let wcag = wcag_contrast(text, background); // 1 to 21; AA body text needs 4.5
let lc = apca_contrast(text, background);   // signed; sign gives polarity
```

## Parse CSS colours

```rust
# extern crate tpt_colour;
use tpt_colour::parse::parse_colour;

assert_eq!(parse_colour("rebeccapurple"), Some([102, 51, 153, 255]));
assert!(parse_colour("oklch(62.8% 0.2577 29.23)").is_some());
```

`parse_colour` accepts hex, `rgb()`, `rgba()`, `oklch()`, the 148 CSS named colours and
`transparent`.

## Process images

Interleaved 8-bit and 16-bit buffers go straight through a transform, with alpha left alone. The
`*_image` forms take a row stride, so padded or sub-image buffers work too.

```rust
# extern crate tpt_colour;
# use tpt_colour::icc::{ColourTransform, IccProfile, RenderingIntent};
let t = ColourTransform::new(
    &IccProfile::srgb(),
    &IccProfile::display_p3(),
    RenderingIntent::RelativeColorimetric,
)?;
let mut pixels = vec![255_u8, 0, 0, 255];
t.transform_u8(&mut pixels, 4)?;
# Ok::<(), tpt_colour::icc::IccError>(())
```

## Run the examples

```sh
cargo run -p tpt-colour --example convert
cargo run -p tpt-colour --example wcag_check
cargo run -p tpt-colour --example batch_pixels
```
