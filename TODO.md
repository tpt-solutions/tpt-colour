# tpt-colour – Task Checklist

Conventions: British English everywhere (`colour`, `ColourSpace`, `tpt_colour`, "grey", "normalise"). Pure Rust, no C-FFI in shipped crates. `no_std` (+ `alloc` for ICC). Scalar first; `simd` is an optional feature.

## Phase 0 – Setup
- [x] Read spec, resolve clarifying questions
- [x] Cargo workspace: `tpt-colour-core`, `tpt-colour-icc`, `tpt-colour-manage`, `tpt-colour` (facade)
- [x] `no_std` skeletons compile (`cargo check`)
- [x] Excluded `tpt-colour-oracle` crate placeholder (LittleCMS, tests only)
- [x] Rename spec references `tpt-color` → `tpt-colour`
- [x] Add `rustfmt.toml`, clippy config, `deny.toml` (ban C-FFI/`-sys` crates in shipped crates)
- [x] CI: build, test, clippy, `no_std` target check (e.g. `thumbv7em-none-eabihf`)
- [x] README with British spelling and crate overview
- [ ] Confirm licence (currently placeholder `MIT OR Apache-2.0`)
- [ ] Decide `tpt-simd` location/API; define internal SIMD trait behind `simd` feature

## Phase 1 – Maths & Compositing (Weeks 1–2)
### `tpt_colour::space`
- [x] Define `ColourSpace` trait (const-generic channel count; `to_xyz` / `from_xyz`)
- [x] Wrap `palette`: sRGB, Linear RGB, Display P3, Rec. 2020 (implemented in-house with fixed op order; `palette` is a dev-dependency used as the parity oracle)
- [x] XYZ, Lab, LCh, Oklab, Oklch
- [x] CMYK type (naive fallback documented; ICC conversion pending Phase 2)
- [x] Chromatic adaptation (D50 ⇄ D65, Bradford)
- [x] YCbCr → RGB matrices (Rec. 709 / Rec. 2020) for tpt-kinetix
- [ ] Deterministic maths policy: f32 API, f64 for single-colour path, fixed op order, no FMA, `libm` (no platform `std` maths)
- [x] Tests: round-trips, known reference values

### `tpt_colour::composite`
- [x] `PremultipliedRgba` and straight ⇄ premultiplied conversion
- [x] `blend_over`
- [x] Porter-Duff: SourceOver, SourceIn, SourceOut, SourceAtop, Destination*, Xor, Clear
- [x] Blend modes: Multiply, Screen, Overlay (+ Darken, Lighten, etc.)
- [x] Tests for edge alphas (0, 1), no dark fringing on interpolation
- [x] Confirm `no_std` build of whole core crate

## Phase 2 – ICC Parser & Matrix/TRC (Weeks 3–5)
- [x] Header parsing (v2 and v4), profile class, data/PCS colour space, intent
- [x] Tag table parsing with bounds checking
- [x] Tag types: `XYZ `, `curv`, `para` (types 0–4), `text`/`mluc`/`desc` (text, mluc and desc are parsed but only desc is exercised by real files)
- [x] TRC evaluation (parametric + table, interpolation)
- [x] `rXYZ/gXYZ/bXYZ`, `wtpt`, `chad` handling
- [x] Matrix/TRC transform fast path
- [x] `IccProfile`, `ProfileConnectionSpace`, `RenderingIntent`, `IccVersion`
- [x] `ColorTransform` (→ `ColourTransform`) with `transform_pixels(&mut [[f32; 4]])`
- [x] Transform building: source → PCS → destination
- [x] **Milestone:** sRGB ⇄ Display P3 accurate
- [x] Built-in sRGB / Display P3 profiles (no file needed)

## Phase 3 – Complex LUTs & CMYK (Weeks 6–8)
- [x] `lut8Type` (`mft1`) and `lut16Type` (`mft2`) — the matrix element is ignored (XYZ-encoded data only)
- [x] `lutAtoBType` / `lutBtoAType` (`mAB `/`mBA `)
- [x] `multiProcessElements` (`mpet`) — **decided: out of scope for v1.0**; returns `Unsupported`
- [x] `TransformGraph` stage evaluator (`Pipeline`) with safe, bounded execution (stage, channel and CLUT-size limits)
- [x] n-D CLUT: tetrahedral for 3 inputs, multilinear otherwise (4-input CMYK uses multilinear)
- [x] Rendering-intent tags (`A2B0/1/2`, `B2A0/1/2`), black point compensation
- [x] Lab PCS encodings (v2 vs v4)
- [ ] Optional `simd` acceleration of LUT lookups and matrix maths
- [x] **Milestone:** CMYK (FOGRA39 / SWOP) ⇄ Lab, ready for tpt-doc (SWOP verified against the system `RSWOP.icm`; FOGRA39 not available here, so not yet verified)

## Phase 4 – Gamut Mapping & Polish (Weeks 9–10)
- [x] `RenderingIntent`: Perceptual, Relative, Saturation, Absolute (Perceptual gamut-maps; Saturation and Absolute clip, with black point compensation on all but Absolute)
- [x] CSS Color 4 OKLCH gamut mapping (binary search on chroma with ΔEOK)
- [x] Gamut checks for sRGB, Display P3, Rec. 2020
- [x] `ColourManager` – profile resolution and transform caching (re-registering a profile drops its cached transforms)
- [x] HDR tone mapping (for tpt-kinetix): PQ (ST 2084) transfer and extended Reinhard on luminance; HLG not yet
- [x] Docs and examples for tpt-doc, tpt-vertex, tpt-kinetix integration (facade doctest, `convert` example, README)
- [ ] Publish v1.0.0 (not done: publishing is outward-facing and needs confirmation; version is still 0.1.0)

## Testing & Quality
- [ ] Oracle crate: LittleCMS comparison against official ICC test profiles (not shipped)
- [ ] Define parity tolerances (ΔE) against LittleCMS
- [ ] Golden-file gradient mesh test (raw pixel bytes vs PNG, PNG decoding as dev-dependency only)
- [ ] Fuzzing (`cargo-fuzz`) of `icc::parse()` – malformed, truncated, malicious files; no panics, no leaks
- [ ] Cross-platform determinism test (identical bits on x86-64 / ARM / WASM)
- [ ] Benchmarks (scalar vs `simd`)
- [ ] `#![forbid(unsafe_code)]` where possible; audit any `unsafe`

## Open Items (non-blocking, assumed defaults)
- [ ] Minimum Rust version (set to 1.85, edition 2024)
- [ ] Which ICC test profile set to use as fixtures
- [ ] Scope of `mpet` support in v1.0
