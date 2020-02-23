use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::distributions::Uniform;
use rand::Rng;
use stampede_map::StampedeMap;

const INSERTION_SIZE: usize = 1_000;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

pub fn stampede_benches(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let dist = Uniform::new(0, 1 << 32);
    let mut values = Vec::with_capacity(INSERTION_SIZE);
    for _ in 0..INSERTION_SIZE {
        values.push((
            rng.sample::<usize, Uniform<usize>>(dist),
            rng.sample::<usize, Uniform<usize>>(dist),
        ));
    }
    c.bench_function("random insertion", |b| {
        let mut map: StampedeMap<_, _> = StampedeMap::with_capacity(1 << 28);
        b.iter(|| {
            for (k, v) in &values {
                black_box(map.set(*k, *v));
            }
        })
    });
    c.bench_function("random lookup", |b| {
        let mut map: StampedeMap<_, _> = StampedeMap::with_capacity(1 << 28);
        for (k, v) in &values {
            map.set(*k, *v);
        }
        b.iter(|| {
            black_box(map.get(rand::random::<usize>()));
        })
    });
}

criterion_group!(benches, stampede_benches);
criterion_main!(benches);
