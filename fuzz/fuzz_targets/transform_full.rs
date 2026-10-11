#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

fuzz_target!(|data: &[u8]| {
    // The first byte picks the intent, so every intent is reached. A parsed profile must build
    // a transform against sRGB and both directions must run without panicking.
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    let Ok(profile) = IccProfile::parse(rest) else {
        return;
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
});
