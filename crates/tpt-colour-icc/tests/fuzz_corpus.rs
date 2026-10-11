//! Replays the fuzz corpus through the same logic the fuzz targets run, plus byte flips. This
//! runs on every platform, so the seeds are checked even where libFuzzer cannot run.

use std::fs;
use std::path::PathBuf;

use tpt_colour_icc::{ColourTransform, IccProfile, ProfileModel, RenderingIntent};

fn corpus(target: &str) -> Vec<Vec<u8>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus")
        .join(target);
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).expect("corpus directory exists") {
        out.push(fs::read(entry.expect("entry").path()).expect("seed readable"));
    }
    assert!(!out.is_empty(), "no seeds in {}", dir.display());
    out
}

/// Each seed, plus single-byte mutations at a spread of positions.
fn variants(seed: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    let step = (seed.len() / 64).max(1);
    std::iter::once(seed.to_vec()).chain((0..seed.len()).step_by(step).flat_map(move |i| {
        [0x00_u8, 0xff].into_iter().map(move |v| {
            let mut m = seed.to_vec();
            m[i] = v;
            m
        })
    }))
}

#[test]
fn lut_eval_seeds_never_panic_and_stay_in_range() {
    for seed in corpus("lut_eval") {
        for data in variants(&seed) {
            let Ok(profile) = IccProfile::parse(&data) else {
                continue;
            };
            let ProfileModel::Lut(tables) = &profile.model else {
                continue;
            };
            for intent in [
                RenderingIntent::Perceptual,
                RenderingIntent::RelativeColorimetric,
                RenderingIntent::Saturation,
                RenderingIntent::AbsoluteColorimetric,
            ] {
                let Some(pipeline) = tables.a2b(intent) else {
                    continue;
                };
                for p in [0.0_f32, 0.5, 1.0, -3.0, 7.0, f32::NAN, f32::INFINITY] {
                    let input = vec![p; pipeline.input_channels()];
                    let mut out = vec![0.0_f32; pipeline.output_channels()];
                    if pipeline.eval(&input, &mut out).is_ok() {
                        assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "{out:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn transform_full_seeds_never_panic() {
    for seed in corpus("transform_full") {
        for data in variants(&seed) {
            let Some((&selector, rest)) = data.split_first() else {
                continue;
            };
            let Ok(profile) = IccProfile::parse(rest) else {
                continue;
            };
            let intent = match selector % 4 {
                0 => RenderingIntent::Perceptual,
                1 => RenderingIntent::RelativeColorimetric,
                2 => RenderingIntent::Saturation,
                _ => RenderingIntent::AbsoluteColorimetric,
            };
            let srgb = IccProfile::srgb();
            let forward = ColourTransform::new(&profile, &srgb, intent);
            let backward = ColourTransform::new(&srgb, &profile, intent);
            for t in [forward, backward].into_iter().flatten() {
                let input = vec![0.25_f32; t.src_channels()];
                let mut out = vec![0.0_f32; t.dst_channels()];
                let _ = t.transform(&input, &mut out);
                let mut pixels = [[0.5_f32; 4]; 3];
                let _ = t.transform_pixels(&mut pixels);
            }
        }
    }
}
