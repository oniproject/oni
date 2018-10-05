use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use crate::{
    token,
    VERSION_BYTES,
    VERSION,
    crypto::{map_err, new_nonce, Key, MAC_BYTES},
    chacha20poly1305::{encrypt, decrypt},
};
pub use crate::protection::{Protection, NoProtection, ReplayProtection};

// prefix:
//      00000000 - request
//      01000000 - challenge or response
//      10000000 - disconnect or denied
//      11000000 - payload
//
//      00xxxxxx reserved
//      01xxxxxx reserved
//      10xxxxxx reserved
//      11xxxxxx reserved
//
// encrypted packet:
//      [prefix] (1 byte)
//      [sequence] (4 bytes)
//      [body] (variable length according to packet type)
//      [hmac] (16 bytes)

pub const REQUEST: u8 =     0b00;
pub const DISCONNECT: u8 =  0b01; // also denied
pub const CHALLENGE: u8 =   0b10; // also response
pub const PAYLOAD: u8 =     0b11;

pub const HEADER_BYTES: usize = 5;
pub const MIN_PACKET_BYTES: usize = HEADER_BYTES + crate::crypto::MAC_BYTES;
pub const MAX_PACKET_BYTES: usize = 1200;
pub const MAX_PAYLOAD_BYTES: usize = 1100;

#[derive(PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum Kind {
    Request = REQUEST,
    Disconnect = DISCONNECT,
    Challenge = CHALLENGE,
    Payload = PAYLOAD,
}

bitflags! {
    pub struct Allowed: u8 {
        const REQUEST =     1 << REQUEST;
        const DISCONNECT =  1 << DISCONNECT;
        const CHALLENGE =   1 << CHALLENGE;
        const PAYLOAD =     1 << PAYLOAD;

        const SENDING_REQUEST   = Self::DISCONNECT.bits | Self::CHALLENGE.bits;
        const SENDING_RESPONSE  = Self::DISCONNECT.bits | Self::PAYLOAD.bits;
        const CONNECTED         = Self::DISCONNECT.bits | Self::PAYLOAD.bits;
    }
}

impl Allowed {
    #[inline] pub fn payload(self) -> bool    { self.contains(Allowed::PAYLOAD) }
    #[inline] pub fn request(self) -> bool    { self.contains(Allowed::REQUEST) }
    #[inline] pub fn disconnect(self) -> bool { self.contains(Allowed::DISCONNECT) }
    #[inline] pub fn challenge(self) -> bool  { self.contains(Allowed::CHALLENGE) }

    pub fn packet_type(self, k: Kind) -> bool {
        match k {
        Kind::Payload   => self.contains(Allowed::PAYLOAD),
        Kind::Request   => self.contains(Allowed::REQUEST),
        Kind::Disconnect=> self.contains(Allowed::DISCONNECT),
        Kind::Challenge => self.contains(Allowed::CHALLENGE),
        }
    }
}

pub fn is_request_packet(buffer: &[u8]) -> bool {
    buffer[0] == 0
}

pub fn is_encrypted_packet(buffer: &[u8]) -> bool {
    buffer[0] != 0
}


/// 0 (uint8) // prefix byte of zero
/// [version info] (13 bytes)       // "NETCODE 1.02" ASCII with null terminator.
/// [protocol id] (8 bytes)
/// [connect token expire timestamp] (8 bytes)
/// [connect token sequence number] (8 bytes)
/// [encrypted private connect token data] (1024 bytes)
pub struct Request {
    /// connect token expire timestamp
    pub expire: u64,
    /// connect token sequence number
    pub sequence: u64,
    /// encrypted private connect token data
    pub token: [u8; token::Private::BYTES],
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

        let expire = buffer.read_u64::<LE>().ok()?;
        if expire <= current_timestamp {
            return None;
        }

        let sequence = buffer.read_u64::<LE>().ok()?;

        let mut token = [0u8; token::Private::BYTES];
        buffer.read_exact(&mut token[..]).ok()?;

        if token::Private::decrypt(
            &mut token[..], protocol_id, expire, sequence, key,
        ).is_err() {
            println!("!!! decrypt !!!");
            return None;
        }

