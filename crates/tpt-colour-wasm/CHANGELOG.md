# Changelog

All notable changes to `tpt-colour-wasm` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The crate is not published, so its version
tracks the workspace.

## [Unreleased]

### Added

- WebAssembly bindings through `wasm-bindgen`: parsing, contrast, vision simulation, harmonies, the
  `Manager` profile registry, and `profileDescription`.
- The playground in `playground/` is built from this crate. Its tests run under Node with
  `wasm-pack test --node`.

## [0.1.0]

Initial workspace release. The crate was unpublished at this point.
