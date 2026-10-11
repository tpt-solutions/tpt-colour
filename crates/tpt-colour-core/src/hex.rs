//! Parsing of CSS-style hex colours: `#rgb`, `#rgba`, `#rrggbb` and `#rrggbbaa`.

/// Parses a hex colour into 8-bit `[r, g, b, a]`. The leading `#` is optional, and alpha is 255
/// when omitted. Returns `None` for any other length or a non-hex digit.
///
/// # Example
///
/// ```
/// use tpt_colour_core::hex::parse_hex;
/// assert_eq!(parse_hex("#ff8000"), Some([255, 128, 0, 255]));
/// ```
#[must_use]
pub fn parse_hex(text: &str) -> Option<[u8; 4]> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() > 8 || !digits.is_ascii() {
        return None;
    }
    let mut n = [0_u8; 8];
    for (slot, c) in n.iter_mut().zip(digits.bytes()) {
        *slot = char::from(c).to_digit(16)? as u8;
    }
    let expand = |i: usize| n[i] * 17;
    let pair = |i: usize| n[i] * 16 + n[i + 1];
    match digits.len() {
        3 => Some([expand(0), expand(1), expand(2), 255]),
        4 => Some([expand(0), expand(1), expand(2), expand(3)]),
        6 => Some([pair(0), pair(2), pair(4), 255]),
        8 => Some([pair(0), pair(2), pair(4), pair(6)]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn parses_all_four_forms() {
        assert_eq!(parse_hex("#f00"), Some([255, 0, 0, 255]));
        assert_eq!(parse_hex("#f008"), Some([255, 0, 0, 136]));
        assert_eq!(parse_hex("102030"), Some([16, 32, 48, 255]));
        assert_eq!(parse_hex("#10203040"), Some([16, 32, 48, 64]));
    }

    #[test]
    fn rejects_bad_input() {
        for bad in [
            "",
            "#",
            "#12",
            "#12345",
            "#1234567",
            "#gggggg",
            "#123456789",
            "#12 456",
        ] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
    }
}
