//#![allow(unused_variables)]
//#![allow(dead_code)]

use crossbeam_channel as channel;
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
        USER,
    },
    protocol::*,
    utils::{keygen, err_ret, none_ret, time_secs, ReplayProtection},
    server_list::ServerList,
};

const HMAC_RETAIN_THRIESOLD: usize = 100;

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
    fn process_payload(&mut self, protocol: u64, time: Instant, buf: &mut [u8]) {
        let p = &mut self.replay_protection;
        if let Ok(p) = read_packet(protocol, &self.recv_key, buf, |seq| p.packet_already_received(seq)) {
            self.last_recv = time;
            if p.len() != 0 {
                let mut packet = [0u8; MAX_PAYLOAD];
                &packet[..p.len()].copy_from_slice(p);
                self.recv_queue.send((p.len() as u16, packet));
            }
        }
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

            pending: HashMap::new(),
            connected: HashMap::new(),
            connected_by_id: HashMap::new(),
            token_history: HashMap::new(),

            global_sequence: AtomicU32::new(0x0800_0000),
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

        let mut buf = [0u8; MTU];
        while let Ok((len, from)) = self.socket.recv_from(&mut buf[..]) {
            // Ignore small packets.
            if len < OVERHEAD { continue; }

            let buf = &mut buf[..len];
            match buf[0] >> 6 {
                REQUEST     => self.process_request(buf, from),
                CHALLENGE   => self.process_response(buf, from, &mut callback),
                DISCONNECT  => {
                    if let Some(client) = self.connected.get_mut(&from) {
                        if client.process_disconnect(self.protocol, buf) {
                            let client = self.connected.remove(&from).unwrap();
                            self.connected_by_id.remove(&client.id).expect("client_id not saved");
                        }
                    }
                }
                PAYLOAD     => {
                    if let Some(client) = self.connected.get_mut(&from) {
                        client.process_payload(self.protocol, self.time, buf);
                    }
                }
                _ => unsafe { std::hint::unreachable_unchecked() },
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

        // TODO expirity

        // remove old token's hmac
        if self.token_history.len() >= HMAC_RETAIN_THRIESOLD {
            self.token_history.retain(|_, v| v.1 < timestamp);
        }
    }

    fn is_already_connected(&self, addr: SocketAddr, id: u64) -> bool {
        self.connected_by_id.contains_key(&id) || self.connected.contains_key(&addr)
    }

    fn send_denied(&self, addr: SocketAddr, key: &[u8; KEY]) {
        let seq = self.global_sequence.fetch_add(1, Ordering::Relaxed);
        let packet = denied_packet(self.protocol, seq, key);
        let _ = self.socket.send_to(&packet, addr);
    }

    fn process_request(&mut self, buf: &mut [u8], addr: SocketAddr) {
        // If the packet is not the expected size of MTU, ignore the packet.
        let r = err_ret!(RequestPacket::read(buf));

        // If the version info in the packet doesn't match VERSION, ignore the packet.
        // If the protocol id in the packet doesn't match the expected protocol id of the dedicated server, ignore the packet.
        // If the connect token expire timestamp is <= the current timestamp, ignore the packet.
        if !r.is_valid(self.protocol, self.timestamp) { return; }

        // If the encrypted private connect token data doesn't decrypt with the private key,
        // using the associated data constructed from:
        //  - version info
        //  - protocol id
        //  - expire timestamp
        // ignore the packet.
        let token = err_ret!(r.token(&self.private));

        let client_id = token.client_id();

        if false {
            // If the decrypted private connect token fails to be read for any reason,
            // for example, having a number of server addresses outside of the expected range of [1,32],
            // or having an address type value outside of range [0,1],
            // ignore the packet.
            // If the dedicated server public address is not in the list of server addresses in the private connect token, ignore the packet.
            let list = none_ret!(ServerList::deserialize(token.data()));
            if !list.contains(&self.local_addr) { return; }
        }

        // If a client from the packet IP source address and port is already connected, ignore the packet.
        // If a client with the client id contained in the private connect token data is already connected, ignore the packet.
        if self.is_already_connected(addr, client_id) { return; }

        // If the connect token has already been used by a different packet source IP address and port, ignore the packet.
        // Otherwise, add the private connect token hmac + packet source IP address and port to the history of connect tokens already used.
        if self.token_history.entry(*token.hmac()).or_insert((addr, r.expire())).0 != addr { return; }

        // If no client slots are available, then the server is full.
        // Respond with a connection denied packet.
        if self.capacity != 0 && self.capacity <= self.connected.len() {
            return self.send_denied(addr, token.server_key());
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
        let challenge = ChallengePacket::write(
            self.challenge_sequence,
            &self.challenge_key,
            ChallengeToken::new(client_id, *token.user()),
        );
        self.challenge_sequence += 1;

        let seq = self.global_sequence.fetch_add(1, Ordering::Relaxed);
        let key = token.server_key();

        let packet = new_challenge_packet(self.protocol, seq, key, &challenge);

        let _ = self.socket.send_to(&packet[..], addr);
    }

    fn process_response<F>(&mut self, buf: &mut [u8], addr: SocketAddr, callback: &mut F)
        where F: FnMut(Connection, &[u8; USER])
    {
        if buf.len() != CHALLENGE_PACKET_LEN { return; }

        let pending = none_ret!(self.pending.get(&addr));
        let mut ciphertext = err_ret!(ChallengePacket::client_read(self.protocol, buf, &pending.recv_key));

        // If the encrypted challenge token data fails to decrypt, ignore the packet.
        let token = err_ret!(ResponsePacket::read(&mut ciphertext, &self.challenge_key));

        let client_id = token.client_id();

        // If a client from the packet source address and port is already connected, ignore the packet.
        // If a client with the client id contained in the encrypted challenge token data is already connected, ignore the packet.
        if self.is_already_connected(addr, client_id) { return; }

        // If no client slots are available, then the server is full.
        // Respond with a connection denied packet.
        if self.capacity != 0 && self.capacity <= self.connected.len() {
            return self.send_denied(addr, &pending.send_key);
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
            sequence: Arc::new(AtomicU32::new(1)),
            recv_queue,
            closed: closed.clone(),
        });

        callback(Connection {
            closed,
            recv_ch,
            send_ch: self.send_ch.clone(),
            addr,
            id: client_id,
        }, token.user());

        // Respond with a connection keep-alive packet.
        let send_key = keys.send_key;
        let packet = keep_alive_packet(self.protocol, 0, &send_key);
        let _ = self.socket.send_to(&packet, addr);
    }
}
