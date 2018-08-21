use std::net::SocketAddr;
use std::io::ErrorKind;
use bincode::{serialize, deserialize};
use nalgebra::{Point2, Vector2};
use oni::simulator::Socket;
use crate::sequence::Sequence;
use crate::components::Acks;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Input {
    pub stick: Vector2<f32>,
    pub rotation: f32,
    pub press_time: f32,
    pub sequence: Sequence<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldState {
    pub ack: (Sequence<u8>, Acks<u128>),
    pub states: Vec<EntityState>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityState {
    pub entity_id: u16,
    pub position: Point2<f32>,
    //pub velocity: Vector2<f32>,
    pub rotation: f32,
}

pub trait Endpoint {
    fn send_ser<T: Serialize>(&self, msg: T, addr: SocketAddr);
    fn recv_de<T: for<'de> Deserialize<'de>>(&self) -> Option<(T, SocketAddr)>;

    fn send_input(&self, input: Input, addr: SocketAddr) {
        self.send_ser(input, addr)
    }
    fn recv_input(&self) -> Option<(Input, SocketAddr)> {
        self.recv_de()
    }

    fn send_world(&self, world: WorldState, addr: SocketAddr) {
        self.send_ser(world, addr)
    }
    fn recv_world(&self) -> Option<(WorldState, SocketAddr)> {
        self.recv_de()
    }
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
