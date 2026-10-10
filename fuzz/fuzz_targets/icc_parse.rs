#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

fuzz_target!(|data: &[u8]| {
    // Parsing must never panic, whatever the bytes. If it parses, building and running a
    // transform must not panic either.
    if let Ok(profile) = IccProfile::parse(data) {
        let srgb = IccProfile::srgb();
        if let Ok(t) = ColourTransform::new(&profile, &srgb, RenderingIntent::Perceptual) {
            let input = vec![0.5_f32; t.src_channels()];
            let mut out = [0.0_f32; 3];
            let _ = t.transform(&input, &mut out);
        }
    }
});
