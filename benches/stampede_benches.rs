use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
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
    for size in [16, 128, KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB]
        .iter()
        .copied()
    {
        let data = generate_uniform_values(size);
        let mut map: StampedeMap<_, _> = black_box(StampedeMap::with_capacity(size));
        for (k, v) in &data {
            black_box(map.set(*k, *v));
        }
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter(|| {
                black_box(map.get(1));
            });
        });
    }
}

pub fn lookup_existing(c: &mut Criterion) {
    static KB: usize = 1024;
    let mut group = c.benchmark_group("get existing");
    for size in [KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB].iter().copied() {
        let data = generate_uniform_values(size);
        let mut map: StampedeMap<_, _> = StampedeMap::with_capacity(size);
        for (k, v) in &data {
            map.set(*k, *v);
        }
        map.set(1, 5);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter(|| {
                black_box(map.get(1));
            })
        });
    }
}

pub fn insertion(c: &mut Criterion) {
    static KB: usize = 1024;
    let mut group = c.benchmark_group("get not existing");
    for size in [KB, 2 * KB, 4 * KB, 8 * KB, 16 * KB].iter().copied() {
        let data = generate_uniform_values(size);
        let mut map: StampedeMap<_, _> = StampedeMap::with_capacity(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _size| {
            b.iter(|| {
                for (k, v) in &data {
                    map.set(*k, *v);
                }
            })
        });
    }
}

criterion_group!(benches, lookup_not_existing, insertion);
criterion_main!(benches);
