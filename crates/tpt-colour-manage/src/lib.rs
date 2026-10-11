//! Rendering intents, gamut mapping, profile management and HDR tone mapping (`no_std` + `alloc`).
//!
//! - [`Gamut`] describes an RGB gamut as a boundary in XYZ, and tests or clips colours against it.
//! - [`map_to_gamut`] maps out-of-gamut colours into a gamut by reducing OKLCH chroma, as
//!   CSS Color Level 4 specifies. Lightness and hue are kept.
//! - [`ColourManager`] resolves profiles by name and caches the transforms built from them.
//! - [`SoftProof`] previews a working-space image through a proof profile and flags the colours
//!   the proof cannot reproduce.
//! - [`tone`] holds the PQ transfer function and luminance tone mapping for HDR content.
//!
//! # Example
//!
//! Gamut-map a Display P3 green into sRGB, keeping its lightness and hue.
//!
//! ```
//! use tpt_colour_core::space::{ColourSpace, DisplayP3};
//! use tpt_colour_manage::{Gamut, map_to_gamut};
//!
//! let srgb = Gamut::srgb();
//! let green = DisplayP3.to_xyz([0.0, 1.0, 0.0]);
//! assert!(!srgb.contains(green));
//! assert!(srgb.contains(map_to_gamut(&srgb, green)));
//! ```
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
