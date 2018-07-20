use rand::{
    prelude::*,
    distributions::{Distribution, Uniform},
};

use std::time::{Instant, Duration};
use std::mem::replace;

#[derive(Default)]
pub struct Config {
    /// Latency in milliseconds
    pub latency: Duration,
    /// Jitter in milliseconds +/-
    pub jitter: Duration,
    /// Packet loss percentage.
    pub loss: f64,
    /// Duplicate packet percentage
    pub duplicate: f64,
}

pub struct Simulator<P> where P: Clone {
    config: Config,

    rng: SmallRng,

    /// Current time from last call to advance time.
    time: Instant,
    /// Current index in the packet entry array.
    /// New packets are inserted here.
    current: usize,
    /// Pointer to dynamically allocated packet entries.
    /// This is where buffered packets are stored.
    entries: Vec<Option<(Instant, P)>>,
    /// List of packets pending receive.
    /// Updated each time you call NetworkSimulator::AdvanceTime.
    pending: Vec<P>,
}

impl<P> Simulator<P> where P: Clone {
    /// Create a network simulator.
    pub fn new(capacity: usize, config: Config) -> Self {
        assert!(capacity != 0);
        Self {
            entries: vec![None; capacity],
            pending: Vec::with_capacity(capacity),
            current: 0,
            time: Instant::now(),

            config,
            rng: SmallRng::from_entropy(),
        }
    }

    fn push(&mut self, data: P, delivery: Instant) {
        let i = (self.current + 1) % self.entries.len();
        let i = replace(&mut self.current, i);
        self.entries[i] = Some((
            delivery,
            data,
        ));
    }

    /// Queue a packet up for send.
    /// It makes a copy of the data instead.
    pub fn send(&mut self, data: P) {
        let zero = Duration::from_secs(0);
        let one = Duration::from_secs(1);
        let percent = Uniform::new(0.0, 100.0);
        if self.config.loss > percent.sample(&mut self.rng) {
            return;
        }

        let mut delivery = self.time + self.config.latency;

        if percent.sample(&mut self.rng) <= self.config.duplicate {
            let dt = Uniform::new(zero, one).sample(&mut self.rng);
            self.push(data.clone(), delivery + dt);
        }

        if self.config.jitter != zero {
            let dt = Uniform::new(zero, self.config.jitter).sample(&mut self.rng);
            if self.rng.gen() { delivery += dt } else { delivery -= dt }
        }

        self.push(data, delivery);

    }

    /// Receive packets.
    pub fn receive(&mut self) -> impl Iterator<Item=P> + '_ {
        self.pending.drain(..)
    }

    /// Receive packets with filter.
    pub fn receive_filter<'a, F>(&'a mut self, f: F) -> impl Iterator<Item=P> + 'a
        where F: FnMut(&mut P) -> bool + 'a
    {
        self.pending.drain_filter(f)
    }

    /// Discard all packets in the network simulator.
    /// This is useful if the simulator needs to be reset and used for another purpose.
    pub fn clear(&mut self) {
        for p in &mut self.entries {
            *p = None;
        }
        self.pending.clear();
    }

    /// Advance network simulator time.
    /// You must pump this regularly otherwise the network simulator won't work.
    pub fn advance(&mut self, dt: Duration) {
        self.time += dt;

        // walk across packet entries and move any that are ready
        // to be received into the pending receive buffer
        for e in &mut self.entries {
            if let Some(p) = e.take() {
                if p.0 <= self.time {
                    self.pending.push(p.1);
                } else {
                    *e = Some(p);
                }
            }
        }
    }
}

#[test]
fn main() {
    const CAP: usize = 10;
    let mut s = Simulator::new(CAP, Config::default());
    let mut v = Vec::new();
    for i in 0..5 {
        s.send(i);
        s.advance(Duration::new(10, 0));
        v.extend(s.receive());
        assert_eq!(v, &[i]);
        v.clear();
        v.extend(s.receive());
        assert_eq!(v, &[]);
        v.clear();
    }
}
