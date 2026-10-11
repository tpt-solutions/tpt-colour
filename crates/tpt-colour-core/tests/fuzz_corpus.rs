//! Replays the `parse_css` fuzz corpus, plus single-byte mutations, through the same logic the
//! fuzz target runs. This runs on every platform, so the seeds are checked even where libFuzzer
//! cannot run.

use std::fs;
use std::path::PathBuf;

use tpt_colour_core::parse::parse_colour;

fn seeds() -> Vec<Vec<u8>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus/parse_css");
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).expect("corpus directory exists") {
        out.push(fs::read(entry.expect("entry").path()).expect("seed readable"));
    }
    assert!(!out.is_empty(), "no seeds in {}", dir.display());
    out
}

#[test]
fn every_seed_and_mutation_parses_without_panicking() {
    for seed in seeds() {
        let step = (seed.len() / 32).max(1);
        let mut inputs = vec![seed.clone()];
        for i in (0..seed.len()).step_by(step) {
            for value in [0x00_u8, b'(', b'%', 0xff] {
                let mut mutated = seed.clone();
                mutated[i] = value;
                inputs.push(mutated);
            }
        }
        for bytes in inputs {
            if let Ok(text) = core::str::from_utf8(&bytes) {
                let _ = parse_colour(text);
            }
        }
    }
}

#[test]
fn seeds_cover_both_accepted_and_rejected_inputs() {
    let parsed = seeds()
        .iter()
        .filter(|s| parse_colour(core::str::from_utf8(s).unwrap_or("")).is_some())
        .count();
    let rejected = seeds().len() - parsed;
    assert!(
        parsed > 0 && rejected > 0,
        "parsed {parsed}, rejected {rejected}"
    );
}
