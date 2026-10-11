//! C ABI for tpt-colour.
//!
//! Every function that can fail returns `0` on success and a negative [`Status`] otherwise.
//! Colours are arrays of `float` in `[0, 1]` (three values, or four where noted), and 8-bit
//! colours are arrays of `uint8_t`. Strings are UTF-8 given with an explicit length, so they need
//! no terminating NUL.
//!
//! The caller owns every pointer, and each function documents what it reads and writes. Only
//! `tpt_manager_new` allocates: release its result with `tpt_manager_free`.
#![allow(unsafe_code)]

use core::ffi::c_char;
use core::slice;

use tpt::icc::RenderingIntent;
use tpt::vision::{self, Deficiency};
use tpt::{ColourManager, contrast, parse};

/// Status codes returned by the functions in this interface.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The call succeeded.
    Ok = 0,
    /// A required pointer was null.
    NullPointer = -1,
    /// A string was not UTF-8, or named an unknown colour, intent or option.
    InvalidInput = -2,
    /// The colour could not be converted, for example because a profile is missing or invalid.
    ConversionFailed = -3,
}

/// Opaque colour manager: the built-in profiles plus any registered ICC profiles, with a
/// transform cache. Create it with `tpt_manager_new` and free it with `tpt_manager_free`.
pub struct Manager {
    inner: ColourManager,
}

/// Reads `len` bytes at `ptr` as UTF-8.
///
/// # Safety
///
/// If `len` is non-zero, `ptr` must be valid for reads of `len` bytes.
unsafe fn read_text<'a>(ptr: *const c_char, len: usize) -> Result<&'a str, Status> {
    if ptr.is_null() {
        return Err(Status::NullPointer);
    }
    // SAFETY: the caller guarantees `ptr` is valid for `len` bytes.
    let bytes = unsafe { slice::from_raw_parts(ptr.cast::<u8>(), len) };
    core::str::from_utf8(bytes).map_err(|_| Status::InvalidInput)
}

/// Reads three floats at `ptr`.
///
/// # Safety
///
/// `ptr` must be valid for reads of three `float`s.
unsafe fn read3(ptr: *const f32) -> Result<[f32; 3], Status> {
    if ptr.is_null() {
        return Err(Status::NullPointer);
    }
    // SAFETY: the caller guarantees three readable floats at `ptr`.
    let v = unsafe { slice::from_raw_parts(ptr, 3) };
    Ok([v[0], v[1], v[2]])
}

/// Writes `values` to `out`, which must have room for `values.len()` floats.
///
/// # Safety
///
/// `out` must be valid for writes of `values.len()` floats.
unsafe fn write_all(out: *mut f32, values: &[f32]) -> Result<(), Status> {
    if out.is_null() {
        return Err(Status::NullPointer);
    }
    // SAFETY: the caller guarantees room for `values.len()` floats at `out`.
    let dst = unsafe { slice::from_raw_parts_mut(out, values.len()) };
    dst.copy_from_slice(values);
    Ok(())
}

fn intent_from(code: u32) -> Result<RenderingIntent, Status> {
    match code {
        0 => Ok(RenderingIntent::Perceptual),
        1 => Ok(RenderingIntent::RelativeColorimetric),
        2 => Ok(RenderingIntent::Saturation),
        3 => Ok(RenderingIntent::AbsoluteColorimetric),
        _ => Err(Status::InvalidInput),
    }
}

fn status(result: Result<(), Status>) -> i32 {
    match result {
        Ok(()) => Status::Ok as i32,
        Err(code) => code as i32,
    }
}

/// The library version as a static NUL-terminated string. Do not free it.
#[unsafe(no_mangle)]
pub extern "C" fn tpt_version() -> *const c_char {
    c"0.1.0".as_ptr()
}

/// Parses a CSS colour: hex, `rgb()`, `oklch()`, a named colour, or `transparent`.
///
/// Reads `len` bytes of UTF-8 at `text`, and writes four `uint8_t` (`r, g, b, a`) to `out`.
///
/// # Safety
///
/// `text` must be valid for `len` bytes, and `out` for four writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tpt_parse_colour(text: *const c_char, len: usize, out: *mut u8) -> i32 {
    let result = (|| {
        // SAFETY: forwarded to this function's contract.
        let s = unsafe { read_text(text, len) }?;
        let colour = parse::parse_colour(s).ok_or(Status::InvalidInput)?;
        if out.is_null() {
            return Err(Status::NullPointer);
        }
        // SAFETY: the caller guarantees four writable bytes at `out`.
        unsafe { slice::from_raw_parts_mut(out, 4) }.copy_from_slice(&colour);
        Ok(())
    })();
    status(result)
}

/// WCAG 2 contrast ratio of two `float[3]` colours, written to `*out`.
///
/// # Safety
///
/// `a` and `b` must each be valid for three reads, and `out` for one write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tpt_wcag_contrast(a: *const f32, b: *const f32, out: *mut f32) -> i32 {
    let result = (|| {
        // SAFETY: forwarded to this function's contract.
        let ratio = contrast::wcag_contrast(unsafe { read3(a) }?, unsafe { read3(b) }?);
        unsafe { write_all(out, &[ratio]) }
    })();
    status(result)
}

