# Black point compensation

Black point compensation (BPC) maps the source's black point onto the destination's, so shadow
detail is not crushed when the destination is darker. tpt-colour follows LittleCMS here, and the
rules are the ones LittleCMS uses:

- **Perceptual and saturation** turn BPC on when the destination is an ICC v4 profile. For v2
  destinations it is off, unless you ask for it.
- **Relative colorimetric** keeps BPC off by default.
- **Absolute colorimetric** never uses BPC.

To force it, build the transform with `with_black_point_compensation`:

```rust
# extern crate tpt_colour;
# use tpt_colour::icc::{ColourTransform, IccProfile, RenderingIntent};
let t = ColourTransform::with_black_point_compensation(
    &IccProfile::srgb(),
    &IccProfile::display_p3(),
    RenderingIntent::RelativeColorimetric,
)?;
# Ok::<(), tpt_colour::icc::IccError>(())
```

The constructor applies to the perceptual and relative intents. Absolute colorimetric transforms
come back without it, whatever you ask for.

## Black point detection

For matrix/TRC profiles, the black point comes from the profile's own colorants. For LUT
profiles, tpt-colour follows LittleCMS's round-trip search (the Adobe method) and its darker
colorant rule for v2 profiles. The v4 perceptual black is fixed.

If the source and destination black points are equal, BPC is the identity, so the fast paths
stay on. Nothing is compensated in that case.
