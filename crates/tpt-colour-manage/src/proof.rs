//! Soft-proofing: shows how a working-space colour will look through a proof profile, such as a
//! CMYK press profile, and flags the colours the proof cannot reproduce.
//!
//! The mask compares each colour with its round trip working → proof → working, in D50 Lab. A
//! colour is flagged when that difference exceeds the threshold, which is the proof clipping or
//! compressing it. The proof can have any channel count (up to four), so CMYK is supported.

use alloc::sync::Arc;

use tpt_colour_core::difference::delta_e_2000;
use tpt_colour_core::space::{ColourSpace, Lab, d50_to_d65};
use tpt_colour_icc::{IccError, RenderingIntent};

use crate::{ColourManager, ManageError, ManagedTransform};

/// The default ΔE₀₀ above which a colour is flagged as out of gamut for the proof.
pub const DEFAULT_THRESHOLD: f32 = 2.0;

/// One proofed pixel: the colour as it appears on the display, and whether the proof changed it
/// by more than the threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Proofed {
    /// Display-space device values in `[0, 1]`.
    pub rgb: [f32; 3],
    /// Whether the proof cannot reproduce this colour.
    pub out_of_gamut: bool,
}

/// A prepared soft-proof: three cached transforms and a ΔE threshold.
#[derive(Clone, Debug)]
pub struct SoftProof {
    to_proof: Arc<ManagedTransform>,
    proof_to_working: Arc<ManagedTransform>,
    proof_to_display: Arc<ManagedTransform>,
    threshold: f32,
}

impl SoftProof {
    /// Prepares a proof of `working` through `proof`, shown on `display`.
    ///
    /// The three profiles are resolved through `manager`, so the transforms are cached like any
    /// other.
    ///
    /// # Errors
    ///
    /// Returns [`ManageError::UnknownProfile`] if a name is not registered, or
    /// [`ManageError::Icc`] if a transform cannot be built.
    pub fn new(
        manager: &mut ColourManager,
        working: &str,
        proof: &str,
        display: &str,
        intent: RenderingIntent,
    ) -> Result<Self, ManageError> {
        Ok(Self {
            to_proof: manager.transform(working, proof, intent)?,
            proof_to_working: manager.transform(proof, working, intent)?,
            proof_to_display: manager.transform(proof, display, intent)?,
            threshold: DEFAULT_THRESHOLD,
        })
    }

    /// Sets the ΔE₀₀ threshold for the out-of-gamut mask.
    #[must_use]
    pub fn with_threshold(mut self, delta_e: f32) -> Self {
        self.threshold = delta_e;
        self
    }

    /// Proofs one working-space pixel. The slice must hold the working profile's channels.
    ///
    /// # Errors
    ///
    /// Returns [`IccError::ChannelCount`] if `working` has the wrong number of channels for the
    /// working profile, or if either profile has more than four.
    pub fn proof_pixel(&self, working: &[f32]) -> Result<Proofed, IccError> {
        let working_channels = self.to_proof.inner().src_channels();
        let proof_channels = self.to_proof.inner().dst_channels();
        if working_channels > 4 || proof_channels > 4 {
            return Err(IccError::ChannelCount);
        }
        let mut proof = [0.0_f32; 4];
        self.to_proof
            .transform(working, &mut proof[..proof_channels])?;

        let mut rgb = [0.0_f32; 3];
        self.proof_to_display
            .transform(&proof[..proof_channels], &mut rgb)?;

        let mut back = [0.0_f32; 4];
        self.proof_to_working
            .transform(&proof[..proof_channels], &mut back[..working_channels])?;

        let original = lab_of(self.to_proof.inner().device_to_pcs(working)?);
        let round_trip = lab_of(
            self.to_proof
                .inner()
                .device_to_pcs(&back[..working_channels])?,
        );
        Ok(Proofed {
            rgb: rgb.map(|v| v.clamp(0.0, 1.0)),
            out_of_gamut: delta_e_2000(original, round_trip) > self.threshold,
        })
    }
}

/// D50 PCS XYZ to CIE Lab with the D65 reference white, which is what [`delta_e_2000`] expects.
fn lab_of(pcs: [f32; 3]) -> [f32; 3] {
    Lab.from_xyz(d50_to_d65(pcs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(working: &str, proof: &str) -> SoftProof {
        let mut manager = ColourManager::new();
        SoftProof::new(
            &mut manager,
            working,
            proof,
            "srgb",
            RenderingIntent::RelativeColorimetric,
        )
        .unwrap()
    }

    #[test]
    fn colours_inside_the_proof_gamut_are_not_flagged() {
        // sRGB sits inside Display P3, so the round trip through P3 is faithful.
        let p = proof("srgb", "display-p3");
        let out = p.proof_pixel(&[0.5, 0.4, 0.3]).unwrap();
        assert!(!out.out_of_gamut);
        for (a, b) in out.rgb.iter().zip([0.5, 0.4, 0.3]) {
            assert!((a - b).abs() < 2e-3, "{:?}", out.rgb);
        }
    }

    #[test]
    fn colours_outside_the_proof_gamut_are_flagged() {
        // Saturated Display P3 green lies outside sRGB, so an sRGB proof clips it.
        let p = proof("display-p3", "srgb");
        let out = p.proof_pixel(&[0.0, 1.0, 0.0]).unwrap();
        assert!(out.out_of_gamut);
        assert!(out.rgb.iter().all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn a_higher_threshold_stops_flagging() {
        let p = proof("display-p3", "srgb").with_threshold(1_000.0);
        assert!(!p.proof_pixel(&[0.0, 1.0, 0.0]).unwrap().out_of_gamut);
    }

    #[test]
    fn wrong_channel_count_is_an_error() {
        let p = proof("srgb", "display-p3");
        assert_eq!(p.proof_pixel(&[0.1, 0.2]), Err(IccError::ChannelCount));
    }
}
