use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::VecDeque;
use crate::{
    Socket,
    protocol::{Packet, Request, MTU, PACKET_SEND_DELTA, MAX_PAYLOAD, NUM_DISCONNECT_PACKETS},
    token::{PublicToken, PRIVATE_LEN, CHALLENGE_LEN},
    replay_protection::ReplayProtection,
    crypto::{KEY, XNONCE},
};

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum ConnectingState {
    SendingRequest,
    SendingResponse,
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum Error {
    ConnectTokenExpired,
    InvalidConnectToken,

    ConnectionTimedOut,
    ConnectionResponseTimedOut,
    ConnectionRequestTimedOut,
    ConnectionDenied,
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum State {
    Disconnected,
    Connecting(ConnectingState),
    Connected,
    Failed(Error),
}

use self::Error::*;
use self::State::*;
use self::ConnectingState::*;

pub struct Client<S: Socket = UdpSocket> {
    state: State,
    socket: S,

    protocol: u64,
    expire_timestamp: u64,
    expire: Duration,
    timeout: Duration,

    nonce: [u8; XNONCE],
    token: [u8; PRIVATE_LEN],

    time: Instant,
    start_time: Instant,
    last_send: Instant,
    last_recv: Instant,

    send_key: [u8; KEY],
    recv_key: [u8; KEY],

    sequence: AtomicU64,
    response: [u8; 8 + CHALLENGE_LEN],

    replay_protection: ReplayProtection,
    recv_queue: VecDeque<(usize, [u8; MAX_PAYLOAD])>,
}

impl Client<UdpSocket> {
    pub fn new(protocol: u64, token: &PublicToken, addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        Self::with_socket(protocol, token, socket)
    }
}

impl Client<crate::SimulatedSocket> {
    pub fn simulated(protocol: u64, token: &PublicToken) -> Self {
        let socket = crate::SimulatedSocket::new();
        Self::with_socket(protocol, token, socket).unwrap()
    }
}

impl<S: Socket> Client<S> {
    pub fn with_socket(protocol: u64, token: &PublicToken, socket: S) -> std::io::Result<Self> {
        socket.set_nonblocking(true)?;

        let now = Instant::now();

        let expire = Duration::from_secs(token.expire_timestamp() - token.create_timestamp());
        let timeout = Duration::from_secs(token.timeout_seconds().into());

        Ok(Self {
            state: Disconnected,
            socket,

            protocol,
            expire_timestamp: token.expire_timestamp(),
            expire,
            timeout,

            nonce: token.nonce(),
            token: *token.token(),

            time: now,
            start_time: now,
            last_send: now - Duration::from_secs(1),
            last_recv: now,

            send_key: token.client_key(),
            recv_key: token.server_key(),

            sequence: AtomicU64::new(0),
            response: [0u8; 8 + CHALLENGE_LEN],

            replay_protection: ReplayProtection::new(),
            recv_queue: VecDeque::new(),
        })
    }

    pub fn state(&self) -> State { self.state }

    pub fn connect(&mut self, addr: SocketAddr) -> std::io::Result<()> {
        self.socket.connect(addr)?;
        self.state = Connecting(SendingRequest);
        Ok(())
    }

    pub fn recv(&mut self) -> Option<(usize, [u8; MAX_PAYLOAD])> {
        self.recv_queue.pop_front()
    }

    pub fn close(&mut self) {
        for _ in 0..NUM_DISCONNECT_PACKETS {
            let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
            let mut buf = [0u8; MTU];
            let len = Packet::encode_close(self.protocol, &mut buf, seq, &self.send_key)
                .unwrap();
            self.send_packet(&buf[..len]);
        }
        self.state = Disconnected;
    }

    pub fn update(&mut self) {
        // early exit
        match self.state {
            Disconnected | Failed(_) => return,
            _ => (),
        }

        // update time
        self.time = Instant::now();

        // check token
        if self.time - self.start_time >= self.expire {
            self.state = Failed(ConnectTokenExpired);
            return;
        }

        // check for timeout
        if self.last_recv + self.timeout < self.time {
            self.state = Failed(match self.state {
                Connected => ConnectionTimedOut,
                Connecting(SendingRequest) => ConnectionRequestTimedOut,
                Connecting(SendingResponse) => ConnectionResponseTimedOut,
                _ => unreachable!(),
            });
            return;
        }

        // recv packets
        let mut buf = [0u8; MTU];
        while let Ok(len) = self.socket.recv(&mut buf) {
            self.process_packet(&mut buf[..len]);
        }

        // send packets
        if self.last_send + PACKET_SEND_DELTA < self.time {
            match self.state {
                // KEEP_ALIVE is PAYLOAD with zero length
                Connected => self.send(&mut []).unwrap(),
                Connecting(SendingRequest) => self.send_request(),
                Connecting(SendingResponse) => self.send_response(),
                _ => unreachable!(),
            }
        }
    }

    pub fn send(&mut self, m: &mut [u8]) -> std::io::Result<()> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut buf = [0u8; MTU];
        let len = Packet::encode_payload(self.protocol, &mut buf, seq, &self.send_key, m)?;
        self.send_packet(&buf[..len]);
        Ok(())
    }

    fn send_packet(&mut self, data: &[u8]) {
        let _ = self.socket.send(&data);
        self.last_send = self.time;
    }
    fn send_request(&mut self) {
        let req = Request::new(self.protocol, self.expire_timestamp, self.nonce, self.token);
        self.send_packet(&req.write());
    }
    fn send_response(&mut self) {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let mut response = self.response;
        let mut buf = [0u8; MTU];
        let len = Packet::encode_handshake(self.protocol, &mut buf, seq, &self.send_key, &mut response)
            .unwrap();
        self.send_packet(&buf[..len]);
    }

    fn process_packet(&mut self, buf: &mut [u8]) {
        let packet = match Packet::decode(buf) {
            Some(packet) => packet,
            None => return,
        };

        match (self.state, packet) {
            (Connected, Packet::Payload { seq, buf, tag }) |
            (Connecting(SendingResponse), Packet::Payload { seq, buf, tag }) => {
                if self.replay_protection.already_received(seq) {
                    return;
                }
                if Packet::open(self.protocol, buf, seq, 0, tag, &self.recv_key).is_err() {
                    return;
                }
                self.last_recv = self.time;
                if !buf.is_empty() {
                    let mut packet = [0u8; MAX_PAYLOAD];
                    packet[..buf.len()].copy_from_slice(buf);
                    self.recv_queue.push_back((buf.len(), packet));
                }
                self.state = Connected;
            }
            (Connected, Packet::Close { prefix, seq, tag }) => {
                if self.replay_protection.already_received(seq) {
                    return;
                }
                if Packet::open(self.protocol, &mut [], seq, prefix, tag, &self.recv_key).is_err() {
                    return;
                }
                self.state = Disconnected;
            }
            (Connecting(_), Packet::Close { prefix, seq, tag })  => {
                if Packet::open(self.protocol, &mut [], seq, prefix, tag, &self.recv_key).is_err() {
                }
                self.state = Failed(ConnectionDenied);
            }
            (Connecting(SendingRequest), Packet::Handshake { prefix, seq, buf, tag }) => {
                if Packet::open(self.protocol, buf, seq, prefix, tag, &self.recv_key).is_err() {
                    return;
                }
                self.response.copy_from_slice(buf);
                self.state = Connecting(SendingResponse);

                self.send_response();
            }
            //_ => panic!("!!!!! bad: {} {:?}", buf.len(), buf),
            _ => (),
        }
    }
}

#[test]
fn error_token_expired() {
    use crate::{SimulatedSocket, crypto::{keygen, crypto_random}, token::{USER, DATA}};

    const PROTOCOL: u64 = 0x1122334455667788;

    let server = "[::1]:40000".parse().unwrap();
    let client_id = 666;
    let private_key = keygen();

    let expire = 0;
    let timeout = 0;

    let mut data = [0u8; DATA];
    let mut user = [0u8; USER];
    crypto_random(&mut data[..]);
    crypto_random(&mut user[..]);

    let token = PublicToken::generate(
        data,
        user,
        expire, // in seconds
        timeout, // in seconds
        client_id,
        PROTOCOL,
        &private_key,
    );

    let socket = SimulatedSocket::new();

    let mut client = Client::with_socket(PROTOCOL, &token, socket).unwrap();
    client.connect(server).unwrap();
    client.update();

    assert_eq!(client.state(), Failed(ConnectTokenExpired));
}
