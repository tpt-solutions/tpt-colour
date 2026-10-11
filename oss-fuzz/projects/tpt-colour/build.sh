#!/bin/bash -eu
# `set` is repeated in the body, because the shebang is ignored when the script is run as `bash build.sh`.
set -eu
# Builds every fuzz target in the repository's fuzz/ workspace and copies each binary, with its
# seed corpus, into $OUT. OSS-Fuzz runs the binaries; this script only builds them.

cd "$SRC/tpt-colour/fuzz"

TARGETS="icc_parse lut_eval transform_full parse_css"

for target in $TARGETS; do
  cargo fuzz build -O --debug-assertions "$target"
  cp "target/x86_64-unknown-linux-gnu/release/$target" "$OUT/"

  # Seed corpus, zipped as OSS-Fuzz expects: <target>_seed_corpus.zip.
  if [ -d "corpus/$target" ]; then
    zip -j "$OUT/${target}_seed_corpus.zip" "corpus/$target/"*
  fi
done
