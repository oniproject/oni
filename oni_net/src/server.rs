use crossbeam_channel as channel;
use crossbeam::queue::SegQueue;
use fnv::FnvHashMap;
use std::{
    net::{SocketAddr, UdpSocket},
    time::{Instant, Duration},
    collections::HashMap,
    mem::uninitialized,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    sync::Arc,
};

use crate::{
    protocol::*,
    incoming::{Incoming, KeyPair},
    token::USER,
    utils::ReplayProtection,
    server_list::ServerList,
};

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;

/*
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
*/

pub type Payload = (u16, [u8; MAX_PAYLOAD]);

/*
struct Channel<A, B> {
    closed: AtomicBool,
    a: SegQueue<A>,
    b: SegQueue<B>,
}

impl<A, B> Channel<A, B> {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            closed: AtomicBool::new(false),
            a: SegQueue::new(),
            b: SegQueue::new(),
        })
    }

    #[inline]
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
    #[inline]
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    #[inline]
    fn send_a(&self, t: A) -> Result<(), A> {
        if self.is_closed() {
            Err(t)
        } else {
            self.a.push(t);
            Ok(())
        }
    }
    #[inline]
    fn recv_a(&self) -> Option<A> {
        self.a.try_pop()
    }

    #[inline]
    fn send_b(&self, t: B) -> Result<(), B> {
        if self.is_closed() {
            Err(t)
        } else {
            self.b.push(t);
            Ok(())
        }
    }
    #[inline]
    fn recv_b(&self) -> Option<B> {
        self.b.try_pop()
    }
}
*/

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
        if self.is_closed() { return; }
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
            recv_key: *keys.recv_key(),
            send_key: *keys.send_key(),
            timeout: keys.timeout(),
            id,
            replay_protection: ReplayProtection::new(),
            sequence: Arc::new(AtomicU32::new(1)),
            recv_queue,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn check(&self, time: Instant) -> bool {
        self.closed.load(Ordering::SeqCst) || self.last_recv + self.timeout < time
    }

    fn seq_send(&mut self, time: Instant) -> u32 {
        self.last_send = time;
        self.sequence.fetch_add(1, Ordering::Relaxed)
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

use self::ConnectionError::*;

enum ConnectionError {
    InvalidPacket,
    AlreadyConnected,
    TokenAlreadyUsed,
    ConnectionDenied([u8; KEY]),
}

pub struct Server {
    time: Instant,
    protocol: u64,

    socket: UdpSocket,
    local_addr: SocketAddr,

    recv_ch: channel::Receiver<(SocketAddr, Payload)>,
    send_ch: channel::Sender<(SocketAddr, Payload)>,

    connected: HashMap<SocketAddr, Conn>,
    connected_by_id: FnvHashMap<u64, SocketAddr>,

    incoming: Incoming,

    global_sequence: AtomicU32,

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
            protocol,
            incoming: Incoming::new(protocol, private),

            socket,
            local_addr,

            recv_ch,
            send_ch,

            connected: HashMap::default(),
            connected_by_id: HashMap::default(),

            global_sequence: AtomicU32::new(0x0100_0000),

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
        let mut buffer = [0u8; MTU];
        while let Ok((len, from)) = self.socket.recv_from(&mut buffer[..]) {
            match self.process_packet(&mut buffer[..len], from, &mut callback) {
                Ok(0) => (),
                Ok(len) => {
                    let _ = self.socket.send_to(&buffer[..len], from);
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
        let socket = &mut self.socket;
        while let Some((addr, (len, payload))) = self.recv_ch.try_recv() {
            let client = match self.connected.get_mut(&addr) {
                Some(c) => c,
                None => continue,
            };
            let seq = client.seq_send(now);
            let key = &client.send_key;
            if len == 0 {
                let p = disconnect_packet(self.protocol, seq, key);
                let _ = socket.send_to(&p, addr);
            } else {
                send_payload(
                    self.protocol, seq, key,
                    &payload[..len as usize],
                    |buf| { let _ = socket.send_to(buf, addr); }
                );
            }
        }

        // check for timeout
        let by_id = &mut self.connected_by_id;
        self.connected.retain(|_, c| {
            let remove = c.check(now);
            if remove { by_id.remove(&c.id).unwrap(); }
            !remove
        });

        // send keep-alive
        for (addr, c) in self.connected.iter_mut().filter(|(_, c)| c.last_send + PACKET_SEND_DELTA > now) {
            let seq = c.seq_send(now);
            let packet = keep_alive_packet(self.protocol, seq, &c.send_key);
            let _ = socket.send_to(&packet, *addr);
        }
    }

    fn is_already_connected(&self, addr: SocketAddr, id: u64) -> bool {
        self.connected.contains_key(&addr) || self.connected_by_id.contains_key(&id)
    }

    fn can_connect(&self) -> bool {
        self.capacity == 0 || self.capacity <= self.connected.len()
    }

    fn process_packet<F>(&mut self, buf: &mut [u8], addr: SocketAddr, callback: &mut F) -> Result<usize, ConnectionError>
        where F: FnMut(Connection, &[u8; USER])
    {
        // Ignore small packets.
        if buf.len() < OVERHEAD { return Err(InvalidPacket); }

        match buf[0] >> 6 {
            REQUEST => {
                let (expire, token) = self.incoming.open_request(buf).map_err(|_| InvalidPacket)?;

                let list = ServerList::deserialize(token.data()).map_err(|_| InvalidPacket)?;
                if !list.contains(&self.local_addr) { return Err(InvalidPacket); }

                if self.is_already_connected(addr, token.client_id()) { return Err(AlreadyConnected); }
                if !self.incoming.add_token_history(*token.hmac(), addr, expire) { return Err(TokenAlreadyUsed); }
                if !self.can_connect() { return Err(ConnectionDenied(*token.server_key())); }
                self.incoming.insert(addr, expire, &token);

                let seq = self.global_sequence.fetch_add(1, Ordering::Relaxed);
                let packet = self.incoming.gen_challenge(seq, &token);
                buf[..CHALLENGE_PACKET_LEN].copy_from_slice(&packet);
                Ok(CHALLENGE_PACKET_LEN)
            }
            CHALLENGE => {
                let (send_key, token) = self.incoming.open_response(buf, &addr).map_err(|_| InvalidPacket)?;

                if self.is_already_connected(addr, token.client_id()) { return Err(AlreadyConnected); }
                if !self.can_connect() { return Err(ConnectionDenied(send_key)); }
                let keys = self.incoming.remove(&addr).unwrap();

                // Respond with a connection keep-alive packet.
                let packet = keep_alive_packet(self.protocol, 0, keys.send_key());
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
}

enum ServerEvent {
    Accept {
        id: u64,
        // TODO
    },
    Denied {
        addr: SocketAddr,
        send_key: [u8; KEY]
    },
    Disconnect {
    },
}
