# Soft-proofing

Soft-proofing shows how an image will look through a proof profile, such as a CMYK press profile,
and flags the colours the proof cannot reproduce.

```rust,ignore
use tpt_colour::icc::RenderingIntent;
use tpt_colour::manage::{ColourManager, SoftProof};

let mut manager = ColourManager::new();
// manager.register_icc("press", &press_profile_bytes)?;  // your CMYK or RGB proof profile
let proof = SoftProof::new(
    &mut manager,
    "display-p3", // the working space of the image
    "srgb",       // the proof: what the output can show
    "srgb",       // the display you are viewing on
    RenderingIntent::RelativeColorimetric,
)?;

let shown = proof.proof_pixel(&[0.0, 1.0, 0.0])?;
if shown.out_of_gamut {
    // the proof cannot reproduce this colour
}
```

## How the mask works

The mask compares each colour with its round trip, working → proof → working, in D50 Lab. It is
ΔE₀₀ above a threshold, which defaults to 2.0 (`DEFAULT_THRESHOLD`). Change it with
`with_threshold`.

The proof can have up to four channels, so CMYK profiles work. The round trip uses the same
intent you pass in, so `RelativeColorimetric` shows clipping, while `Perceptual` also shows the
compression it applies.

## Limits

The on-screen result is a single transform, not a calibrated display model. Use the display
profile for `display`, and treat the mask as a warning, not a measurement.
