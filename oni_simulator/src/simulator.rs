use rand::prelude::*;
use generic_array::ArrayLength;
use crossbeam_channel::{Sender, Receiver, unbounded};
use slotmap::{SlotMap, Key};
use std::{
    time::Instant,
    net::SocketAddr,
    sync::{Arc, Mutex},
    collections::HashMap,
};
use crate::{Config, DefaultMTU, Socket, Datagram};

#[derive(Clone)]
crate struct Entry<MTU: ArrayLength<u8>> {
    pub delivery_time: Instant,
    pub id: usize,
    pub dup: bool,
    pub payload: Datagram<MTU>,
}

impl<MTU: ArrayLength<u8>> Entry<MTU> {
    fn new(id: usize, delivery_time: Instant, payload: Datagram<MTU>) -> Self {
        Self { id, delivery_time, payload, dup: false }
    }
    fn dup(id: usize, delivery_time: Instant, payload: Datagram<MTU>) -> Self {
        Self { id, delivery_time, payload, dup: true }
    }
}

/// Network simulator.
#[derive(Clone)]
pub struct Simulator<MTU: ArrayLength<u8> = DefaultMTU> {
    sim: Arc<Mutex<Inner<MTU>>>,
}

impl Default for Simulator<DefaultMTU> {
    fn default() -> Self { Simulator::new() }
}

impl<MTU: ArrayLength<u8>> Simulator<MTU> {
    /// Constructs a new, empty `Simulator`.
    pub fn new() -> Self {
        Self { sim: Arc::new(Mutex::new(Inner::new())) }
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
        Socket::new(self.sim.clone(), local_addr, name)
    }

    pub fn add_mapping<A>(&self, from: SocketAddr, to: A, config: Config)
        where A: Into<Option<SocketAddr>>
    {
        self.sim.lock().unwrap().insert(from, to, config);
    }

    pub fn remove_mapping<A>(&self, from: SocketAddr, to: A)
        where A: Into<Option<SocketAddr>>
    {
        self.sim.lock().unwrap().remove(from, to);
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
}

pub struct Inner<MTU: ArrayLength<u8>> {
    rng: SmallRng,
    time: Instant,
    entries: Vec<Entry<MTU>>,

    queue: Receiver<Datagram<MTU>>,
    sender: Sender<Datagram<MTU>>,
    mapping: HashMap<SocketAddr, Sender<Entry<MTU>>>,

    store: SlotMap<Config>,
    connect: HashMap<(SocketAddr, SocketAddr), Key>,
    bind: HashMap<SocketAddr, Key>,
}

impl<MTU: ArrayLength<u8>> Inner<MTU> {
    fn new() -> Self {
        let (sender, queue) = unbounded();
        Self {
            entries: Vec::new(),
            time: Instant::now(),
            rng: SmallRng::from_entropy(),

            queue,
            sender,
            mapping: HashMap::default(),

            store: SlotMap::new(),
            connect: HashMap::default(),
            bind: HashMap::default(),
        }
    }

    crate fn attach(&mut self, addr: SocketAddr, sender: Sender<Entry<MTU>>) -> Sender<Datagram<MTU>> {
        self.mapping.insert(addr, sender);
        self.sender.clone()
    }

    crate fn detach(&mut self, addr: SocketAddr) {
        self.mapping.remove(&addr);
    }

    fn advance(&mut self, now: Instant) {
        let count = self.queue.len();
        for _ in 0..count {
            let name = "";
            let payload = self.queue.recv().unwrap();

            let id = oni_trace::generate_id();

            let from = *payload.from();
            let to = *payload.to();

            let key = self.connect.get(&(from, to))
                .or_else(|| self.bind.get(&from))
                .cloned()
                .unwrap_or_default();
            let config = self.store.get(key);

            if let Some(config) = config {
                let delivery_time = match config.delivery(&mut self.rng, self.time) {
                    Some(t) => t,
                    None => return,
                };

                oni_trace::flow_start!(name, id, oni_trace::colors::GREY);

                let dup = config.duplicate(&mut self.rng, delivery_time);
                if let Some(delivery_time) = dup {
                    let id = oni_trace::generate_id();
                    oni_trace::flow_start!(name, id, oni_trace::colors::PEACH);
                    self.entries.push(Entry::dup(id, delivery_time, payload.clone()));
                }
                self.entries.push(Entry::new(id, delivery_time, payload.clone()));
            } else {
                oni_trace::flow_start!(name, id);
                self.entries.push(Entry::new(id, self.time, payload));
            }
        }

        for entry in self.entries.drain_filter(|e| e.delivery_time <= now) {
            if let Some(to) = self.mapping.get(entry.payload.to()) {
                to.send(entry);
            }
        }
    }

    fn insert<U: Into<Option<SocketAddr>>>(&mut self, from: SocketAddr, to: U, data: Config) {
        let key = self.store.insert(data);
        let key = if let Some(to) = to.into() {
            self.connect.insert((from, to), key)
        } else {
            self.bind.insert(from, key)
        };
        if let Some(key) = key {
            self.store.remove(key);
        }
    }

    fn remove<U: Into<Option<SocketAddr>>>(&mut self, from: SocketAddr, to: U) -> Option<Config> {
        let to = to.into();
        let key = if let Some(to) = to {
            self.connect.get(&(from, to))
        } else {
            self.bind.get(&from)
        };
        let key = key.cloned().unwrap_or_default();
        self.store.remove(key)
    }
}


#[test]
fn all() {
    let sim = Simulator::default();

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

/*
#[test]
fn bind() {
    let mut store: Store<usize, u16> = Store::new();

    store.insert(5, None, 1234);

    assert_eq!(store.any_find(5, None).cloned(), Some(1234));
    assert_eq!(store.any_find(5, 7).cloned(), Some(1234));

    assert_eq!(store.remove(5, 7), None);
    assert_eq!(store.remove(5, None), Some(1234));
}

#[test]
fn connect() {
    let mut store: Store<usize, u16> = Store::new();

    store.insert(5, 7, 1234);

    assert_eq!(store.any_find(5, None).cloned(), None);
    assert_eq!(store.any_find(5, 7).cloned(), Some(1234));

    assert_eq!(store.remove(5, None), None);
    assert_eq!(store.remove(5, 7), Some(1234));
}

#[test]
fn bind_and_connect() {
    let mut store: Store<usize, u16> = Store::new();

    store.insert(5, None, 1234);
    store.insert(5, 7, 4321);

    assert_eq!(store.any_find(5, None).cloned(), Some(1234));
    assert_eq!(store.any_find(5, 7).cloned(), Some(4321));

    assert_eq!(store.remove(5, None).clone(), Some(1234));
    assert_eq!(store.remove(5, 7).clone(), Some(4321));
}
*/
