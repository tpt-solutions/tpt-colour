//! ICC profile identifiers.
//!
//! The header stores a 16-byte profile ID, which the ICC specification defines as the MD5 digest
//! of the profile with three header fields set to zero: the profile flags, the rendering intent,
//! and the ID itself. A profile with an all-zero ID carries no ID.

use crate::md5::Md5;

/// Result of checking a profile's stored ID against its contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProfileIdCheck {
    /// The profile stores no ID (all zeros), or it is too short to have a header.
    Absent,
    /// The stored ID matches the MD5 of the profile.
    Match,
    /// The stored ID does not match. The profile was changed after its ID was written, or the
    /// ID is wrong.
    Mismatch,
}

/// Computes the profile ID of `data` as the ICC specification defines it.
///
/// Returns `None` if `data` is shorter than the 128-byte header.
///
/// # Example
///
/// ```
/// use tpt_colour_icc::computed_profile_id;
/// assert!(computed_profile_id(&[0; 64]).is_none());
/// ```
#[must_use]
pub fn computed_profile_id(data: &[u8]) -> Option<[u8; 16]> {
    if data.len() < 128 {
        return None;
    }
    let mut md5 = Md5::new();
    md5.update(&data[..44]);
    md5.update(&[0; 4]); // profile flags
    md5.update(&data[48..64]);
    md5.update(&[0; 4]); // rendering intent
    md5.update(&data[68..84]);
    md5.update(&[0; 16]); // profile ID
    md5.update(&data[100..]);
    Some(md5.finalize())
}

/// Checks the stored profile ID of `data` against its contents.
///
/// # Example
///
/// ```
/// use tpt_colour_icc::{ProfileIdCheck, check_profile_id};
/// assert_eq!(check_profile_id(&[0; 200]), ProfileIdCheck::Absent);
/// ```
#[must_use]
pub fn check_profile_id(data: &[u8]) -> ProfileIdCheck {
    let Some(computed) = computed_profile_id(data) else {
        return ProfileIdCheck::Absent;
    };
    let stored = &data[84..100];
    if stored.iter().all(|&b| b == 0) {
        ProfileIdCheck::Absent
    } else if stored == computed {
        ProfileIdCheck::Match
    } else {
        ProfileIdCheck::Mismatch
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{ProfileIdCheck, check_profile_id, computed_profile_id};

    #[test]
    fn short_or_unidentified_data_has_no_id() {
        assert_eq!(check_profile_id(&[]), ProfileIdCheck::Absent);
        assert!(computed_profile_id(&[0; 127]).is_none());
        assert_eq!(check_profile_id(&[0; 128]), ProfileIdCheck::Absent);
    }

    #[test]
    fn a_changed_byte_outside_the_masked_fields_mismatches() {
        let mut data = vec![7_u8; 256];
        let id = computed_profile_id(&data).unwrap();
        data[84..100].copy_from_slice(&id);
        assert_eq!(check_profile_id(&data), ProfileIdCheck::Match);
        data[200] ^= 1;
        assert_eq!(check_profile_id(&data), ProfileIdCheck::Mismatch);
    }

    #[test]
    fn masked_fields_do_not_change_the_id() {
        let mut data = vec![9_u8; 256];
        let id = computed_profile_id(&data).unwrap();
        data[44..48].copy_from_slice(&[1, 2, 3, 4]);
        data[64..68].copy_from_slice(&[5, 6, 7, 8]);
        data[84..100].copy_from_slice(&[0xaa; 16]);
        assert_eq!(computed_profile_id(&data).unwrap(), id);
    }
}
