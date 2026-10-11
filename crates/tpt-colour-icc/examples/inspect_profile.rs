//! Prints the header and the device model of an ICC profile.
//!
//! Run with a path, `cargo run -p tpt-colour-icc --example inspect_profile -- path/to/profile.icc`,
//! or without one to inspect the built-in sRGB profile.

use std::env;
use std::fs;

use tpt_colour_icc::{IccProfile, ProfileModel, RenderingIntent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = match env::args().nth(1) {
        Some(path) => IccProfile::parse(&fs::read(path)?)?,
        None => IccProfile::srgb(),
    };

    println!(
        "description  {}",
        profile.description.as_deref().unwrap_or("(none)")
    );
    println!(
        "version      {}.{}",
        profile.version.major, profile.version.minor
    );
    println!("class        {:?}", profile.class);
    println!("PCS          {:?}", profile.pcs);
    println!("channels     {}", profile.device_channels);
    println!("white point  {:.4?}", profile.white_point);

    match &profile.model {
        ProfileModel::Matrix(_) => println!("model        matrix/TRC"),
        ProfileModel::Gray(_) => println!("model        grey TRC"),
        ProfileModel::Lut(tables) => {
            println!("model        LUT");
            // Missing intents fall back to the perceptual table, as the ICC spec allows.
            for intent in [
                RenderingIntent::Perceptual,
                RenderingIntent::RelativeColorimetric,
                RenderingIntent::Saturation,
                RenderingIntent::AbsoluteColorimetric,
            ] {
                let has = tables.a2b(intent).is_some();
                println!("  device to PCS for {intent:?}: {has}");
            }
        }
    }
    Ok(())
}
