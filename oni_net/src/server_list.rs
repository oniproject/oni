use arrayvec::ArrayVec;
use bincode::{deserialize, serialize_into};
use std::net::SocketAddr;
use crate::token::DATA;

pub const SERVER_LIST_LEN: usize = 32;

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
