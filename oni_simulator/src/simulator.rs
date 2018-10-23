use rand::prelude::*;
use generic_array::ArrayLength;
use crossbeam_channel::{Sender, Receiver, unbounded};

use std::{
    time::{Instant, Duration},
    net::SocketAddr,
    sync::{Arc, Mutex},
    collections::HashMap,
};

use crate::{Config, DefaultMTU, Socket, store::Store, payload::Payload};

pub const DEAD_TIME: Duration = Duration::from_secs(42);

/* XXX
fn fast_eq(a: &'static str, b: &'static str) -> bool {
    a.len() == b.len() && a.as_ptr() == b.as_ptr()
}
*/

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
        let (sender, queue) = unbounded();
        let inner = Inner {
            entries: Vec::new(),
            time: Instant::now(),
            rng: SmallRng::from_entropy(),
            store: Store::new(),
            queue,
            sender,
            mapping: HashMap::default(),
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
        Socket::new(self.sim.clone(), local_addr, name)
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
}

pub struct Inner<MTU: ArrayLength<u8>> {
    store: Store<Config, SocketAddr>,
    rng: SmallRng,
    time: Instant,
    entries: Vec<Entry<MTU>>,
    queue: Receiver<(SocketAddr, SocketAddr, Payload<MTU>)>,
    sender: Sender<(SocketAddr, SocketAddr, Payload<MTU>)>,
    mapping: HashMap<SocketAddr, Sender<Entry<MTU>>>,
}

impl<MTU: ArrayLength<u8>> Inner<MTU> {
    crate fn attach(&mut self, addr: SocketAddr, sender: Sender<Entry<MTU>>)
        -> Sender<(SocketAddr, SocketAddr, Payload<MTU>)>
    {
        self.mapping.insert(addr, sender);
        self.sender.clone()
    }

    crate fn detach(&mut self, addr: SocketAddr) {
        self.mapping.remove(&addr);
    }

    /// Queue a payload up for send.
    /// It makes a copy of the data instead.
    fn send(&mut self, name: &'static str, from: SocketAddr, to: SocketAddr, payload: Payload<MTU>) {
        let dead_time = self.time + DEAD_TIME;
        let id = oni_trace::generate_id();

        if let Some(config) = self.store.any_find(from, to) {
            let delivery_time = match config.delivery(&mut self.rng, self.time) {
                Some(t) => t,
                None => return,
            };

            oni_trace::flow_start!(name, id, oni_trace::colors::GREY);

            let dup = config.duplicate(&mut self.rng, delivery_time);
            if let Some(delivery_time) = dup {
                let id = oni_trace::generate_id();
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
            oni_trace::flow_start!(name, id);
            self.entries.push(Entry {
                from, to, dead_time, payload,
                delivery_time: self.time,
                id, dup: true,
            });
        }
    }

    fn advance(&mut self, now: Instant) {
        let count = self.queue.len();
        for _ in 0..count {
            let p = self.queue.recv().unwrap();
            self.send("", p.0, p.1, p.2);
        }

        let packets = self.entries.drain_filter(|e| e.delivery_time < now);
        for entry in packets {
            if let Some(to) = self.mapping.get(&entry.to) {
                to.send(entry);
            }
        }
    }
}

#[test]
fn all() {
    let sim = Simulator::new();

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
