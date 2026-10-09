# tpt-colour – Task Checklist

Conventions: British English everywhere (`colour`, `ColourSpace`, `tpt_colour`, "grey", "normalise"). Pure Rust, no C-FFI in shipped crates. `no_std` (+ `alloc` for ICC). Scalar first; `simd` is an optional feature.

## Phase 0 – Setup
- [x] Read spec, resolve clarifying questions
- [x] Cargo workspace: `tpt-colour-core`, `tpt-colour-icc`, `tpt-colour-manage`, `tpt-colour` (facade)
- [x] `no_std` skeletons compile (`cargo check`)
- [x] Excluded `tpt-colour-oracle` crate placeholder (LittleCMS, tests only)
- [ ] Rename spec references `tpt-color` → `tpt-colour`
- [ ] Add `rustfmt.toml`, clippy config, `deny.toml` (ban C-FFI/`-sys` crates in shipped crates)
- [ ] CI: build, test, clippy, `no_std` target check (e.g. `thumbv7em-none-eabihf`)
- [ ] README with British spelling and crate overview
- [ ] Confirm licence (currently placeholder `MIT OR Apache-2.0`)
- [ ] Decide `tpt-simd` location/API; define internal SIMD trait behind `simd` feature

## Phase 1 – Maths & Compositing (Weeks 1–2)
### `tpt_colour::space`
- [ ] Define `ColourSpace` trait (const-generic channel count; `to_xyz` / `from_xyz`)
- [ ] Wrap `palette`: sRGB, Linear RGB, Display P3, Rec. 2020
- [ ] XYZ, Lab, LCh, Oklab, Oklch
- [ ] CMYK type (conversion via ICC; naive fallback documented)
- [ ] Chromatic adaptation (D50 ⇄ D65, Bradford)
- [ ] YCbCr → RGB matrices (Rec. 709 / Rec. 2020) for tpt-kinetix
- [ ] Deterministic maths policy: f32 API, f64 for single-colour path, fixed op order, no FMA, `libm` (no platform `std` maths)
- [ ] Tests: round-trips, known reference values

### `tpt_colour::composite`
- [ ] `PremultipliedRgba` and straight ⇄ premultiplied conversion
- [ ] `blend_over`
- [ ] Porter-Duff: SourceOver, SourceIn, SourceOut, SourceAtop, Destination*, Xor, Clear
- [ ] Blend modes: Multiply, Screen, Overlay (+ Darken, Lighten, etc.)
- [ ] Tests for edge alphas (0, 1), no dark fringing on interpolation
- [ ] Confirm `no_std` build of whole core crate

## Phase 2 – ICC Parser & Matrix/TRC (Weeks 3–5)
- [ ] Header parsing (v2 and v4), profile class, data/PCS colour space, intent
- [ ] Tag table parsing with bounds checking
- [ ] Tag types: `XYZ `, `curv`, `para` (types 0–4), `text`/`mluc`/`desc`
- [ ] TRC evaluation (parametric + table, interpolation)
- [ ] `rXYZ/gXYZ/bXYZ`, `wtpt`, `chad` handling
- [ ] Matrix/TRC transform fast path
- [ ] `IccProfile`, `ProfileConnectionSpace`, `RenderingIntent`, `IccVersion`
- [ ] `ColorTransform` (→ `ColourTransform`) with `transform_pixels(&mut [[f32; 4]])`
- [ ] Transform building: source → PCS → destination
- [ ] **Milestone:** sRGB ⇄ Display P3 accurate
- [ ] Built-in sRGB / Display P3 profiles (no file needed)

## Phase 3 – Complex LUTs & CMYK (Weeks 6–8)
- [ ] `lut8Type` (`mft1`) and `lut16Type` (`mft2`)
- [ ] `lutAtoBType` / `lutBtoAType` (`mAB `/`mBA `)
- [ ] `multiProcessElements` (`mpet`) – decide scope for v1
- [ ] `TransformGraph` stage evaluator (curve → CLUT → matrix → curve) with safe, bounded execution
- [ ] n-D CLUT with trilinear and tetrahedral interpolation
- [ ] Rendering-intent tags (`A2B0/1/2`, `B2A0/1/2`), black point compensation
- [ ] Lab PCS encodings (v2 vs v4)
- [ ] Optional `simd` acceleration of LUT lookups and matrix maths
- [ ] **Milestone:** CMYK (FOGRA39 / SWOP) ⇄ Lab, ready for tpt-doc

## Phase 4 – Gamut Mapping & Polish (Weeks 9–10)
- [ ] `RenderingIntent`: Perceptual, Relative, Saturation, Absolute
- [ ] CSS Color 4 OKLCH gamut mapping (binary search on chroma with ΔEOK)
- [ ] Gamut checks for sRGB, Display P3, Rec. 2020
- [ ] `ColourManager` – profile resolution and transform caching
- [ ] HDR tone mapping (for tpt-kinetix)
- [ ] Docs and examples for tpt-doc, tpt-vertex, tpt-kinetix integration
- [ ] Publish v1.0.0

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
