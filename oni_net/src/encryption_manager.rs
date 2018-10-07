use std::{
    net::SocketAddr,
    time::{Instant, Duration},
    collections::hash_map::{HashMap, Entry},
};

use crate::crypto::{Key, Private};

pub struct Keys {
    timeout: Duration,
    last_access: Instant,
    send_key: Key,
    recv_key: Key,
}

impl Keys {
    pub fn expired(&self, time: Instant) -> bool {
        self.last_access + self.timeout < time
        //self.expire < time
    }
    pub fn disable_expire(&mut self) { unimplemented!() }
    pub fn send_key(&self) -> &Key { &self.send_key }
    pub fn recv_key(&self) -> &Key { &self.recv_key }
    pub fn timeout(&self) -> Duration { self.timeout }
}

pub struct Mapping {
    mapping: HashMap<SocketAddr, Keys>,
    time: Instant,
}

impl Mapping {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            time: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        //debug!("reset encryption manager");
        self.time = Instant::now();
        self.mapping.clear();
    }

    pub fn advance(&mut self) {
        self.time = Instant::now();
    }

    pub fn add_time(&mut self, dt: Duration) {
        self.time += dt;
    }

    /*
    pub fn is_valid(&self, addr: SocketAddr) -> bool {
        self.mapping.contains_key(key.0)
    }

    pub fn is_expired(&self, key: EncryptionKey) -> bool {
        self.mapping.get(key.0).map(|e| e.expired(time)).unwrap_or(true)
    }
    */

    pub fn insert_token(&mut self, addr: SocketAddr, token: &Private) -> bool {
        self.insert(
            addr,
            token.server_key,
            token.client_key,
            token.timeout,
        )
    }

    pub fn insert(&mut self, addr: SocketAddr, send_key: Key, recv_key: Key, timeout: u32) -> bool {
        self.mapping.insert(addr, Keys {
            send_key,
            recv_key,
            timeout: Duration::from_secs(timeout as u64),
            last_access: self.time,
        })
        .is_none()
    }

    pub fn remove(&mut self, addr: SocketAddr) -> bool {
        self.mapping.remove(&addr).is_some()
    }

    pub fn contains(&self, addr: SocketAddr) -> bool {
        self.mapping.contains_key(&addr)
    }

    pub fn find(&mut self, addr: SocketAddr) -> Option<&mut Keys> {
        match self.mapping.entry(addr) {
            Entry::Occupied(mut o) => {
                if !o.get().expired(self.time) {
                    o.get_mut().last_access = self.time;
                    Some(o.into_mut())
                } else {
                    o.remove_entry();
                    None
                }
            }
            Entry::Vacant(_) => None,
        }
    }

    pub fn touch(&mut self, addr: SocketAddr) -> bool {
        unimplemented!()
        /*
        match self.mapping.get(&addr) {
            Some(e) => { e.last_access = self.time; true }
            None => false,
        }
        */
    }

    /*
    pub fn expire(&self, addr: SocketAddr) -> Option<Instant> {
        self.mapping.get(key.0).map(|e| e.expire)
    }
    pub fn set_expire(&mut self, key: EncryptionKey, expire: Instant) -> bool {
        let e = self.mapping.get_mut(key.0);
        if let Some(e) = e { e.expire = expire }
        e.is_some()
    }
    pub fn send_key(&self, key: EncryptionKey) -> Option<&Key> {
        self.mapping.get(key.0).map(|e| &e.send_key)
    }
    */
    pub fn recv_key(&self, addr: SocketAddr) -> Option<&Key> {
        self.mapping.get(&addr).map(|e| &e.recv_key)
    }
}
