use std::mem::{transmute, zeroed};
use std::os::raw::c_ulonglong;

use crate::{
    server::{KEY, HMAC, NONCE, XNONCE},
    token::{
        ChallengeToken, CHALLENGE_LEN,
        PrivateToken, PRIVATE_LEN,
    },
    utils::slice_to_array,
    VERSION, VERSION_BYTES as VERSION_LEN,
};

const PREFIX_SHIFT: u32 = 30;
const PREFIX_MASK: u32 = 0xC0000000;
const SEQUENCE_MASK: u32 = 0x3FFFFFFF;

pub const MTU: usize = 1200;
pub const HEADER: usize = 4;
pub const OVERHEAD: usize = HEADER + HMAC;

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
pub const DISCONNECT: u8 =  0b01;
pub const DENIED: u8 =      0b01;
pub const CHALLENGE: u8 =   0b10;
pub const RESPONSE: u8 =    0b10;
pub const PAYLOAD: u8 =     0b11;
pub const KEEP_ALIVE: u8 =  0b11;

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
    (((kind as u32) << PREFIX_SHIFT) | seq & SEQUENCE_MASK).to_le_bytes()
}

pub fn extract_header(buffer: &[u8]) -> Result<(u8, u32), ()> {
    let prefix = u32::from_le_bytes(slice_to_array!(&buffer, HEADER)?);
    let kind = (prefix >> PREFIX_SHIFT) & 0b11;
    let sequence = prefix & SEQUENCE_MASK;
    Ok((kind as u8, sequence as u32))
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

pub fn new_challenge_packet(protocol: u64, seq: u32, key: &[u8; KEY], challenge: &[u8; CHALLENGE_LEN + 8]) -> [u8; HEADER + 8 + CHALLENGE_LEN + HMAC] {
    let mut packet = [0u8; HEADER + 8 + CHALLENGE_LEN + HMAC];
    packet[..HEADER].copy_from_slice(&write_header(CHALLENGE, seq));
    packet[HEADER..8+CHALLENGE_LEN].copy_from_slice(&challenge[..]);
    let m = &mut packet[HEADER..8+CHALLENGE_LEN];
    let hmac = encrypt_packet(protocol, CHALLENGE, seq, m, key);
    packet[HEADER + 8 + CHALLENGE_LEN..].copy_from_slice(&hmac[..]);
    packet
}

#[repr(C)]
struct EncryptedAd {
    _version: [u8; VERSION_LEN],
    _protocol: [u8; 8],
    _kind: u8,
}

pub fn encrypt_packet(protocol: u64, kind: u8, seq: u32, m: &mut [u8], k: &[u8; KEY]) -> [u8; HMAC] {
    let mut n = [0u8; NONCE];
    n[4..8].copy_from_slice(&seq.to_le_bytes()[..]);

    let ad = EncryptedAd {
        _version: VERSION,
        _protocol: protocol.to_le_bytes(),
        _kind: kind,
    };

    let ad_p = (&ad as *const EncryptedAd) as *const _;
    let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

    let mut tag = [0u8; HMAC];
    let mut maclen = HMAC as c_ulonglong;

    let ret = unsafe {
        crate::sodium::crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
        _kind: kind,
    };

    let ad_p = (&ad as *const EncryptedAd) as *const _;
    let ad_len = std::mem::size_of::<EncryptedAd>() as c_ulonglong;

    unsafe {
        let ret = crate::sodium::crypto_aead_chacha20poly1305_ietf_decrypt_detached(
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
    pub fn token(&self, private_key: &[u8; KEY]) -> Result<PrivateToken, ()> {
        let protocol = u64::from_le_bytes(self.protocol);
        PrivateToken::decrypt(&self.token, protocol, self.expire(), &self.nonce, private_key)
    }

    pub fn is_valid(&self, protocol: u64, timestamp: u64) -> bool {
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

    pub fn read(buf: &mut [u8]) -> Result<&mut Self, ()> {
        if buf.len() == MTU {
            Ok(unsafe { &mut *(buf.as_ptr() as *mut Self) })
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

#[test]
fn challenge_packet() {
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
    let req = RequestPacket::read(&mut req[..]).unwrap();
    assert_eq!(req.expire(), tok.expire_timestamp());
    assert!(req.is_valid(protocol, crate::utils::time_secs()));

    let private = req.token(&private_key).unwrap();
    assert_eq!(&private.data()[..], &tok.data()[..]);
}
