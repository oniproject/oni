#![feature(drain_filter)]

use rand::{
    prelude::*,
    distributions::{Distribution, Uniform},
};
use generic_array::{
    typenum::{Sum, U200, U500, U1000},
    ArrayLength,
    GenericArray,
};

use std::{
    time::{Instant, Duration},
    mem::{replace, zeroed},
    net::{SocketAddr, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

pub type MTU1500 = Sum<U1000, U500>;
pub type MTU1200 = Sum<U1000, U200>;

pub type MTUv6 = MTU1500;
pub type MTUv4 = MTU1200;

// TODO: use io::Error ?
pub enum Error {
    Empty,
}

#[derive(Clone)]
struct Packet<MTU: ArrayLength<u8> = MTU1200> {
    len: usize,
    data: GenericArray<u8, MTU>,
}

impl<'a, MTU: ArrayLength<u8>> From<&'a [u8]> for Packet<MTU> {
    fn from(packet: &'a [u8]) -> Self {
        assert!(packet.len() <= MTU::to_usize());

        let mut data: GenericArray<u8, MTU> = unsafe { zeroed() };
        let len = packet.len();
        (&mut data[..len]).copy_from_slice(packet);
        Self { data, len }
    }
}

#[derive(Clone)]
struct Entry {
    from: SocketAddr,
    to: SocketAddr,
    delivery_time: Instant,
    // TODO: time-to-live ?
    // dead_time: Instant,
    packet: Packet,
}

pub struct Socket {
    simulator: Arc<Mutex<Simulator>>,
    local_addr: SocketAddr,
    // TODO: read/write timeout? Always nonblocking?

    send_bytes: AtomicUsize,
    recv_bytes: AtomicUsize,
}

impl Socket {
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
        let mut sim = self.simulator.lock().unwrap();
        sim.send(self.local_addr, addr, buf);
        Ok(buf.len())
    }

    pub fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        let mut sim = self.simulator.lock().unwrap();
        sim.recv(self.local_addr, buf).ok_or(Error::Empty)
    }
}

impl Drop for Socket {
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
    /// Duplicate packet percentage
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

    pub fn packet_loss(mut self, loss: f64) -> Self {
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
    /// Current index in the packet entry array.
    /// New packets are inserted here.
    current: usize,
    /// Pointer to dynamically allocated packet entries.
    /// This is where buffered packets are stored.
    entries: Vec<Option<Entry>>,
    /// List of packets pending receive.
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

    fn push(&mut self, from: SocketAddr, to: SocketAddr, packet: &[u8], delivery_time: Instant) {
        let packet = Packet::from(packet);
        let i = (self.current + 1) % self.entries.len();
        let i = replace(&mut self.current, i);
        self.entries[i] = Some(Entry {
            delivery_time,
            packet,
            from,
            to,
        });
    }

    /// Queue a packet up for send.
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

    fn recv(&mut self, to: SocketAddr, packet: &mut [u8]) -> Option<(usize, SocketAddr)> {
        self.pending.drain_filter(|entry| {
            entry.to == to
        })
        .next()
        .map(|e| {
            let len = e.packet.data.len().min(packet.len());
            (&mut packet[..len]).copy_from_slice(&e.packet.data[..len]);
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
    pub fn advance(&mut self) {
        self.time = Instant::now();

        // walk across packet entries and move any that are ready
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
