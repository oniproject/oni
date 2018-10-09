use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::VecDeque;

use crate::server::{KEY, HMAC, XNONCE};
use crate::token::{PublicToken, PRIVATE_LEN, CHALLENGE_LEN};
use crate::protocol::*;
use crate::utils::{err_ret, slice_to_array, ReplayProtection};

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
    Connected,
    Connecting(ConnectingState),
    Disconnected,
    Failed(Error),
}

use self::Error::*;
use self::State::*;
use self::ConnectingState::*;

pub struct Client {
    state: State,
    socket: UdpSocket,

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

    sequence: AtomicU32,
    response: [u8; 8 + CHALLENGE_LEN],

    replay_protection: ReplayProtection,
    recv_queue: VecDeque<(usize, [u8; MAX_PAYLOAD])>,
}

impl Client {
    pub fn new(protocol: u64, token: &PublicToken, addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
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

            sequence: AtomicU32::new(0),
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

    pub fn send(&mut self, buf: &[u8]) {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let key = self.send_key;
        send_payload(self.protocol, seq, &key, &buf, |buf| self.send_packet(buf));
    }

    fn send_packet(&mut self, data: &[u8]) {
        let _ = self.socket.send(&data);
        self.last_send = self.time;
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
        if  self.last_recv + self.timeout < self.time {
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
                Connected => self.send(&[]),

                Connecting(SendingRequest) => {
                    let req = RequestPacket::new(self.protocol, self.expire_timestamp, self.nonce, self.token);
                    self.send_packet(&req.write());
                }
                Connecting(SendingResponse) => {
                    let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
                    let resp = new_challenge_packet(self.protocol, seq, &self.send_key, &self.response);
                    self.send_packet(&resp);
                }

                _ => unreachable!(),
            }
        }
    }

    fn process_packet(&mut self, buf: &mut [u8]) {
        if buf.len() < OVERHEAD { return; }
        match (self.state, buf[0] >> 6) {
            (Connected, PAYLOAD) => self.process_payload(buf),
            (Connected, DISCONNECT) => self.process_disconnect(buf),

            (Connecting(_), DENIED)  => self.process_denied(buf),
            (Connecting(SendingRequest), CHALLENGE) => self.process_challenge(buf),
            (Connecting(SendingResponse), PAYLOAD) => {
                self.state = Connected;
                self.process_payload(buf);
            }

            _ => (),
        }
    }

    fn process_challenge(&mut self, buf: &mut [u8]) {
        self.response = err_ret!(ChallengePacket::client_read(self.protocol, buf, &self.recv_key));
        self.state = Connecting(SendingResponse);
    }

    fn process_denied(&mut self, buf: &mut [u8]) {
        err_ret!(EmptyPacket::read(self.protocol, buf, &self.recv_key));
        self.state = Failed(ConnectionDenied);
    }

    fn process_disconnect(&mut self, buf: &mut [u8]) {
        // TODO: replay protection?
        err_ret!(EmptyPacket::read(self.protocol, buf, &self.recv_key));
        self.state = Disconnected;
    }

    fn process_payload(&mut self, buf: &mut [u8]) {
        let is_keep_alive = buf.len() == OVERHEAD;

        let (header, buf) = buf.split_at_mut(HEADER);

        let (kind, seq) = err_ret!(extract_header(header));
        if self.replay_protection.packet_already_received(seq) { return; }

        let (ciphertext, tag) = buf.split_at_mut(buf.len() - HMAC);
        let tag = err_ret!(slice_to_array!(tag, HMAC));

        err_ret!(decrypt_packet(self.protocol, kind, seq, ciphertext, tag, &self.recv_key));
        self.last_recv = self.time;

        let mut packet = [0u8; MAX_PAYLOAD];
        &packet[..ciphertext.len()].copy_from_slice(ciphertext);

        if !is_keep_alive {
            self.recv_queue.push_back((ciphertext.len(), packet));
        }
    }
}

#[test]
fn error_token_expired() {
    const PROTOCOL: u64 = 0x1122334455667788;

    let addr = "[::]:0".parse().unwrap();
    let server = "[::1]:40000".parse().unwrap();
    let client_id = 666;
    let private_key = crate::utils::keygen();

    let expire = 0;
    let timeout = 0;

    let mut data = [0u8; crate::token::DATA];
    let mut user = [0u8; crate::token::USER];
    crate::utils::crypto_random(&mut data[..]);
    crate::utils::crypto_random(&mut user[..]);

    let token = PublicToken::generate(
        data,
        user,
        expire, // in seconds
        timeout, // in seconds
        client_id,
        PROTOCOL,
        &private_key,
    );

    let mut client = Client::new(PROTOCOL, &token, addr).unwrap();
    client.connect(server).unwrap();
    client.update();

    assert_eq!(client.state(), Failed(ConnectTokenExpired));
}
