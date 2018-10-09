use std::os::raw::c_ulonglong;
use std::mem::transmute;
use crate::server::{
    KEY,
    HMAC,
    NONCE,
};

use super::{USER, CHALLENGE_LEN};

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

    pub fn encrypt(mut self, seq: u64, k: &[u8; KEY]) -> [u8; CHALLENGE_LEN]  {
        let mut n = [0u8; NONCE];
        n[0..8].copy_from_slice(&seq.to_le_bytes()[..]);

        let mut maclen = HMAC as c_ulonglong;

        let m = (&mut self) as *mut Self as *mut _;
        let tag = &mut self.hmac as *mut [u8; HMAC] as *mut _;

        unsafe {
            crate::utils::crypto_aead_chacha20poly1305_ietf_encrypt_detached(
                m,
                tag,
                &mut maclen,
                m, (CHALLENGE_LEN - HMAC) as c_ulonglong,
                0 as *const _, 0,
                0 as *mut _,
                n.as_ptr(), k.as_ptr(),
            );
            (m as *const [u8; CHALLENGE_LEN]).read()
        }
    }

    pub fn decrypt(mut cc: [u8; CHALLENGE_LEN], seq: u64, k: &[u8; KEY]) -> Result<Self, ()> {
        let mut n = [0u8; NONCE];
        n[0..8].copy_from_slice(&seq.to_le_bytes()[..]);

        let (c, t) = &mut cc[..].split_at_mut(CHALLENGE_LEN-HMAC);

        unsafe {
            let ret = crate::utils::crypto_aead_chacha20poly1305_ietf_decrypt_detached(
                c.as_mut_ptr(),
                0 as *mut _,
                c.as_ptr(),
                c.len() as c_ulonglong,
                t.as_ptr(),
                0 as *const _, 0,
                n.as_ptr(), k.as_ptr(),
            );
            if ret != 0 {
                Err(())
            } else {
                Ok(transmute(cc))
            }
        }
    }
}

#[test]
fn challenge_token() {
    assert_eq!(std::mem::size_of::<ChallengeToken>(), CHALLENGE_LEN);
    let client_id = 0x1122334455667788;
    let seq = 0x1122334455667799;
    let key = crate::utils::keygen();
    let mut user = [0u8; USER];
    crate::utils::crypto_random(&mut user[..]);
    let tok = ChallengeToken::new(client_id, user);
    let tok = ChallengeToken::encrypt(tok, seq, &key);
    let tok = ChallengeToken::decrypt(tok, seq, &key).unwrap();

    assert_eq!(tok.client_id(), client_id);
    assert_eq!(&tok.user()[..], &user[..]);
    assert_eq!(tok._reserved, [0u8; 20]);
}
