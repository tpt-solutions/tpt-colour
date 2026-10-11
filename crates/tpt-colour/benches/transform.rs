#![allow(missing_docs)] // criterion's macros generate undocumented entry points.

//! Criterion benchmarks for the hot paths: bulk 8-bit transforms, CSS parsing and APCA.
//!
//! Run with `cargo bench -p tpt-colour`. CI compares a pull request with its base commit
//! (see .github/workflows/ci.yml, job `bench-regression`).

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use tpt_colour::contrast::apca_contrast;
use tpt_colour::icc::{ColourTransform, IccProfile, RenderingIntent};
use tpt_colour::parse::parse_colour;

fn bench_transform(c: &mut Criterion) {
    let transform = ColourTransform::new(
        &IccProfile::display_p3(),
        &IccProfile::srgb(),
        RenderingIntent::RelativeColorimetric,
    )
    .expect("built-in profiles convert");
    let pixels = 4096_u64;
    let mut buffer: Vec<u8> = (0..pixels * 4).map(|i| (i % 251) as u8).collect();

    let mut group = c.benchmark_group("transform");
    group.throughput(Throughput::Elements(pixels));
    group.bench_function("p3_to_srgb_u8_rgba", |b| {
        b.iter(|| {
            transform
                .transform_u8(black_box(&mut buffer), 4)
                .expect("four channels");
        });
    });
    group.finish();
}

fn bench_parse_and_contrast(c: &mut Criterion) {
    c.bench_function("parse_colour_oklch", |b| {
        b.iter(|| parse_colour(black_box("oklch(62.8% 0.2577 29.23)")));
    });
    c.bench_function("apca_contrast", |b| {
        b.iter(|| apca_contrast(black_box([0.1; 3]), black_box([0.9; 3])));
    });
}

criterion_group!(benches, bench_transform, bench_parse_and_contrast);
criterion_main!(benches);
