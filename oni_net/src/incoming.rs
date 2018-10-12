use std::{
    net::SocketAddr,
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{
    token::{
        ChallengeToken,
        PrivateToken,
    },
    utils::{keygen, time_secs},
    protocol::*,
};


pub struct KeyPair {
    expire: u64,
    timeout: u32,
    send_key: [u8; KEY],
    recv_key: [u8; KEY],
}

impl KeyPair {
    fn new(expire: u64, token: &PrivateToken) -> Self {
        Self {
            recv_key: *token.client_key(),
            send_key: *token.server_key(),
            timeout: token.timeout(),
            expire,
        }
    }

    pub fn send_key(&self) -> &[u8; KEY] { &self.send_key }
    pub fn recv_key(&self) -> &[u8; KEY] { &self.recv_key }
    pub fn timeout_secs(&self) -> u32 { self.timeout }
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout as u64)
    }
}

pub struct Incoming {
    protocol: u64,
    timestamp: u64,
    private: [u8; KEY],
    key: [u8; KEY],
    sequence: AtomicU64,

    pending: HashMap<SocketAddr, KeyPair>,
    token_history: HashMap<[u8; HMAC], (SocketAddr, u64)>,
}

impl Incoming {
    pub fn new(protocol: u64, private: [u8; KEY]) -> Self {
        Self {
            protocol,
            private,
            key: keygen(),
            sequence: AtomicU64::new(0),
            timestamp: time_secs(),
            pending: HashMap::new(),
            token_history: HashMap::new(),
        }
    }

    pub fn open_request(&self, buf: &mut [u8]) -> Result<(u64, PrivateToken), ()> {
        RequestPacket::open(buf, self.protocol, self.timestamp, &self.private)
    }

    pub fn open_response(&self, buf: &mut [u8], addr: &SocketAddr) -> Result<([u8; KEY], ChallengeToken), ()> {
        if buf.len() != CHALLENGE_PACKET_LEN { return Err(()); }
        let pending = self.pending.get(addr).ok_or(())?;
        let token = ResponsePacket::open_token(self.protocol, buf, &pending.recv_key, &self.key)?;
        Ok((pending.send_key, token))
    }

    pub fn gen_challenge(&self, seq: u32, token: &PrivateToken) -> [u8; CHALLENGE_PACKET_LEN] {
        let client_id = token.client_id();
        let key = token.server_key();

        let challenge = ChallengePacket::write(
            self.sequence.fetch_add(1, Ordering::Relaxed),
            &self.key,
            ChallengeToken::new(client_id, *token.user()),
        );

        new_challenge_packet(self.protocol, seq, key, &challenge)
    }

    pub fn remove(&mut self, addr: &SocketAddr) -> Option<KeyPair> {
        self.pending.remove(addr)
    }
    pub fn insert(&mut self, addr: SocketAddr, expire: u64, token: &PrivateToken) {
        self.pending.entry(addr).or_insert_with(|| KeyPair::new(expire, &token));
    }
    pub fn add_token_history(&mut self, hmac: [u8; HMAC], addr: SocketAddr, expire: u64) -> bool {
        self.token_history.entry(hmac).or_insert((addr, expire)).0 == addr
    }
    pub fn update(&mut self) {
        let timestamp = time_secs();
        self.pending.retain(|_, p| p.expire > timestamp);
        self.token_history.retain(|_, v| v.1 > timestamp);
        self.timestamp = timestamp;
    }
}
