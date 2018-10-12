//! Overview:
//!
//! ```txt
//! Client  →       auth       →  Relay
//! Client  ←       token      ←  Relay
//! Client  →      request     →  Server ×10 ≡ 10hz ≤ 1sec
//! Client  ←  response/close  ←  Server
//! Client  →     challenge    →  Server ×10 ≡ 10hz ≤ 1sec
//! Client  ↔   payload/close  ↔  Server
//! ```
//!
//! Packet format:
//! ```txt
//! [vvvvvvv0] [sequence 1-8 bytes] [ciphertext] [hmac] - payload packet
//!  xxxxxx10  14 bits sequence in 2 bytes (including prefix)
//!  xxxxx100  21 bits sequence in 3 bytes
//!  xxxx1000  28 bits sequence in 4 bytes
//!  xxx10000  35 bits sequence in 5 bytes
//!  xx100000  42 bits sequence in 6 bytes
//!  x1000000  49 bits sequence in 7 bytes
//!  10000000  56 bits sequence in 8 bytes
//!  00000000  64 bits sequence in 9 bytes
//! [00000001] [content ....] - request packet
//! [0000xxx1] - reserved for future use
//! [0010sss1] [sequence 1-8 bytes] [ciphertext] [hmac] - challenge / response packets
//! [0011sss1] [sequence 1-8 bytes] [ciphertext] [hmac] - disconnect / denied packets
//!      sss   size of the sequence number:
//!      000    1 byte
//!      001    2 bytes
//!      ...
//!      111    8 bytes
//! [01xxxxx1] - reserved for future use
//! [10xxxxx1] - reserved for future use
//! [11xxxxx1] - reserved for future use
//! ```
//!
//! Encrypted packet format:
//!
//! ```txt
//! [prefix byte] (u8)
//! [sequence] (1-8 bytes)
//! [ciphertext] (0-1175 bytes)     // 0 for disconnect/denied and 308 for challenge/response
//! [hmac] (16 bytes)
//! ```
//!

use byteorder::{LE, ByteOrder, WriteBytesExt};
use std::{
    fmt,
    mem::{transmute, size_of},
    time::Duration,
    io::{self, Write},
    slice::from_raw_parts,
};
use crate::{
    token::{CHALLENGE_LEN, PrivateToken, PRIVATE_LEN},
    crypto::{
        nonce_from_u64,
        open_chacha20poly1305,
        seal_chacha20poly1305,
        KEY,
        HMAC,
        XNONCE,
    },
};

/// Protocol version.
pub const VERSION: [u8; VERSION_LEN] = *b"ONI\0";
/// Protocol version length.
pub const VERSION_LEN: usize = 4;

/// Maximum Transmission Unit.
pub const MTU: usize = 1200;

// 1 byte for prefix
// at least 1 byte for sequence
/// Minimum size of packet.
pub const MIN_PACKET: usize = 2 + HMAC;

/// Maximum overhead in bytes.
pub const MAX_OVERHEAD: usize = 9 + HMAC;

/// Maximum length of payload in bytes.
pub const MAX_PAYLOAD: usize = MTU - MAX_OVERHEAD;

pub const NUM_DISCONNECT_PACKETS: usize = 10;

pub const PACKET_SEND_RATE: u64 = 10;
pub const PACKET_SEND_DELTA: Duration =
    Duration::from_nanos(1_000_000_000 / PACKET_SEND_RATE);

#[repr(C)]
pub struct Request {
    prefix: u8,
    version: [u8; VERSION_LEN],
    protocol: [u8; 8],
    expire: [u8; 8],
    nonce: [u8; XNONCE],
    _reserved: [u8; 131],
    // NOTE: 45 + 131 = 176
    token: [u8; PRIVATE_LEN],
}

impl PartialEq for Request {
    fn eq(&self, _: &Self) -> bool {
        //self.prefix == 1
        unimplemented!("<Request as PartialEq>::eq")
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        //write!(f, "Request")
        unimplemented!("<Request as Debug>::fmt")
    }
}

impl Request {
    pub fn expire(&self) -> u64 {
        u64::from_le_bytes(self.expire)
    }

    pub fn open_token(&mut self, private_key: &[u8; KEY]) -> Result<(u64, &PrivateToken), ()> {
        let protocol = u64::from_le_bytes(self.protocol);
        let expire = self.expire();
        let token = PrivateToken::open(&mut self.token, protocol, expire, &self.nonce, private_key)?;
        Ok((expire, token))
    }

