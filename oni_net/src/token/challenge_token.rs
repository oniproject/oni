use byteorder::{LE, ByteOrder};
use std::{slice::from_raw_parts_mut, mem::size_of};
use crate::protocol::{KEY, HMAC};
use crate::utils::{
    nonce_from_u64,
    open_chacha20poly1305,
    seal_chacha20poly1305,
};

use super::{USER, CHALLENGE_LEN};

#[repr(C)]
pub struct ChallengeToken {
    client_id: [u8; 8],
    _reserved: [u8; 20],
    user: [u8; USER],
    hmac: [u8; HMAC],
}

impl ChallengeToken {
    pub fn new(client_id: u64, user: [u8; USER]) -> Self {
        Self {
            client_id: client_id.to_le_bytes(),
            user,
            _reserved: [0u8; 20],
            hmac: [0u8; HMAC],
        }
    }

    pub fn client_id(&self) -> u64 {
        u64::from_le_bytes(self.client_id)
    }
    pub fn user(&self) -> &[u8; USER] { &self.user }

    pub fn encode_packet(mut self, seq: u64, k: &[u8; KEY]) -> [u8; 8+CHALLENGE_LEN] {
        let mut buffer = [0u8; 8+CHALLENGE_LEN];
        buffer[..8].copy_from_slice(&seq.to_le_bytes()[..]);
        buffer[8..].copy_from_slice(self.seal(seq, k));
        buffer
    }

    pub fn decode_packet<'a>(buf: &'a mut [u8; 8 + CHALLENGE_LEN], k: &[u8; KEY]) -> Result<&'a Self, ()> {
        let (seq, buf) = buf.split_at_mut(8);
        let seq = LE::read_u64(seq);
        let token = unsafe { &mut *(buf.as_mut_ptr() as *mut [u8; CHALLENGE_LEN]) };
        ChallengeToken::open(token, seq, k)
    }

    pub fn seal<'a>(&'a mut self, seq: u64, k: &[u8; KEY]) -> &'a mut [u8; CHALLENGE_LEN] {
        assert_eq!(size_of::<Self>(), CHALLENGE_LEN);
        let p: *mut Self = self;
        let m = unsafe { from_raw_parts_mut(p as *mut u8, CHALLENGE_LEN-HMAC) };
        self.hmac = seal_chacha20poly1305(m, None, &nonce_from_u64(seq), k);
        unsafe { &mut *(p as *mut [u8; CHALLENGE_LEN]) }
    }

    pub fn open<'a>(buf: &'a mut [u8; CHALLENGE_LEN], seq: u64, k: &[u8; KEY]) -> Result<&'a Self, ()> {
        assert_eq!(size_of::<Self>(), CHALLENGE_LEN);
        let (c, t) = &mut buf[..].split_at_mut(CHALLENGE_LEN-HMAC);
        let t = unsafe { &*(t.as_ptr() as *const [u8; HMAC]) };
        open_chacha20poly1305(c, None, t, &nonce_from_u64(seq), k)?;
        let buf: *mut [u8; CHALLENGE_LEN] = buf;
        Ok(unsafe { &*(buf as *const Self) })
    }
}

#[test]
fn challenge_token() {
    let client_id = 0x1122334455667788;
    let seq = 0x1122334455667799;
    let key = crate::utils::keygen();
    let mut user = [0u8; USER];
    crate::utils::crypto_random(&mut user[..]);
    let tok = &mut ChallengeToken::new(client_id, user);
    let tok = ChallengeToken::seal(tok, seq, &key);
    let tok = ChallengeToken::open(tok, seq, &key).unwrap();

    assert_eq!(tok.client_id(), client_id);
    assert_eq!(&tok.user()[..], &user[..]);
    assert_eq!(tok._reserved, [0u8; 20]);
}
