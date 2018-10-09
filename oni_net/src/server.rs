#![allow(unused_variables)]
#![allow(dead_code)]

//! Client  →       auth       →  Relay
//! Client  ←       token      ←  Relay
//! Client  →      request     →  Server ×10≡10hz ≤ 1sec
//! Client  ←  response/close  ←  Server
//! Client  →     challenge    →  Server ×10≡10hz ≤ 1sec
//! Client  ↔   payload/close  ↔  Server

use crossbeam_channel as channel;
use std::{
    net::{SocketAddr, UdpSocket},
    time::{Instant, Duration},
    collections::{HashMap, HashSet},
    mem::{transmute, uninitialized, zeroed},
    os::raw::c_ulonglong,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    sync::Arc,
};

use crate::{
    token::{
        PrivateToken, PRIVATE_LEN,
        ChallengeToken, CHALLENGE_LEN,
    },
    sodium::keygen,
    utils::{err_ret, none_ret, time_secs, slice_to_array},
    packet::{
        REQUEST,
        CHALLENGE,
        DISCONNECT,
        PAYLOAD,

        MIN_PACKET,
        MAX_PACKET,

        CHALLENGE_PACKET_BYTES as CHALLENGE_PACKET_LEN,
    },
};

pub use crate::packet::MAX_PAYLOAD;
pub use crate::{VERSION, VERSION_BYTES as VERSION_LEN};

const HMAC_RETAIN_THRIESOLD: usize = 100;

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;
pub const PUBLIC_LEN: usize = 2048;

const PREFIX_SHIFT: u32 = 30;
const PREFIX_MASK: u32 = 0xC0000000;
const SEQUENCE_MASK: u32 = 0x3FFFFFFF;

pub const DATA: usize = 640;
pub const USER: usize = 256;

pub const ENCRYPTED_HEADER: usize = 4;

pub const REQUEST_PACKET_LEN: usize = 1 + VERSION_LEN + 8 * 2 + XNONCE + PRIVATE_LEN;
pub const DISCONNECT_PACKET_LEN: usize = MIN_PACKET;

type ServerList = arrayvec::ArrayVec<[SocketAddr; 32]>;

fn example() {
    //use bincode::{serialize, deserialize};
    //use arrayvec::ArrayVec;

    let addr = "[::1]:40000".parse().unwrap();
    let private_key = crate::sodium::keygen();
    let mut server = Server::new(666, private_key, addr).unwrap();

    let local_addr = server.local_addr();
    let mut connected: HashSet<Connection> = HashSet::new();
    loop {
        server.update(|c, user| {
            println!("connected {}:{:?} with data {:?}", c.id(), c.addr(), &user[..]);
            connected.insert(c);
        });

        connected.retain(|c| !c.is_closed());

        let mut buf = [0u8; MAX_PAYLOAD];
        for client in &connected {
            match client.recv(&mut buf[..]) {
                Ok(0) => (),
                Ok(len) => println!("recv: {:?}", &buf[..len as usize]),
                Err(()) => continue,
            }

            let _ = client.send(b"fuck you").is_err();
        }
    }
}

pub type Payload = (u16, [u8; MAX_PAYLOAD]);

pub struct Connection {
    closed: Arc<AtomicBool>,
    recv_ch: channel::Receiver<Payload>,
    send_ch: channel::Sender<(SocketAddr, Payload)>,
    addr: SocketAddr,
    id: u64,
}

impl std::hash::Hash for Connection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        Arc::into_raw(self.closed.clone()).hash(state)
    }
}

impl Eq for Connection {}
impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.closed, &other.closed)
    }
}

impl Connection {
    pub fn id(&self) -> u64 { self.id }
    pub fn addr(&self) -> SocketAddr { self.addr }

