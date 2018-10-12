//! see https://github.com/networkprotocol/netcode.io/blob/master/STANDARD.md

#![recursion_limit="1024"]
#![feature(
    decl_macro,
    drain_filter,
    ptr_offset_from,
    const_fn,
    const_int_ops,
    int_to_from_bytes,
    try_blocks,
    const_let,
    try_from,
    integer_atomics,
    generators
)]

#[macro_use] extern crate specs_derive;
#[macro_use] extern crate serde_derive;

mod utils;
mod client;
mod server;
mod server_list;
mod incoming;

pub mod token;
pub mod protocol;

//pub mod server_system;

pub use crate::{
    client::{Client, State, ConnectingState, Error},
    server::Server,
    utils::{keygen, crypto_random},
    token::{PublicToken, USER, DATA},
    protocol::{MAX_PAYLOAD},
    server_list::ServerList,
    incoming::Incoming,
};

/*
pub const IP4_HEADER: usize = 20 + 8;
pub const IP6_HEADER: usize = 40 + 8;
*/


// #[inline(always)] fn is_control(b: u8) -> bool { (b & 1) == 1 }
// #[inline(always)] fn is_payload(b: u8) -> bool { (b & 1) == 0 }

use crate::protocol::{VERSION, VERSION_LEN};
use crate::server::{KEY, NONCE};
use std::os::raw::c_ulonglong;
use std::io::Write;
use byteorder::{LE, ByteOrder, WriteBytesExt};

pub const HMAC: usize = 16;
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
