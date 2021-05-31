use std::collections::HashSet;
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::Rng;
use stampede_map::StampedeMap;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[inline(always)]
fn generate_uniform_values(size: usize) -> Vec<(usize, usize)> {
    let mut values = vec![(0, 0); size];
    for i in values.iter_mut() {
        *i = (rand::random(), rand::random());
    }
    values
}

pub fn lookup_not_existing(c: &mut Criterion) {
    static KB: usize = 1024;
    let mut group = c.benchmark_group("get not existing");
    let mut rng = rand::thread_rng();
    for size in [16, 128, KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB]
        .iter()
        .copied()
    {
        let data = generate_uniform_values(size);
        let mut map: StampedeMap<_, _> = black_box(StampedeMap::with_capacity(size));
        let mut set = HashSet::with_capacity(size);
        for (k, v) in &data {
            black_box(map.set(*k, *v));
            set.insert(*k);
        }
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                let mut total = start;
                for _ in 0..iters {
                    let key = loop {
                        let key: usize = rng.gen();
                        if !set.contains(&key) {
                            break key;
                        }
                    };
                    let value = Instant::now();
                    black_box(map.get(key));
                    total += value.elapsed();
                }
                total.duration_since(start)
            })
        });
    }
}

pub fn lookup_existing(c: &mut Criterion) {
    static KB: usize = 1024;
    let mut group = c.benchmark_group("get existing");
    let mut rng = rand::thread_rng();
    for size in [KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB].iter().copied() {
        let data = generate_uniform_values(size);
        let mut map: StampedeMap<_, _> = StampedeMap::with_capacity(size);
        let mut set: HashSet<_> = HashSet::with_capacity(size);
        for (k, v) in &data {
            map.set(*k, *v);
            set.insert(*k);
        }
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                let mut total = start;
                for _ in 0..iters {
                    let offset = rng.gen_range(0..set.len() - 1);
                    if let Some(key) = set.iter().skip(offset).next() {
                        let value = Instant::now();
                        black_box(map.get(*key));
                        total += value.elapsed();
                    }
                }
                total.duration_since(start)
            })
        });
    }
}

pub fn insertion(c: &mut Criterion) {
    static KB: usize = 1024;
    let mut group = c.benchmark_group("insertion");
    for size in [KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB].iter().copied() {
        let data = generate_uniform_values(size);
        let mut map: StampedeMap<_, _> = StampedeMap::with_capacity(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                let mut total = start;
                for _ in 0..iters {
                    for (k, v) in &data {
                        let value = Instant::now();
                        black_box(map.set(*k, *v));
                        total += value.elapsed();
                    }
                }
                total.duration_since(start)
            })
        });
    }
}

criterion_group!(benches, lookup_existing, lookup_not_existing, insertion);
criterion_main!(benches);
