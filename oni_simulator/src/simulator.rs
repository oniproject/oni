use rand::prelude::*;
use generic_array::ArrayLength;

use std::{
    time::{Instant, Duration},
    net::SocketAddr,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
};

use crate::{Config, DefaultMTU, Socket, store::Store, payload::Payload};

pub const DEAD_TIME: Duration = Duration::from_secs(42);

#[derive(Clone)]
crate struct Entry<MTU: ArrayLength<u8>> {
    crate from: SocketAddr,
    crate to: SocketAddr,

    delivery_time: Instant,
    dead_time: Instant,

    crate payload: Payload<MTU>,

    crate id: usize,
    crate dup: bool,
}

/// Network simulator.
#[derive(Clone)]
pub struct Simulator<MTU: ArrayLength<u8> = DefaultMTU> {
    sim: Arc<Mutex<Inner<MTU>>>,
}

impl<MTU: ArrayLength<u8>> Simulator<MTU> {
    /// Constructs a new, empty `Simulator`.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Constructs a new, empty `Simulator` with the specified capacity.
    pub fn with_capacity(cap: usize) -> Self {
        let inner = Inner {
            entries: Vec::with_capacity(cap),
            pending: Vec::with_capacity(cap),
            time: Instant::now(),
            rng: SmallRng::from_entropy(),
            store: Store::new(),

            id: AtomicUsize::new(0),
        };
        Self { sim: Arc::new(Mutex::new(inner)) }
    }

    /// Creates a socket from the given address.
    ///
    /// **Warning**: it produces small memory leak.
    pub fn add_socket(&self, local_addr: SocketAddr) -> Socket<MTU> {
        let name = Box::leak(local_addr.to_string().into_boxed_str());
        self.add_socket_with_name(local_addr, name)
    }

    /// Creates a named socket from the given address.
    pub fn add_socket_with_name(&self, local_addr: SocketAddr, name: &'static str) -> Socket<MTU> {
        Socket {
            simulator: self.sim.clone(),
            local_addr,

            send_bytes: AtomicUsize::new(0),
            recv_bytes: AtomicUsize::new(0),

            name,
        }
    }

    pub fn add_mapping<A>(&self, from: SocketAddr, to: A, config: Config)
        where A: Into<Option<SocketAddr>>
    {
        self.sim.lock().unwrap().store.insert(from, to, config);
    }

    pub fn remove_mapping<A>(&self, from: SocketAddr, to: A)
        where A: Into<Option<SocketAddr>>
    {
        self.sim.lock().unwrap().store.remove(from, to);
    }

    /// Advance network simulator time.
    ///
    /// You must pump this regularly otherwise the network simulator won't work.
    pub fn advance(&self) {
        oni_trace::scope![Simulator advance];

        let mut sim = self.sim.lock().unwrap();
        let now = Instant::now();
        sim.advance(now);
        sim.time = now;
    }

    /// Discard all payloads in the network simulator.
    ///
    /// This is useful if the simulator needs to be reset and used for another purpose.
    pub fn clear(&self) {
        let mut sim = self.sim.lock().unwrap();
        sim.entries.clear();
        sim.pending.clear();
    }
}

pub struct Inner<MTU: ArrayLength<u8>> {
    store: Store<Config, SocketAddr>,
    rng: SmallRng,

    /// Current time from last call to advance time.
    time: Instant,

    /// Pointer to dynamically allocated payload entries.
    /// This is where buffered payloads are stored.
    crate entries: Vec<Entry<MTU>>,
    /// List of payloads pending receive.
    /// Updated each time you call Simulator::AdvanceTime.
    crate pending: Vec<Entry<MTU>>,

    id: AtomicUsize,
}

impl<MTU: ArrayLength<u8>> Inner<MTU> {
    /// Queue a payload up for send.
    /// It makes a copy of the data instead.
    crate fn send(&mut self, name: &'static str, from: SocketAddr, to: SocketAddr, payload: Payload<MTU>) {
        let dead_time = self.time + DEAD_TIME;

        if let Some(config) = self.store.any_find(from, to) {
            let delivery_time = match config.delivery(&mut self.rng, self.time) {
                Some(t) => t,
                None => return,
            };
            let id = self.id.fetch_add(1, Ordering::Relaxed);

            oni_trace::flow_start!(name, id, oni_trace::colors::TANGERINE);

            let dup = config.duplicate(&mut self.rng, delivery_time);
            if let Some(delivery_time) = dup {
                let id = self.id.fetch_add(1, Ordering::Relaxed);
                oni_trace::flow_start!(name, id, oni_trace::colors::PEACH);
                self.entries.push(Entry {
                    from, to, dead_time, delivery_time,
                    payload: payload.clone(),
                    id, dup: true,
                });
            }
            self.entries.push(Entry {
                from, to, dead_time, payload, delivery_time,
                id, dup: false,
            });
        } else {
            let id = self.id.fetch_add(1, Ordering::Relaxed);
            oni_trace::flow_start!(name, id);
            self.entries.push(Entry {
                from, to, dead_time, payload,
                delivery_time: self.time,
                id, dup: true,
            });
        }
    }

    fn advance(&mut self, now: Instant) {
        // walk across payload entries and move any that are ready
        // to be received into the pending receive buffer
        let packets = self.entries.drain_filter(|e| e.delivery_time < now);
        self.pending.extend(packets);

        // retain deaded
        let dead_time = now + DEAD_TIME;
        self.pending.retain(|e| e.dead_time < dead_time);
    }
}

#[test]
fn all() {
    let sim = Simulator::<crate::DefaultMTU>::new();

    let from = sim.add_socket("[::1]:1111".parse().unwrap());
    let to   = sim.add_socket("[::1]:2222".parse().unwrap());

    for i in 0..5u8 {
        from.send_to(&[i], to.local_addr()).unwrap();
        sim.advance();

        let mut buf = [0u8; 4];
        let (bytes, addr) = to.recv_from(&mut buf[..]).unwrap();
        assert_eq!(bytes, 1);
        assert_eq!(addr, from.local_addr());
        assert_eq!(buf[0], i);

        let err = to.recv_from(&mut buf[..]).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    }
}