    pub fn is_valid(&self, protocol: u64, timestamp: u64) -> bool {
        // XXX prefix = 1
        if self.prefix != 1 { return false; }
        // If the version info in the packet doesn't match VERSION, ignore the packet.
        if self.version != VERSION { return false; }
        // If the protocol id in the packet doesn't match the expected protocol id of the dedicated server, ignore the packet.
        if u64::from_le_bytes(self.protocol) != protocol { return false; }
        // If the connect token expire timestamp is <= the current timestamp, ignore the packet.
        if self.expire() <= timestamp { return false; }

        true
    }

    pub fn new(protocol: u64, expire: u64, nonce: [u8; 24], token: [u8; PRIVATE_LEN]) -> Self {
        Self {
            prefix: 1,
            version: VERSION,
            protocol: protocol.to_le_bytes(),
            expire: expire.to_le_bytes(),
            nonce,
            _reserved: [0u8; 131],
            token,
        }
    }

    pub fn write(self) -> [u8; MTU] {
        unsafe { transmute(self) }
    }

    fn _read(buf: &mut [u8]) -> Result<&mut Self, ()> {
        if buf.len() == MTU {
            Ok(unsafe { &mut *(buf.as_ptr() as *mut Self) })
        } else {
            Err(())
        }
    }
}

pub enum Packet<'a> {
    Payload {
        /// Contains `[ciphertext]`.
        buf: &'a mut [u8],
        /// Sequence number of this packet.
        seq: u64,
        /// Contains `[hmac]`.
        tag: &'a [u8; HMAC],
    },
    Handshake {
        /// Prefix byte.
        prefix: u8,
        /// Contains `[ciphertext]`.
        buf: &'a mut [u8; 8 + CHALLENGE_LEN],
        /// Sequence number of this packet.
        seq: u64,
        /// Contains `[hmac]`.
        tag: &'a [u8; HMAC],
    },
    Close {
        /// Prefix byte.
        prefix: u8,
        /// Sequence number of this packet.
        seq: u64,
        /// Contains `[hmac]`.
        tag: &'a [u8; HMAC],
    },
    Request(&'a mut Request),
}

impl<'a> PartialEq for Packet<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Packet::Payload { buf, seq, tag }, Packet::Payload { buf: _buf, seq: _seq, tag: _tag }) => {
                buf == _buf && tag == _tag && seq == _seq
            }
            _ => unimplemented!("<Packet as PartialEq>::eq")
        }
    }
}

impl<'a> fmt::Debug for Packet<'a> {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        //write!(f, "Request")
        unimplemented!("<Packet as Debug>::fmt")
    }
}

#[repr(C)]
struct EncryptedAd {
    _version: [u8; VERSION_LEN],
    _protocol: [u8; 8],
    _prefix: u8,
}

impl EncryptedAd {
    fn new(protocol: u64, prefix: u8) -> Self {
        Self {
            _version: VERSION,
            _protocol: protocol.to_le_bytes(),
            _prefix: prefix,
        }
    }
    fn as_slice(&self) -> &[u8] {
        let p: *const Self = self;
        unsafe { from_raw_parts(p as *const u8, size_of::<Self>()) }
    }
}

#[inline]
fn sequence_bytes_required(sequence: u64) -> u32 {
    1 + (64 - (sequence | 1).leading_zeros() - 1) / 8
}

impl<'a> Packet<'a> {
    pub fn encode_close(protocol: u64, mut buf: &mut [u8], seq: u64, k: &[u8; KEY]) -> io::Result<usize> {
        let start_len = buf.len();

        let sss = sequence_bytes_required(seq);
        let prefix = 0b0011_0001 | ((sss - 1) as u8) << 1;
        buf.write_u8(prefix)?;
        buf.write_uint::<LE>(seq, sss as usize)?;

        let tag = Self::seal(protocol, &mut [], seq, prefix, k);
        buf.write_all(&tag)?;

        Ok(start_len - buf.len())
    }

    // TODO: version without encryption?
    pub fn encode_handshake(protocol: u64, mut buf: &mut [u8], seq: u64, k: &[u8; KEY], m: &mut [u8; 8 + CHALLENGE_LEN]) -> io::Result<usize> {
        let start_len = buf.len();

        let sss = sequence_bytes_required(seq);
        let prefix = 0b0010_0001 | ((sss - 1) as u8) << 1;
        buf.write_u8(prefix)?;
        buf.write_uint::<LE>(seq, sss as usize)?;

        let tag = Self::seal(protocol, m, seq, prefix, k);

        buf.write_all(m)?;
        buf.write_all(&tag)?;

        Ok(start_len - buf.len())
    }

