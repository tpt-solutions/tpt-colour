//! tpt-colour: pure-Rust colour management for the TPT stack.
//!
//! This facade re-exports the workspace crates, so one dependency covers the whole stack:
//!
//! - [`space`] and [`composite`]: colour spaces, conversions and alpha compositing (`no_std`).
//! - [`icc`]: ICC profile parsing and transforms, for matrix/TRC and LUT profiles.
//! - [`manage`]: gamut mapping, the [`ColourManager`] profile registry, and HDR tone mapping.
//!
//! # Example
//!
//! Convert an sRGB colour to Display P3, then gamut-map a wide-gamut colour back into sRGB.
//!
//! ```
//! use tpt_colour::ColourManager;
//! use tpt_colour::icc::RenderingIntent;
//!
//! let mut colour = ColourManager::new();
//! let to_p3 = colour.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
//! let p3 = to_p3.transform_rgb([0.8, 0.2, 0.1])?;
//!
//! let to_srgb = colour.transform("display-p3", "srgb", RenderingIntent::Perceptual)?;
//! let back = to_srgb.transform_rgb(p3)?;
//! assert!((back[0] - 0.8).abs() < 2e-3);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![no_std]

pub use tpt_colour_core::{composite, contrast, difference, hex, mix, models, space};
pub use tpt_colour_icc as icc;
pub use tpt_colour_icc::RenderingIntent;
pub use tpt_colour_manage as manage;
pub use tpt_colour_manage::tone;
pub use tpt_colour_manage::{ColourManager, Gamut, ManageError, ManagedTransform, map_to_gamut};

/// The types most programs need, in one import: `use tpt_colour::prelude::*;`.
pub mod prelude {
    pub use crate::composite::{BlendMode, PorterDuff, PremultipliedRgba};
    pub use crate::icc::RenderingIntent;
    pub use crate::manage::{ColourManager, Gamut, ManagedTransform, map_to_gamut};
    pub use crate::space::{AdobeRgb, ColourSpace, DisplayP3, Lab, Oklab, Oklch, Rec2020, SRgb};
}
