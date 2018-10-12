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
//! Prefix byte format:
//!
//! ```txt
//! 00000000 - request packet
//! 00xxxxxx - invalid packet
//! 01ssssss - challenge or response packets
//! 10ssssss - disconnect or denied packets
//! 11ssssss - payload packet
//!
//! s - high bits of sequence
//! ```
//!
//! Encrypted packet format:
//!
//! ```txt
//! [prefix byte] (u8)
//! [big endian sequence] (u24)
//! [ciphertext] (0-1180 bytes)     // 0 for disconnect/denied and 308 for challenge/response
//! [hmac] (16 bytes)
//! ```
//!


use std::mem::transmute;
use std::time::Duration;

use crate::{
    token::{
        ChallengeToken, CHALLENGE_LEN,
        PrivateToken, PRIVATE_LEN,
    },
    utils::slice_to_array,
};

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;

/// Protocol version.
pub const VERSION: [u8; VERSION_LEN] = *b"ONI\0";
/// Protocol version length.
pub const VERSION_LEN: usize = 4;

/// Maximum Transmission Unit.
pub const MTU: usize = 1200;
/// Header size in bytes.
pub const HEADER: usize = 4;
/// Overhead in bytes.
pub const OVERHEAD: usize = HEADER + HMAC;
/// Max length of payload in bytes.
pub const MAX_PAYLOAD: usize = MTU - OVERHEAD;

pub const CHALLENGE_PACKET_LEN: usize = 8 + CHALLENGE_LEN + OVERHEAD;
pub const RESPONSE_PACKET_LEN: usize = 8 + CHALLENGE_LEN + OVERHEAD;
pub const DENIED_PACKET_LEN: usize = OVERHEAD;
pub const DISCONNECT_PACKET_LEN: usize = OVERHEAD;
pub const REQUEST_PACKET_LEN: usize = MTU;

const PREFIX_SHIFT: u32 = 30;
//const PREFIX_MASK: u32 = 0xC0000000;
const SEQUENCE_MASK: u32 = 0x3FFFFFFF;

pub const NUM_DISCONNECT_PACKETS: usize = 10;

pub const PACKET_SEND_RATE: u64 = 10;
pub const PACKET_SEND_DELTA: Duration =
    Duration::from_nanos(1_000_000_000 / PACKET_SEND_RATE);


pub const REQUEST: u8 =     0b00;
pub const DISCONNECT: u8 =  0b01;
pub const DENIED: u8 =      0b01;
pub const CHALLENGE: u8 =   0b10;
pub const RESPONSE: u8 =    0b10;
pub const PAYLOAD: u8 =     0b11;
pub const KEEP_ALIVE: u8 =  0b11;


pub fn read_packet<'a, F>(protocol: u64, key: &[u8; KEY], buf: &'a mut [u8], filter: F) -> Result<&'a [u8], ()>
    where F: FnOnce(u32) -> bool
{
    if buf.len() < HEADER + HMAC { return Err(()); }

    let (header, buf) = buf.split_at_mut(HEADER);
    let (ciphertext, tag) = buf.split_at_mut(buf.len() - HMAC);
    let tag = unsafe { &*(tag.as_ptr() as *const [u8; HMAC]) };

    let (kind, seq) = extract_header(header)?;
    if filter(seq) { return Err(()); }
    decrypt_packet(protocol, kind, seq, ciphertext, *tag, key)?;

    Ok(ciphertext)
}


pub fn keep_alive_packet(protocol: u64, seq: u32, key: &[u8; KEY]) -> [u8; OVERHEAD] {
    simple_packet(protocol, key, KEEP_ALIVE, seq)
}

pub fn disconnect_packet(protocol: u64, seq: u32, key: &[u8; KEY]) -> [u8; OVERHEAD] {
    simple_packet(protocol, key, DISCONNECT, seq)
}

pub fn denied_packet(protocol: u64, seq: u32, key: &[u8; KEY]) -> [u8; OVERHEAD] {
    simple_packet(protocol, key, DENIED, seq)
}

fn simple_packet(protocol: u64, key: &[u8; KEY], kind: u8, seq: u32) -> [u8; OVERHEAD] {
    let header = write_header(kind, seq);
    let hmac = encrypt_packet(protocol, kind, seq, &mut [], key);
    unsafe { transmute((header, hmac)) }
}

