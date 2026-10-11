# ICC profiles: writing and identifiers

## Writing a profile

`IccProfile::to_bytes` writes a matrix/TRC RGB profile or a grey profile as an ICC version 2.1
file, with an XYZ PCS. A profile read with `IccProfile::parse` can be written back out, and reads
the same model again.

```rust
# extern crate tpt_colour;
# use tpt_colour::icc::{IccProfile, RenderingIntent};
let bytes = IccProfile::display_p3().to_bytes()?;
let back = IccProfile::parse(&bytes)?;
assert_eq!(back.intent, RenderingIntent::Perceptual);
# Ok::<(), tpt_colour::icc::IccError>(())
```

The colorants and white point are stored as `s15Fixed16`, so they are quantised to about 1.5e-5.
Table curves are stored as 16-bit codes. The transform built from a written profile agrees with
the original to within one 8-bit code. Gamma curves are written as `para` function type 0, which
reads back as `Trc::Gamma`, so their exponent is kept.

LUT profiles, and profiles with a Lab PCS, return `IccError::Unsupported`.

## Independent check

The writer's output has been checked with LittleCMS 2.19, through Pillow. Three profiles were
written: the built-in sRGB, a Display P3 with gamma 2.2, and a Display P3 with a 256-point sampled
sRGB curve. LittleCMS converted nine 8-bit colours from each written file, and matched the
results from tpt-colour exactly, with no code of difference. The check is in
`tools/icc/check_written_profiles.py`, and the profiles are made by the `write_profile` example.
The checker also fails on a corrupted expectation, so a pass is meaningful.

## Profile IDs

An ICC v4 profile can store a 16-byte ID, which is the MD5 digest of the profile with three header
fields set to zero: the profile flags, the rendering intent, and the ID itself.
`check_profile_id` tests a profile against its stored ID.

```rust
# extern crate tpt_colour;
# use tpt_colour::icc::{ProfileIdCheck, check_profile_id};
# let data = [0_u8; 200];
match check_profile_id(&data) {
    ProfileIdCheck::Match => {}
    ProfileIdCheck::Mismatch => { /* the profile changed after its ID was written */ }
    ProfileIdCheck::Absent => { /* no ID stored: the usual case for v2 profiles */ }
    _ => {}
}
```

The rule was checked against 87 profiles in iccDEV's test corpus that store an ID, and all 87
match. The 11 that do not are fuzzing and crash-regression files, which are malformed on purpose.
The official sRGB fixtures also match, and the MD5 implementation is checked against the RFC 1321
test suite.
