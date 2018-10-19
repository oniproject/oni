#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::poly1305::Poly1305;
use crate::chacha20::ChaCha20;

use std::slice::from_raw_parts;

pub fn seal(c: *mut u8, m: &[u8], ad: &[u8], npub: [u8; 8], k: &[u8; 32]) -> [u8; 16] {
    let mut block0 = [0u8; 64];
    ChaCha20::stream(&mut block0, npub, k);
    ChaCha20::stream_xor(c, m.as_ptr(), m.len() as u64, npub, 1, k);

    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update(ad);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update(unsafe { from_raw_parts(c, m.len()) });
    poly1305.update_u64(m.len() as u64);
    poly1305.finish()
}

pub fn seal_inplace(m: &mut [u8], ad: &[u8], npub: [u8; 8], k: &[u8; 32]) -> [u8; 16] {
    let mut block0 = [0u8; 64];
    ChaCha20::stream(&mut block0, npub, k);
    ChaCha20::stream_xor(m.as_mut_ptr(), m.as_ptr(), m.len() as u64, npub, 1, k);

    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update(ad);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update(m);
    poly1305.update_u64(m.len() as u64);
    poly1305.finish()
}

pub fn verify(c: &[u8], mac: &[u8; 16], ad: &[u8], npub: [u8; 8], k: &[u8; 32]) -> Result<(), ()> {
    let mut block0 = [0u8; 64];
    ChaCha20::stream(&mut block0, npub, k);

    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update(ad);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update(&c);
    poly1305.update_u64(c.len() as u64);
    if poly1305.finish_verify(mac) {
        Ok(())
    } else {
        Err(())
    }
}

pub fn open(m: &mut [u8], c: &[u8], mac: &[u8; 16], ad: &[u8], npub: [u8; 8], k: &[u8; 32]) -> Result<(), ()> {
    assert_eq!(m.len(), c.len());
    if let Err(()) = verify(c, mac, ad, npub, k) {
        m.iter_mut().for_each(|v| *v = 0);
        return Err(());
    } else {
        ChaCha20::stream_xor(m.as_mut_ptr(), c.as_ptr(), m.len() as u64, npub, 1, k);
        Ok(())
    }
}

pub fn ietf_seal(c: *mut u8, m: &[u8], ad: &[u8], npub: &[u8; 12], k: &[u8; 32]) -> [u8; 16] {
    let mut block0 = [0u8; 64];
    ChaCha20::stream_ietf(&mut block0, npub, k);
    ChaCha20::stream_ietf_xor(c, m.as_ptr(), m.len() as u64, npub, 1, k);

    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(unsafe { from_raw_parts(c, m.len()) });
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(m.len() as u64);
    poly1305.finish()
}

pub fn ietf_verify(c: &[u8], mac: &[u8; 16], ad: &[u8], npub: &[u8; 12], k: &[u8; 32]) -> Result<(), ()> {
    let mut block0 = [0u8; 64];
    ChaCha20::stream_ietf(&mut block0, npub, k);

    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(&c);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(c.len() as u64);
    if poly1305.finish_verify(mac) {
        Ok(())
    } else {
        Err(())
    }
}

pub fn ietf_open(m: &mut [u8], c: &[u8], mac: &[u8; 16], ad: &[u8], npub: &[u8; 12], k: &[u8; 32]) -> Result<(), ()> {
    assert_eq!(m.len(), c.len());
    if let Err(()) = ietf_verify(c, mac, ad, npub, k) {
        m.iter_mut().for_each(|v| *v = 0);
        return Err(());
    } else {
        ChaCha20::stream_ietf_xor(m.as_mut_ptr(), c.as_ptr(), m.len() as u64, npub, 1, k);
        Ok(())
    }
}
