use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustloader::utils::organizer::{get_quality_folder, sanitize_filename};

fn benchmark_sanitize_filename(c: &mut Criterion) {
    let mut group = c.benchmark_group("Filename Sanitization");

    group.bench_function("simple", |b| {
        b.iter(|| sanitize_filename(black_box("video.mp4")))
    });

    group.bench_function("complex", |b| {
        b.iter(|| sanitize_filename(black_box("My Video (2024) - Best Quality [1080p].mp4")))
    });

    group.bench_function("malicious", |b| {
        b.iter(|| sanitize_filename(black_box("../../../etc/passwd")))
    });

    let long_name = "a".repeat(500) + ".mp4";
    group.bench_function("long", |b| {
        b.iter(|| sanitize_filename(black_box(&long_name)))
    });

    group.finish();
}

fn benchmark_quality_folder(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quality Folder Selection");
    let heights = [240, 360, 480, 720, 1080, 1440, 2160, 4320];

    for height in heights {
        group.bench_function(format!("{}p", height), |b| {
            b.iter(|| get_quality_folder(black_box(height)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_sanitize_filename,
    benchmark_quality_folder
);
criterion_main!(benches);
