use std::{
    net::SocketAddr,
    io::ErrorKind,
};
use bincode::{serialize, deserialize};
use nalgebra::{Point2, Vector2};
use oni::{
    simulator::Socket,
    reliable::Sequence,
};
use crate::components::Acks;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Client {
    Input(Input),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Joystick {
    pub magnitude: f32,
    pub angle: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InputSample {
    pub server_tick: u16,
    pub local_tick: u8, // and flags?
    pub movement: Joystick,
    pub aim: Joystick,
    pub shot_target: Option<u32>,
    //pub aim_magnitude_compressed: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Server {
    Snapshot {
        frame_seq: Sequence<u16>,
        ack: (Sequence<u8>, Acks<u128>),
        states: Vec<EntityState>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Input {
    pub stick: Vector2<f32>,
    pub rotation: f32,
    pub press_delta: f32,
    pub sequence: Sequence<u8>,
    pub fire: bool,

    pub frame_ack: Sequence<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityState {
    pub entity_id: u16,
    pub position: Point2<f32>,
    //pub velocity: Vector2<f32>,
    pub rotation: f32,

    pub fire: bool,
    pub damage: bool,
}

pub trait Endpoint {
    fn send_ser<T: Serialize>(&self, msg: T, addr: SocketAddr);
    fn recv_de<T: for<'de> Deserialize<'de>>(&self) -> Option<(T, SocketAddr)>;

    fn send_client(&self, m: Client, addr: SocketAddr) { self.send_ser(m, addr) }
    fn recv_client(&self) -> Option<(Client, SocketAddr)> { self.recv_de() }

    fn send_server(&self, m: Server, addr: SocketAddr) { self.send_ser(m, addr) }
    fn recv_server(&self) -> Option<(Server, SocketAddr)> { self.recv_de() }
}

const ENPOINT_BUFFER: usize = 1024;

impl Endpoint for Socket {
    fn send_ser<T: Serialize>(&self, msg: T, addr: SocketAddr) {
        let buf: Vec<u8> = serialize(&msg).unwrap();
        self.send_to(&buf, addr).map(|_| ()).unwrap();
    }

    fn recv_de<T: for<'de> Deserialize<'de>>(&self) -> Option<(T, SocketAddr)> {
        let mut buf = [0u8; ENPOINT_BUFFER];
        match self.recv_from(&mut buf) {
            Ok((len, addr)) => Some((deserialize(&buf[..len]).unwrap(), addr)),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => None,
            Err(e) => panic!("encountered IO error: {}", e),
        }
    }
}