pub fn write_header(kind: u8, seq: u32) -> [u8; 4] {
    (((kind as u32) << PREFIX_SHIFT) | seq & SEQUENCE_MASK).to_be_bytes()
}

pub fn extract_header(buffer: &[u8]) -> Result<(u8, u32), ()> {
    let prefix = u32::from_be_bytes(slice_to_array!(&buffer, HEADER)?);
    let kind = (prefix >> PREFIX_SHIFT) & 0b11;
    let sequence = prefix & SEQUENCE_MASK;
    Ok((kind as u8, sequence as u32))
}

#[test]
fn header_rw() {
    for i in 0..=3 {
        let h = write_header(i, 1234);
        assert_eq!(extract_header(&h).unwrap(), (i, 1234));
    }
}

pub fn send_payload<F>(protocol: u64, seq: u32, key: &[u8; KEY], payload: &[u8], send: F)
    where F: FnOnce(&[u8])
{
    let len = payload.len().min(MTU - OVERHEAD);
    let mut packet = [0u8; MTU];
    packet[      ..HEADER    ].copy_from_slice(&write_header(PAYLOAD, seq));
    packet[HEADER..HEADER+len].copy_from_slice(&payload[..len]);
    let m = &mut packet[HEADER..HEADER+len];
    let hmac = encrypt_packet(protocol, PAYLOAD, seq, m, key);
    packet[HEADER+len..HEADER+len+HMAC].copy_from_slice(&hmac[..]);
    send(&packet[..len+OVERHEAD])
}

pub fn new_challenge_packet(protocol: u64, seq: u32, key: &[u8; KEY], challenge: &[u8; CHALLENGE_LEN + 8]) -> [u8; CHALLENGE_PACKET_LEN] {
    let mut buffer = [0u8; HEADER + 8 + CHALLENGE_LEN + HMAC];
    let (header, packet) = &mut buffer[..].split_at_mut(HEADER);
    header.copy_from_slice(&write_header(CHALLENGE, seq));
    let (m, tag) = &mut packet[..].split_at_mut(8+CHALLENGE_LEN);
    m.copy_from_slice(&challenge[..]);
    let hmac = encrypt_packet(protocol, CHALLENGE, seq, m, key);
    tag.copy_from_slice(&hmac[..]);
    buffer
}

pub fn encrypt_packet(protocol: u64, kind: u8, seq: u32, m: &mut [u8], k: &[u8; KEY]) -> [u8; HMAC] {
    let mut n = [0u8; NONCE];
    n[4..8].copy_from_slice(&seq.to_le_bytes()[..]);

    let ad = EncryptedAd {
        _version: VERSION,
        _protocol: protocol.to_le_bytes(),
        _prefix: kind,
    };

    let ad_p = (&ad as *const EncryptedAd) as *const _;
    let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

    let mut tag = [0u8; HMAC];
    let mut maclen = HMAC as c_ulonglong;

    let _ = unsafe {
        crate::utils::crypto_aead_chacha20poly1305_ietf_encrypt_detached(
            m.as_mut_ptr(),
            tag.as_mut_ptr(),
            &mut maclen,
            m.as_ptr(),
            m.len() as c_ulonglong,
            ad_p,
            ad_len,
            0 as *mut _,
            n.as_ptr(),
            k.as_ptr()
        )
    };
    tag
}

pub fn decrypt_packet(protocol: u64, kind: u8, seq: u32, c: &mut [u8], t: [u8; HMAC], k: &[u8; KEY]) -> Result<(), ()> {
    let mut n = [0u8; NONCE];
    n[4..8].copy_from_slice(&seq.to_le_bytes()[..]);

    let ad = EncryptedAd {
        _version: VERSION,
        _protocol: protocol.to_le_bytes(),
        _prefix: kind,
    };

    let ad_p = (&ad as *const EncryptedAd) as *const _;
    let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

    unsafe {
        let ret = crate::utils::crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            c.as_mut_ptr(),
            0 as *mut _,
            c.as_ptr(),
            c.len() as c_ulonglong,
            t.as_ptr(),
            ad_p, ad_len,
            n.as_ptr(), k.as_ptr(),
        );
        if ret != 0 {
            Err(())
        } else {
            Ok(())
        }
    }
}

