//! Stacks two semi-transparent layers with Porter-Duff and blend modes.
//!
//! Run with `cargo run -p tpt-colour --example composite_layers`.

use tpt_colour::composite::{BlendMode, PorterDuff, PremultipliedRgba, blend};

fn main() {
    // A 50% red layer over an opaque blue backdrop.
    let backdrop = PremultipliedRgba::from_straight([0.0, 0.0, 1.0, 1.0]);
    let layer = PremultipliedRgba::from_straight([1.0, 0.0, 0.0, 0.5]);

    let over = PorterDuff::SourceOver.compose(layer, backdrop);
    println!("source-over      {:.3?}", over.to_straight());

    // Multiply darkens the backdrop wherever the layer is coloured.
    let multiplied = blend(BlendMode::Multiply, layer, backdrop);
    println!("multiply         {:.3?}", multiplied.to_straight());

    // Colour dodge brightens it instead.
    let dodged = blend(BlendMode::ColorDodge, layer, backdrop);
    println!("color dodge      {:.3?}", dodged.to_straight());
}
