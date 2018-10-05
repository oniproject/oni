use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use crate::{
    token::{Challenge, Private, Public},
    VERSION_BYTES,
    VERSION,
    crypto::{map_err, new_nonce, Key, MAC_BYTES},
    chacha20poly1305::{encrypt, decrypt},
};
pub use crate::protection::{Protection, ChallengeFilter, ChallengeOrDisconnectFilter, ReplayProtection};

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
pub const MIN_PACKET_BYTES: usize = HEADER_BYTES + MAC_BYTES;
pub const MAX_PACKET_BYTES: usize = 1200;
pub const MAX_PAYLOAD_BYTES: usize = MAX_PACKET_BYTES - MIN_PACKET_BYTES;
pub const CHALLENGE_INNER_SIZE: usize = 8 + Challenge::BYTES;
pub const CHALLENGE_PACKET_BYTES: usize = HEADER_BYTES + MAC_BYTES + CHALLENGE_INNER_SIZE;

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
/// [connect token sequence number] (8 bytes)
/// [encrypted private connect token data] (1024 bytes)
pub struct Request {
    /// connect token expire timestamp
    pub expire: u64,
    /// connect token sequence number
    pub sequence: u64,
    /// encrypted private connect token data
    pub token: [u8; Private::BYTES],
}

impl Request {
    pub const BYTES: usize = 1 + VERSION_BYTES + 8 * 3 + Private::BYTES;

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

        let mut token = [0u8; Private::BYTES];
        buffer.read_exact(&mut token[..]).ok()?;

        if Private::decrypt(&mut token[..], protocol_id, expire, sequence, key).is_err() {
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

    pub fn write_token(token: &Public) -> [u8; Self::BYTES] {
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
        private_data: [u8; Private::BYTES],
    ) -> [u8; Self::BYTES] {
        let mut buffer: [u8; Self::BYTES] = unsafe { std::mem::uninitialized() };
        let mut p = &mut buffer[..];
        p.write_u8(REQUEST).unwrap();
        p.write_all(&VERSION[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u64::<LE>(expire_timestamp).unwrap();
        p.write_u64::<LE>(sequence).unwrap();
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
        data: [u8; MAX_PAYLOAD_BYTES],
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
        if len > MAX_PAYLOAD_BYTES {
            None
        } else {
            let mut data: [u8; MAX_PAYLOAD_BYTES] = unsafe { std::mem::uninitialized() };
            (&mut data[..len]).copy_from_slice(&payload[..len]);
            Some(Encrypted::Payload {
                len,
                data,
            })
        }
    }

    pub fn read_challenge(buf: &mut [u8], key: &Key, protocol: u64, ckey: &Key) -> Option<Challenge> {
        let (kind, mut buf) = Self::decrypt(buf, &mut ChallengeFilter, key, protocol, Allowed::CHALLENGE)?;
        if kind == Kind::Challenge {
            let seq = buf.read_u64::<LE>().ok()?;
            let mut data = read_array_ok!(buf, Challenge::BYTES);
            Challenge::decrypt_and_read(&mut data, seq, ckey).ok()
        } else {
            None
        }
    }

    pub fn read_intermediate(buf: &mut [u8], key: &Key, protocol: u64) -> Option<Encrypted> {
        let (kind, mut buf) = Self::decrypt(buf, &mut ChallengeOrDisconnectFilter, key, protocol, Allowed::CHALLENGE)?;
        match kind {
        Kind::Disconnect => Some(Encrypted::Disconnect),
        Kind::Challenge => Some(Encrypted::Challenge {
            seq: buf.read_u64::<LE>().ok()?,
            data: read_array_ok!(buf, Challenge::BYTES),
        }),
        _ => None,
        }
    }

    fn decrypt<'a, T>(mut buffer: &'a mut [u8], protection: &mut T, key: &Key, protocol: u64, allowed: Allowed) -> Option<(Kind, &'a [u8])>
        where T: Protection
    {
        let buf_len = buffer.len();
        // ignore small or large packages
        if buf_len < MIN_PACKET_BYTES || buf_len > MAX_PACKET_BYTES {
            return None;
        }

        let (header, body) = buffer.split_at_mut(HEADER_BYTES);
        let mut header = &header[..];

        // extract the packet type and number of sequence bytes from the prefix byte
        let prefix = header.read_u8().ok()?;
        let sequence = header.read_u32::<LE>().ok()? as u64;

        let kind: Kind = unsafe { std::mem::transmute(prefix >> 6) };

        // filter unexpected packets
        if !allowed.packet_type(kind) {
            return None;
        }

        match kind {
        // replay protection
        Kind::Disconnect if buf_len != MIN_PACKET_BYTES => return None,
        Kind::Challenge if buf_len != CHALLENGE_PACKET_BYTES => return None,
        Kind::Request => panic!("u mad"),
        _ => (),
        }

        match kind {
        // replay protection
        Kind::Payload | Kind::Disconnect => {
            if protection.packet_already_received(sequence) {
                return None;
            }
        }
        _ => (),
        }

        // decrypt the per-packet type data
        if decrypt(
            body,
            &associated(protocol, prefix)[..],
            &new_nonce(sequence),
            key,
            ).is_err()
        {
            return None;
        }

        let len = body.len() - MAC_BYTES;
        Some((kind, &body[..len]))
    }

    pub fn read<T>(mut buffer: &mut [u8], protection: &mut T, key: &Key, protocol: u64, allowed: Allowed) -> Option<Self>
        where T: Protection
    {
        let (kind, mut buffer) = Self::decrypt(buffer, protection, key, protocol, allowed)?;
        let len = buffer.len();

        // process the per-packet type data that was just decrypted
        match kind {
        Kind::Disconnect => Some(Encrypted::Disconnect),
        Kind::Challenge => {
            Some(Encrypted::Challenge {
                seq: buffer.read_u64::<LE>().ok()?,
                data: read_array_ok!(buffer, Challenge::BYTES),
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
        let (mut header, mut body) = buffer.split_at_mut(HEADER_BYTES);

        let (prefix, len) = match self {
            Encrypted::Payload { len, data } => {
                body.write_all(&data[..len])?;
                (PAYLOAD << 6, len)
            }
            Encrypted::Disconnect => (DISCONNECT << 6, 0),
            Encrypted::Challenge { seq, data } => {
                body.write_u64::<LE>(seq)?;
                body.write_all(&data[..])?;
                (CHALLENGE << 6, CHALLENGE_INNER_SIZE)
            }
        };

        header.write_u8(prefix)?;
        header.write_u32::<LE>(sequence as u32)?;

        let m = &mut buffer[HEADER_BYTES..HEADER_BYTES+len];
        let ad = &associated(protocol_id, prefix)[..];
        let nonce = &new_nonce(sequence);
        encrypt(m, ad, nonce, key).map_err(map_err)?;
        Ok(HEADER_BYTES + len + MAC_BYTES)
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
