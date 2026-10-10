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
- [x] Confirm licence: dual MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`; MIT copyright holder "TPT Solutions", matching tpt-simd)
- [x] Decide `tpt-simd` location/API: git dependency on `tpt-simd-vector` (github.com/tpt-solutions/tpt-simd, pinned to `be4f4c4`), behind the `simd` feature. No internal trait

## Phase 1 – Maths & Compositing (Weeks 1–2)
### `tpt_colour::space`
- [x] Define `ColourSpace` trait (const-generic channel count; `to_xyz` / `from_xyz`)
- [x] Wrap `palette`: sRGB, Linear RGB, Display P3, Rec. 2020 (implemented in-house with fixed op order; `palette` is a dev-dependency used as the parity oracle)
- [x] XYZ, Lab, LCh, Oklab, Oklch
- [x] CMYK type (naive fallback documented; ICC conversion pending Phase 2)
- [x] Chromatic adaptation (D50 ⇄ D65, Bradford)
- [x] YCbCr → RGB matrices (Rec. 709 / Rec. 2020) for tpt-kinetix
- [x] Deterministic maths policy: f32 API, f64 for single-colour path, fixed op order, no FMA, `libm` (no platform `std` maths). Single-colour matrix/TRC transforms (`ColourTransform::transform_rgb`, and managed transforms without a gamut) use exact f64 curves. LUT pipelines and gamut mapping still run in f32
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
- [ ] Optional `simd` acceleration of LUT lookups and matrix maths (matrix/TRC batches done; LUT lookups not yet)
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
- [x] Oracle crate: LittleCMS comparison (LittleCMS 2.17 via Pillow generated the fixtures). Covers system RSWOP CMYK and gamma-1.8 RGB, official sRGB v2 and v4 (LUT), and the Probe, PRMG and ROMM test profiles from `TPT_ICC_DIR`. Five cases diverge, see the ICC test profile item
- [x] Define parity tolerances against LittleCMS: ≤ 1 8-bit code (measured worst case is 1). Expressed in codes, not ΔE
- [x] Golden-file gradient mesh test (raw pixel bytes vs PNG, `png` is a dev-dependency only). The golden is self-generated, so it guards regressions rather than providing an independent reference
- [x] Fuzzing (`cargo-fuzz`) of `icc::parse()`: target in `fuzz/` compiles here and runs in CI on Linux. Not run locally (libFuzzer needs Linux). A deterministic mutation test runs in the normal suite (20 000 cases)
- [x] Cross-platform determinism test: recorded bit patterns on x86-64; ARM64 CI job added and wasm32 build checked. Only x86-64 has been run here
- [x] Benchmarks: `examples/bench.rs` (best of 5). Baked TRC tables took the scalar matrix path from about 2 to about 17 Mpx/s. The `simd` batch is bit-identical to scalar but about 2× slower in this kernel. Plain `[f32; 8]` arrays run at scalar speed, so the overhead is in tpt-simd-vector's lane ops. Feed this back upstream
- [x] `#![forbid(unsafe_code)]` where possible; audit any `unsafe`: `unsafe_code = forbid` at workspace level and no `unsafe` in the source

## LittleCMS parity follow-ups
- [ ] Port LittleCMS's destination-version rule: perceptual and saturation force black point compensation when the destination profile is v4 (we check the source). Land together with the next item, since RSWOP depends on the detected black
- [ ] Port LittleCMS's round-trip black point detection for CMYK, RGB and grey LUT profiles (Adobe-style search). Our compensation uses the device black directly
- [ ] Explain the PRMG relative residual: 7 codes on 5 out-of-gamut samples, where LittleCMS keeps a small positive linear red and we clip to 0. Ruled out so far: black compensation, white patch, 16-bit PCS quantisation
- [ ] Explain the ROMM residuals: 9 codes relative and 6 codes perceptual on saturated colours, with XYZ PCS
- [ ] Explain the Probe v4 divergence (84 codes relative, 255 perceptual). The profile's proofing tables are deliberately distorted, so this may be a stress case rather than a bug
- [ ] Re-enable the five `#[ignore]`d oracle cases once the items above are resolved
- [ ] Run iccDEV's Profile Dump validator on the official and test profiles

## Open Items (non-blocking, assumed defaults)
- [x] Minimum Rust version: 1.85 for the default build (checked). The `simd` feature needs 1.95 (tpt-simd)
- [ ] Which ICC test profile set to use as fixtures. Done: official sRGB v2 and v4 committed (ICC licence allows copying); Probe v4, PRMG and ROMM checked against LittleCMS from `TPT_ICC_DIR` (not committed; "as is" terms). Matched within 1 code: sRGB v2/v4, PRMG perceptual, RSWOP. Implemented from the LittleCMS source: fallback to the perceptual table when an intent's table is missing, and the white-on-white node patch for Lab-PCS LUTs. Still open, five cases ignored in the oracle with measured errors: (1) Probe v4 CMYK: 84 codes relative and 255 perceptual; its proofing tables are deliberately distorted, and LittleCMS's black handling differs. (2) PRMG relative: 7 codes on 5 out-of-gamut samples, where LittleCMS keeps a small positive linear red. Ruled out: black compensation, white patch, and 16-bit PCS quantisation. (3) ROMM relative 9 and perceptual 6 codes on saturated colours, XYZ PCS, cause not isolated. Not yet ported: LittleCMS's destination-version rule for perceptual compensation (LittleCMS checks the destination profile, we check the source), and its Adobe-style round-trip black detection for LUT profiles. These must go together, because RSWOP depends on the detected black. iccDEV's validator is still not run.
- [x] Scope of `mpet` support in v1.0: out of scope. Returns `Unsupported`
