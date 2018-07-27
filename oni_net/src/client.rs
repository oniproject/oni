pub const NUM_DISCONNECT_PACKETS: usize = 10;

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use PACKET_SEND_DELTA;
use Socket;
use utils::time;
use packet::{
    Allowed, Request,
    Encrypted,
    MAX_PACKET_BYTES,
    MAX_PAYLOAD_BYTES,
};
use crypto::Key;

use token;
use replay_protection::ReplayProtection;

pub trait Callback {
    fn state_change(&mut self, old: State, new: State);
    fn receive(&mut self, sequence: u64, data: &[u8]);
}

pub enum Event<'a> {
    State {
        old: State,
        new: State,
    },
    Payload {
        sequence: u64,
        packet: &'a [u8],
    },
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum Error {
    TokenExpired,
    //InvalidToken,
    TimedOut,
    ResponseTimedOut,
    RequestTimedOut,
    Denied,
    Disconnected,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum State {
    SendingRequest,
    SendingResponse,
    Connected,
    Disconnected { err: Error },
}

impl State {
    pub fn is_connecting(&self) -> bool {
        match self {
            State::SendingRequest |
            State::SendingResponse => true,
            _ => false,
        }
    }
}

pub struct Client<S: Socket, C: Callback> {
    socket: S,
    callback: C,

    state: State,

    time: Instant,
    connect_start_time: Instant,
    last_send: Instant,
    last_recv: Instant,

    sequence: u64,

    client_index: u32,
    max_clients: u32,

    addr: SocketAddr,
    replay_protection: ReplayProtection,
    challenge_token_sequence: u64,
    challenge_token_data: [u8; token::Challenge::BYTES],

    protocol_id: u64,
    read_key: Key,
    write_key: Key,

    token_timeout: Duration,
    token_private_data: [u8; token::Private::BYTES],
    token_expire_timestamp: u64,
    token_create_timestamp: u64,

    token_expire: Duration,
    token_sequence: u64,
}

impl<S: Socket, C: Callback> Client<S, C> {
    pub fn connect(socket: S, callback: C, addr: usize, token: &token::Public) -> Self {
        let addr = token.server_addresses[addr];
        let time = Instant::now();
        Self {
            socket,
            callback,

            state: State::SendingRequest,

            time,
            connect_start_time: time,
            last_send: time - Duration::from_secs(1),
            last_recv: time,

            sequence: 0,

            client_index: 0,
            max_clients: 0,

            addr,
            replay_protection: ReplayProtection::default(),
            challenge_token_sequence: 0,
            challenge_token_data: [0u8; token::Challenge::BYTES],

            protocol_id: token.protocol_id,
            read_key: token.server_to_client_key.clone(),
            write_key: token.client_to_server_key.clone(),
            token_timeout: Duration::from_secs(token.timeout_seconds.into()),

            token_private_data: token.private_data,
            token_expire_timestamp: token.expire_timestamp,
            token_create_timestamp: token.create_timestamp,
            token_sequence: token.sequence,

            token_expire: Duration::from_secs(token.expire_timestamp - token.create_timestamp),
        }
    }

    pub fn update(&mut self) {
        self.time = Instant::now();

        if let Err(err) = self.receive_packets() {
            return self.disconnect(err);
        }

        if self.last_send + PACKET_SEND_DELTA < self.time {
            match self.state {
                State::SendingRequest => {
                    self.send_request();
                }
                State::SendingResponse => {
                    let p = Encrypted::Response {
                        challenge_sequence: self.challenge_token_sequence,
                        challenge_data: self.challenge_token_data,
                    };
                    self.send_packet(p);
                }
                State::Connected => {
                    let p = Encrypted::KeepAlive {
                        client_index: 0,
                        max_clients: 0,
                    };
                    self.send_packet(p);
                }
                _ => (),
            }
        }

        let expire = self.time - self.connect_start_time >= self.token_expire;
        let timedout = self.last_recv + self.token_timeout < self.time;

        if self.state.is_connecting() && expire {
            return self.disconnect(Error::TokenExpired);
        }

        if timedout {
            match self.state {
                State::SendingRequest => return self.disconnect(Error::RequestTimedOut),
                State::SendingResponse => return self.disconnect(Error::ResponseTimedOut),
                State::Connected => return self.disconnect(Error::TimedOut),
                _ => (),
            }
        }
    }

    pub fn close(&mut self) {
        for _ in 0..NUM_DISCONNECT_PACKETS {
            self.send_packet(Encrypted::Disconnect);
        }
        self.disconnect(Error::Closed);
    }

    pub fn next_packet_sequence(&self) -> u64 { self.sequence }
    pub fn port(&self) -> u16 { self.addr.port() }
    //pub fn server_address(&self) -> SocketAddr { self.server_address }

    pub fn state(&self) -> State { self.state }
    pub fn index(&self) -> u32 { self.client_index }
    pub fn max_clients(&self) -> u32 { self.max_clients }

    pub fn send_payload(&mut self, payload: &[u8]) {
        assert!(payload.len() <= MAX_PAYLOAD_BYTES);
        if self.state != State::Connected {
            return;
        }
        let (data, len) = array_from_slice_uninitialized!(payload, MAX_PAYLOAD_BYTES);
        self.send_packet(Encrypted::Payload {
            sequence: 0,
            len,
            data,
        });
    }

    fn disconnect(&mut self, err: Error)  {
        if let State::Disconnected { .. } = self.state {
            return;
        }
        let state = State::Disconnected { err };
        self.callback.state_change(self.state, state);
        self.state = state;
    }

    fn send_packet(&mut self, packet: Encrypted) {
        let sequence = self.sequence;
        self.sequence += 1;

        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(
            &mut data[..],
            &self.write_key,
            self.protocol_id,
            sequence,
        ).unwrap();

        assert!(bytes <= MAX_PACKET_BYTES);
        self.socket.send(self.addr, &data[..bytes]);
        self.last_send = self.time;
    }

    fn send_request(&mut self) {
        let data = Request::write_request(
            self.protocol_id,
            self.token_expire_timestamp,
            self.token_sequence,
            self.token_private_data,
        );
        self.socket.send(self.addr, &data[..]);
        self.last_send = self.time;
    }

    fn receive_packets(&mut self) -> Result<(), Error> {
        let mut buf = [0u8; MAX_PACKET_BYTES];
        while let Some((bytes, from)) = self.socket.recv(&mut buf[..]) {
            if from != self.addr {
                continue;
            }

            let allowed = match self.state {
                State::Connected =>       Allowed::CLIENT_CONNECTED,
                State::SendingResponse => Allowed::CLIENT_SENDING_RESPONSE,
                State::SendingRequest =>  Allowed::CLIENT_SENDING_REQUEST,
                _ => break,
            };

            let packet = if let Some(packet) = Encrypted::read(
                &mut buf[..bytes],
                Some(&mut self.replay_protection),
                &self.read_key, self.protocol_id,
                allowed,
            )
            { packet } else { continue };

            match (self.state, packet) {
                (State::Connected, Encrypted::Payload { sequence, len, data }) => {
                    self.callback.receive(sequence, &data[..len]);
                }
                (State::Connected, Encrypted::KeepAlive { .. }) => {}
                (State::Connected, Encrypted::Disconnect) => return Err(Error::Disconnected),

                (State::SendingRequest, Encrypted::Denied) => return Err(Error::Denied),
                (State::SendingRequest, Encrypted::Challenge { challenge_sequence, challenge_data }) => {
                    self.challenge_token_sequence = challenge_sequence;
                    self.challenge_token_data = challenge_data;

                    self.callback.state_change(self.state, State::SendingResponse);
                    self.state = State::SendingResponse;
                }

                (State::SendingResponse, Encrypted::Denied) => return Err(Error::Denied),
                (State::SendingResponse, Encrypted::KeepAlive { client_index, max_clients }) => {
                    self.client_index = client_index;
                    self.max_clients = max_clients;

                    self.callback.state_change(self.state, State::Connected);
                    self.state = State::Connected;
                }
                _ => unreachable!(),
            }

            self.last_recv = self.time;
        }
        Ok(())
    }
}

#[test]
fn client_error_token_expired() {
    use {TEST_TIMEOUT_SECONDS, TEST_PROTOCOL_ID};

    struct NoSocket;

    impl Socket for NoSocket {
        fn send(&mut self, addr: SocketAddr, packet: &[u8]) {}
        fn recv(&mut self, packet: &mut [u8]) -> Option<(usize, SocketAddr)> { None }
    }

    struct Cb;
    impl Callback for Cb {
        fn state_change(&mut self, old: State, new: State) {
            println!("state: {:?} -> {:?}", old, new);
        }
        fn receive(&mut self, sequence: u64, data: &[u8]) {
        }
    }

    let addr = "[::1]:40000".parse().unwrap();
    let client_id = ::crypto::random_u64();
    let private_key = Key::generate();
    let token = token::Public::new(
        vec![addr], vec![addr],
        0, TEST_TIMEOUT_SECONDS, client_id, TEST_PROTOCOL_ID,
        0, &private_key,
    ).unwrap();

    let mut client = Client::connect(NoSocket, Cb, 0, &token);

    client.update();

    assert_eq!(client.state(), State::Disconnected {
        err: Error::TokenExpired,
    });
}