    #[inline]
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn close(&self) {
        if self.is_closed() {
            return;
        }
        self.closed.store(true, Ordering::SeqCst);

        for _ in 0..10 {
            // send disconnect packets
            self.send_ch.send((self.addr, (0, unsafe { uninitialized() })));
        }
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<u16, ()> {
        if self.is_closed() {
            Err(())
        } else {
            match self.recv_ch.try_recv() {
                Some((len, payload)) => {
                    let len = buf.len().min(len as usize);
                    buf.copy_from_slice(&payload[..len]);
                    Ok(len as u16)
                }
                None => Ok(0)
            }
        }
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize, ()> {
        if self.is_closed() {
            Err(())
        } else {
            let len = buf.len().min(MAX_PAYLOAD);
            if len == 0 {
                Ok(0)
            } else {
                let mut payload = [0u8; MAX_PAYLOAD];
                payload[..len].copy_from_slice(&buf[..len]);
                self.send_ch.send((self.addr, (len as u16, payload)));
                Ok(len)
            }
        }
    }
}

struct Conn {
    closed: Arc<AtomicBool>,
    recv_queue: channel::Sender<Payload>,
    sequence: Arc<AtomicU32>,

    last_recv: Instant,
    last_send: Instant,
    timeout: Duration,
    send_key: [u8; KEY],
    recv_key: [u8; KEY],
    id: u64,
    replay_protection: ReplayProtection,
}

impl Drop for Conn {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}

struct KeyPair {
    expire: u64,
    timeout: u32,
    send_key: [u8; KEY],
    recv_key: [u8; KEY],
}

pub struct Server {
    time: Instant,
    timestamp: u64,
    protocol: u64,
    private: [u8; KEY],

    socket: UdpSocket,
    local_addr: SocketAddr,

    recv_ch: channel::Receiver<(SocketAddr, Payload)>,
    send_ch: channel::Sender<(SocketAddr, Payload)>,

    pending: HashMap<SocketAddr, KeyPair>,
    connected: HashMap<SocketAddr, Conn>,
    connected_by_id: HashMap<u64, SocketAddr>,
    token_history: HashMap<[u8; HMAC], (SocketAddr, u64)>,

    global_sequence: u32,
    challenge_sequence: u64,
    challenge_key: [u8; KEY],

    capacity: usize,
}

impl Server {
    pub fn new(protocol: u64, private: [u8; KEY], addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        let local_addr = socket.local_addr()?;

        let (send_ch, recv_ch) = channel::unbounded();

        Ok(Self {
            time: Instant::now(),
            timestamp: time_secs(),
            protocol,
            private,

            socket,
            local_addr,

            recv_ch,
            send_ch,

            pending: HashMap::new(),
            connected: HashMap::new(),
            connected_by_id: HashMap::new(),
            token_history: HashMap::new(),

            global_sequence: 0x8000_0000,
            challenge_sequence: 0,
            challenge_key: keygen(),

            capacity: 0,
        })
    }

    pub fn local_addr(&self) -> SocketAddr { self.local_addr }

    pub fn update<F>(&mut self, mut callback: F)
        where F: FnMut(Connection, &[u8; USER])
    {
        let now = Instant::now();
        self.time = now;
        let timestamp = time_secs();
        self.timestamp = timestamp;

        let mut buf = [0u8; MAX_PACKET];
        while let Ok((len, from)) = self.socket.recv_from(&mut buf[..]) {
            // Ignore small packets.
            if len < MIN_PACKET { continue; }

            let buf = &mut buf[..len];
            match buf[0] >> 6 {
                REQUEST     => self.process_request(buf, from),
                CHALLENGE   => self.process_response(buf, from, &mut callback),
                DISCONNECT  => self.process_disconnect(buf, from),
                PAYLOAD     => self.process_payload(buf, from),
                _ => unsafe { std::hint::unreachable_unchecked() },
            }
        }

        // events
        for (addr, (len, payload)) in &self.recv_ch {
            let client = match self.connected.get(&addr) {
                Some(c) => c,
                None => continue,
            };

            let len = payload.len().min(MAX_PACKET - MIN_PACKET);
            let seq = client.sequence.fetch_add(1, Ordering::Relaxed);

            let mut packet = [0u8; MAX_PACKET];
            packet[0..4].copy_from_slice(&write_header(PAYLOAD, seq));
            packet[4..4+len].copy_from_slice(&payload[..len]);

            let kind = if len == 0 { DISCONNECT } else { PAYLOAD };

            let hmac = encrypt_packet(
                self.protocol, kind, seq, &mut packet[4..4+len], &client.send_key);

            packet[4+len..len+MIN_PACKET].copy_from_slice(&hmac[..]);

            self.socket.send_to(&packet[..len+MIN_PACKET], addr);
        }

        // check for timeout
        let by_id = &mut self.connected_by_id;
        self.connected.retain(|addr, c| {
            let is_closed = c.closed.load(Ordering::SeqCst);
            let remove = is_closed || c.last_recv + c.timeout < now;
            if remove {
                by_id.remove(&c.id).unwrap();
            }
            remove
        });

        // send keep-alive
        for (addr, c) in self.connected.iter_mut().filter(|(_, c)| c.last_send + crate::PACKET_SEND_DELTA > now) {
            let seq = c.sequence.fetch_add(1, Ordering::Relaxed);
            let packet = simple_packet(self.protocol, &c.send_key, PAYLOAD, seq);
            self.socket.send_to(&packet, *addr);
        }

        // remove old token's hmac
        if self.token_history.len() >= HMAC_RETAIN_THRIESOLD {
            self.token_history.retain(|_, v| v.1 < timestamp);
        }
    }

    fn is_already_connected(&self, addr: SocketAddr, id: u64) -> bool {
        self.connected_by_id.contains_key(&id) || self.connected.contains_key(&addr)
    }

    fn process_request(&mut self, buf: &mut [u8], addr: SocketAddr) {
        // If the packet is not the expected size of 1062 bytes, ignore the packet.
        let r = err_ret!(Request::read(buf));

        if r.prefix != 0 { return; }

        // If the version info in the packet doesn't match VERSION, ignore the packet.
        if r.version != VERSION { return; }
        // If the protocol id in the packet doesn't match the expected protocol id of the dedicated server, ignore the packet.
        if r.protocol() != self.protocol { return; }
        // If the connect token expire timestamp is <= the current timestamp, ignore the packet.
        if r.expire() <= self.timestamp { return; }
        // If the encrypted private connect token data doesn't decrypt with the private key,
        // using the associated data constructed from:
        //  - version info
        //  - protocol id
        //  - expire timestamp
        // ignore the packet.
        let token = err_ret!(r.token(&self.private));

        let client_id = token.client_id();

        // If the decrypted private connect token fails to be read for any reason,
        // for example, having a number of server addresses outside of the expected range of [1,32],
        // or having an address type value outside of range [0,1],
        // ignore the packet.
        // If the dedicated server public address is not in the list of server addresses in the private connect token, ignore the packet.
        //if callback(addr, &token.data()[..]) { return; }

        /* FIXME
        let servers: ServerList = err_ret!(bincode::deserialize(token.data()));
        let local_addr = self.local_addr();
        if !servers.iter().any(|addr| addr == &local_addr) { return; }
        */

        // If a client from the packet IP source address and port is already connected, ignore the packet.
        // If a client with the client id contained in the private connect token data is already connected, ignore the packet.
        if self.is_already_connected(addr, client_id) { return; }

        // If the connect token has already been used by a different packet source IP address and port, ignore the packet.
        // Otherwise, add the private connect token hmac + packet source IP address and port to the history of connect tokens already used.
        if self.token_history.entry(*token.hmac()).or_insert((addr, r.expire())).0 != addr { return; }

        // If no client slots are available, then the server is full.
        // Respond with a connection denied packet.
        if self.capacity != 0 && self.capacity <= self.connected.len() {
            self.global_sequence += 1;
            let packet = simple_packet(self.protocol, token.server_key(), DISCONNECT, self.global_sequence);
            self.socket.send_to(&packet, addr);
            return;
        }

        // Add an encryption mapping for the packet source IP address and port so that packets read from
        // that address and port are decrypted with the client to server key in the private connect token,
        // and packets sent to that address and port are encrypted with the server to client key in the private connect token.
        // This encryption mapping expires in timeout seconds of no packets being sent to or received from that address and port,
        // or if a client fails to establish a connection with the server within timeout seconds.
        // If for some reason this encryption mapping cannot be added, ignore the packet.
        self.pending.entry(addr).or_insert(KeyPair {
            recv_key: *token.client_key(),
            send_key: *token.server_key(),
            timeout: token.timeout(),
            expire: r.expire(),
        });

        // Otherwise, respond with a connection challenge packet
        // and increment the connection challenge sequence number.
        let payload = Challenge::write(
            self.challenge_sequence,
            &self.challenge_key,
            ChallengeToken::new(client_id, *token.user()),
        );
        self.challenge_sequence += 1;

        let seq = self.global_sequence;
        self.global_sequence += 1;

        let mut packet = [0u8; CHALLENGE_PACKET_LEN];
        packet[0..4].copy_from_slice(&write_header(CHALLENGE, seq));
        packet[4..CHALLENGE_PACKET_LEN - HMAC].copy_from_slice(&payload[..]);

        let hmac = encrypt_packet(
            self.protocol, PAYLOAD, seq, &mut packet[4..CHALLENGE_PACKET_LEN - HMAC], token.server_key());

        packet[CHALLENGE_PACKET_LEN - HMAC..].copy_from_slice(&hmac[..]);

        self.socket.send_to(&packet[..], addr);
    }

    fn process_response<F>(&mut self, buf: &mut [u8], addr: SocketAddr, callback: &mut F)
        where F: FnMut(Connection, &[u8; USER])
    {
        if buf.len() != CHALLENGE_PACKET_LEN { return; }

        let pending = none_ret!(self.pending.get(&addr));

        let (kind, seq) = err_ret!(extract_header(buf));
        let buf = &mut buf[ENCRYPTED_HEADER..];
        let (ciphertext, tag) = buf.split_at_mut(buf.len() - HMAC);
        let tag = err_ret!(slice_to_array!(tag, HMAC));
        err_ret!(decrypt_packet(self.protocol, kind, seq, ciphertext, tag, &pending.recv_key));

        // If the encrypted challenge token data fails to decrypt, ignore the packet.
        let token = err_ret!(Challenge::read(ciphertext, &self.challenge_key));

        let client_id = token.client_id();

        // If a client from the packet source address and port is already connected, ignore the packet.
        // If a client with the client id contained in the encrypted challenge token data is already connected, ignore the packet.
        if self.is_already_connected(addr, client_id) { return; }

        // If no client slots are available, then the server is full.
        // Respond with a connection denied packet.
        if self.capacity != 0 && self.capacity <= self.connected.len() {
            self.global_sequence += 1;
            let packet = simple_packet(self.protocol, &pending.send_key, DISCONNECT, self.global_sequence);
            self.socket.send_to(&packet, addr);
            return;
        }

        // Assign the packet IP address + port and client id to a free client slot and mark that client as connected.
        // Copy across the user data from the challenge token into the client slot so it is accessible to the server application.
        // Set the confirmed flag for that client slot to false.
        let keys = self.pending.remove(&addr).unwrap();

        let closed = Arc::new(AtomicBool::new(false));

        let (recv_queue, recv_ch) = channel::unbounded();

        self.connected_by_id.insert(client_id, addr);
        self.connected.insert(addr, Conn {
            last_send: self.time,
            last_recv: self.time,
            recv_key: keys.recv_key,
            send_key: keys.send_key,
            timeout: Duration::from_secs(keys.timeout as u64),
            id: client_id,
            replay_protection: ReplayProtection::new(),
            sequence: Arc::new(AtomicU32::new(0)),
            recv_queue,
            closed: closed.clone(),
        });

        // Respond with a connection keep-alive packet.
        let send_key = keys.send_key;
        let packet = simple_packet(self.protocol, &send_key, PAYLOAD, 1);
        self.socket.send_to(&packet, addr);

        callback(Connection {
            closed,
            recv_ch,
            send_ch: self.send_ch.clone(),
            addr,
            id: client_id,
        }, token.user());
    }

    fn process_disconnect(&mut self, buf: &mut [u8], addr: SocketAddr) {
        if buf.len() != MIN_PACKET { return; }

        let client = none_ret!(self.connected.get_mut(&addr));
        let (kind, seq) = err_ret!(extract_header(buf));
        if client.replay_protection.packet_already_received(seq) {
            return;
        }

        let tag = err_ret!(slice_to_array!(buf[ENCRYPTED_HEADER..], HMAC));
        err_ret!(decrypt_packet(self.protocol, kind, seq, &mut [], tag, &client.recv_key));

        let client = self.connected.remove(&addr).unwrap();
        self.connected_by_id.remove(&client.id).expect("client_id not saved");
    }

    fn process_payload(&mut self, buf: &mut [u8], addr: SocketAddr) {
        let is_keep_alive = buf.len() == MIN_PACKET;

        let client = none_ret!(self.connected.get_mut(&addr));

        let (kind, seq) = err_ret!(extract_header(buf));
        if client.replay_protection.packet_already_received(seq) {
            return;
        }

        let buf = &mut buf[ENCRYPTED_HEADER..];
        let (ciphertext, tag) = buf.split_at_mut(buf.len() - HMAC);
        let tag = err_ret!(slice_to_array!(tag, HMAC));

        err_ret!(decrypt_packet(self.protocol, kind, seq, ciphertext, tag, &client.recv_key));
        client.last_recv = self.time;
        let mut packet = [0u8; MAX_PAYLOAD];
        &packet[..ciphertext.len()].copy_from_slice(ciphertext);
        client.recv_queue.send((ciphertext.len() as u16, packet));
    }
}

fn simple_packet(protocol: u64, key: &[u8; KEY], kind: u8, seq: u32) -> [u8; MIN_PACKET] {
    let header = write_header(kind, seq);
    let hmac = encrypt_packet(protocol, kind, seq, &mut [], key);
    unsafe { transmute((header, hmac)) }
}

pub fn write_header(kind: u8, seq: u32) -> [u8; 4] {
    (((kind as u32) << PREFIX_SHIFT) | seq & SEQUENCE_MASK).to_le_bytes()
}

pub fn extract_header(buffer: &[u8]) -> Result<(u8, u32), ()> {
    let prefix = u32::from_le_bytes(slice_to_array!(&buffer, ENCRYPTED_HEADER)?);
    let kind = (prefix >> PREFIX_SHIFT) & 0b11;
    let sequence = prefix & SEQUENCE_MASK;
    Ok((kind as u8, sequence as u32))
}

#[repr(packed)]
struct EncryptedAd {
    _version: [u8; VERSION_LEN],
    _protocol: [u8; 8],
    _kind: u8,
}

pub fn encrypt_packet(protocol: u64, kind: u8, seq: u32, m: &mut [u8], k: &[u8; KEY]) -> [u8; HMAC] {
    let mut n = [0u8; NONCE];
    n[4..8].copy_from_slice(&seq.to_le_bytes()[..]);

    let ad = EncryptedAd {
        _version: VERSION,
        _protocol: protocol.to_le_bytes(),
        _kind: kind,
    };

    let ad_p = (&ad as *const EncryptedAd) as *const _;
    let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

    let mut tag = [0u8; HMAC];
    let mut maclen = HMAC as c_ulonglong;

    let ret = unsafe {
        crate::sodium::crypto_aead_chacha20poly1305_ietf_encrypt_detached(
            m.as_mut_ptr(),
            tag.as_mut_ptr(),
            &mut maclen,
            m.as_ptr(),
            m.len() as c_ulonglong,
            ad_p,
            ad_len,
            0 as *mut _,
            n.as_ptr(),
            k.as_ptr()
        )
    };
    tag
}

pub fn decrypt_packet(protocol: u64, kind: u8, seq: u32, c: &mut [u8], t: [u8; HMAC], k: &[u8; KEY]) -> Result<(), ()> {
    let mut n = [0u8; NONCE];
    n[4..8].copy_from_slice(&seq.to_le_bytes()[..]);

    let ad = EncryptedAd {
        _version: VERSION,
        _protocol: protocol.to_le_bytes(),
        _kind: kind,
    };

    let ad_p = (&ad as *const EncryptedAd) as *const _;
    let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

    unsafe {
        let ret = crate::sodium::crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            c.as_mut_ptr(),
            0 as *mut _,
            c.as_ptr(),
            c.len() as c_ulonglong,
            t.as_ptr(),
            ad_p, ad_len,
            n.as_ptr(), k.as_ptr(),
        );
        if ret != 0 {
            Err(())
        } else {
            Ok(())
        }
    }
}

struct Request {
    prefix: u8,
    version: [u8; VERSION_LEN],
    protocol: [u8; 8],
    expire: [u8; 8],
    nonce: [u8; XNONCE],
    token: [u8; PRIVATE_LEN],
}

impl Request {
    pub fn protocol(&self) -> u64 {
        u64::from_le_bytes(self.protocol)
    }
    pub fn expire(&self) -> u64 {
        u64::from_le_bytes(self.expire)
    }
    pub fn token(&self, private_key: &[u8; KEY]) -> Result<PrivateToken, ()> {
        PrivateToken::decrypt(&self.token, self.protocol(), self.expire(), &self.nonce, private_key)
    }

