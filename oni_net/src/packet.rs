use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use crate::{
    token::{Challenge, Private, Public},
    VERSION_BYTES,
    VERSION,
    crypto::{map_err, new_nonce, Key, MAC_BYTES},
    sodium::{seal, open},
};
pub use crate::protection::{Protection, ChallengeFilter, ChallengeOrDisconnectFilter, ReplayProtection};

// prefix:
//      00000000 - request
//      00xxxxxx - invalid packet
//      01ssssss - challenge or response
//      10ssssss - disconnect or denied
//      11ssssss - payload
//
//      s - high bits of sequence
//
// encrypted packet:
//      [prefix & sequence] (4 bytes)
//      [ciphertext] (variable length according to packet type)
//      [mac] (16 bytes)

pub const REQUEST: u8 =     0b00;
pub const DISCONNECT: u8 =  0b01; // also denied
pub const CHALLENGE: u8 =   0b10; // also response
pub const PAYLOAD: u8 =     0b11;

pub const HEADER_SIZE: usize = 4;
pub const MIN_PACKET: usize = HEADER_SIZE + MAC_BYTES;
pub const MAX_PACKET: usize = 1200;
pub const MAX_PAYLOAD: usize = MAX_PACKET - MIN_PACKET;
pub const CHALLENGE_INNER_SIZE: usize = 8 + Challenge::BYTES;
pub const CHALLENGE_PACKET_BYTES: usize = HEADER_SIZE + MAC_BYTES + CHALLENGE_INNER_SIZE;

const PREFIX_SHIFT: u32 = 30;
const PREFIX_MASK: u32 = 0xC0000000;
const SEQUENCE_MASK: u32 = 0x3FFFFFFF;

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

/// 0 (uint8) // prefix byte of zero
/// [version info] (13 bytes)       // "NETCODE 1.02" ASCII with null terminator.
/// [protocol id] (8 bytes)
/// [connect token expire timestamp] (8 bytes)
/// [connect token nonce] (24 bytes)
/// [sealed private connect token data] (1024 bytes)
pub struct Request {
    /// connect token expire timestamp
    pub expire: u64,
    /// sealed private connect token data
    pub token: [u8; Private::BYTES],
}

impl Request {
    pub const BYTES: usize = 1 + VERSION_BYTES + 8 * 2 + 24 + Private::BYTES;

    pub fn read(buffer: &[u8; Self::BYTES], current_timestamp: u64, current_protocol_id: u64, key: &Key) -> Option<Self> {
        let mut buffer = &buffer[..];

        let prefix = buffer.read_u8().ok()?;
        if prefix != 0 { return None; }

        let mut version = [0u8; VERSION_BYTES];
        buffer.read_exact(&mut version[..]).ok()?;
        let protocol_id = buffer.read_u64::<LE>().ok()?;
        let expire = buffer.read_u64::<LE>().ok()?;
        let mut nonce = [0u8; 24];
        buffer.read_exact(&mut nonce[..]).ok()?;

        if version != VERSION { return None; }
        if protocol_id != current_protocol_id { return None; }
        if expire <= current_timestamp { return None; }

        let mut token = [0u8; Private::BYTES];
        buffer.read_exact(&mut token[..]).ok()?;

        if Private::open(&mut token[..], protocol_id, expire, &nonce, key).is_err() {
            println!("!!! open !!!");
            return None;
        }

        Some(Self {
            expire,
            token,
        })
    }

    pub fn write_token(token: &Public) -> [u8; Self::BYTES] {
        Self::write_request(
            token.protocol_id,
            token.expire,
            token.nonce,
            token.token,
        )
    }