#[repr(C)]
pub struct RequestPacket {
    prefix: u8,
    version: [u8; VERSION_LEN],
    protocol: [u8; 8],
    expire: [u8; 8],
    nonce: [u8; XNONCE],
    _reserved: [u8; 131],
    // NOTE: 45 + 131 = 176
    token: [u8; PRIVATE_LEN],
}

impl RequestPacket {
    pub fn expire(&self) -> u64 {
        u64::from_le_bytes(self.expire)
    }
    pub fn open_token(&self, private_key: &[u8; KEY]) -> Result<PrivateToken, ()> {
        let protocol = u64::from_le_bytes(self.protocol);
        PrivateToken::decrypt(&self.token, protocol, self.expire(), &self.nonce, private_key)
    }

    fn is_valid(&self, protocol: u64, timestamp: u64) -> bool {
        if self.prefix != 0 { return false; }
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
            prefix: REQUEST,
            version: VERSION,
            protocol: protocol.to_le_bytes(),
            expire: expire.to_le_bytes(),
            nonce: nonce,
            _reserved: [0u8; 131],
            token: token,
        }
    }

    pub fn write(self) -> [u8; MTU] {
        unsafe { transmute(self) }
    }

    pub fn open(buf: &mut [u8], protocol: u64, timestamp: u64, key: &[u8; KEY]) -> Result<(u64, PrivateToken), ()> {
        let r = Self::read(buf, protocol, timestamp)?;
        let token = r.open_token(key)?;
        Ok((r.expire(), token))
    }

    fn read(buf: &mut [u8], protocol: u64, timestamp: u64) -> Result<&mut Self, ()> {
        if buf.len() == MTU {
            let r = unsafe { &mut *(buf.as_ptr() as *mut Self) };
            if !r.is_valid(protocol, timestamp) { return Err(()); }
            Ok(r)
        } else {
            Err(())
        }
    }
}

pub type ResponsePacket = ChallengePacket;

#[repr(C)]
pub struct ChallengePacket {
    sequence: [u8; 8],
    token: [u8; CHALLENGE_LEN],
}

impl ChallengePacket {
    pub fn open_token(protocol: u64, buf: &mut [u8], recv_key: &[u8; KEY], challenge_key: &[u8; KEY]) -> Result<ChallengeToken, ()> {
        let mut ciphertext = ResponsePacket::client_read(protocol, buf, recv_key)?;
        ResponsePacket::read(&mut ciphertext, challenge_key)
    }

    pub fn client_read(protocol: u64, buf: &mut [u8], key: &[u8; KEY]) -> Result<[u8; 8+CHALLENGE_LEN], ()> {
        if buf.len() != CHALLENGE_PACKET_LEN { return Err(()); }
        let (header, buf) = buf.split_at_mut(HEADER);
        let (kind, seq) = extract_header(header)?;
        let (ciphertext, tag) = buf.split_at_mut(8+CHALLENGE_LEN);
        let tag = slice_to_array!(tag, HMAC).unwrap();
        decrypt_packet(protocol, kind, seq, ciphertext, tag, key)?;
        Ok(slice_to_array!(ciphertext, 8+CHALLENGE_LEN).unwrap())
    }

    pub fn write(sequence: u64, key: &[u8; KEY], token: ChallengeToken) -> [u8; 8+CHALLENGE_LEN] {
        unsafe { transmute(Self {
            sequence: sequence.to_le_bytes(),
            token: token.encrypt(sequence, key),
        }) }
    }
    pub fn read(buf: &mut [u8], key: &[u8; KEY]) -> Result<ChallengeToken, ()> {
        if buf.len() == 8 + CHALLENGE_LEN {
            let ch = unsafe { &mut *(buf.as_ptr() as *mut Self) };
            let seq = u64::from_le_bytes(ch.sequence);
            ChallengeToken::decrypt(ch.token, seq, key)
        } else {
            Err(())
        }
    }
}

pub struct EmptyPacket;

impl EmptyPacket {
    pub fn read(protocol: u64, buf: &[u8], key: &[u8; KEY]) -> Result<(), ()> {
        if buf.len() != HEADER + HMAC { return Err(()); }
        let (header, tag) = buf.split_at(HEADER);
        let (kind, seq) = extract_header(header)?;
        let tag = slice_to_array!(tag, HMAC).unwrap();
        decrypt_packet(protocol, kind, seq, &mut [], tag, key)
    }

