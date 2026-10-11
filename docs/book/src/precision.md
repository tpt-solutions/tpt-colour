# Precision: f32 and f64

tpt-colour uses two numeric paths, and the difference is visible in the last bit or two.

- **Bulk transforms are f32.** Pixel buffers, LUT pipelines and gamut mapping run in `f32`.
  Curves are baked into 4096-point tables, which agrees with the exact curve to about 2e-5.
- **Single-colour matrix/TRC transforms are f64.** `ColourTransform::transform_rgb` and the
  managed transforms without a gamut use the profile's own curves in `f64`. The result is
  accurate to f64 precision.

The two paths differ by about 2e-5 in the normal range, which is well under one 8-bit code.

## Why not all f64

Bulk paths are the hot ones. Per-pixel `f64` would roughly halve throughput for little visible
gain, since the output is 8 or 16 bits. The single-colour path is the one where exactness is
cheap and easy to check.

## Pure power curves stay analytic

A table is accurate to about 2e-5 in general, but near black a pure power curve (γ) baked into a
table is off by about 1.6 codes. Pure power curves are therefore kept as analytic formulas, with
the inverse `Gamma(1/γ)`. The sRGB parametric curve was already within 0.004 codes, so it is
unchanged.

## Choosing

- Use `transform_rgb` for one colour where you want the exact answer.
- Use `transform_u8`, `transform_u16` or the image forms for buffers.
- Use `bake_clut` to turn a transform into a 33³ table for a GPU or a fast loop. Its mean error
  against the exact transform is about 0.3 codes for Display P3 to sRGB.
