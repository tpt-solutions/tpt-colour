## What this changes

## Why

## Checklist

- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] `cargo test --workspace` passes (and `--features tpt-colour/simd` if the change touches batching)
- [ ] New behaviour has a test, and any new parser path has a bounded-input test
- [ ] Recorded values (determinism bits, golden PNG) changed only on purpose, and the reason is above
- [ ] `CHANGELOG.md` updated under `[Unreleased]`, with breaking changes marked
- [ ] British English in code, comments and docs
