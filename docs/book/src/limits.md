# Limits and what is not supported

These are the known gaps. Each one returns an error rather than a wrong answer, where that is
possible.

## ICC

- `multiProcessElements` (`mpet`, the `D2Bx`/`B2Dx` tags, and `mpet` in `A2B`/`B2A`) returns
  `Unsupported`. The element layouts for the supported elements (`cvst`, `matf`, `clut`) are
  known from iccDEV's source, but no independent evaluator is available here to check a
  transform built from them. LittleCMS 2.19 cannot build a transform from the `mpet` test profile
  in iccDEV's corpus. So the implementation waits for a reference. The calculator (`calc`)
  elements are a separate, larger language, and are not planned.
- `ncl2` (named colour lists) is not read.
- Lab, XYZ, device-link and abstract profile classes with matrix/TRC data are not handled. LUT
  profiles with Lab data spaces are read through their tables.
- The 3×3 matrix on `mft1`/`mft2` tables is ignored. It applies only to XYZ-encoded PCS in B2A,
  which is rare, and checking it needs a reference profile.
- The ICC writer covers matrix/TRC RGB and grey profiles with an XYZ PCS. It does not write LUT
  profiles or Lab PCS profiles. See [ICC profiles](icc-profiles.md).

## Colour

- BT.2390, ACES tone mapping and AgX are not implemented. No reference implementation is installed
  in this environment, and the formulas are not written from memory. HLG and PQ tone mapping are
  there.
- Arbitrary white points are supported in chromatic adaptation (`adaptation_matrix`), but there is
  no `Lab` type with a configurable white. `Lab50` is the D50 case.
- YCoCg-R is implemented, and so is the float YCoCg. See [Colour spaces](colour-spaces.md).

## Bindings and packaging

- The crates are not published. Publishing is a decision for the maintainer.
- `tpt-simd-vector` is a git dependency, so the `simd` feature cannot be published to crates.io
  until `tpt-simd` is.
- The OSS-Fuzz project files are staged in `oss-fuzz/`, but not submitted. Submitting is a pull
  request against google/oss-fuzz from the maintainer's account.

## Precision

- Bulk paths are f32. Single-colour matrix/TRC transforms are f64. Perceptual HDR curves
  (ICtCp and Jzazbz) are evaluated in f64, because their exponents amplify f32 rounding. See
  [Precision](precision.md).
- Lookups in the `simd` feature are scalar. The LUT gather is slower than the scalar path with the
  current `tpt-simd` lane operations.
