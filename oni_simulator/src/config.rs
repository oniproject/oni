use rand::{
    prelude::*,
    distributions::{Distribution, Uniform},
};
use std::time::{Instant, Duration};

const ZERO: Duration = Duration::from_secs(0);
const ONE: Duration = Duration::from_secs(1);

#[derive(Debug, Default, Clone, Copy)]
pub struct Config {
    pub latency: Duration,
    pub jitter: Duration,
    pub loss: f64,
    pub duplicate: f64,
}

impl Config {
    crate fn delivery<R>(&self, rng: &mut R, delivery: Instant) -> Option<Instant>
        where R: Rng + ?Sized
    {
        if self.loss > Uniform::new(0.0, 100.0).sample(rng) {
            return None;
        }

        let delivery = delivery + self.latency;

        if self.jitter == ZERO {
            Some(delivery)
        } else {
            let dt = Uniform::new(ZERO, self.jitter).sample(rng);
            if rng.gen() {
                Some(delivery + dt)
            } else {
                Some(delivery - dt)
            }
        }
    }

    crate fn duplicate<R>(&self, rng: &mut R, delivery: Instant) -> Option<Instant>
        where R: Rng + ?Sized
    {
        if self.duplicate > Uniform::new(0.0, 100.0).sample(rng) {
            Some(delivery + Uniform::new(ZERO, ONE).sample(rng))
        } else {
            None
        }
    }
}
