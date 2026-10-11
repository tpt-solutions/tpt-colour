//! Command-line tool for tpt-colour: inspect ICC profiles, convert colours, and check gamut.
//!
//! ```text
//! tpt-colour info <profile>
//! tpt-colour doctor <profile>
//! tpt-colour convert <from> <to> [--intent NAME] R G B
//! tpt-colour gamut-check <space> R G B
//! tpt-colour cube <from> <to> <size> [--intent NAME]
//! ```
//!
//! A profile is `srgb`, `display-p3`, or a path to an ICC file. Colour values are in `0..=1`.

use std::process::ExitCode;

use tpt_colour::icc::{ColourTransform, IccProfile, ProfileModel, RenderingIntent};
use tpt_colour::space::{ColourSpace, DisplayP3, Rec2020, SRgb};
use tpt_colour::{ColourManager, Gamut, map_to_gamut};

const USAGE: &str = "usage:
  tpt-colour info <profile>
  tpt-colour doctor <profile>
  tpt-colour convert <from> <to> [--intent perceptual|relative|saturation|absolute] R G B
  tpt-colour gamut-check <srgb|display-p3|rec2020> R G B
  tpt-colour cube <from> <to> <size> [--intent NAME]   (writes a .cube LUT to stdout)

<profile> is srgb, display-p3, or a path to an ICC file.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("info") => info(args.get(1).ok_or("info needs a profile")?),
        Some("convert") => convert(&args[1..]),
        Some("gamut-check") => gamut_check(&args[1..]),
        Some("cube") => cube(&args[1..]),
        Some("doctor") => doctor(args.get(1).ok_or("doctor needs a profile")?),
        _ => Err("expected a command: info, convert or gamut-check".into()),
    }
}

/// Loads a built-in profile by name, or parses the ICC file at `name`.
fn load(name: &str) -> Result<IccProfile, String> {
    match name {
        "srgb" => Ok(IccProfile::srgb()),
        "display-p3" => Ok(IccProfile::display_p3()),
        path => {
            let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
            IccProfile::parse(&bytes).map_err(|e| format!("{path}: {e}"))
        }
    }
}

fn info(path: &str) -> Result<(), String> {
    let profile = load(path)?;
    let model = match profile.model {
        ProfileModel::Matrix(_) => "matrix/TRC",
        ProfileModel::Gray(_) => "grey TRC",
        ProfileModel::Lut(_) => "LUT",
    };
    println!(
        "version:     {}.{}",
        profile.version.major, profile.version.minor
    );
    println!("class:       {:?}", profile.class);
    println!("device:      {} channel(s)", profile.device_channels);
    println!("pcs:         {:?}", profile.pcs);
    println!("model:       {model}");
    println!(
        "description: {}",
        profile.description.as_deref().unwrap_or("(none)")
    );
    println!("intents, against sRGB:");
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ] {
        let status = match ColourTransform::new(&profile, &IccProfile::srgb(), intent) {
            Ok(_) => "available".to_string(),
            Err(e) => format!("unavailable ({e})"),
        };
        println!("  {intent:?}: {status}");
    }
    Ok(())
}

fn parse_intent(name: &str) -> Result<RenderingIntent, String> {
    name.parse()
        .map_err(|e| format!("unknown intent {name:?}: {e}"))
}

fn parse_rgb(values: &[String]) -> Result<[f32; 3], String> {
    if values.len() != 3 {
        return Err(format!(
            "expected three colour values, got {}",
            values.len()
        ));
    }
    let mut rgb = [0.0_f32; 3];
    for (slot, text) in rgb.iter_mut().zip(values) {
        let v: f32 = text
            .parse()
            .map_err(|_| format!("{text:?} is not a number"))?;
        if !(0.0..=1.0).contains(&v) {
            return Err(format!("{v} is outside 0..=1"));
        }
        *slot = v;
    }
    Ok(rgb)
}

fn convert(args: &[String]) -> Result<(), String> {
    let mut intent = RenderingIntent::Perceptual;
    let mut positional = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--intent" {
            let value = iter.next().ok_or("--intent needs a value")?;
            intent = parse_intent(value)?;
        } else {
            positional.push(arg.clone());
        }
    }
    let [from, to, rest @ ..] = positional.as_slice() else {
        return Err("convert needs <from> and <to>".into());
    };
    let rgb = parse_rgb(rest)?;

    // Profiles from files are registered under the name given on the command line.
    let mut colour = ColourManager::new();
    for name in [from, to] {
        if name != "srgb" && name != "display-p3" {
            colour.register(name, load(name)?);
        }
    }
    let transform = colour
        .transform(from, to, intent)
        .map_err(|e| e.to_string())?;
    let out = transform.transform_rgb(rgb).map_err(|e| e.to_string())?;
    println!("{out:.5?}");
    Ok(())
}

