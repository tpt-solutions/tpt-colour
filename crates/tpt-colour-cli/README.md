# tpt-colour-cli

A command-line tool for inspecting ICC profiles, converting colours and checking gamut. It is
installed as the `tpt-colour` binary.

Part of [tpt-colour](https://github.com/tpt-solutions/tpt-colour). This crate is not published to
crates.io. Build it from the repository.

## Build

```sh
cargo build --release -p tpt-colour-cli
./target/release/tpt-colour --help
```

## Commands

```text
tpt-colour info <profile>
tpt-colour doctor <profile>
tpt-colour convert <from> <to> [--intent NAME] R G B
tpt-colour gamut-check <space> R G B
tpt-colour cube <from> <to> <size> [--intent NAME]
```

A `<profile>` is `srgb`, `display-p3`, or a path to an ICC file. Colour values are in `0..=1`.
Intents are `perceptual`, `relative`, `saturation` and `absolute`.

| Command | Does |
| --- | --- |
| `info` | Prints a profile's version, class, PCS, model, description and the intents it supports, against sRGB |
| `doctor` | Reports problems that commonly break transforms: `multiProcessElements` tables, missing descriptions and tags, and sRGB round-trip error |
| `convert` | Converts one colour between two profiles |
| `gamut-check` | Says whether a colour is inside `srgb`, `display-p3` or `rec2020` |
| `cube` | Writes an Adobe `.cube` 3-D LUT to stdout |

## Examples

```sh
$ tpt-colour convert srgb display-p3 0.8 0.2 0.1
[0.73701, 0.25292, 0.15943]

$ tpt-colour gamut-check srgb 0 1 0
inside sRGB: yes

$ tpt-colour cube srgb display-p3 17 --intent relative > srgb_to_p3.cube
```

To inspect your own profile, run `tpt-colour info FOGRA39.icc`. If a transform misbehaves, run
`tpt-colour doctor` on the same file.

## Where this fits

The CLI wraps the [`tpt-colour`](../tpt-colour/README.md) facade, so its output matches the
library.

## Licence

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your
option.