    pub fn encode_keep_alive(protocol: u64, buf: &mut [u8], seq: u64, k: &[u8; KEY]) -> io::Result<usize> {
        Self::encode_payload(protocol, buf, seq, k, &mut [])
    }

    pub fn encode_payload(protocol: u64, mut buf: &mut [u8], seq: u64, k: &[u8; KEY], m: &mut [u8]) -> io::Result<usize> {
        let start_len = buf.len();

        let bits = (64 - (seq | 1).leading_zeros()).max(14);
        let bytes = 1 + (bits - 1) / 7;

        if bits > 56 {
            buf.write_u8(0u8).unwrap();
            buf.write_u64::<LE>(seq).unwrap();
        } else {
            let mut x = (2 * seq + 1) << (bytes - 1);
            for _ in 0..bytes {
                buf.write_u8((x & 0xff) as u8)?;
                x >>= 8;
            }
        }

        let tag = Self::seal(protocol, m, seq, 0, k);

        buf.write_all(m)?;
        buf.write_all(&tag)?;

        Ok(start_len - buf.len())
    }

    pub fn decode(buf: &'a mut [u8]) -> Option<Self> {
        // FUCKING BLACK MAGIC HERE
        // So, dont't touch it.
        //
        // TODO: early check size?
        if buf.len() < MIN_PACKET { return None; }

        let prefix = buf[0];
        if prefix & 1 == 0 {
            let z = prefix.trailing_zeros() + 1;
            debug_assert!(z >= 1 && z <= 9, "bad prefix: {}", z);
            assert!(cfg!(target_endian = "little"), "big endian doesn't support yet");

            if buf.len() >= HMAC + z as usize {
                #[allow(clippy::cast_ptr_alignment)]
                let p = buf.as_ptr() as *const u64;
                let seq = if z == 9 {
                    unsafe { p.add(1).read_unaligned() }
                } else {
                    let u = 64 - 8 * z;
                    (unsafe { p.read_unaligned() } << u) >> (u + z)
                };
                let buf = &mut buf[z as usize..];
                let (buf, tag) = buf.split_at_mut(buf.len() - HMAC);
                let tag = unsafe { &*(tag.as_ptr() as *const [u8; HMAC]) };
                return Some(Packet::Payload { seq, buf, tag })
            }
        } else if prefix == 1 && buf.len() == MTU {
            return Some(Packet::Request(unsafe { &mut *(buf.as_ptr() as *mut Request) }));
        } else if prefix & 0b1110_0000 == 0b0010_0000 {
            let typ = (prefix & 0b0001_0000) >> 4 != 0;
            let sss = (prefix & 0b0000_1110) >> 1;
            let len = sss + 1;
            debug_assert!(len >= 1 && len <= 8);

            if buf.len() >= 1 + HMAC + len as usize {
                let seq = LE::read_uint(&buf[1..], len as usize);
                let buf = &mut buf[1 + len as usize..];

                let (buf, tag) = buf.split_at_mut(buf.len() - HMAC);
                let tag = unsafe { &*(tag.as_ptr() as *const [u8; HMAC]) };
                if typ && buf.is_empty() {
                    return Some(Packet::Close { prefix, seq, tag });
                } else if !typ && buf.len() == 8 + CHALLENGE_LEN {
                    let buf = unsafe { &mut *(buf.as_mut_ptr() as *mut [u8; 8 + CHALLENGE_LEN]) };
                    return Some(Packet::Handshake { prefix, seq, buf, tag });
                }
            }
        }
        None
    }

    pub fn seal(protocol: u64, m: &mut [u8], seq: u64, prefix: u8, k: &[u8; KEY]) -> [u8; HMAC] {
        let ad = EncryptedAd::new(protocol, prefix);
        seal_chacha20poly1305(m, Some(ad.as_slice()), &nonce_from_u64(seq), k)
    }

    pub fn open(protocol: u64, c: &mut [u8], seq: u64, prefix: u8, t: &[u8; HMAC], k: &[u8; KEY]) -> Result<(), ()> {
        let ad = EncryptedAd::new(protocol, prefix);
        open_chacha20poly1305(c, Some(ad.as_slice()), t, &nonce_from_u64(seq), k)
    }
}

