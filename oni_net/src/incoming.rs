use byteorder::{LE, ByteOrder};
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
        CHALLENGE_LEN,
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

    pub fn open_request(&self, r: &mut Request) -> Result<(u64, PrivateToken), ()> {
        if !r.is_valid(self.protocol, self.timestamp) { return Err(()) }
        let token = r.open_token(&self.private)?;
        Ok((r.expire(), token))
    }

    pub fn open_response(&self, buf: &mut [u8; 8 + CHALLENGE_LEN], addr: &SocketAddr, seq: u64, prefix: u8, tag: &[u8; HMAC]) -> Result<([u8; KEY], ChallengeToken), ()> {
        let pending = self.pending.get(addr).ok_or(())?;

        Packet::open(self.protocol, buf, seq, prefix, tag, &pending.recv_key)?;

        let (seq, buf) = buf.split_at_mut(8);
        let seq = LE::read_u64(seq);
        let mut cc = [0u8; CHALLENGE_LEN];
        cc[..].copy_from_slice(buf);

        let token = ChallengeToken::decrypt(cc, seq, &self.key)?;
        Ok((pending.send_key, token))
    }

    pub fn gen_challenge(&self, seq: u64, buf: &mut [u8], token: &PrivateToken) -> usize {
        let client_id = token.client_id();
        let key = token.server_key();
        let mut m = ChallengePacket::write(
            self.sequence.fetch_add(1, Ordering::Relaxed),
            &self.key,
            ChallengeToken::new(client_id, *token.user()),
        );
        Packet::encode_handshake(self.protocol, buf, seq, key, &mut m).unwrap()
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
