#[macro_use] extern crate criterion;
extern crate rand;
use criterion::{Criterion, black_box};

use rand::{
    Rng, FromEntropy,
    rngs::SmallRng,
    distributions::Uniform,
};

/*
fn fast_inv_sqrt(x: f32) -> f32 {
    let i: u32 = unsafe { std::mem::transmute(x) };
    let j = 0x5f3759df - (i >> 1);
    let y: f32 = unsafe { std::mem::transmute(j) };
    y * (1.5 - 0.5 * x * y * y)
}
*/

fn sq_dist_mult(min: (f32, f32), max: (f32, f32)) -> f32 {
    let dx = min.0 - max.1;
    let dy = min.0 - max.1;
    dx * dx + dy * dy
}

fn sq_dist_powf(min: (f32, f32), max: (f32, f32)) -> f32 {
    let dx = min.0 - max.1;
    let dy = min.0 - max.1;
    dx.powf(2.0) + dy.powf(2.0)
}

fn sq_dist_powi(min: (f32, f32), max: (f32, f32)) -> f32 {
    let dx = min.0 - max.1;
    let dy = min.0 - max.1;
    dx.powi(2) + dy.powi(2)
}

fn make_dists() -> Vec<(f32, f32)> {
    let mut rng = SmallRng::from_entropy();
    let dist = Uniform::new(-1000.0, 1000.0);
    (0..100)
        .map(|_| (rng.sample(dist), rng.sample(dist)))
        .collect()
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("sq_dist mul", |b| b.iter_with_large_setup(make_dists, |dists| {
        for &min in &dists {
            for &max in &dists {
                black_box(sq_dist_mult(min, max));
            }
        }
    }));

    c.bench_function("sq_dist powf", |b| b.iter_with_large_setup(make_dists, |dists| {
        for &min in &dists {
            for &max in &dists {
                black_box(sq_dist_powf(min, max));
            }
        }
    }));

    c.bench_function("sq_dist powi", |b| b.iter_with_large_setup(make_dists, |dists| {
        for &min in &dists {
            for &max in &dists {
                black_box(sq_dist_powi(min, max));
            }
        }
    }));
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
