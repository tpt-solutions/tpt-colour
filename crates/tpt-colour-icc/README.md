# tpt-colour-icc

ICC profile parsing (v2 and v4) and colour transforms, in pure Rust.

- Matrix/TRC profiles, and LUT profiles (`mft1`, `mft2`, `mAB `, `mBA `) for RGB, CMYK and grey.
- Rendering intents, with black point compensation that follows LittleCMS.
- Absolute colorimetric scales by the media white points.
- Bounded parsing: malformed or hostile data returns an error, and a per-profile CLUT budget
  limits memory.

`multiProcessElements` (`mpet`) tables are not supported, and intents that need them return
`Unsupported`. The crate is `no_std` and needs `alloc`.

Part of the [tpt-colour](https://github.com/tpt-solutions/tpt-colour) workspace.