    pub fn new(protocol: u64, expire: u64, nonce: [u8; 24], token: [u8; PRIVATE_LEN]) -> Self {
        let mut packet: Self = unsafe { zeroed() };
        packet.prefix = REQUEST;
        packet.version = VERSION;
        packet.protocol = protocol.to_le_bytes();
        packet.expire = expire.to_le_bytes();
        packet.nonce = nonce;
        packet.token = token;
        packet
    }

    pub fn read(buf: &mut [u8]) -> Result<&mut Self, ()> {
        if buf.len() == REQUEST_PACKET_LEN {
            Ok(unsafe { &mut *(buf.as_ptr() as *mut Self) })
        } else {
            Err(())
        }
    }
}

struct Challenge {
    sequence: [u8; 8],
    token: [u8; CHALLENGE_LEN],
}

impl Challenge {
    pub fn write(sequence: u64, key: &[u8; KEY], token: ChallengeToken) -> [u8; 8+CHALLENGE_LEN] {
        unsafe { transmute(Self {
            sequence: sequence.to_le_bytes(),
            token: token.encrypt(sequence, key),
        }) }
    }
    pub fn read(buf: &mut [u8], key: &[u8; KEY]) -> Result<ChallengeToken, ()> {
        if buf.len() == 8 + CHALLENGE_LEN {
            let ch = unsafe { &mut *(buf.as_ptr() as *mut Self) };
            let seq = u64::from_le_bytes(ch.sequence);
            ChallengeToken::decrypt(ch.token, seq, key)
        } else {
            Err(())
        }
    }
}

use generic_array::GenericArray;
use generic_array::typenum::U256;

struct ReplayProtection {
    seq: u32,
    bits: GenericArray<u8, U256>,
}

impl ReplayProtection {
    fn new() -> Self {
        Self {
            seq: 0,
            bits: GenericArray::default(),
        }
    }

