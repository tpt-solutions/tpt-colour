//! Previews a Display P3 working image through an sRGB "proof", and flags the colours the
//! proof cannot reproduce.
//!
//! Run with `cargo run -p tpt-colour-manage --example soft_proof`. In production the proof
//! profile would be a press or device profile, such as a CMYK FOGRA39 file loaded with
//! `register_icc`. sRGB stands in here because it is built in and clearly narrower than P3.

use tpt_colour_icc::RenderingIntent;
use tpt_colour_manage::{ColourManager, SoftProof};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Working space, proof and display. The display is the monitor showing the preview.
    let mut colour = ColourManager::new();
    let proof = SoftProof::new(
        &mut colour,
        "display-p3",
        "srgb",
        "srgb",
        RenderingIntent::Perceptual,
    )?
    .with_threshold(2.0);

    // Working-space pixels in [0, 1]. Pure P3 green is far outside sRGB.
    let samples: [[f32; 3]; 3] = [[0.9, 0.5, 0.2], [0.0, 1.0, 0.0], [0.2, 0.6, 0.9]];
    for working in samples {
        let proofed = proof.proof_pixel(&working)?;
        println!(
            "working {working:.2?} -> proof {:.3?} out of gamut: {}",
            proofed.rgb, proofed.out_of_gamut
        );
    }
    Ok(())
}
