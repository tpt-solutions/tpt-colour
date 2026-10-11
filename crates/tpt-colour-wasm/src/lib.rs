//! WebAssembly bindings for tpt-colour.
//!
//! Colours cross the boundary as flat `Float32Array`s of gamma-encoded values in `[0, 1]`, and
//! 8-bit colours as `Uint8Array`s. Failures are thrown as JavaScript `Error`s with a message.
//! The functions are thin wrappers over the facade crate, so their behaviour is the same as the
//! Rust API.

use tpt_colour::icc::{IccProfile, RenderingIntent};
use tpt_colour::palette;
use tpt_colour::vision::{self, Deficiency};
use tpt_colour::{ColourManager, contrast, parse};
use wasm_bindgen::prelude::*;

fn rgb(values: &[f32]) -> Result<[f32; 3], JsError> {
    match values {
        [r, g, b] => Ok([*r, *g, *b]),
        _ => Err(JsError::new("expected three channels: [r, g, b]")),
    }
}

fn intent(name: &str) -> Result<RenderingIntent, JsError> {
    name.parse()
        .map_err(|_| JsError::new("intent must be perceptual, relative, saturation or absolute"))
}

/// Parses a CSS colour (`#rgb`, `rgb()`, `oklch()`, a named colour, `transparent`) into 8-bit
/// `[r, g, b, a]`.
#[wasm_bindgen(js_name = parseColour)]
/// # Errors
/// Throws if the text is not a recognised CSS colour.
pub fn parse_colour(text: &str) -> Result<Vec<u8>, JsError> {
    parse::parse_colour(text)
        .map(|c| c.to_vec())
        .ok_or_else(|| JsError::new(&format!("not a recognised colour: {text}")))
}

/// WCAG 2 contrast ratio between two `[r, g, b]` colours, from 1 to 21.
#[wasm_bindgen(js_name = wcagContrast)]
/// # Errors
/// Throws if either colour does not have three channels.
pub fn wcag_contrast(a: &[f32], b: &[f32]) -> Result<f32, JsError> {
    Ok(contrast::wcag_contrast(rgb(a)?, rgb(b)?))
}

/// APCA lightness contrast `Lc` of `text` on `background`. Positive for dark text on light.
#[wasm_bindgen(js_name = apcaContrast)]
/// # Errors
/// Throws if either colour does not have three channels.
pub fn apca_contrast(text: &[f32], background: &[f32]) -> Result<f32, JsError> {
    Ok(contrast::apca_contrast(rgb(text)?, rgb(background)?))
}

/// Simulates a colour-vision deficiency: `"protanopia"`, `"deuteranopia"` or `"tritanopia"`.
#[wasm_bindgen(js_name = simulateDeficiency)]
/// # Errors
/// Throws for an unknown deficiency name, or a colour without three channels.
pub fn simulate_deficiency(rgb_in: &[f32], deficiency: &str) -> Result<Vec<f32>, JsError> {
    let kind = match deficiency {
        "protanopia" => Deficiency::Protanopia,
        "deuteranopia" => Deficiency::Deuteranopia,
        "tritanopia" => Deficiency::Tritanopia,
        _ => {
            return Err(JsError::new(
                "deficiency must be protanopia, deuteranopia or tritanopia",
            ));
        }
    };
    Ok(vision::simulate(rgb(rgb_in)?, kind).to_vec())
}

/// A colour harmony of `[r, g, b]`, flattened: `"complement"` gives 3 values, and the others give
/// two colours (6 values). Kinds: `complement`, `analogous`, `triadic`, `split-complementary`.
#[wasm_bindgen]
/// # Errors
/// Throws for an unknown harmony kind, or a colour without three channels.
pub fn harmony(rgb_in: &[f32], kind: &str) -> Result<Vec<f32>, JsError> {
    let base = rgb(rgb_in)?;
    let flat = match kind {
        "complement" => palette::complement(base).to_vec(),
        "analogous" => palette::analogous(base).concat(),
        "triadic" => palette::triadic(base).concat(),
        "split-complementary" => palette::split_complementary(base).concat(),
        _ => return Err(JsError::new("unknown harmony kind")),
    };
    Ok(flat)
}

/// A colour manager with the built-in `"srgb"` and `"display-p3"` profiles, plus any ICC profiles
/// registered from bytes. Transforms are cached inside it.
#[wasm_bindgen]
pub struct Manager {
    inner: ColourManager,
}

#[wasm_bindgen]
impl Manager {
    /// Creates a manager with the built-in profiles.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: ColourManager::new(),
        }
    }

    /// Registers an ICC profile under `name`, replacing any profile with that name.
    #[wasm_bindgen(js_name = registerIcc)]
    /// # Errors
    /// Throws if the bytes are not a supported ICC profile.
    pub fn register_icc(&mut self, name: &str, bytes: &[u8]) -> Result<(), JsError> {
        self.inner
            .register_icc(name, bytes)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Converts one `[r, g, b]` colour from profile `src` to profile `dst`.
    /// # Errors
    /// Throws for unknown profiles or intents, a colour without three channels, or a transform
    /// that cannot be built.
    #[wasm_bindgen(js_name = transformRgb)]
    pub fn transform_rgb(
        &mut self,
        src: &str,
        dst: &str,
        intent_name: &str,
        rgb_in: &[f32],
    ) -> Result<Vec<f32>, JsError> {
        let transform = self
            .inner
            .transform(src, dst, intent(intent_name)?)
            .map_err(|e| JsError::new(&e.to_string()))?;
        let out = transform
            .transform_rgb(rgb(rgb_in)?)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(out.to_vec())
    }

    /// Number of cached transforms, for diagnostics.
    #[wasm_bindgen(js_name = cachedTransforms)]
    #[must_use]
    pub fn cached_transforms(&self) -> usize {
        self.inner.cached_transforms()
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

/// The description stored in an ICC profile, or an empty string if it has none.
/// # Errors
/// Throws if the bytes are not a readable ICC profile.
#[wasm_bindgen(js_name = profileDescription)]
pub fn profile_description(bytes: &[u8]) -> Result<String, JsError> {
    let profile = IccProfile::parse(bytes).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(profile.description.unwrap_or_default())
}
