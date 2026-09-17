use std::str::FromStr;

use crab_nbt::NbtTag;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

fn benchmark_input(
    criterion: &mut Criterion,
    name: &str,
    input: &str
) {
    let mut group = criterion.benchmark_group(name);
    group.bench_function(name, |b| {
        b.iter_batched(|| { }, |_| {
            NbtTag::from_str(input)
                .expect("Benchmark failed");
        }, BatchSize::SmallInput);
    });
}

fn benchmark(criterion: &mut Criterion) {
    benchmark_input(
        criterion,
        "bigdata",
        include_str!("../tests/data/bigdata.snbt")
    );
}

criterion_group!(benches, benchmark);
criterion_main!(benches);