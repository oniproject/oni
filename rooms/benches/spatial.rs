#![feature(test)]
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate criterion;
extern crate rand;
use criterion::{Criterion, Bencher, black_box};
//use test::{Bencher, black_box};

use specs::prelude::*;
use rooms::{SpatialHashMap, Tuple32};

use rand::{
    Rng, FromEntropy,
    rngs::SmallRng,
    distributions::Uniform,
};

lazy_static! {
    static ref EN: Vec<(f32, f32, u32)> = {
        let mut rng = SmallRng::from_entropy();
        let side = Uniform::new(-1000.0, 1000.0);
        //let ids = Uniform::new(0, 0xFFFF); // max id 0xFF_FFFF
        let ids = Uniform::new(0, 100_000);
        (0..100_000)
            .map(|_| (rng.sample(side), rng.sample(side), rng.sample(ids)))
            .collect()
    };
    static ref QU: Vec<(f32, f32)> = {
        let mut rng = SmallRng::from_entropy();
        let side = Uniform::new(-1000.0, 1000.0);
        (0..1_000)
            .map(|_| (rng.sample(side), rng.sample(side)))
            .collect()
    };
}

fn run_bench_spatial1<WH: typenum::Unsigned>(b: &mut Bencher) {
    let mut map: SpatialHashMap<WH, WH, Tuple32> = SpatialHashMap::new();
    b.iter(|| {
        map.clear();
        for (x, y, e) in EN.iter().cloned() {
            map.insert((x, y), e);
        }
        for q in QU.iter().cloned() {
            for v in map.iter_at(q) {
                black_box(v);
            }
        }
    })
}

fn run_bench_spatial2<WH: typenum::Unsigned>(b: &mut Bencher) {
    b.iter_with_large_setup(|| {
        let mut map: SpatialHashMap<WH, WH, Tuple32> = SpatialHashMap::new();
        for (x, y, e) in EN.iter().cloned() {
            map.insert((x, y), e);
        }
        map
    }, |mut map| for q in QU.iter().cloned() {
        for v in map.iter_at(q) {
            black_box(v);
        }
    })
}

fn criterion_benchmark(c: &mut Criterion) {
    //c.bench_function("1 50 ", |b| run_bench_spatial1::<typenum::U50 >(b));
    //c.bench_function("2 4  ", |b| run_bench_spatial1::<typenum::U4  >(b));
    //c.bench_function("2 5  ", |b| run_bench_spatial1::<typenum::U5  >(b));
    c.bench_function("2 50 ", |b| run_bench_spatial2::<typenum::U50 >(b));
    //c.bench_function("2 500", |b| run_bench_spatial2::<typenum::U500>(b));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
