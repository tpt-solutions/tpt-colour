# tpt-colour compared with other libraries

This page says what tpt-colour has been checked against, and what has not. It is deliberately
narrow. Where we have not measured a library, we say so.

## LittleCMS (lcms2)

LittleCMS is the reference for ICC behaviour here. We do not link it at runtime. The
`tpt-colour-oracle` crate compares our output with LittleCMS 2.17 fixtures, which were generated
with Pillow and `python-lcms2`.

- Matrix/TRC and LUT transforms agree with LittleCMS to within one 8-bit code in the tested cases.
  That includes the official sRGB v2 and v4 profiles, system RSWOP CMYK, and the test profiles
  read from `TPT_ICC_DIR`.
- Known divergence: perceptual device-to-PCS (B2A) into the official sRGB v4 LUT profile differs
  by up to 12 codes at some colours. Relative intent matches within one code. This is recorded as
  a known limitation for v1.0.
- Some residuals against LittleCMS's 8-bit path are LittleCMS's own rounding. Against its
  double-precision path we agree to about 0.2 codes.
- `multiProcessElements` (`mpet`, the `D2B`/`B2D` tags) is not supported. Transforms that need it
  return `Unsupported`. LittleCMS reads them in preference to `A2B` for some profiles, so results
  can differ for those.

## palette

The `palette` crate is a development-only dependency. Its job is a parity oracle for the
colour-space maths (sRGB, Display P3, Rec. 2020 and the Lab family), with a tolerance of 2e-3.
It is not a source of the implementation, which is in-house with a fixed operation order.

palette does not handle ICC profiles, so there is no ICC comparison to make against it.

## moxcms

We have not benchmarked or compared outputs against moxcms. Until we have, we make no claims about
its accuracy or speed. A parity report regenerated in CI is listed as open work.

## Choosing

- Use tpt-colour when you want pure Rust with no C dependency, `no_std` ICC transforms, and
  documented determinism.
- Use LittleCMS when you need its full feature set, including `mpet` and every profile class,
  and a C dependency is acceptable.
