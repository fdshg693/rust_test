use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn sort_random_vec() {
    let mut v: Vec<i32> = (0..1_000).map(|i| ((i * 37) % 1_000) as i32).collect();
    // simple shuffle-ish by rotating based on a small transform to avoid RNG dependency
    v.reverse();
    v.sort_unstable();
    black_box(v);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("sort 1000 ints", |b| b.iter(|| sort_random_vec()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
