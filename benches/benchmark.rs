//! Benchmarks for ruley performance-critical operations.
//!
//! Run with: `cargo bench`

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

/// Benchmark token counting operations.
fn bench_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_counting");

    // Sample code snippets of various sizes
    let small_code = "fn main() { println!(\"Hello\"); }";
    let medium_code = include_str!("../src/main.rs");

    group.bench_with_input(
        BenchmarkId::new("small_snippet", small_code.len()),
        &small_code,
        |b, code| {
            b.iter(|| {
                // Placeholder for actual token counting
                std::hint::black_box(code.split_whitespace().count())
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("medium_snippet", medium_code.len()),
        &medium_code,
        |b, code| b.iter(|| std::hint::black_box(code.split_whitespace().count())),
    );

    group.finish();
}

/// Benchmark file pattern matching.
fn bench_pattern_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_matching");

    let patterns = vec!["*.rs", "*.ts", "*.py", "*.go"];
    let test_files = vec![
        "src/main.rs",
        "src/lib.rs",
        "tests/test.rs",
        "package.json",
        "README.md",
    ];

    group.bench_function("glob_matching", |b| {
        b.iter(|| {
            for pattern in &patterns {
                for file in &test_files {
                    std::hint::black_box(file.ends_with(&pattern[1..]));
                }
            }
        })
    });

    group.finish();
}

/// Benchmark string operations common in code processing.
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    let sample_code = r#"
        fn example() {
            let x = 42;
            println!("Value: {}", x);
        }
    "#;

    group.bench_function("line_splitting", |b| {
        b.iter(|| std::hint::black_box(sample_code.lines().count()))
    });

    group.bench_function("whitespace_normalization", |b| {
        b.iter(|| std::hint::black_box(sample_code.split_whitespace().collect::<Vec<_>>()))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_token_counting,
    bench_pattern_matching,
    bench_string_operations,
);
criterion_main!(benches);
