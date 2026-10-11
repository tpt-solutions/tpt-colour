# tpt-colour

Pure-Rust colour management: colour spaces, ICC profiles, gamut mapping and compositing.
This facade re-exports the workspace crates, so one dependency covers them all.

```rust
use tpt_colour::ColourManager;
use tpt_colour::icc::RenderingIntent;

let mut colour = ColourManager::new();
let to_p3 = colour.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
let p3 = to_p3.transform_rgb([0.8, 0.2, 0.1])?;
```

`use tpt_colour::prelude::*;` brings in the types most programs need. The `examples/` folder
has short recipes for compositing, HDR tone mapping, batch pixels, Y′CbCr, gradients and
WCAG contrast.

Part of the [tpt-colour](https://github.com/tpt-solutions/tpt-colour) workspace.
