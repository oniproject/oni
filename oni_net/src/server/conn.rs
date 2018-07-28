use std::{
    net::SocketAddr,
    time::{Instant, Duration},
    collections::HashMap,
    mem::transmute,
};

use crate::{
    token::Challenge,
    packet::{Protection, ReplayProtection},
};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(slotmap::Key);

impl Slot {
    pub fn is_null(&self) -> bool { self.0.is_null() }

    pub fn index(self) -> u32 {
        struct UnsafeKey { idx: u32, version: u32 }
        let UnsafeKey { idx, .. } = unsafe { transmute(self) };
        idx
    }
}

pub struct Connection {
    key: slotmap::Key,
    timeout: Duration,
    confirmed: bool,
    sequence: u64,
    last_send: Instant,
    last_recv: Instant,
    challenge: Challenge,
    addr: SocketAddr,

    protection: ReplayProtection,
}

impl Protection for Connection {
    fn packet_already_received(&mut self, sequence: u64) -> bool {
        self.protection.packet_already_received(sequence)
    }
}

impl Connection {
    fn new(key: slotmap::Key, addr: SocketAddr, challenge: Challenge, time: Instant, timeout: Duration) -> Self {
        Self {
            key,
            addr,
            challenge,
            timeout,

            confirmed: false,
            sequence: 0,
            last_send: time,
            last_recv: time,

            protection: ReplayProtection::default(),
        }
    }

    pub fn slot(&self) -> Slot {
        Slot(self.key)
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirmed
    }

    pub fn recv(&mut self, time: Instant) {
        self.confirmed = true;
        self.last_recv = time;
    }

    pub fn send(&mut self, time: Instant) -> u64 {
        self.last_send = time;
        let seq = self.sequence + 1;
        std::mem::replace(&mut self.sequence, seq)
    }
}

pub struct Clients {
    clients: slotmap::SlotMap<Connection>,
    by_id: HashMap<u64, slotmap::Key>,
    by_addr: HashMap<SocketAddr, slotmap::Key>,
}

impl Clients {
    pub fn new() -> Self {
        Self {
            clients: slotmap::SlotMap::new(),
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn insert(&mut self, addr: SocketAddr, challenge: Challenge, time: Instant, timeout: Duration) -> Slot {
        let id = challenge.client_id;
        let key = self.clients.insert_with_key(|key| Connection::new(key, addr, challenge, time, timeout));
        self.by_addr.insert(addr, key);
        self.by_id.insert(id, key);
        Slot(key)
    }

    pub fn keys(&mut self) -> impl Iterator<Item=Slot> + '_ {
        self.clients.keys().map(Slot)
    }

    pub fn remove(&mut self, slot: Slot) -> Option<Connection> {
        match self.clients.remove(slot.0) {
            Some(client) => {
                self.by_addr.remove(&client.addr);
                self.by_id.remove(&client.challenge.client_id);
                Some(client)
            }
            None => None,
        }
    }

    pub fn get(&self, slot: Slot) -> Option<&Connection> {
        self.clients.get(slot.0)
    }
    pub fn get_mut(&mut self, slot: Slot) -> Option<&mut Connection> {
        self.clients.get_mut(slot.0)
    }

    pub fn has_id(&self, id: u64) -> bool {
        self.by_id.contains_key(&id)
    }

    pub fn slot_by_id(&self, id: u64) -> Slot {
        let key = self.by_id.get(&id)
            .map(|&s| s)
            .unwrap_or(slotmap::Key::null());
        Slot(key)
    }
    pub fn slot_by_addr(&self, addr: SocketAddr) -> Slot {
        let key = self.by_addr.get(&addr)
            .map(|&s| s)
            .unwrap_or(slotmap::Key::null());
        Slot(key)
    }
}