    fn reset(&mut self) {
        self.seq = 0;
        self.bits = GenericArray::default();
    }

    fn packet_already_received(&mut self, seq: u32) -> bool {
        if seq >= SEQUENCE_MASK { return true; }
        let len = self.bits.len() as u32;
        if seq.wrapping_add(len) <= self.seq {
            return true;
        }
        if seq > self.seq {
            for bit in self.seq+1..seq+1 {
                let bit = bit % len;
                unsafe { self.clear_unchecked(bit); }
            }
            if seq >= self.seq + len {
                self.bits = GenericArray::default();
            }
            self.seq = seq;
        }
        unsafe {
            let bit = seq % len;
            let ret = self.get_unchecked(bit);
            self.set_unchecked(bit);
            ret
        }
    }

    #[inline(always)] unsafe fn get_unchecked(&self, bit: u32) -> bool {
        let bit = bit as usize;
        *self.bits.get_unchecked(bit >> 3) & (1 << (bit & 0b111)) != 0
    }
    #[inline(always)] unsafe fn set_unchecked(&mut self, bit: u32) {
        let bit = bit as usize;
        *self.bits.get_unchecked_mut(bit >> 3) |= 1 << (bit & 0b111);
    }
    #[inline(always)] unsafe fn clear_unchecked(&mut self, bit: u32) {
        let bit = bit as usize;
        *self.bits.get_unchecked_mut(bit >> 3) &= !(1 << (bit & 0b111));
    }
}

#[test]
fn replay_protection() {
    const SIZE: u32 = 256;
    const MAX: u32 = 4 * SIZE as u32;

    let mut rp = ReplayProtection::new();

    for _ in 0..2 {
        rp.reset();

        assert_eq!(rp.seq, 0);

        for sequence in 0..MAX {
            assert!(!rp.packet_already_received(sequence),
            "The first time we receive packets, they should not be already received");
        }

        assert!(rp.packet_already_received(0),
        "Old packets outside buffer should be considered already received");

        for sequence in MAX - 10..MAX {
            assert!(rp.packet_already_received(sequence),
            "Packets received a second time should be flagged already received");
        }

        assert!(!rp.packet_already_received(MAX + SIZE),
        "Jumping ahead to a much higher sequence should be considered not already received");


        for sequence in 0..MAX {
            assert!(rp.packet_already_received(sequence),
            "Old packets should be considered already received");
        }
    }
}
