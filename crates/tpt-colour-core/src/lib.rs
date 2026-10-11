//! Colour spaces, maths and alpha compositing (`no_std`).
//!
//! Operations use `f32` with a fixed evaluation order and no fused multiply-add.
#![no_std]

mod math;

pub mod composite;
pub mod contrast;
pub mod difference;
pub mod hex;
pub mod mix;
pub mod models;
pub mod space;