#[test]
fn test_sequence() {
    let tests = [
        (0x______________00, 1),
        (0x______________11, 1),
        (0x______________FF, 1),

        (0x____________0100, 2),
        (0x____________1122, 2),
        (0x____________FFFF, 2),

        (0x__________010000, 3),
        (0x__________112233, 3),
        (0x__________FFFFFF, 3),

        (0x________01000000, 4),
        (0x________11223344, 4),
        (0x________FFFFFFFF, 4),

        (0x______0100000000, 5),
        (0x______1122334455, 5),
        (0x______FFFFFFFFFF, 5),

        (0x____010000000000, 6),
        (0x____112233445566, 6),
        (0x____FFFFFFFFFFFF, 6),

        (0x__01000000000000, 7),
        (0x__11223344556677, 7),
        (0x__FFFFFFFFFFFFFF, 7),

        (0x0100000000000000, 8),
        (0x1122334455667788, 8),
        (0xFFFFFFFFFFFFFFFF, 8),
    ];

    for (seq, bytes) in &tests {
        assert_eq!(sequence_bytes_required(*seq), *bytes);
    }
}


#[test]
fn decode_payload_packet() {
    let mut buffer = [0u8; MIN_PACKET];

    // full 8 bit sequence and bad size
    assert_eq!(Packet::decode(&mut buffer), None);

    // full 8 bit sequence and ok size
    // XXX: It can be used for some black magic?
    //      Payload packets for IoT or something?
    //      In this case we have only 56 bits for common sequence.
    //      also see https://tools.ietf.org/id/draft-mattsson-core-security-overhead-01.html
    assert_eq!(Packet::decode(&mut [0u8; 9+HMAC]).unwrap(), Packet::Payload {
        seq: 0,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // zero sequence
    buffer[0] = 0b00000010;
    assert_eq!(Packet::decode(&mut buffer).unwrap(), Packet::Payload {
        seq: 0,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // one sequence
    buffer[0] = 0b00000110;
    assert_eq!(Packet::decode(&mut buffer).unwrap(), Packet::Payload {
        seq: 1,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    buffer[0] = 0b11111110;
    assert_eq!(Packet::decode(&mut buffer).unwrap(), Packet::Payload {
        seq: 0x3F,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // maximum 14 bit sequence
    buffer[0] = 0b11111110;
    buffer[1] = 0b11111111;
    assert_eq!(Packet::decode(&mut buffer).unwrap(), Packet::Payload {
        seq: 0x3fff,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // 21 bit sequence and bad size
    buffer[0] = 0b00000100;
    buffer[1] = 0b00000000;
    assert_eq!(Packet::decode(&mut buffer), None);
}

#[test]
#[ignore]
fn decode_packet() {
    // TODO

    let mut data = [0u8; 123];
    let buf = &mut data[..];

    match Packet::decode(buf).unwrap() {
        Packet::Payload { seq, buf, tag } => {
            unimplemented!("payload packet: {} {:?} {:?}", seq, buf, tag)
        }
        Packet::Close { prefix, seq, tag } => {
            unimplemented!("close packet: {} {} {:?}", prefix, seq, tag)
        }
        Packet::Handshake { prefix, seq, buf, tag } => {
            unimplemented!("challenge packet: {} {} {:?} {:?}", prefix, seq, &buf[..], tag)
        }
        Packet::Request(_request) => {
            unimplemented!("request packet")
        }
    }

    /*
    let n = 0b00101000u8;
    assert_eq!(n.trailing_zeros(), 3);

    let n = 0u8;
    assert_eq!(n.trailing_zeros(), 8);
    */
}

#[test]
fn request_packet() {
    use crate::{unix_time, crypto::{keygen, crypto_random}, token::{USER, DATA, PublicToken}};
    assert_eq!(size_of::<Request>(), MTU);

    let protocol  = 0x11223344_55667788;
    let client_id = 0x55667788_11223344;

    let expire = 0x12345678;
    let timeout = 0x87654321;

    let private_key = keygen();

    let mut data = [0u8; DATA];
    let mut user = [0u8; USER];
    crypto_random(&mut data);
    crypto_random(&mut user);

    let tok = PublicToken::generate(data, user, expire, timeout, client_id, protocol, &private_key);

    let req = Request::new(protocol, tok.expire_timestamp(), tok.nonce(), *tok.token());
    let mut req = Request::write(req);

    let timestamp = unix_time();
    let r = Request::_read(&mut req[..]).unwrap();
    assert!(r.is_valid(protocol, timestamp));
    let (expire, private) = r.open_token(&private_key).unwrap();

    assert_eq!(expire, tok.expire_timestamp());
    assert_eq!(&private.data()[..], &tok.data()[..]);
}
