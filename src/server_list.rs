use arrayvec::ArrayVec;
use bincode::{deserialize, serialize_into};
use std::net::SocketAddr;
use crate::token::DATA;

pub const SERVER_LIST_LEN: usize = 28;

#[derive(Default)]
pub struct ServerList {
    servers: ArrayVec<[SocketAddr; SERVER_LIST_LEN]>,
}

impl ServerList {
    pub fn new() -> Self {
        Self { servers: ArrayVec::new() }
    }

    pub fn push(&mut self, addr: SocketAddr) -> Result<(), SocketAddr> {
        self.servers.try_push(addr).map_err(|err| err.element())
    }

    pub fn contains(&self, addr: &SocketAddr) -> bool {
        self.servers.iter().any(|a| a == addr)
    }

    pub fn as_slice(&self) -> &[SocketAddr] {
        self.servers.as_slice()
    }

    pub fn deserialize(data: &[u8; DATA]) -> Result<Self, ()> {
        Ok(Self { servers: deserialize(&data[..]).map_err(|_| ())? })
    }

    pub fn serialize(&self) -> Option<[u8; DATA]> {
        self.serialize_noalloc(&mut Vec::new())
    }

    pub fn serialize_noalloc(&self, mut temp: &mut Vec<u8>) -> Option<[u8; DATA]> {
        temp.clear();
        serialize_into(&mut temp, &self.servers).ok()?;
        if temp.len() > DATA {
            None
        } else {
            let mut data = [0u8; DATA];
            data[..temp.len()].copy_from_slice(temp.as_slice());
            Some(data)
        }
    }
}

#[test]
fn server_arrayvec() {
    let addr = "[::1]:32".parse().unwrap();
    let mut arr: ArrayVec<[SocketAddr; SERVER_LIST_LEN]> = ArrayVec::new();
    for _ in 0..SERVER_LIST_LEN {
        arr.push(addr);
    }

    let mut v = Vec::new();
    serialize_into(&mut v, &arr).unwrap();
    // 32 => 712
    // 16 => 360
    // 12 => 272
    // 2 => 52
    // 1 => 30
    // 0 => 8
    //
    //  8 bytes overhead
    //
    // 22 bytes per ip
    //
    // ip6:port is 8*2+2 = 18 bytes (19 bytes)
    // ip4:port is 4 + 2 = 6 (7 bytes)
    assert_eq!(v.len(), 624); // wtf?

    v.clear();

    serialize_into(&mut v, &addr).unwrap();

    assert_eq!(v.len(), 22); // wtf?

}
