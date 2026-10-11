# Contributing

Thanks for looking at tpt-colour. It is a colour-management library, so correctness and
determinism matter more than speed of change.

## Ground rules

- **British English** in code, comments and docs: `colour`, `grey`, `normalise`.
- **Pure Rust in shipped crates.** No C-FFI or `-sys` crates. The LittleCMS oracle lives in
  `crates/tpt-colour-oracle` and is never a dependency of a shipped crate.
- **Deterministic maths.** Keep the operation order fixed, avoid fused multiply-add, and use
  `libm` rather than platform `std` maths. `determinism` and `golden` tests check this.
- **No `unsafe`.** It is forbidden at the workspace level.
- **Bounded parsing.** Malformed input must return an error, never panic and never allocate
  without limit. Add a regression test for any new parser path.

## Before you open a pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --features tpt-colour/simd   # needs Rust 1.95
cargo check --workspace --target thumbv7em-none-eabihf
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The oracle tests need ICC test profiles that are not in the repository. Set `TPT_ICC_DIR` to a
directory that holds them. Under CI (`CI` set), a missing profile fails the test.

## Changing a golden or a recorded value

The golden PNG and the bit-pattern test in `crates/tpt-colour-manage/tests/` are regression
guards, not independent references. If a change is meant to alter them, say so in the pull
request, and explain why the new values are right. Regenerate the golden with
`TPT_UPDATE_GOLDEN=1 cargo test -p tpt-colour-manage --test golden`.

## Changelog

Add a line to `CHANGELOG.md` under `[Unreleased]` for any user-visible change. Mark breaking
changes as such.

## Licence

Contributions are dual-licensed under MIT OR Apache-2.0, as the rest of the project is.
