use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustloader::downloader::segment::calculate_segments;

fn benchmark_segment_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Segment Calculation");

    let file_sizes = [
        1_000_000u64,  // 1 MB
        10_000_000,    // 10 MB
        100_000_000,   // 100 MB
        1_000_000_000, // 1 GB
    ];

    for size in file_sizes {
        group.bench_with_input(
            BenchmarkId::new("calculate_segments", format!("{}MB", size / 1_000_000)),
            &size,
            |b, &size| b.iter(|| calculate_segments(black_box(size), black_box(16))),
        );
    }

    group.finish();
}

fn benchmark_segment_count_variation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Segment Count Variation");
    let file_size = 100_000_000u64; // 100 MB
    let segment_counts = [4usize, 8, 16, 32, 64];

    for count in segment_counts {
        group.bench_with_input(BenchmarkId::new("segments", count), &count, |b, &count| {
            b.iter(|| calculate_segments(black_box(file_size), black_box(count)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_segment_calculation,
    benchmark_segment_count_variation
);
criterion_main!(benches);
