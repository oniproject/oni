use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

use crate::{
    token,
    VERSION_BYTES,
    VERSION,
    crypto::Key,
    packet::REQUEST,
};

pub struct Request {
    pub sequence: u64,
    pub version: [u8; VERSION_BYTES],
    pub protocol_id: u64,
    pub expire_timestamp: u64,
    pub private_data: [u8; token::Private::BYTES],
}

impl Request {
    pub const BYTES: usize = 1 + VERSION_BYTES + 8 * 3 + token::Private::BYTES;

    pub fn read(mut buffer: &[u8], current_timestamp: u64, current_protocol_id: u64, key: &Key) -> Option<Self> {
        if buffer.len() != Self::BYTES {
            return None;
        }

        if buffer.read_u8().ok()? != 0 {
            return None;
        }

        let mut version = [0u8; VERSION_BYTES];
        buffer.read_exact(&mut version[..]).ok()?;
        if version != VERSION {
            return None;
        }

        let protocol_id = buffer.read_u64::<LE>().ok()?;
        if protocol_id != current_protocol_id {
            return None;
        }

        let expire_timestamp = buffer.read_u64::<LE>().ok()?;
        if expire_timestamp <= current_timestamp {
            return None;
        }

        let sequence = buffer.read_u64::<LE>().ok()?;

        let mut private_data = [0u8; token::Private::BYTES];
        buffer.read_exact(&mut private_data[..]).ok()?;

        if token::Private::decrypt(
            &mut private_data[..], protocol_id, expire_timestamp, sequence, key,
        ).is_err() {
            println!("!!! decrypt !!!");
            return None;
        }

        Some(Self {
            version,
            protocol_id,
            expire_timestamp,
            sequence,
            private_data,
        })
    }

    pub fn write(self) -> [u8; Self::BYTES] {
        Self::write_request(
            self.protocol_id,
            self.expire_timestamp,
            self.sequence,
            self.private_data,
        )
    }

    pub fn write_token(token: &token::Public) -> [u8; Self::BYTES] {
        Self::write_request(
            token.protocol_id,
            token.expire_timestamp,
            token.sequence,
            token.private_data,
        )
    }

    pub fn write_request(
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        private_data: [u8; token::Private::BYTES],
    ) -> [u8; Self::BYTES] {
        let mut buffer: [u8; Self::BYTES] =
            unsafe { std::mem::uninitialized() };
        {
            let mut buffer = &mut buffer[..];
            buffer.write_u8(REQUEST).unwrap();
            buffer.write_all(&VERSION[..]).unwrap();
            buffer.write_u64::<LE>(protocol_id).unwrap();
            buffer.write_u64::<LE>(expire_timestamp).unwrap();
            buffer.write_u64::<LE>(sequence).unwrap();
            buffer.write_all(&private_data[..]).unwrap();
        }
        buffer
    }
}
