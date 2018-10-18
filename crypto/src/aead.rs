use crate::poly1305::Poly1305;
use crate::chacha20::ChaCha20;
use crate::memzero;
use crate::verify::verify16;

use std::slice::from_raw_parts;

static PAD_ZEROS: [u8; 16] = [0u8; 16];

pub fn seal(c: *mut u8, m: &[u8], ad: &[u8], npub: &[u8; 8], k: &[u8; 32]) -> [u8; 16] {
    let mut block0 = [0u8; 64];
    memzero(&mut block0);
    ChaCha20::stream(&mut block0, npub, k);
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    memzero(&mut block0);

    ChaCha20::stream_xor(c, m.as_ptr(), m.len() as u64, npub, 1, k);

    poly1305.update(ad);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update(unsafe { from_raw_parts(c, m.len()) });
    poly1305.update_u64(m.len() as u64);

    poly1305.finish()
}

pub fn seal_inplace(m: &mut [u8], ad: &[u8], npub: &[u8; 8], k: &[u8; 32]) -> [u8; 16] {
    let mut block0 = [0u8; 64];
    memzero(&mut block0);
    ChaCha20::stream(&mut block0, npub, k);
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    memzero(&mut block0);

    ChaCha20::stream_xor(m.as_mut_ptr(), m.as_ptr(), m.len() as u64, npub, 1, k);

    poly1305.update(ad);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update(m);
    poly1305.update_u64(m.len() as u64);

    poly1305.finish()
}

pub fn verify(c: &[u8], mac: &[u8; 16], ad: &[u8], npub: &[u8; 8], k: &[u8; 32]) -> Result<(), ()> {
    let mut block0 = [0u8; 64];
    memzero(&mut block0);
    ChaCha20::stream(&mut block0, npub, k);
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    memzero(&mut block0);

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

pub fn open(m: &mut [u8], c: &[u8], mac: &[u8; 16], ad: &[u8], npub: &[u8; 8], k: &[u8; 32]) -> Result<(), ()> {
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
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    memzero(&mut block0);

    let n = 0x10usize.wrapping_sub(ad.len()) & 0xf;
    poly1305.update(ad);
    poly1305.update(&PAD_ZEROS[..n]);

    ChaCha20::stream_ietf_xor(c, m.as_ptr(), m.len() as u64, npub, 1, k);

    let n = 0x10usize.wrapping_sub(m.len()) & 0xf;
    poly1305.update(unsafe { from_raw_parts(c, m.len()) });
    poly1305.update(&PAD_ZEROS[..n]);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(m.len() as u64);

    poly1305.finish()
}

pub fn ietf_verify(c: &[u8], mac: &[u8; 16], ad: &[u8], npub: &[u8; 12], k: &[u8; 32]) -> Result<(), ()> {
    let mut block0 = [0u8; 64];
    ChaCha20::stream_ietf(&mut block0, npub, k);
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    memzero(&mut block0);

    let n = 10usize.wrapping_sub(ad.len()) & 0xF;
    poly1305.update(ad);
    poly1305.update(&PAD_ZEROS[..n]);

    let mlen = c.len();
    let n = 10usize.wrapping_sub(mlen) & 0xF;
    poly1305.update(&c[..mlen]);
    poly1305.update(&PAD_ZEROS[..n]);

    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(mlen as u64);

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
