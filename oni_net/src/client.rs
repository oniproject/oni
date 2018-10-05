pub const NUM_DISCONNECT_PACKETS: usize = 10;

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use crate::{
    PACKET_SEND_DELTA,
    Socket,
    token::{Challenge, Private, Public},
    packet::{
        Allowed, Request,
        Encrypted,
        MAX_PACKET_BYTES,
        MAX_PAYLOAD_BYTES,
        ReplayProtection,
    },
    crypto::{Key, keygen},
};

pub trait Callback {
    fn state_change(&mut self, old: State, new: State);
    fn receive(&mut self, data: &[u8]);
}

pub enum Event<'a> {
    State {
        old: State,
        new: State,
    },
    Payload(&'a [u8]),
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

    addr: SocketAddr,
    replay_protection: ReplayProtection,
    challenge_token_sequence: u64,
    challenge_token_data: [u8; Challenge::BYTES],

    /*
    protocol_id: u64,
    read_key: Key,
    write_key: Key,
    */

    token: Public,

    token_timeout: Duration,
    token_expire: Duration,

    /*
    token_private_data: [u8; Private::BYTES],
    token_expire_timestamp: u64,
    token_create_timestamp: u64,
    token_sequence: u64,
    */
}

impl<S: Socket, C: Callback> Client<S, C> {
    pub fn connect(socket: S, callback: C, addr: SocketAddr, token: Public) -> Self {
        let time = Instant::now();
        let token_expire = Duration::from_secs(token.expire_timestamp - token.create_timestamp);
        let token_timeout = Duration::from_secs(token.timeout_seconds.into());
        Self {
            socket,
            callback,

            state: State::SendingRequest,

            time,
            connect_start_time: time,
            last_send: time - Duration::from_secs(1),
            last_recv: time,

            sequence: 0,

            addr,
            replay_protection: ReplayProtection::default(),
            challenge_token_sequence: 0,
            challenge_token_data: [0u8; Challenge::BYTES],

            token,
            token_timeout,
            token_expire,

            /*
            protocol_id: token.protocol_id,
            read_key: token.server_to_client_key.clone(),
            write_key: token.client_to_server_key.clone(),
            token_timeout: ,

            token_private_data: token.token,
            token_expire_timestamp: token.expire_timestamp,
            token_create_timestamp: token.create_timestamp,
            token_sequence: token.sequence,

            token_expire: ,
            */
        }
    }

    pub fn update(&mut self) {
        self.time = Instant::now();

        if let Err(err) = self.receive_packets() {
            return self.disconnect(err);
        }

        if self.last_send + PACKET_SEND_DELTA < self.time {
            match self.state {
                State::SendingRequest => self.send_request(),
                State::SendingResponse => self.send_packet(Encrypted::Challenge {
                    challenge_sequence: self.challenge_token_sequence,
                    challenge_data: self.challenge_token_data,
                }),
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

    pub fn server_addr(&self) -> SocketAddr { self.addr }

    pub fn state(&self) -> State { self.state }

    pub fn send_payload(&mut self, payload: &[u8]) {
        assert!(payload.len() <= MAX_PAYLOAD_BYTES);
        if self.state != State::Connected {
            return;
        }
        let (data, len) = array_from_slice_uninitialized!(payload, MAX_PAYLOAD_BYTES);
        self.send_packet(Encrypted::Payload {
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

    fn send(&self, data: &[u8]) {
        self.socket.send(self.addr, data);
    }

    fn send_packet(&mut self, packet: Encrypted) {
        let sequence = self.sequence;
        self.sequence += 1;

        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(
            &mut data[..],
            &self.token.client_to_server_key,
            self.token.protocol_id,
            sequence,
        ).unwrap();

        assert!(bytes <= MAX_PACKET_BYTES);
        self.send(&data[..bytes]);
        self.last_send = self.time;
    }

    fn send_request(&mut self) {
        let data = Request::write_request(
            self.token.protocol_id,
            self.token.expire_timestamp,
            self.token.sequence,
            self.token.token,
        );
        self.send(&data[..]);
        self.last_send = self.time;
    }

    fn receive_packets(&mut self) -> Result<(), Error> {
        let mut buf = [0u8; MAX_PACKET_BYTES];
        while let Some((bytes, from)) = self.socket.recv(&mut buf[..]) {
            if from != self.addr {
                continue;
            }

            let r = Encrypted::read(
                &mut buf[..bytes],
                &mut self.replay_protection,
                &self.token.server_to_client_key,
                self.token.protocol_id,
                match self.state {
                    State::Connected =>       Allowed::CONNECTED,
                    State::SendingResponse => Allowed::SENDING_RESPONSE,
                    State::SendingRequest =>  Allowed::SENDING_REQUEST,
                    _ => break,
                },
            );

            let packet = if let Some(p) = r {
                p
            } else {
                continue
            };

            match (self.state, packet) {
                (State::Connected, Encrypted::Payload { len, data }) => {
                    self.callback.receive(&data[..len]);
                }
                (State::Connected, Encrypted::Disconnect) => return Err(Error::Disconnected),

                (State::SendingRequest, Encrypted::Disconnect) => return Err(Error::Denied),
                (State::SendingRequest, Encrypted::Challenge { challenge_sequence, challenge_data }) => {
                    self.challenge_token_sequence = challenge_sequence;
                    self.challenge_token_data = challenge_data;

                    self.callback.state_change(self.state, State::SendingResponse);
                    self.state = State::SendingResponse;
                }

                (State::SendingResponse, Encrypted::Disconnect) => return Err(Error::Denied),
                (State::SendingResponse, Encrypted::Payload { len, data }) => {
                    self.callback.state_change(self.state, State::Connected);
                    self.state = State::Connected;
                    self.callback.receive(&data[..len]);
                }
                _ => unreachable!(),
            }

            self.last_recv = self.time;
        }
        Ok(())
    }
}