    /*
    pub fn write(protocol: u64, buf: &[u8], key: &[u8; KEY]) -> Result<(), ()> {
        if buf.len() != HEADER + HMAC { return Err(()); }
        let (header, tag) = buf.split_at(HEADER);
        let (kind, seq) = extract_header(header)?;
        let tag = unsafe { cast_slice_to_array!(tag, HMAC) };
        decrypt_packet(protocol, kind, seq, &mut [], *tag, key)
    }
    */
}

#[test]
fn challenge_packet() {
    let protocol  = 0x11223344_55667788;
    let seq = 123;
    let key = crate::utils::keygen();

    let mut challenge = [0u8; 8 + CHALLENGE_LEN];
    crate::utils::crypto_random(&mut challenge);

    let mut p = new_challenge_packet(protocol, seq, &key, &challenge);
    let v = ChallengePacket::client_read(protocol, &mut p, &key).unwrap();

    assert_eq!(&challenge[..], &v[..]);
}

#[test]
fn request_packet() {
    assert_eq!(std::mem::size_of::<RequestPacket>(), MTU);

    let protocol  = 0x11223344_55667788;
    let client_id = 0x55667788_11223344;

    let expire = 0x12345678;
    let timeout = 0x87654321;

    let private_key = crate::utils::keygen();

    let mut data = [0u8; crate::token::DATA];
    let mut user = [0u8; crate::token::USER];
    crate::utils::crypto_random(&mut data);
    crate::utils::crypto_random(&mut user);

    let tok = crate::token::PublicToken::generate(
        data, user, expire, timeout, client_id, protocol, &private_key);

    let req = RequestPacket::new(protocol, tok.expire_timestamp(), tok.nonce(), *tok.token());
    let mut req = RequestPacket::write(req);

    let timestamp = crate::utils::time_secs();
    let (expire, private) = RequestPacket::open(&mut req[..], protocol, timestamp, &private_key).unwrap();
    assert_eq!(expire, tok.expire_timestamp());
    assert_eq!(&private.data()[..], &tok.data()[..]);
}































use std::os::raw::c_ulonglong;
use std::io::Write;
use byteorder::{LE, ByteOrder, WriteBytesExt};

const MIN_PACKET: usize = 2 + HMAC;

/// Format:
/// ```txt
/// [vvvvvvv0] [sequence 1-8 bytes] [ciphertext] [hmac] - payload packet
/// [xxxxxx10] 14 bits sequence in 2 bytes (including prefix)
/// [xxxxx100] 21 bits sequence in 3 bytes
/// [xxxx1000] 28 bits sequence in 4 bytes
/// [xxx10000] 35 bits sequence in 5 bytes
/// [xx100000] 42 bits sequence in 6 bytes
/// [x1000000] 49 bits sequence in 7 bytes
/// [10000000] 56 bits sequence in 8 bytes
/// [00000000] 64 bits sequence in 9 bytes
/// [00000001] [content ....] - request packet
/// [0000xxx1] - reserved
/// [0010sss1] [sequence 1-8 bytes] [ciphertext] [hmac] - challenge / response packets
/// [0011sss1] [sequence 1-8 bytes] [ciphertext] [hmac] - disconnect / denied packets
///      sss   - size of the sequence number
///      000   - 1 byte
///      001   - 2 bytes
///      ...
///      111   - 8 bytes
/// [01xxxxx1] - reserved
/// [10xxxxx1] - reserved
/// [11xxxxx1] - reserved
/// ```
#[derive(Debug, PartialEq)]
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
        /// Contains `[ciphertext]`.
        buf: &'a mut [u8],
        /// Sequence number of this packet.
        seq: u64,
        /// Contains `[hmac]`.
        tag: &'a [u8; HMAC],
    },
    Close {
        /// Contains `[ciphertext]`.
        buf: &'a mut [u8],
        /// Sequence number of this packet.
        seq: u64,
        /// Contains `[hmac]`.
        tag: &'a [u8; HMAC],
    },
    Request {
        buf: &'a mut [u8],
    },
    Invalid(&'a mut [u8]),
}

#[repr(C)]
struct EncryptedAd {
    _version: [u8; VERSION_LEN],
    _protocol: [u8; 8],
    _prefix: u8,
}

