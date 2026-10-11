//! Tone maps an HDR colour into display range, then encodes its luminance as PQ and HLG.
//!
//! Run with `cargo run -p tpt-colour --example hdr_tonemap`.

use tpt_colour::space::{ColourSpace, LinearSRgb, SRgb};
use tpt_colour::tone::{hlg_oetf, pq_oetf, tone_map_xyz};

/// Reference white for SDR content in PQ, in cd/m² (ITU-R BT.2408).
const REFERENCE_WHITE_NITS: f32 = 203.0;
/// PQ is absolute: a signal of 1.0 is 10 000 cd/m².
const PQ_PEAK_NITS: f32 = 10_000.0;

fn main() {
    // Linear light, where 1.0 is the reference white. The red is twice white, so it is HDR.
    let scene = [2.0_f32, 1.0, 0.25];
    let xyz = LinearSRgb.to_xyz(scene);
    println!("scene XYZ        {xyz:.4?}");

    // Roll off luminance above 4× white, keeping chromaticity. Then clamp for display.
    let mapped = tone_map_xyz(xyz, 4.0);
    let display = SRgb.from_xyz(mapped).map(|v| v.clamp(0.0, 1.0));
    println!("display sRGB     {display:.4?}");

    // PQ encodes absolute light, so scale the luminance to nits first.
    let nits = xyz[1] * REFERENCE_WHITE_NITS;
    println!("PQ signal        {:.4}", pq_oetf(nits / PQ_PEAK_NITS));

    // HLG encodes scene-relative light in [0, 1].
    println!("HLG signal       {:.4}", hlg_oetf(xyz[1].min(1.0)));
}
