//! Rendering intents, gamut mapping, profile management and HDR tone mapping (`no_std` + `alloc`).
//!
//! - [`Gamut`] describes an RGB gamut as a boundary in XYZ, and tests or clips colours against it.
//! - [`map_to_gamut`] maps out-of-gamut colours into a gamut by reducing OKLCH chroma, as
//!   CSS Color Level 4 specifies. Lightness and hue are kept.
//! - [`ColourManager`] resolves profiles by name and caches the transforms built from them.
//! - [`SoftProof`] previews a working-space image through a proof profile and flags the colours
//!   the proof cannot reproduce.
//! - [`tone`] holds the PQ transfer function and luminance tone mapping for HDR content.
#![no_std]
extern crate alloc;

mod gamut;
mod manager;
mod map;
mod proof;
pub mod tone;

pub use gamut::{Gamut, WHITE_D50};
pub use manager::{ColourManager, ManageError, ManagedTransform};
pub use map::map_to_gamut;
pub use proof::{DEFAULT_THRESHOLD, Proofed, SoftProof};
