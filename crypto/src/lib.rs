#![feature(
    int_to_from_bytes,
    wrapping_int_impl,
    decl_macro,
)]

pub mod chacha20;
pub mod poly1305;
pub mod verify;

pub mod aead;

pub mod hchacha20;

//pub mod subtle;

#[inline(always)]
fn memzero<T: Sized>(p: &mut T) {
    let p: *mut T = p;
    let p: *mut u8 = p as *mut u8;
    for i in 0..std::mem::size_of::<T>() {
        unsafe { p.add(i).write_volatile(0) }
    }
}

#[inline(always)]
fn memzero_slice(p: &mut [u8]) {
    let count = p.len();
    for i in 0..p.len() {
        unsafe { p.as_mut_ptr().add(i).write_volatile(0) }
    }
}

pub fn crypto_random(buf: &mut [u8]) {
    use rand::Rng;
    rand::thread_rng().fill(buf)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;

use crate::chacha20::ChaCha20;
use crate::poly1305::Poly1305;

#[inline]
pub fn seal_chacha20poly1305(m: &mut [u8], ad: Option<&[u8]>, npub: &[u8; NONCE], key: &[u8; KEY]) -> [u8; HMAC] {
    let ad = ad.unwrap_or(&[]);

    let mut block0 = [0u8; 64];
    ChaCha20::stream_ietf(&mut block0, npub, key);
    ChaCha20::stream_ietf_xor(m.as_mut_ptr(), m.as_ptr(), m.len() as u64, npub, 1, key);
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(m);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(m.len() as u64);
    poly1305.finish()
}

#[inline]
pub fn open_chacha20poly1305(c: &mut [u8], ad: Option<&[u8]>, tag: &[u8; HMAC], npub: &[u8; NONCE], key: &[u8; KEY]) -> Result<(), ()> {
    let ad = ad.unwrap_or(&[]);

    let mut block0 = [0u8; 64];
    ChaCha20::stream_ietf(&mut block0, npub, key);
    let mut poly1305 = Poly1305::with_key(&block0[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(c);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(c.len() as u64);

    if poly1305.finish_verify(tag) {
        ChaCha20::stream_ietf_xor(c.as_mut_ptr(), c.as_ptr(), c.len() as u64, npub, 1, key);
        Ok(())
    } else {
        c.iter_mut().for_each(|v| *v = 0);
        Err(())
    }
}

#[inline]
fn split(n: &[u8; XNONCE], key: &[u8; KEY]) -> ([u8; NONCE], [u8; KEY]) {
    let mut input = [0u8; 16];
    let mut npub = [0u8; 12];
    input[..].copy_from_slice(&n[..16]);
    npub[4..].copy_from_slice(&n[16..]);
    (npub, crate::hchacha20::hchacha20(&input, key, None))
}

#[inline]
pub fn seal_xchacha20poly1305(m: &mut [u8], ad: Option<&[u8]>, npub: &[u8; XNONCE], key: &[u8; KEY]) -> [u8; HMAC] {
    let (npub, key) = split(&npub, &key);
    seal_chacha20poly1305(m, ad, &npub, &key)
}

#[inline]
pub fn open_xchacha20poly1305(c: &mut [u8], ad: Option<&[u8]>, t: &[u8; HMAC], npub: &[u8; XNONCE], key: &[u8; KEY]) -> Result<(), ()> {

    let (npub, key) = split(&npub, &key);
    open_chacha20poly1305(c, ad, t, &npub, &key)
}

#[inline]
pub fn nonce_from_u64(sequence: u64) -> [u8; NONCE] {
    let mut n = [0u8; NONCE];
    n[0..8].copy_from_slice(&sequence.to_le_bytes()[..]);
    n
}


#[inline]
pub fn keygen() -> [u8; KEY] {
    let mut k = [0u8; KEY];
    crypto_random(&mut k);
    k
}

#[inline]
pub fn generate_nonce() -> [u8; XNONCE] {
    let mut nonce = [0u8; XNONCE];
    crypto_random(&mut nonce);
    nonce
}
