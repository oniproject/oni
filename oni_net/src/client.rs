use std::{
    net::{UdpSocket, SocketAddr},
    time::{Duration, Instant},
};

use crate::{
    PACKET_SEND_DELTA,
    NUM_DISCONNECT_PACKETS,
    socket::Socket,
    token::{Challenge, Public},
    packet::{
        Allowed,
        Request,
        Encrypted,
        MAX_PACKET,
        ReplayProtection,
    },
};

pub enum Event<'a> {
    Connected,
    Disconnected(Error),
    Packet(&'a [u8]),
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum Error {
    TokenExpired,
    TimedOut,
    ResponseTimedOut,
    RequestTimedOut,
    Denied,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum State {
    SendingRequest,
    SendingResponse,
    Connected,
    Disconnected(Error),
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

pub struct Client {
    socket: UdpSocket,

    state: State,

    time: Instant,
    connect_start_time: Instant,
    last_send: Instant,
    last_recv: Instant,

    sequence: u64,

    addr: SocketAddr,
    replay_protection: ReplayProtection,
    challenge_token: (u64, [u8; Challenge::BYTES]),

    token: Public,
    protocol: u64,
    token_timeout: Duration,
    token_expire: Duration,
}

impl Client {
    pub fn new(protocol: u64, token: Public, addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;

        let time = Instant::now();
        let token_expire = Duration::from_secs(token.expire - token.create);
        let token_timeout = Duration::from_secs(token.timeout.into());
        Ok(Self {
            socket,

            state: State::SendingRequest,

            time,
            connect_start_time: time,
            last_send: time - Duration::from_secs(1),
            last_recv: time,

            sequence: 0,

            addr,
            replay_protection: ReplayProtection::default(),
            challenge_token: (0, [0u8; Challenge::BYTES]),

            token,
            protocol,
            token_timeout,
            token_expire,
        })
    }

    pub fn update<F>(&mut self, mut callback: F)
        where F: FnMut(Event)
    {
        self.time = Instant::now();

        if let Err(err) = self.receive_packets(&mut callback) {
            return self.disconnect(err, &mut callback);
        }

        if self.last_send + PACKET_SEND_DELTA < self.time {
            match self.state {
                State::SendingRequest => self.send_request(),
                State::SendingResponse => self.send_challenge(),
                State::Connected => self.send_packet(Encrypted::keep_alive()),
                _ => (),
            }
        }

        let expire = self.time - self.connect_start_time >= self.token_expire;
        let timedout = self.last_recv + self.token_timeout < self.time;

        if self.state.is_connecting() && expire {
            return self.disconnect(Error::TokenExpired, &mut callback);
        }

        if timedout {
            let err = match self.state {
                State::SendingRequest => Error::RequestTimedOut,
                State::SendingResponse => Error::ResponseTimedOut,
                State::Connected => Error::TimedOut,
                _ => return,
            };
            self.disconnect(err, &mut callback)
        }
    }

    pub fn close<F>(&mut self, mut callback: F)
        where F: FnMut(Event)
    {
        for _ in 0..NUM_DISCONNECT_PACKETS {
            self.send_packet(Encrypted::Disconnect);
        }
        self.disconnect(Error::Closed, &mut callback);
    }

    pub fn server_addr(&self) -> SocketAddr { self.addr }
    pub fn state(&self) -> State { self.state }

    pub fn send(&mut self, payload: &[u8]) {
        if self.state != State::Connected {
            return;
        }
        let packet = Encrypted::payload(payload)
            .expect("payload length must less or equal MAX_PAYLOAD");
        self.send_packet(packet);
    }

    fn send_challenge(&mut self) {
        self.send_packet(Encrypted::Challenge {
            seq: self.challenge_token.0,
            data: self.challenge_token.1,
        });
    }

    fn send_request(&mut self) {
        self.socket.send_to(&Request::write_token(&self.token)[..], self.addr);
        self.last_send = self.time;
    }

    fn send_packet(&mut self, packet: Encrypted) {
        let sequence = self.sequence;
        self.sequence += 1;

        let mut data = [0u8; MAX_PACKET];
        let bytes = packet.write(
            &mut data[..],
            &self.token.client_key,
            self.token.protocol_id,
            sequence,
        ).unwrap();

        assert!(bytes <= MAX_PACKET);
        self.socket.send_to(&data[..bytes], self.addr);
        self.last_send = self.time;
    }

    fn disconnect<F>(&mut self, err: Error, callback: &mut F)
        where F: FnMut(Event)
    {
        if let State::Disconnected { .. } = self.state {
            return;
        }
        callback(Event::Disconnected(err));
        self.state = State::Disconnected(err);
    }

    fn receive_packets<F>(&mut self, callback: &mut F) -> Result<(), Error>
        where F: FnMut(Event)
    {
        let mut buf = [0u8; MAX_PACKET];
        while let Ok((bytes, from)) = self.socket.recv_from(&mut buf[..]) {
            if from != self.addr {
                continue;
            }

            let r = Encrypted::read(
                &mut buf[..bytes],
                &mut self.replay_protection,
                &self.token.server_key,
                self.token.protocol_id,
                match self.state {
                    State::Connected =>       Allowed::CONNECTED,
                    State::SendingResponse => Allowed::SENDING_RESPONSE,
                    State::SendingRequest =>  Allowed::SENDING_REQUEST,
                    _ => break,
                },
            );

            let packet = if let Some(p) = r { p } else { continue };

            match (self.state, packet) {
                (State::Connected, Encrypted::Payload { len, data }) => {
                    if len != 0 {
                        callback(Event::Packet(&data[..len]));
                    }
                }

                (State::SendingRequest, Encrypted::Challenge { seq, data }) => {
                    self.challenge_token = (seq, data);
                    self.state = State::SendingResponse;
                }

                (State::SendingResponse, Encrypted::Payload { len, data }) => {
                    callback(Event::Connected);
                    self.state = State::Connected;
                    if len != 0 {
                        callback(Event::Packet(&data[..len]));
                    }
                }

                (State::Connected, Encrypted::Disconnect) => return Err(Error::Closed),
                (State::SendingResponse, Encrypted::Disconnect) => return Err(Error::Denied),
                (State::SendingRequest, Encrypted::Disconnect) => return Err(Error::Denied),

                _ => unreachable!(),
            }

            self.last_recv = self.time;
        }
        Ok(())
    }
}
