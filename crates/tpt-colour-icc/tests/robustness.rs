//! Deterministic mutation testing of the parser and transforms.
//!
//! A seeded xorshift PRNG applies random multi-byte edits, truncations and splices to valid
//! profiles, then parses and transforms each result. Any panic fails the test. Real coverage-
//! guided fuzzing lives in `fuzz/` and needs Linux (libFuzzer); this test runs everywhere.

use tpt_colour_icc::{ColourTransform, IccProfile, RenderingIntent};

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// A valid CMYK → Lab mAB profile, used as the mutation seed.
fn seed_lut_profile() -> Vec<u8> {
    let grid = 3_usize;
    let mut samples = Vec::new();
    for i in 0..grid.pow(4) {
        for k in 0..3 {
            samples.push(((i * 7 + k * 13) % 65_536) as u16);
        }
    }
    let mut tag = vec![0_u8; 32];
    tag[0..4].copy_from_slice(b"mAB ");
    tag[8] = 4;
    tag[9] = 3;
    let clut_at = 32 + 12 * 4;
    tag[24..28].copy_from_slice(&(clut_at as u32).to_be_bytes());
    tag[28..32].copy_from_slice(&32_u32.to_be_bytes());
    for _ in 0..4 {
        tag.extend(b"curv\0\0\0\0\0\0\0\0");
    }
    let mut grid_bytes = [0_u8; 16];
    grid_bytes[..4].fill(grid as u8);
    tag.extend(grid_bytes);
    tag.extend([2, 0, 0, 0]);
    for v in samples {
        tag.extend(v.to_be_bytes());
    }
    assemble(*b"CMYK", *b"Lab ", &tag)
}

fn assemble(data: [u8; 4], pcs: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = vec![0_u8; 128];
    let total = 128 + 16 + body.len();
    out[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    out[8] = 4;
    out[12..16].copy_from_slice(b"prtr");
    out[16..20].copy_from_slice(&data);
    out[20..24].copy_from_slice(&pcs);
    out[36..40].copy_from_slice(b"acsp");
    out.extend(1_u32.to_be_bytes());
    out.extend(b"A2B0");
    out.extend(144_u32.to_be_bytes());
    out.extend((body.len() as u32).to_be_bytes());
    out.extend(body);
    out
}

fn exercise(bytes: &[u8]) {
    if let Ok(profile) = IccProfile::parse(bytes) {
        if let Ok(t) =
            ColourTransform::new(&profile, &IccProfile::srgb(), RenderingIntent::Perceptual)
        {
            let mut out = [0.0_f32; 3];
            let _ = t.transform(&vec![0.5; t.src_channels()], &mut out);
        }
    }
}

#[test]
fn random_mutations_never_panic() {
    let seed = seed_lut_profile();
    IccProfile::parse(&seed).expect("the seed profile is valid");
    let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
    for _ in 0..5_000 {
        let mut bytes = seed.clone();
        match rng.below(3) {
            0 => {
                // Overwrite a few random bytes.
                for _ in 0..=rng.below(8) {
                    let i = rng.below(bytes.len());
                    bytes[i] = rng.next() as u8;
                }
            }
            1 => {
                // Truncate, with the declared size left intact.
                let keep = rng.below(bytes.len());
                bytes.truncate(keep);
            }
            _ => {
                // Splice a random run of bytes over a random position.
                let at = rng.below(bytes.len());
                let len = rng.below(64).min(bytes.len() - at);
                for b in &mut bytes[at..at + len] {
                    *b = rng.next() as u8;
                }
            }
        }
        exercise(&bytes);
    }
}