#[inline]
fn sequence_bytes_required(sequence: u64) -> u32 {
    1 + (64 - (sequence | 1).leading_zeros() - 1) / 8
}

impl<'a> Packet<'a> {
    pub fn encode_close(protocol: u64, buf: &mut [u8], seq: u64, k: &[u8; KEY]) -> std::io::Result<usize> {
        Self::encode_close_custom(protocol, buf, seq, k, &mut [])
    }

    pub fn encode_handshake(protocol: u64, mut buf: &mut [u8], seq: u64, k: &[u8; KEY], m: &mut [u8]) -> std::io::Result<usize> {
        let start_len = buf.len();

        let sss = sequence_bytes_required(seq);
        let prefix = 0b0010_0001 | (sss as u8) << 1;
        buf.write_u8(prefix)?;
        buf.write_uint::<LE>(seq, sss as usize)?;

        let tag = Self::seal(protocol, m, seq, prefix, k);

        buf.write_all(m)?;
        buf.write_all(&tag)?;

        Ok(buf.len() - (buf.len() - start_len))
    }

    pub fn encode_close_custom(protocol: u64, mut buf: &mut [u8], seq: u64, k: &[u8; KEY], m: &mut [u8]) -> std::io::Result<usize> {
        let start_len = buf.len();

        let sss = sequence_bytes_required(seq);
        let prefix = 0b0011_0001 | (sss as u8) << 1;
        buf.write_u8(prefix)?;
        buf.write_uint::<LE>(seq, sss as usize)?;

        let tag = Self::seal(protocol, m, seq, prefix, k);

        buf.write_all(m)?;
        buf.write_all(&tag)?;

        Ok(buf.len() - (buf.len() - start_len))
    }

    pub fn encode_keep_alive(protocol: u64, buf: &mut [u8], seq: u64, k: &[u8; KEY]) -> std::io::Result<usize> {
        Self::encode_payload(protocol, buf, seq, k, &mut [])
    }

    pub fn encode_payload(protocol: u64, mut buf: &mut [u8], seq: u64, k: &[u8; KEY], m: &mut [u8]) -> std::io::Result<usize> {
        let start_len = buf.len();

        let bits = (64 - (seq | 1).leading_zeros()).min(14);
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

        Ok(buf.len() - (buf.len() - start_len))
    }

    pub fn decode(buf: &'a mut [u8]) -> Self {
        // FUCKING BLACK MAGIC HERE
        // So, dont't touch it.

        // 1 byte for prefix
        // at least 1 byte for sequence
        if buf.len() < 2 + HMAC {
            return Packet::Invalid(buf);
        }

        let prefix = buf[0];
        if (prefix & 1) == 0 {
            let z = prefix.trailing_zeros() + 1;
            debug_assert!(z >= 1 && z <= 9, "bad prefix: {}", z);
            assert!(cfg!(target_endian = "little"), "big endian doesn't support yet");

            if buf.len() >= HMAC + z as usize {
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
                Packet::Payload { seq, buf, tag }
            } else {
                Packet::Invalid(buf)
            }
        } else {
            if prefix & 0b11000000 != 0 {
                Packet::Invalid(buf)
            } else if prefix & 0b00100000 != 0 {
                let typ = (prefix & 0b00010000) >> 4 != 0;
                let sss = (prefix & 0b00001110) >> 1;
                let len = sss + 1;
                debug_assert!(len >= 1 && len <= 8);

                if buf.len() >= 1 + HMAC + len as usize {
                    let seq = LE::read_uint(&buf[1..], len as usize);
                    let buf = &mut buf[1 + len as usize..];

                    let (buf, tag) = buf.split_at_mut(buf.len() - HMAC);
                    let tag = unsafe { &*(tag.as_ptr() as *const [u8; HMAC]) };
                    if typ {
                        Packet::Close { seq, buf, tag }
                    } else {
                        Packet::Handshake { seq, buf, tag }
                    }
                } else {
                    Packet::Invalid(buf)
                }
            } else {
                // TODO: check size?
                Packet::Request { buf: &mut buf[1..] }
            }
        }
    }