/// APCA lightness contrast `Lc` of text on a background, written to `*out`. Positive for dark
/// text on light.
///
/// # Safety
///
/// `text` and `background` must each be valid for three reads, and `out` for one write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tpt_apca_contrast(
    text: *const f32,
    background: *const f32,
    out: *mut f32,
) -> i32 {
    let result = (|| {
        // SAFETY: forwarded to this function's contract.
        let lc = contrast::apca_contrast(unsafe { read3(text) }?, unsafe { read3(background) }?);
        unsafe { write_all(out, &[lc]) }
    })();
    status(result)
}

/// Simulates a dichromacy on a `float[3]` colour. `kind` is 0 for protanopia, 1 for
/// deuteranopia and 2 for tritanopia. Writes `float[3]` to `out`.
///
/// # Safety
///
/// `rgb` must be valid for three reads, and `out` for three writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tpt_simulate_deficiency(rgb: *const f32, kind: u32, out: *mut f32) -> i32 {
    let kind = match kind {
        0 => Deficiency::Protanopia,
        1 => Deficiency::Deuteranopia,
        2 => Deficiency::Tritanopia,
        _ => return Status::InvalidInput as i32,
    };
    let result = (|| {
        // SAFETY: forwarded to this function's contract.
        let seen = vision::simulate(unsafe { read3(rgb) }?, kind);
        unsafe { write_all(out, &seen) }
    })();
    status(result)
}

/// Creates a colour manager with the built-in `srgb` and `display-p3` profiles. Free it with
/// `tpt_manager_free`. Returns null only if allocation fails.
#[unsafe(no_mangle)]
pub extern "C" fn tpt_manager_new() -> *mut Manager {
    Box::into_raw(Box::new(Manager {
        inner: ColourManager::new(),
    }))
}

/// Frees a manager from `tpt_manager_new`. A null pointer is ignored.
///
/// # Safety
///
/// `manager` must be null, or a pointer from `tpt_manager_new` that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tpt_manager_free(manager: *mut Manager) {
    if !manager.is_null() {
        // SAFETY: the pointer came from `Box::into_raw` in `tpt_manager_new`, and is freed once.
        drop(unsafe { Box::from_raw(manager) });
    }
}

/// Converts one `float[3]` colour from profile `src` to profile `dst`, writing `float[3]` to
/// `out`. Intents: 0 perceptual, 1 relative, 2 saturation, 3 absolute.
///
/// # Safety
///
/// `manager` must be a live manager from `tpt_manager_new`, used by one thread at a time. `src`
/// and `dst` must each be valid for their length in bytes, `rgb` for three reads, and `out` for
/// three writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tpt_manager_transform_rgb(
    manager: *mut Manager,
    src: *const c_char,
    src_len: usize,
    dst: *const c_char,
    dst_len: usize,
    intent: u32,
    rgb: *const f32,
    out: *mut f32,
) -> i32 {
    if manager.is_null() {
        return Status::NullPointer as i32;
    }
    let result = (|| {
        // SAFETY: forwarded to this function's contract.
        let src = unsafe { read_text(src, src_len) }?;
        let dst = unsafe { read_text(dst, dst_len) }?;
        let intent = intent_from(intent)?;
        let input = unsafe { read3(rgb) }?;
        // SAFETY: `manager` is live and not aliased, per the contract above.
        let manager = unsafe { &mut *manager };
        let transform = manager
            .inner
            .transform(src, dst, intent)
            .map_err(|_| Status::ConversionFailed)?;
        let converted = transform
            .transform_rgb(input)
            .map_err(|_| Status::ConversionFailed)?;
        unsafe { write_all(out, &converted) }
    })();
    status(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_through_the_c_interface() {
        let text = "rebeccapurple";
        let mut out = [0_u8; 4];
        let code = unsafe { tpt_parse_colour(text.as_ptr().cast(), text.len(), out.as_mut_ptr()) };
        assert_eq!(code, 0);
        assert_eq!(out, [102, 51, 153, 255]);
    }

    #[test]
    fn null_and_bad_input_are_reported() {
        let mut out = [0_u8; 4];
        let code = unsafe { tpt_parse_colour(core::ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(code, Status::NullPointer as i32);
        let bad = "nope";
        let code = unsafe { tpt_parse_colour(bad.as_ptr().cast(), bad.len(), out.as_mut_ptr()) };
        assert_eq!(code, Status::InvalidInput as i32);
    }

    #[test]
    fn manager_round_trips_and_frees() {
        let m = tpt_manager_new();
        let src = "srgb";
        let dst = "display-p3";
        let input = [0.8_f32, 0.2, 0.1];
        let mut p3 = [0.0_f32; 3];
        let mut back = [0.0_f32; 3];
        unsafe {
            let forward = tpt_manager_transform_rgb(
                m,
                src.as_ptr().cast(),
                src.len(),
                dst.as_ptr().cast(),
                dst.len(),
                0,
                input.as_ptr(),
                p3.as_mut_ptr(),
            );
            assert_eq!(forward, 0);
            let reverse = tpt_manager_transform_rgb(
                m,
                dst.as_ptr().cast(),
                dst.len(),
                src.as_ptr().cast(),
                src.len(),
                0,
                p3.as_ptr(),
                back.as_mut_ptr(),
            );
            assert_eq!(reverse, 0);
            tpt_manager_free(m);
        }
        for (a, b) in back.iter().zip(input) {
            assert!((a - b).abs() < 2e-3);
        }
    }

    #[test]
    fn version_is_a_static_c_string() {
        let v = unsafe { core::ffi::CStr::from_ptr(tpt_version()) };
        assert_eq!(v.to_str().unwrap(), "0.1.0");
    }
}
