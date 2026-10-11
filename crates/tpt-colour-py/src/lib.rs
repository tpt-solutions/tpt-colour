//! Python bindings for tpt-colour, built with PyO3.
//!
//! Colours are sequences of floats in `[0, 1]`, and 8-bit colours are sequences of ints. Errors
//! are raised as `ValueError`. Intents are the strings `"perceptual"`, `"relative"`,
//! `"saturation"` and `"absolute"`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use tpt::icc::RenderingIntent;
use tpt::vision::{self, Deficiency};
use tpt::{ColourManager, contrast, palette, parse};

fn triple(values: Vec<f32>) -> PyResult<[f32; 3]> {
    <[f32; 3]>::try_from(values.as_slice())
        .map_err(|_| PyValueError::new_err("expected three channels: [r, g, b]"))
}

fn intent(name: &str) -> PyResult<RenderingIntent> {
    name.parse().map_err(|_| {
        PyValueError::new_err("intent must be perceptual, relative, saturation or absolute")
    })
}

/// Parses a CSS colour into 8-bit `[r, g, b, a]`.
#[pyfunction]
fn parse_colour(text: &str) -> PyResult<Vec<u32>> {
    // `Vec<u8>` would become Python `bytes`, so the channels are widened to a list of ints.
    parse::parse_colour(text)
        .map(|c| c.iter().map(|&v| u32::from(v)).collect())
        .ok_or_else(|| PyValueError::new_err(format!("not a recognised colour: {text}")))
}

/// WCAG 2 contrast ratio between two `[r, g, b]` colours, from 1 to 21.
#[pyfunction]
fn wcag_contrast(a: Vec<f32>, b: Vec<f32>) -> PyResult<f32> {
    Ok(contrast::wcag_contrast(triple(a)?, triple(b)?))
}

/// APCA lightness contrast `Lc` of text on a background. Positive for dark text on light.
#[pyfunction]
fn apca_contrast(text: Vec<f32>, background: Vec<f32>) -> PyResult<f32> {
    Ok(contrast::apca_contrast(triple(text)?, triple(background)?))
}

/// Simulates `"protanopia"`, `"deuteranopia"` or `"tritanopia"` on an `[r, g, b]` colour.
#[pyfunction]
fn simulate_deficiency(rgb_in: Vec<f32>, deficiency: &str) -> PyResult<Vec<f32>> {
    let kind = match deficiency {
        "protanopia" => Deficiency::Protanopia,
        "deuteranopia" => Deficiency::Deuteranopia,
        "tritanopia" => Deficiency::Tritanopia,
        _ => return Err(PyValueError::new_err("unknown deficiency")),
    };
    Ok(vision::simulate(triple(rgb_in)?, kind).to_vec())
}

/// Colour harmonies of an `[r, g, b]` colour: a list of colours. `"complement"` gives one colour,
/// the others give two. Kinds: `complement`, `analogous`, `triadic`, `split-complementary`.
#[pyfunction]
fn harmony(rgb_in: Vec<f32>, kind: &str) -> PyResult<Vec<Vec<f32>>> {
    let base = triple(rgb_in)?;
    Ok(match kind {
        "complement" => vec![palette::complement(base).to_vec()],
        "analogous" => palette::analogous(base).iter().map(|c| c.to_vec()).collect(),
        "triadic" => palette::triadic(base).iter().map(|c| c.to_vec()).collect(),
        "split-complementary" => palette::split_complementary(base)
            .iter()
            .map(|c| c.to_vec())
            .collect(),
        _ => return Err(PyValueError::new_err("unknown harmony kind")),
    })
}

/// A colour manager with the built-in `"srgb"` and `"display-p3"` profiles. Transforms are cached.
#[pyclass]
struct Manager {
    inner: ColourManager,
}

#[pymethods]
impl Manager {
    #[new]
    fn new() -> Self {
        Self {
            inner: ColourManager::new(),
        }
    }

    /// Registers an ICC profile from bytes under `name`, replacing any profile with that name.
    fn register_icc(&mut self, name: &str, data: &[u8]) -> PyResult<()> {
        self.inner
            .register_icc(name, data)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Converts one `[r, g, b]` colour from profile `src` to profile `dst`.
    fn transform_rgb(
        &mut self,
        src: &str,
        dst: &str,
        intent_name: &str,
        rgb_in: Vec<f32>,
    ) -> PyResult<Vec<f32>> {
        let transform = self
            .inner
            .transform(src, dst, intent(intent_name)?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let out = transform
            .transform_rgb(triple(rgb_in)?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(out.to_vec())
    }

    /// Number of cached transforms.
    #[getter]
    fn cached_transforms(&self) -> usize {
        self.inner.cached_transforms()
    }
}

#[pymodule]
fn tpt_colour(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_colour, m)?)?;
    m.add_function(wrap_pyfunction!(wcag_contrast, m)?)?;
    m.add_function(wrap_pyfunction!(apca_contrast, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_deficiency, m)?)?;
    m.add_function(wrap_pyfunction!(harmony, m)?)?;
    m.add_class::<Manager>()?;
    Ok(())
}