        Some(Self {
            expire,
            sequence,
            token,
        })
    }

    pub fn write(self, protocol_id: u64) -> [u8; Self::BYTES] {
        Self::write_request(
            protocol_id,
            self.expire,
            self.sequence,
            self.token,
        )
    }

    pub fn write_token(token: &token::Public) -> [u8; Self::BYTES] {
        Self::write_request(
            token.protocol_id,
            token.expire_timestamp,
            token.sequence,
            token.token,
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


const CHALLENGE_INNER_SIZE: usize = 8 + token::Challenge::BYTES;
const RESPONSE_INNER_SIZE: usize = 8 + token::Challenge::BYTES;

pub enum Encrypted {
    Challenge {
        challenge_sequence: u64,
        challenge_data: [u8; token::Challenge::BYTES],
    },
    Disconnect,
    Payload {
        len: usize,
        data: [u8; MAX_PAYLOAD_BYTES],
    },
}

impl Encrypted {
    pub fn read<T>(mut buffer: &[u8], protection: &mut T, key: &Key, protocol: u64, allowed: Allowed) -> Option<Self>
        where T: Protection
    {
        // ignore small packages
        if buffer.len() < HEADER_BYTES + MAC_BYTES {
            return None;
        }

        // extract the packet type and number of sequence bytes from the prefix byte
        let prefix = buffer.read_u8().ok()?;
        let sequence = buffer.read_u32::<LE>().ok()? as u64;

        let kind: Kind = unsafe { std::mem::transmute(prefix >> 6) };

        // filter unexpected packets
        if !allowed.packet_type(kind) {
            return None;
        }

        // replay protection
        if kind == Kind::Payload {
            if protection.packet_already_received(sequence) {
                return None;
            }
        }

        // decrypt the per-packet type data
        let len = buffer.len();
        if len < MAC_BYTES {
            return None;
        }

        let mut encrypted: [u8; MAX_PACKET_BYTES] = unsafe { std::mem::uninitialized() };
        (&mut encrypted[..len]).copy_from_slice(buffer);
        let buffer = &mut encrypted[..len];

        if decrypt(
            buffer,
            &associated(protocol, prefix)[..],
            &new_nonce(sequence),
            key,
            ).is_err()
        {
            return None;
        }

        let len = len - MAC_BYTES;
        let mut buffer = &buffer[..len];

        // process the per-packet type data that was just decrypted
        match kind {
        Kind::Disconnect if len == 0 => Some(Encrypted::Disconnect),
        Kind::Challenge if len == CHALLENGE_INNER_SIZE => {
            Some(Encrypted::Challenge {
                challenge_sequence: buffer.read_u64::<LE>().ok()?,
                challenge_data: read_array_ok!(buffer, token::Challenge::BYTES),
            })
        }
        Kind::Payload if len <= MAX_PAYLOAD_BYTES => {
            let mut data: [u8; MAX_PAYLOAD_BYTES] = unsafe { std::mem::uninitialized() };
            (&mut data[..len]).copy_from_slice(&buffer[..]);
            Some(Encrypted::Payload { data, len })
        }
        _ => None,
        }
    }

    pub fn write(self, buffer: &mut [u8], key: &Key, protocol_id: u64, sequence: u64) -> io::Result<usize> {
        let r = Shim { key, protocol_id, };
        match self {
        Encrypted::Payload { data, len } => {
            r.encrypt_packet(buffer, sequence, PAYLOAD, |mut buffer| {
                buffer.write(&data[..len])?;
                Ok(len)
            })
        }
        Encrypted::Disconnect => r.encrypt_packet(buffer, sequence, DISCONNECT, |_| Ok(0)),
        Encrypted::Challenge { challenge_sequence, challenge_data } =>
            r.encrypt_packet(buffer, sequence, CHALLENGE, |mut buffer| {
                buffer.write_u64::<LE>(challenge_sequence)?;
                buffer.write_all(&challenge_data[..])?;
                Ok(CHALLENGE_INNER_SIZE)
            }),
        }
    }
}

struct Shim<'a> {
    key: &'a Key,
    protocol_id: u64,
}

const ASSOCIATED_DATA_BYTES: usize = VERSION_BYTES+8+1;
impl<'a> Shim<'a> {
    fn associated(&self, prefix_byte: u8) -> [u8; ASSOCIATED_DATA_BYTES] {
        let mut data: [u8; ASSOCIATED_DATA_BYTES] = unsafe { std::mem::uninitialized() };
        {
            let p = &mut data[..];
            p[0..VERSION_BYTES].copy_from_slice(&VERSION[..]);
            for i in 0..8 {
                p[VERSION_BYTES + i] = (self.protocol_id >> i * 8 & 0xFF) as u8;
            }
            p[ASSOCIATED_DATA_BYTES - 1] = prefix_byte;
        }
        data
    }
    fn encrypt_packet<'b, F>(&self, mut buffer: &'b mut [u8], sequence: u64, kind: u8, f: F)
        -> io::Result<usize>
        where F: FnOnce(&'b mut [u8]) -> io::Result<usize>
    {
        let prefix = kind << 6;
        buffer.write_u8(prefix)?;
        buffer.write_u32::<LE>(sequence as u32)?;

        let len = unsafe {
            use std::slice::from_raw_parts_mut;
            f(from_raw_parts_mut(buffer.as_mut_ptr(), buffer.len()))?
        };

        let m = &mut buffer[..len];
        let ad = &associated(self.protocol_id, prefix)[..];
        let nonce = &new_nonce(sequence);
        encrypt(m, ad, nonce, self.key).map_err(map_err)?;
        Ok(HEADER_BYTES + len + MAC_BYTES)
    }
}

pub fn associated(protocol_id: u64, prefix_byte: u8) -> [u8; ASSOCIATED_DATA_BYTES] {
    let mut data: [u8; ASSOCIATED_DATA_BYTES] = unsafe { std::mem::uninitialized() };
    {
        let p = &mut data[..];
        p[0..VERSION_BYTES].copy_from_slice(&VERSION[..]);
        for i in 0..8 {
            p[VERSION_BYTES + i] = (protocol_id >> i * 8 & 0xFF) as u8;
        }
        p[ASSOCIATED_DATA_BYTES - 1] = prefix_byte;
    }
    data
}
