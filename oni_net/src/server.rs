use crossbeam_channel as channel;
use fnv::FnvHashMap;
use std::{
    net::{SocketAddr, UdpSocket},
    time::{Instant, Duration},
    collections::{HashMap, HashSet},
    mem::uninitialized,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    sync::Arc,
};

use crate::{
    token::{
        ChallengeToken,
        PrivateToken,
        USER,
    },
    protocol::*,
    utils::{keygen, time_secs, ReplayProtection},
    server_list::ServerList,
};

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;

fn example() {
    let addr = "[::1]:40000".parse().unwrap();
    let private_key = crate::utils::keygen();
    let mut server = Server::new(666, private_key, addr).unwrap();

    //let local_addr = server.local_addr();
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

impl Conn {
    fn new(id: u64, time: Instant, keys: KeyPair, recv_queue: channel::Sender<Payload>) -> Self {
        Self {
            last_send: time,
            last_recv: time,
            recv_key: keys.recv_key,
            send_key: keys.send_key,
            timeout: Duration::from_secs(keys.timeout as u64),
            id,
            replay_protection: ReplayProtection::new(),
            sequence: Arc::new(AtomicU32::new(1)),
            recv_queue,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn process_payload<'a>(&mut self, protocol: u64, time: Instant, buf: &'a mut [u8]) -> Option<&'a [u8]> {
        let p = &mut self.replay_protection;
        if let Ok(p) = read_packet(protocol, &self.recv_key, buf, |seq| p.packet_already_received(seq)) {
            self.last_recv = time;
            if p.len() != 0 {
                let mut packet = [0u8; MAX_PAYLOAD];
                &packet[..p.len()].copy_from_slice(p);
                self.recv_queue.send((p.len() as u16, packet));
                return Some(p);
            }
        }
        None
    }

    fn process_disconnect(&mut self, protocol: u64, buf: &mut [u8]) -> bool {
        if buf.len() != OVERHEAD { return false; }
        let p = &mut self.replay_protection;
        if let Ok(p) = read_packet(protocol, &self.recv_key, buf, |seq| p.packet_already_received(seq)) {
            assert_eq!(p.len(), 0);
            true
        } else {
            false
        }
    }
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

impl KeyPair {
    fn new(expire: u64, token: &PrivateToken) -> Self {
        Self {
            recv_key: *token.client_key(),
            send_key: *token.server_key(),
            timeout: token.timeout(),
            expire,
        }
    }
}

use self::ConnectionError::*;

enum ConnectionError {
    InvalidPacket,
    AlreadyConnected,
    TokenAlreadyUsed,
    ConnectionDenied([u8; KEY]),
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

    connected: HashMap<SocketAddr, Conn>,
    connected_by_id: FnvHashMap<u64, SocketAddr>,

    incoming: Incoming,

    global_sequence: AtomicU32,
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

            connected: HashMap::default(),
            connected_by_id: HashMap::default(),

            global_sequence: AtomicU32::new(0x0100_0000),
            challenge_sequence: 0,
            challenge_key: keygen(),

            incoming: Incoming::new(),

            capacity: 0,
        })
    }

    pub fn local_addr(&self) -> SocketAddr { self.local_addr }

    pub fn update<F>(&mut self, mut callback: F)
        where F: FnMut(Connection, &[u8; USER])
    {
        self.incoming.update();

        let now = Instant::now();
        self.time = now;
        let mut buf = [0u8; MTU];
        while let Ok((len, from)) = self.socket.recv_from(&mut buf[..]) {
            match self.process_packet(&mut buf[..len], from, &mut callback) {
                Ok(0) => (),
                Ok(len) => {
                    let _ = self.socket.send_to(&buf[..len], from);
                }
                Err(ConnectionDenied(key)) => {
                    let seq = self.global_sequence.fetch_add(1, Ordering::Relaxed);
                    let packet = denied_packet(self.protocol, seq, &key);
                    let _ = self.socket.send_to(&packet, from);
                }
                Err(_) => (),
            }
        }

        // events
        while let Some((addr, (len, payload))) = self.recv_ch.try_recv() {
            let client = match self.connected.get(&addr) {
                Some(c) => c,
                None => continue,
            };
            let seq = client.sequence.fetch_add(1, Ordering::Relaxed);
            let key = &client.send_key;
            if len == 0 {
                let p = disconnect_packet(self.protocol, seq, key);
                let _ = self.socket.send_to(&p, addr);
            } else {
                send_payload(
                    self.protocol, seq, key,
                    &payload[..len as usize],
                    |buf| { let _ = self.socket.send_to(buf, addr); }
                );
            }
        }

        // check for timeout
        let by_id = &mut self.connected_by_id;
        self.connected.retain(|_, c| {
            let is_closed = c.closed.load(Ordering::SeqCst);
            let remove = is_closed || c.last_recv + c.timeout < now;
            if remove {
                by_id.remove(&c.id).unwrap();
            }
            !remove
        });

        // send keep-alive
        for (addr, c) in self.connected.iter_mut().filter(|(_, c)| c.last_send + PACKET_SEND_DELTA > now) {
            let seq = c.sequence.fetch_add(1, Ordering::Relaxed);
            let packet = keep_alive_packet(self.protocol, seq, &c.send_key);
            let _ = self.socket.send_to(&packet, *addr);
            c.last_send = self.time;
        }
    }

    fn is_already_connected(&self, addr: SocketAddr, id: u64) -> bool {
        self.connected.contains_key(&addr) || self.connected_by_id.contains_key(&id)
    }

    fn process_packet<F>(&mut self, buf: &mut [u8], addr: SocketAddr, callback: &mut F) -> Result<usize, ConnectionError>
        where F: FnMut(Connection, &[u8; USER])
    {
        // Ignore small packets.
        if buf.len() < OVERHEAD { return Err(InvalidPacket); }

        match buf[0] >> 6 {
            REQUEST => self.process_request(buf, addr),
            CHALLENGE => self.process_response(buf, addr, callback),

            DISCONNECT => {
                if let Some(client) = self.connected.get_mut(&addr) {
                    if client.process_disconnect(self.protocol, buf) {
                        let client = self.connected.remove(&addr).unwrap();
                        self.connected_by_id.remove(&client.id).expect("client_id not saved");
                    }
                }
                Ok(0)
            }
            PAYLOAD => {
                if let Some(client) = self.connected.get_mut(&addr) {
                    client.process_payload(self.protocol, self.time, buf);
                }
                Ok(0)
            }
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn can_connect(&self) -> bool {
        self.capacity == 0 || self.capacity <= self.connected.len()
    }

    fn process_request(&mut self, buf: &mut [u8], addr: SocketAddr) -> Result<usize, ConnectionError> {
        let (expire, token) = RequestPacket::open(buf, self.protocol, self.timestamp, &self.private)
             .map_err(|_| InvalidPacket)?;

        // If the decrypted private connect token fails to be read for any reason, ignore the packet.
        // If the dedicated server public address is not in the list of server addresses in the private connect token, ignore the packet.
        let list = ServerList::deserialize(token.data()).map_err(|_| InvalidPacket)?;
        if !list.contains(&self.local_addr) { return Err(InvalidPacket); }

        if self.is_already_connected(addr, token.client_id()) { return Err(AlreadyConnected); }
        if !self.incoming.add_token_history(*token.hmac(), addr, expire) { return Err(TokenAlreadyUsed); }
        if !self.can_connect() { return Err(ConnectionDenied(*token.server_key())); }
        self.incoming.insert(addr, expire, &token);

        // Otherwise, respond with a connection challenge packet
        // and increment the connection challenge sequence number.
        let client_id = token.client_id();
        let challenge = ChallengePacket::write(
            self.challenge_sequence,
            &self.challenge_key,
            ChallengeToken::new(client_id, *token.user()),
        );
        self.challenge_sequence += 1;

        let seq = self.global_sequence.fetch_add(1, Ordering::Relaxed);
        let key = token.server_key();
        let packet = new_challenge_packet(self.protocol, seq, key, &challenge);
        buf[..CHALLENGE_PACKET_LEN].copy_from_slice(&packet);
        Ok(CHALLENGE_PACKET_LEN)
    }

    fn process_response<F>(&mut self, buf: &mut [u8], addr: SocketAddr, callback: &mut F) -> Result<usize, ConnectionError>
        where F: FnMut(Connection, &[u8; USER])
    {
        if buf.len() != CHALLENGE_PACKET_LEN { return Err(InvalidPacket); }
        let pending = self.incoming.get(&addr).ok_or(InvalidPacket)?;

        let token = ResponsePacket::open_token(self.protocol, buf, &pending.recv_key, &self.challenge_key)
            .map_err(|_| InvalidPacket)?;

        // If a client from the packet source address and port is already connected, ignore the packet.
        // If a client with the client id contained in the encrypted challenge token data is already connected, ignore the packet.
        if self.is_already_connected(addr, token.client_id()) { return Err(AlreadyConnected); }

        // If no client slots are available, then the server is full.
        // Respond with a connection denied packet.
        if !self.can_connect() { return Err(ConnectionDenied(pending.send_key)); }

        // Assign the packet IP address + port and client id to a free client slot and mark that client as connected.
        // Copy across the user data from the challenge token into the client slot so it is accessible to the server application.
        // Set the confirmed flag for that client slot to false.
        let keys = self.incoming.remove(&addr).unwrap();

        // Respond with a connection keep-alive packet.
        let packet = keep_alive_packet(self.protocol, 0, &keys.send_key);
        buf[..OVERHEAD].copy_from_slice(&packet);

        let client_id = token.client_id();
        let (recv_queue, recv_ch) = channel::unbounded();
        let conn = Conn::new(client_id, self.time, keys, recv_queue);

        callback(Connection {
            closed: conn.closed.clone(),
            recv_ch,
            send_ch: self.send_ch.clone(),
            addr,
            id: client_id,
        }, token.user());

        self.connected_by_id.insert(client_id, addr);
        self.connected.insert(addr, conn);

        Ok(OVERHEAD)
    }
}

struct Incoming {
    timestamp: u64,
    pending: HashMap<SocketAddr, KeyPair>,
    token_history: HashMap<[u8; HMAC], (SocketAddr, u64)>,
}

impl Incoming {
    fn new() -> Self {
        Self {
            timestamp: time_secs(),
            pending: HashMap::new(),
            token_history: HashMap::new(),
        }
    }
    fn get(&self, addr: &SocketAddr) -> Option<&KeyPair> {
        self.pending.get(addr)
    }
    fn remove(&mut self, addr: &SocketAddr) -> Option<KeyPair> {
        self.pending.remove(addr)
    }
    fn insert(&mut self, addr: SocketAddr, expire: u64, token: &PrivateToken) {
        self.pending.entry(addr).or_insert_with(|| KeyPair::new(expire, &token));
    }
    fn add_token_history(&mut self, hmac: [u8; HMAC], addr: SocketAddr, expire: u64) -> bool {
        self.token_history.entry(hmac).or_insert((addr, expire)).0 == addr
    }
    fn update(&mut self) {
        let timestamp = time_secs();
        self.pending.retain(|_, p| p.expire > timestamp);
        self.token_history.retain(|_, v| v.1 > timestamp);
        self.timestamp = timestamp;
    }
}
