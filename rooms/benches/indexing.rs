#[macro_use] extern crate lazy_static;
#[macro_use] extern crate criterion;
extern crate rand;
use criterion::{Criterion, Bencher, black_box};
use rand::{
    Rng, FromEntropy,
    rngs::SmallRng,
    distributions::Uniform,
};

use rooms::{
    KDBush, SpatialHashMap,
    SpatialIndex, Shim, Tuple32,
};

lazy_static! {
    static ref POINTS: Vec<(u32, [f32; 2])> = {
        let mut rng = SmallRng::from_entropy();
        let side = Uniform::new(-1000.0, 1000.0);
        let ids = Uniform::new(0, 0xFF_FFFF);
        (0..10_000)
            .map(|_| (rng.sample(ids), [rng.sample(side), rng.sample(side)]))
            .collect()
    };
    static ref RANGE: Vec<([f32; 2], [f32; 2])> = {
        let mut rng = SmallRng::from_entropy();
        let side = Uniform::new(-1000.0f32, 1000.0f32);
        (0..1000)
            .map(|_| {
                let (ax, ay) = (rng.sample(side), rng.sample(side));
                let (bx, by) = (rng.sample(side), rng.sample(side));

                let a = [ax.min(bx), ax.max(bx)];
                let b = [ay.min(by), ay.max(by)];
                (a, b)
            })
            .collect()
    };

    static ref WITHIN: Vec<([f32; 2], f32)> = {
        let mut rng = SmallRng::from_entropy();
        let radius = Uniform::new(0.0f32, 50.0f32);
        let center = Uniform::new(-1000.0f32, 1000.0f32);
        (0..1000)
            .map(|_| {
                let (a, b) = (rng.sample(center), rng.sample(center));
                ([a, b], rng.sample(radius))
            })
            .collect()
    };
}

fn fill<T, S>(b: &mut Bencher, mut index: T)
    where T: SpatialIndex<S>,
          S: Shim<Index=u32, Vector=[f32; 2], Scalar=f32>
{
    b.iter(|| {
        index.fill(POINTS.iter().cloned());
        black_box(&mut index);
    });
}

fn range<T, S>(b: &mut Bencher, mut index: T)
    where T: SpatialIndex<S>,
          S: Shim<Index=u32, Vector=[f32; 2], Scalar=f32>
{
    index.fill(POINTS.iter().cloned());
    b.iter_with_setup(|| &RANGE[..], |range| {
        for r in range {
            index.range(r.0, r.1, |idx| { black_box(idx); });
        }
    });
}

fn within<T, S>(b: &mut Bencher, mut index: T)
    where T: SpatialIndex<S>,
          S: Shim<Index=u32, Vector=[f32; 2], Scalar=f32>
{
    index.fill(POINTS.iter().cloned());
    b.iter_with_setup(|| &WITHIN[..], |within| {
        for r in within {
            index.within(r.0, r.1, |idx| { black_box(idx); });
        }
    });
}

use typenum::U50 as WH;

fn benchmark(c: &mut Criterion) {
    /*
    c.bench_function("kdbush fill",  |b| {
        let index: KDBush<Tuple32> = KDBush::new(10);
        fill(b, index)
    });
    c.bench_function("naive fill",  |b| {
        let index: SpatialHashMap<WH, WH, Tuple32> = SpatialHashMap::new();
        fill(b, index)
    });
    */

    c.bench_function("kdbush range",  |b| {
        let index: KDBush<Tuple32> = KDBush::new(10);
        range(b, index)
    });
    /*
    c.bench_function("naive range",  |b| {
        let index: SpatialHashMap<WH, WH, Tuple32> = SpatialHashMap::new();
        range(b, index)
    });
    */

    c.bench_function("kdbush within", |b| {
        let index: KDBush<Tuple32> = KDBush::new(10);
        within(b, index)
    });
    c.bench_function("naive within", |b| {
        let index: SpatialHashMap<WH, WH, Tuple32> = SpatialHashMap::new();
        within(b, index)
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
