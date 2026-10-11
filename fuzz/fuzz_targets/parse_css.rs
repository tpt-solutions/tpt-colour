#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_colour_core::parse::parse_colour;

fuzz_target!(|data: &[u8]| {
    // Colour text is untrusted. Any byte sequence must parse or be rejected, without panicking.
    // Invalid UTF-8 is skipped, because the API takes `&str`.
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = parse_colour(text);
    }
});
