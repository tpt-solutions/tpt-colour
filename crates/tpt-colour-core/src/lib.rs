//! Colour spaces, maths and alpha compositing (`no_std`).
//!
//! Operations use `f32` with a fixed evaluation order and no fused multiply-add.
//!
//! # Example
//!
//! Convert an sRGB colour to Oklab, and measure the contrast of text on it.
//!
//! ```
//! use tpt_colour_core::contrast::wcag_contrast;
//! use tpt_colour_core::space::{ColourSpace, Oklab, SRgb};
//!
//! let orange = [1.0, 0.5, 0.0];
//! let [l, a, b] = Oklab.from_xyz(SRgb.to_xyz(orange));
//! assert!((l - 0.7312).abs() < 1e-3);
//! assert!(a > 0.0 && b > 0.0);
//!
//! // Black text on white is the highest possible contrast, 21:1.
//! let ratio = wcag_contrast([0.0; 3], [1.0; 3]);
//! assert!((ratio - 21.0).abs() < 1e-3);
//! ```
#![no_std]

mod math;
mod named;

pub mod composite;
pub mod contrast;
pub mod difference;
pub mod hex;
pub mod mix;
pub mod models;
pub mod palette;
pub mod parse;
pub mod space;
pub mod vision;