    pub fn seal(protocol: u64, m: &mut [u8], seq: u64, prefix: u8, k: &[u8; KEY]) -> [u8; HMAC] {
        let mut n = [0u8; NONCE];
        n[0..8].copy_from_slice(&seq.to_le_bytes()[..]);

        let ad = EncryptedAd {
            _version: VERSION,
            _protocol: protocol.to_le_bytes(),
            _prefix: prefix,
        };

        let ad_p = (&ad as *const EncryptedAd) as *const _;
        let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

        let mut tag = [0u8; HMAC];
        let mut maclen = HMAC as c_ulonglong;

        let _ = unsafe {
            crate::utils::crypto_aead_chacha20poly1305_ietf_encrypt_detached(
                m.as_mut_ptr(),
                tag.as_mut_ptr(),
                &mut maclen,
                m.as_ptr(),
                m.len() as c_ulonglong,
                ad_p,
                ad_len,
                0 as *mut _,
                n.as_ptr(),
                k.as_ptr()
            )
        };
        tag
    }

    pub fn open(protocol: u64, c: &mut [u8], seq: u64, prefix: u8, t: &[u8; HMAC], k: &[u8; KEY]) -> Result<(), ()> {
        let mut n = [0u8; NONCE];
        n[0..8].copy_from_slice(&seq.to_le_bytes()[..]);

        let ad = EncryptedAd {
            _version: VERSION,
            _protocol: protocol.to_le_bytes(),
            _prefix: prefix,
        };

        let ad_p = (&ad as *const EncryptedAd) as *const _;
        let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

        unsafe {
            let ret = crate::utils::crypto_aead_chacha20poly1305_ietf_decrypt_detached(
                c.as_mut_ptr(),
                0 as *mut _,
                c.as_ptr(),
                c.len() as c_ulonglong,
                t.as_ptr(),
                ad_p, ad_len,
                n.as_ptr(), k.as_ptr(),
            );
            if ret != 0 {
                Err(())
            } else {
                Ok(())
            }
        }
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
    let mut buffer = [0u8; 2+HMAC];

    // full 8 bit sequence and bad size
    assert_eq!(Packet::decode(&mut buffer), Packet::Invalid(&mut [0u8; 2+HMAC]));

    // full 8 bit sequence and ok size
    // XXX: It can be used for some black magic?
    //      Payload packets for IoT or something?
    //      In this case we have only 56 bits for common sequence.
    //      also see https://tools.ietf.org/id/draft-mattsson-core-security-overhead-01.html
    assert_eq!(Packet::decode(&mut [0u8; 9+HMAC]), Packet::Payload {
        seq: 0,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // zero sequence
    buffer[0] = 0b00000010;
    assert_eq!(Packet::decode(&mut buffer), Packet::Payload {
        seq: 0,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // one sequence
    buffer[0] = 0b00000110;
    assert_eq!(Packet::decode(&mut buffer), Packet::Payload {
        seq: 1,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    buffer[0] = 0b11111110;
    assert_eq!(Packet::decode(&mut buffer), Packet::Payload {
        seq: 0x3F,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // maximum 14 bit sequence
    buffer[0] = 0b11111110;
    buffer[1] = 0b11111111;
    assert_eq!(Packet::decode(&mut buffer), Packet::Payload {
        seq: 0x3fff,
        buf: &mut [],
        tag: &[0u8; HMAC],
    });

    // 21 bit sequence and bad size
    buffer[0] = 0b00000100;
    buffer[1] = 0b00000000;
    assert_eq!(Packet::decode(&mut buffer), Packet::Invalid(&mut [
        4, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
    ]));
}

#[test]
#[ignore]
fn decode_packet() {
    // TODO

    let mut data = [0u8; 123];
    let buf = &mut data[..];

    match Packet::decode(buf) {
        Packet::Payload { seq, buf, tag } => {
            unimplemented!("payload packet: {} {:?} {:?}", seq, buf, tag)
        }
        Packet::Close { seq, buf, tag } => {
            unimplemented!("close packet: {} {:?} {:?}", seq, buf, tag)
        }
        Packet::Handshake { seq, buf, tag } => {
            unimplemented!("challenge packet: {} {:?} {:?}", seq, buf, tag)
        }
        Packet::Request { buf } => {
            unimplemented!("request packet: {:?}", buf)
        }
        Packet::Invalid(_) => { /* just ignore or use for black magic */ }
    }

    /*
    let n = 0b00101000u8;
    assert_eq!(n.trailing_zeros(), 3);

    let n = 0u8;
    assert_eq!(n.trailing_zeros(), 8);
    */
}
