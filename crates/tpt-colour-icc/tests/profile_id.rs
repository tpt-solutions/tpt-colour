//! The stored profile IDs of the official sRGB fixtures match their contents, which checks the
//! ICC rule for computing IDs against real profiles written by another encoder.

use std::fs;
use std::path::PathBuf;

use tpt_colour_icc::{ProfileIdCheck, check_profile_id, computed_profile_id};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tpt-colour-oracle/tests/profiles")
        .join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn official_srgb_fixtures_carry_matching_ids() {
    for name in ["sRGB2014.icc", "sRGB_v4_ICC_preference.icc"] {
        let data = fixture(name);
        assert_eq!(check_profile_id(&data), ProfileIdCheck::Match, "{name}");
        let id = computed_profile_id(&data).unwrap();
        assert_eq!(&data[84..100], &id, "{name}");
    }
}

#[test]
fn editing_the_profile_breaks_the_id() {
    let mut data = fixture("sRGB_v4_ICC_preference.icc");
    // Byte 1000 lies in tag data, not in a masked header field.
    data[1000] ^= 0x01;
    assert_eq!(check_profile_id(&data), ProfileIdCheck::Mismatch);
}

#[test]
fn changing_masked_header_fields_keeps_the_id_valid() {
    let mut data = fixture("sRGB_v4_ICC_preference.icc");
    data[44..48].copy_from_slice(&[0xff; 4]); // profile flags
    data[64..68].copy_from_slice(&[0xff; 4]); // rendering intent
    assert_eq!(check_profile_id(&data), ProfileIdCheck::Match);
}