    pub fn write_request(
        protocol_id: u64,
        expire_timestamp: u64,
        nonce: [u8; 24],
        private_data: [u8; Private::BYTES],
    ) -> [u8; Self::BYTES] {
        let mut buffer: [u8; Self::BYTES] = unsafe { std::mem::uninitialized() };
        let mut p = &mut buffer[..];
        p.write_u8(REQUEST).unwrap();
        p.write_all(&VERSION[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u64::<LE>(expire_timestamp).unwrap();
        p.write_all(&nonce[..]).unwrap();
        p.write_all(&private_data[..]).unwrap();
        buffer
    }
}

pub enum Encrypted {
    Challenge {
        seq: u64,
        data: [u8; Challenge::BYTES],
    },
    Disconnect,
    Payload {
        len: usize,
        data: [u8; MAX_PAYLOAD],
    },
}

impl Encrypted {
    pub fn keep_alive() -> Self {
        Encrypted::Payload {
            len: 0,
            data: unsafe { std::mem::uninitialized() },
        }
    }

    pub fn payload(payload: &[u8]) -> Option<Self> {
        let len = payload.len();
        if len > MAX_PAYLOAD {
            None
        } else {
            let mut data: [u8; MAX_PAYLOAD] = unsafe { std::mem::uninitialized() };
            (&mut data[..len]).copy_from_slice(&payload[..len]);
            Some(Encrypted::Payload {
                len,
                data,
            })
        }
    }

    pub fn read_challenge(buf: &mut [u8], key: &Key, protocol: u64, ckey: &Key) -> Option<Challenge> {
        let (kind, mut buf) = open_packet(buf, |_, _| true, key, protocol, Allowed::CHALLENGE)?;
        if kind == Kind::Challenge {
            let seq = buf.read_u64::<LE>().ok()?;
            let data = read_array_ok!(buf, Challenge::BYTES);
            Challenge::read(data, seq, ckey).ok()
        } else {
            None
        }
    }

    pub fn read<T>(buffer: &mut [u8], protection: &mut T, key: &Key, protocol: u64, allowed: Allowed) -> Option<Self>
        where T: Protection
    {
        let (kind, mut buffer) = open_packet(buffer, |kind, sequence| {
            match kind {
            // replay protection
            Kind::Payload | Kind::Disconnect =>
                !protection.packet_already_received(sequence),
            _ => true,
            }
        }, key, protocol, allowed)?;

        // process the per-packet type data that was just opened
        match kind {
        Kind::Disconnect => Some(Encrypted::Disconnect),
        Kind::Challenge => Some(Encrypted::Challenge {
            seq: buffer.read_u64::<LE>().ok()?,
            data: read_array_ok!(buffer, Challenge::BYTES),
        }),
        Kind::Payload => {
            let len = buffer.len();
            let mut data: [u8; MAX_PAYLOAD] = unsafe { std::mem::uninitialized() };
            (&mut data[..len]).copy_from_slice(&buffer[..]);
            Some(Encrypted::Payload { data, len })
        }
        _ => None,
        }
    }

    pub fn write(self, buffer: &mut [u8], key: &Key, protocol_id: u64, sequence: u64) -> io::Result<usize> {
        let (mut header, mut body) = buffer.split_at_mut(HEADER_SIZE);

        let (prefix, len) = match self {
            Encrypted::Payload { len, data } => {
                body.write_all(&data[..len])?;
                (PAYLOAD, len)
            }
            Encrypted::Disconnect => (DISCONNECT, 0),
            Encrypted::Challenge { seq, data } => {
                body.write_u64::<LE>(seq)?;
                body.write_all(&data[..])?;
                (CHALLENGE, CHALLENGE_INNER_SIZE)
            }
        };

        let a = (sequence as u32) & SEQUENCE_MASK;
        let b = (prefix as u32) << PREFIX_SHIFT;

        header.write_u32::<LE>(a | b)?;

        let m = &mut buffer[HEADER_SIZE..HEADER_SIZE+len];
        let ad = &associated(protocol_id, prefix)[..];
        let nonce = &new_nonce(sequence);
        seal(m, Some(ad), nonce, key).map_err(map_err)?;
        Ok(HEADER_SIZE + len + MAC_BYTES)
    }
}

const ASSOCIATED_DATA_BYTES: usize = VERSION_BYTES+8+1;
fn associated(protocol_id: u64, prefix_byte: u8) -> [u8; ASSOCIATED_DATA_BYTES] {
    let mut p: [u8; ASSOCIATED_DATA_BYTES] = unsafe { std::mem::uninitialized() };
    p[0..VERSION_BYTES].copy_from_slice(&VERSION[..]);
    for i in 0..8 {
        p[VERSION_BYTES + i] = (protocol_id >> i * 8 & 0xFF) as u8;
    }
    p[ASSOCIATED_DATA_BYTES - 1] = prefix_byte;
    p
}

fn open_packet<'a, F>(buffer: &'a mut [u8], filter: F, key: &Key, protocol: u64, allowed: Allowed) -> Option<(Kind, &'a [u8])>
    where F: FnOnce(Kind, u64) -> bool
{
    let buf_len = buffer.len();
    // ignore small or large packages
    if buf_len < MIN_PACKET || buf_len > MAX_PACKET {
        return None;
    }

    let (header, body) = buffer.split_at_mut(HEADER_SIZE);
    let mut header = &header[..];

    // extract the packet type and number of sequence bytes from the prefix byte
    let h = header.read_u32::<LE>().ok()?;

    let prefix = (h >> PREFIX_SHIFT) as u8;
    let sequence = (h & SEQUENCE_MASK) as u64;

    let kind: Kind = unsafe { std::mem::transmute(prefix) };

    // filter unexpected packets
    if kind == Kind::Request { return None; }
    if !allowed.packet_type(kind) { return None; }
    if kind == Kind::Disconnect && buf_len != MIN_PACKET { return None; }
    if kind == Kind::Challenge && buf_len != CHALLENGE_PACKET_BYTES { return None; }
    if !filter(kind, sequence) { return None; }

    // open the per-packet type data
    let ad = &associated(protocol, prefix)[..];
    if open(body, Some(ad), &new_nonce(sequence), key).is_err() {
        return None;
    }

    let len = body.len() - MAC_BYTES;
    Some((kind, &body[..len]))
}
