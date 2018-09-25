use std::{
    net::SocketAddr,
    io::ErrorKind,
};
use bincode::{serialize, deserialize};
use nalgebra::{wrap, UnitComplex, Point2, Vector2};
use oni::{
    simulator::Socket,
    reliable::Sequence,
};
use crate::components::Acks;
use serde::{
    Serialize, Deserialize,
    Serializer, Deserializer,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Client {
    Start,
    Input(Input),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Input {
    pub stick: [f32; 2],
    pub rotation: f32,
    pub press_delta: f32,
    pub sequence: Sequence<u8>,
    pub fire: bool,
    pub frame_ack: Sequence<u16>,
}

/*
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Joystick {
    pub magnitude: f32,
    pub angle: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InputSample {
    pub server_tick: usize,
    pub player_tick: usize, // and flags?
    pub mov: Joystick,
    pub aim: Joystick,
    pub shot_target: Option<u32>,
    //pub aim_magnitude_compressed: f32,
}
*/

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Server {
    Snapshot {
        me_id: u8,
        frame_seq: Sequence<u16>,
        ack: (Sequence<u8>, Acks<u128>),
        states: Vec<EntityState>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntityState {
    pub entity_id: u8,
    pub position: [f32; 2],
    //pub velocity: Vector2<f32>,
    pub rotation: Angle16,
    pub flags: EntityStateFlags,
}

impl EntityState {
    pub fn position(&self) -> Point2<f32> { Point2::from_coordinates(self.position.into()) }
    pub fn fire(&self) -> bool { self.flags.contains(EntityStateFlags::FIRE) }
    pub fn damage(&self) -> bool { self.flags.contains(EntityStateFlags::DAMAGE) }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct EntityStateFlags: u8 {
        const FIRE   = 0b00000001;
        const DAMAGE = 0b00000010;
    }
}

#[derive(Clone, Debug)]
pub struct Angle16(f32);

impl Angle16 {
    pub fn angle(&self) -> UnitComplex<f32> {
        UnitComplex::from_angle(self.0)
    }
}

impl From<f32> for Angle16 {
    fn from(a: f32) -> Self { Angle16(a) }
}

impl Into<f32> for Angle16 {
    fn into(self) -> f32 { self.0 }
}

const PI2: f32 = std::f32::consts::PI * 2.0;

impl Serialize for Angle16 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let angle = wrap(self.0, 0.0, PI2);
        let angle = (angle / PI2) * (u16::max_value() as f32);
        serializer.serialize_u16(angle as u16)
    }
}

impl<'de> Deserialize<'de> for Angle16 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let angle = u16::deserialize(deserializer)?;
        let angle = (angle as f32) / (u16::max_value() as f32) * PI2;
        Ok(Angle16(angle))
    }
}

#[derive(Clone, Debug)]
pub struct Positon16([f32; 2]);

impl Positon16 {
    pub fn vector(&self) -> Vector2<f32> {
        self.0.into()
    }
}

/*
impl From<f32> for Angle16 {
    fn from(a: f32) -> Self { Angle16(a) }
}

impl Into<f32> for Angle16 {
    fn into(self) -> f32 { self.0 }
}

const PI2: f32 = std::f32::consts::PI * 2.0;

impl Serialize for Angle16 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let angle = wrap(self.0, 0.0, PI2);
        let angle = (angle / PI2) * (u16::max_value() as f32);
        serializer.serialize_u16(angle as u16)
    }
}

impl<'de> Deserialize<'de> for Angle16 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let angle = u16::deserialize(deserializer)?;
        let angle = (angle as f32) / (u16::max_value() as f32) * PI2;
        Ok(Angle16(angle))
    }
}
*/

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
