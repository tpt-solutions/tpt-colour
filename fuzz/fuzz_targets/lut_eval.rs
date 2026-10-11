#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_colour_icc::{IccProfile, ProfileModel, RenderingIntent};

fuzz_target!(|data: &[u8]| {
    // Any parsed LUT must evaluate without panicking, for every intent, with inputs that include
    // NaN, infinities and out-of-range values. Output must stay finite and in [0, 1].
    let Ok(profile) = IccProfile::parse(data) else {
        return;
    };
    let ProfileModel::Lut(tables) = &profile.model else {
        return;
    };
    let intents = [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ];
    let probes = [0.0_f32, 0.5, 1.0, -3.0, 7.0, f32::NAN, f32::INFINITY];
    for intent in intents {
        let Some(pipeline) = tables.a2b(intent) else {
            continue;
        };
        for &p in &probes {
            let input = vec![p; pipeline.input_channels()];
            let mut out = vec![0.0_f32; pipeline.output_channels()];
            if pipeline.eval(&input, &mut out).is_ok() {
                assert!(out.iter().all(|v| (0.0..=1.0).contains(v)), "{out:?}");
            }
        }
    }
});
