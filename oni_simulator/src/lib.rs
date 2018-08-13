#![feature(drain_filter)]

use rand::{
    prelude::*,
    distributions::{Distribution, Uniform},
};
use generic_array::{
    typenum::{Sum, Unsigned, U0, U48, U200, U1000},
    ArrayLength,
    GenericArray,
};

use std::{
    marker::PhantomData,
    time::{Instant, Duration},
    mem::{replace, zeroed},
    net::{SocketAddr, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const PACKET_DEAD_TIME: Duration = Duration::from_secs(42);

/// Maximum payload size.
pub type DefaulMTU = Sum<U1000, U200>;

/// For IPv6+UDP headers.
pub type DefaultPseudoHeader = U48;
pub type NoPseudoHeader = U0;

// TODO: use io::Error ?
pub enum Error {
    Empty,
}

#[derive(Clone)]
struct Payload<MTU: ArrayLength<u8> = DefaulMTU> {
    len: usize,
    data: GenericArray<u8, MTU>,
}

impl<'a, MTU: ArrayLength<u8>> From<&'a [u8]> for Payload<MTU> {
    fn from(payload: &'a [u8]) -> Self {
        assert!(payload.len() <= MTU::to_usize());

        let mut data: GenericArray<u8, MTU> = unsafe { zeroed() };
        let len = payload.len();
        (&mut data[..len]).copy_from_slice(payload);
        Self { data, len }
    }
}

#[derive(Clone)]
struct Entry {
    from: SocketAddr,
    to: SocketAddr,

    delivery_time: Instant,
    dead_time: Instant,

    payload: Payload,
}

pub struct Socket<H: Unsigned = DefaultPseudoHeader> {
    simulator: Arc<Mutex<Simulator>>,
    local_addr: SocketAddr,
    // TODO: read/write timeout? Always nonblocking?

    send_bytes: AtomicUsize,
    recv_bytes: AtomicUsize,

    _marker: PhantomData<H>,
}

impl<H: Unsigned> Socket<H> {
    // TODO: connect and send/recv ?
    // TODO: peek/peek_from ?

    pub fn take_send_bytes(&self) -> usize {
        self.send_bytes.swap(0, Ordering::Relaxed)
    }
    pub fn take_recv_bytes(&self) -> usize {
        self.recv_bytes.swap(0, Ordering::Relaxed)
    }

    pub fn local_addr(&self) -> Result<SocketAddr, Error> {
        Ok(self.local_addr)
    }

    pub fn send_to(&mut self, buf: &[u8], addr: SocketAddr) -> Result<usize, Error> {
        self.send_bytes.fetch_add(buf.len() + H::to_usize(), Ordering::Relaxed);

        let mut sim = self.simulator.lock().unwrap();
        sim.send(self.local_addr, addr, buf);
        Ok(buf.len())
    }

    pub fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        self.recv_bytes.fetch_add(buf.len() + H::to_usize(), Ordering::Relaxed);

        let mut sim = self.simulator.lock().unwrap();
        sim.recv(self.local_addr, buf).ok_or(Error::Empty)
    }
}

impl<H: Unsigned> Drop for Socket<H> {
    fn drop(&mut self) {
        let mut sim = self.simulator.lock().unwrap();
        sim.remove_to(self.local_addr);
    }
}

pub struct Config {
    /// Latency in milliseconds
    latency: Duration,
    /// Jitter in milliseconds +/-
    jitter: Duration,
    /// Packet loss percentage.
    loss: f64,
    /// Duplicate payload percentage
    duplicate: f64,
}

impl Config {
    pub fn latency(mut self, dt: Duration) -> Self {
        self.latency = dt;
        self
    }
    pub fn latency_millis(self, ms: u64) -> Self {
        self.latency(Duration::from_millis(ms))
    }

    pub fn jitter(mut self, dt: Duration) -> Self {
        self.jitter = dt;
        self
    }
    pub fn jitter_millis(self, ms: u64) -> Self {
        self.jitter(Duration::from_millis(ms))
    }

    pub fn payload_loss(mut self, loss: f64) -> Self {
        self.loss = loss;
        self
    }

    pub fn duplicate(mut self, dup: f64) -> Self {
        self.duplicate = dup;
        self
    }

    pub fn build(self, capacity: usize) -> Simulator {
        assert!(capacity != 0);
        Simulator {
            entries: vec![None; capacity],
            pending: Vec::with_capacity(capacity),
            current: 0,
            time: Instant::now(),

            config: self,
            rng: SmallRng::from_entropy(),
        }
    }
}

pub struct Simulator {
    config: Config,

    rng: SmallRng,

    /// Current time from last call to advance time.
    time: Instant,
    /// Current index in the payload entry array.
    /// New payloads are inserted here.
    current: usize,
    /// Pointer to dynamically allocated payload entries.
    /// This is where buffered payloads are stored.
    entries: Vec<Option<Entry>>,
    /// List of payloads pending receive.
    /// Updated each time you call NetworkSimulator::AdvanceTime.
    pending: Vec<Entry>,
}

impl Simulator {
    pub fn builder() -> Config {
        Config {
            latency: Duration::default(),
            jitter: Duration::default(),
            loss: 0.0,
            duplicate: 0.0,
        }
    }

    fn push(&mut self, from: SocketAddr, to: SocketAddr, payload: &[u8], delivery_time: Instant) {
        let payload = Payload::from(payload);
        let i = (self.current + 1) % self.entries.len();
        let i = replace(&mut self.current, i);
        self.entries[i] = Some(Entry {
            delivery_time,
            dead_time: delivery_time + PACKET_DEAD_TIME,
            payload,
            from,
            to,
        });
    }

    /// Queue a payload up for send.
    /// It makes a copy of the data instead.
    fn send(&mut self, from: SocketAddr, to: SocketAddr, data: &[u8]) {
        let zero = Duration::from_secs(0);
        let one = Duration::from_secs(1);
        let percent = Uniform::new(0.0, 100.0);
        if self.config.loss > percent.sample(&mut self.rng) {
            return;
        }

        let mut delivery = self.time + self.config.latency;

        if percent.sample(&mut self.rng) <= self.config.duplicate {
            let dt = Uniform::new(zero, one).sample(&mut self.rng);
            self.push(from, to, data, delivery + dt);
        }

        if self.config.jitter != zero {
            let dt = Uniform::new(zero, self.config.jitter).sample(&mut self.rng);
            if self.rng.gen() { delivery += dt } else { delivery -= dt }
        }

        self.push(from, to, data, delivery);
    }

    fn recv(&mut self, to: SocketAddr, payload: &mut [u8]) -> Option<(usize, SocketAddr)> {
        self.pending.drain_filter(|entry| {
            entry.to == to
        })
        .next()
        .map(|e| {
            let len = e.payload.data.len().min(payload.len());
            (&mut payload[..len]).copy_from_slice(&e.payload.data[..len]);
            (len, e.from)
        })
    }

    fn remove_to(&mut self, to: SocketAddr) {
        for p in &mut self.entries {
            if let Some(p) = p {
                if p.to != to {
                    continue;
                }
            }
            *p = None;
        }
        self.pending.retain(|e| e.to == to);
    }

    /// Discard all payloads in the network simulator.
    /// This is useful if the simulator needs to be reset and used for another purpose.
    pub fn clear(&mut self) {
        for p in &mut self.entries {
            *p = None;
        }
        self.pending.clear();
    }

    /// Advance network simulator time.
    /// You must pump this regularly otherwise the network simulator won't work.
    pub fn advance(&mut self) {
        self.time = Instant::now();

        // walk across payload entries and move any that are ready
        // to be received into the pending receive buffer
        for e in &mut self.entries {
            if let Some(p) = e.take() {
                if p.delivery_time <= self.time {
                    self.pending.push(p);
                } else {
                    *e = Some(p);
                }
            }
        }
    }
}

#[test]
fn addr_parse() {
    let _v4: SocketAddr = "127.0.0.1:1111".parse().unwrap();
    let _v6: SocketAddr = "[::1]:1111".parse().unwrap();
}

/*
#[test]
fn main() {
    let from = "127.0.0.1:1111".parse().unwrap();
    let to = "127.0.0.1:2222".parse().unwrap();

    let mut sim = Simulator::builder().build(10);
    let mut v = Vec::new();
    for i in 0..5 {
        sim.send(from, to, i);
        sim.advance();
        v.extend(sim.receive());
        assert_eq!(v, &[i]);
        v.clear();
        v.extend(sim.receive());
        assert_eq!(v, &[]);
        v.clear();
    }
}
*/