/// Checks a profile for problems, and reports each finding as `warn`, `info` or `ok`.
fn doctor(path: &str) -> Result<(), String> {
    let profile = load(path)?;
    let mut warnings = 0_u32;
    let mut warn = |message: String| {
        warnings += 1;
        println!("warn: {message}");
    };

    if let ProfileModel::Lut(tables) = &profile.model {
        for intent in [
            RenderingIntent::Perceptual,
            RenderingIntent::RelativeColorimetric,
            RenderingIntent::Saturation,
            RenderingIntent::AbsoluteColorimetric,
        ] {
            if tables.has_d2b(intent) || tables.has_b2d(intent) {
                warn(format!(
                    "{intent:?} uses multiProcessElements (D2B/B2D) tables, which are not read"
                ));
            }
        }
        if tables.a2b(RenderingIntent::Perceptual).is_none()
            && tables.a2b(RenderingIntent::RelativeColorimetric).is_none()
        {
            warn("no A2B table for perceptual or relative intents".into());
        }
    }
    if profile.description.is_none() {
        warn("no description tag".into());
    }
    if profile.white_point != tpt_colour::space::D50 {
        println!(
            "info: media white point {:.4?} differs from D50, so absolute colorimetric scales",
            profile.white_point
        );
    }

    // Round trip through sRGB, for the perceptual intent, when the profile supports it.
    let srgb = IccProfile::srgb();
    let forward = ColourTransform::new(&srgb, &profile, RenderingIntent::Perceptual);
    let backward = ColourTransform::new(&profile, &srgb, RenderingIntent::Perceptual);
    if let (Ok(forward), Ok(backward)) = (forward, backward) {
        let mut worst = 0.0_f32;
        for step in 1..=8 {
            let v = step as f32 / 8.0;
            let rgb = [v, 1.0 - v, 0.5 * v];
            let mut device = [0.0_f32; 3];
            let mut back = [0.0_f32; 3];
            if forward.transform(&rgb, &mut device).is_ok()
                && backward.transform(&device, &mut back).is_ok()
            {
                for c in 0..3 {
                    worst = worst.max((back[c] - rgb[c]).abs() * 255.0);
                }
            }
        }
        if worst > 3.0 {
            warn(format!(
                "sRGB round trip differs by up to {worst:.1} codes. Some profiles are not invertible, and LittleCMS shows the same gap"
            ));
        } else {
            println!("ok: sRGB round trip differs by up to {worst:.1} codes");
        }
    } else {
        println!("info: no perceptual transform to or from sRGB");
    }

    if warnings == 0 {
        println!("ok: no problems found");
    }
    Ok(())
}

/// Writes a `.cube` 3-D LUT for `<from>` to `<to>` on standard output.
fn cube(args: &[String]) -> Result<(), String> {
    let mut intent = RenderingIntent::Perceptual;
    let mut positional = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--intent" {
            intent = parse_intent(iter.next().ok_or("--intent needs a value")?)?;
        } else {
            positional.push(arg.as_str());
        }
    }
    let [from, to, size] = positional.as_slice() else {
        return Err("cube needs <from>, <to> and <size>".into());
    };
    let size: usize = size
        .parse()
        .map_err(|_| format!("{size:?} is not a size"))?;
    let transform =
        ColourTransform::new(&load(from)?, &load(to)?, intent).map_err(|e| e.to_string())?;
    let text = transform
        .to_cube(size, &format!("{from} to {to}"))
        .map_err(|e| e.to_string())?;
    print!("{text}");
    Ok(())
}

fn gamut_check(args: &[String]) -> Result<(), String> {
    let [space, rest @ ..] = args else {
        return Err("gamut-check needs a colour space".into());
    };
    let rgb = parse_rgb(rest)?;
    let xyz = match space.as_str() {
        "srgb" => SRgb.to_xyz(rgb),
        "display-p3" => DisplayP3.to_xyz(rgb),
        "rec2020" => Rec2020.to_xyz(rgb),
        other => return Err(format!("unknown colour space {other:?}")),
    };
    let srgb = Gamut::srgb();
    let inside = srgb.contains(xyz);
    println!("inside sRGB: {}", if inside { "yes" } else { "no" });
    if !inside {
        let mapped = SRgb.from_xyz(map_to_gamut(&srgb, xyz));
        println!("sRGB after perceptual gamut mapping: {mapped:.5?}");
    }
    Ok(())
}
