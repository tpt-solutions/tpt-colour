//! {{project-name}}: a starting point for a colour pipeline on tpt-colour.

use tpt_colour::contrast::wcag_contrast;
use tpt_colour::parse::parse_colour;
use tpt_colour::{ColourManager, RenderingIntent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse a CSS colour into 8-bit RGBA.
    let brand = parse_colour("#2d7de0").ok_or("not a colour")?;
    println!("brand (8-bit): {brand:?}");

    // Check it against white for WCAG 2 contrast.
    let as_float = [brand[0], brand[1], brand[2]].map(|c| f32::from(c) / 255.0);
    println!("contrast on white: {:.2}:1", wcag_contrast(as_float, [1.0; 3]));

    {%- if colour_space_example %}

    // Convert it to Display P3, with a transform the manager caches for next time.
    let mut colours = ColourManager::new();
    let to_p3 = colours.transform("srgb", "display-p3", RenderingIntent::Perceptual)?;
    let p3 = to_p3.transform_rgb(as_float)?;
    println!("display-p3: {p3:?}");
    {%- endif %}

    Ok(())
}
