//! 16-bit pixels and strided 8-bit or 16-bit images: padding and alpha stay untouched.

use tpt_colour_icc::{ColourTransform, IccError, IccProfile, RenderingIntent};

fn identity() -> ColourTransform {
    ColourTransform::new(
        &IccProfile::srgb(),
        &IccProfile::srgb(),
        RenderingIntent::Perceptual,
    )
    .unwrap()
}

#[test]
fn identity_transform_keeps_16_bit_values_and_alpha() {
    let t = identity();
    let mut pixels = vec![0_u16, 32_768, 65_535, 12_345, 1, 60_000, 4_000, 999];
    let before = pixels.clone();
    t.transform_u16(&mut pixels, 4).unwrap();
    for (i, (a, b)) in pixels.iter().zip(&before).enumerate() {
        if i % 4 == 3 {
            assert_eq!(a, b, "alpha must not change");
        } else {
            assert!(a.abs_diff(*b) <= 1, "{i}: {a} vs {b}");
        }
    }
}

#[test]
fn strided_u8_image_leaves_padding_untouched() {
    let t = identity();
    // 2×2 RGBA, one row of pixels is 8 bytes, stride 10 leaves two padding bytes per row.
    let mut buf = vec![0_u8; 2 * 10];
    let rows = [
        [10_u8, 20, 30, 40, 50, 60, 70, 80],
        [1, 2, 3, 4, 5, 6, 7, 8],
    ];
    for (r, row) in rows.iter().enumerate() {
        buf[r * 10..r * 10 + 8].copy_from_slice(row);
        buf[r * 10 + 8..r * 10 + 10].copy_from_slice(&[0xAA, 0xBB]);
    }
    t.transform_u8_image(&mut buf, 2, 2, 10, 4).unwrap();
    for (r, row) in rows.iter().enumerate() {
        assert_eq!(
            &buf[r * 10 + 8..r * 10 + 10],
            &[0xAA, 0xBB],
            "padding row {r}"
        );
        for px in 0..2 {
            let i = r * 10 + px * 4;
            assert_eq!(buf[i + 3], row[px * 4 + 3], "alpha row {r} px {px}");
            for c in 0..3 {
                assert!(buf[i + c].abs_diff(row[px * 4 + c]) <= 1);
            }
        }
    }
}

#[test]
fn strided_image_matches_contiguous_transform() {
    let t = identity();
    let mut contiguous: Vec<u16> = (0..24).map(|v| v * 2_700).collect();
    let mut strided = contiguous.clone();
    t.transform_u16(&mut contiguous, 4).unwrap();
    t.transform_u16_image(&mut strided, 3, 2, 12, 4).unwrap();
    assert_eq!(contiguous, strided);
}

#[test]
fn zero_sized_images_are_no_ops() {
    let t = identity();
    let mut empty: Vec<u8> = Vec::new();
    t.transform_u8_image(&mut empty, 0, 0, 0, 3).unwrap();
    t.transform_u8_image(&mut empty, 5, 0, 15, 3).unwrap();
}

#[test]
fn bad_layouts_are_reported() {
    let t = identity();
    let mut buf = vec![0_u8; 64];
    // Stride shorter than one row of 3 px × 3 channels.
    assert_eq!(
        t.transform_u8_image(&mut buf, 3, 2, 8, 3),
        Err(IccError::BufferLayout)
    );
    // Last row needs (height - 1) * stride + row_len = 2 * 12 + 9 = 33 bytes; give 20.
    let mut short = vec![0_u8; 20];
    assert_eq!(
        t.transform_u8_image(&mut short, 3, 3, 12, 3),
        Err(IccError::BufferLayout)
    );
    let mut words = vec![0_u16; 5];
    assert_eq!(
        t.transform_u16_image(&mut words, 2, 2, 6, 3),
        Err(IccError::BufferLayout)
    );
    assert_eq!(
        t.transform_u8_image(&mut buf, 2, 2, 8, 5),
        Err(IccError::ChannelCount)
    );
}
