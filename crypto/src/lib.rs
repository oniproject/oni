#![feature(
    int_to_from_bytes,
    wrapping_int_impl,
    decl_macro,
)]

pub mod aead;
pub mod chacha20;
pub mod poly1305;
pub mod hchacha20;

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

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;

pub type Nonce = [u8; NONCE];
pub type Xnonce = [u8; XNONCE];
pub type Key = [u8; KEY];
pub type Tag = [u8; HMAC];

use crate::chacha20::ChaCha20;
use crate::poly1305::Poly1305;

#[inline]
pub fn seal_chacha20poly1305(m: &mut [u8], ad: Option<&[u8]>, npub: &Nonce, key: &Key) -> Tag {
    let ad = ad.unwrap_or(&[]);
    let z = &mut [0u8; 64][..];
    ChaCha20::ietf(z, npub, 0, key);
    ChaCha20::ietf(m, npub, 1, key);
    let mut poly1305 = Poly1305::with_key(&z[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(m);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(m.len() as u64);
    poly1305.finish()
}

#[inline]
pub fn open_chacha20poly1305(c: &mut [u8], ad: Option<&[u8]>, tag: &Tag, npub: &Nonce, key: &Key)
    -> Result<(), ()>
{
    let ad = ad.unwrap_or(&[]);
    let z = &mut [0u8; 64][..];
    ChaCha20::ietf(z, npub, 0, key);
    let mut poly1305 = Poly1305::with_key(&z[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(c);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(c.len() as u64);
    if poly1305.finish_verify(tag) {
        ChaCha20::ietf(c, npub, 1, key);
        Ok(())
    } else {
        c.iter_mut().for_each(|v| *v = 0);
        Err(())
    }
}

#[inline]
pub fn seal_xchacha20poly1305(m: &mut [u8], ad: Option<&[u8]>, npub: &Xnonce, key: &Key) -> Tag {
    let (npub, key) = SafeNonce(*npub).split(key);
    seal_chacha20poly1305(m, ad, &npub, &key)
}

#[inline]
pub fn open_xchacha20poly1305(c: &mut [u8], ad: Option<&[u8]>, t: &Tag, npub: &Xnonce, key: &Key)
    -> Result<(), ()>
{
    let (npub, key) = SafeNonce(*npub).split(key);
    open_chacha20poly1305(c, ad, t, &npub, &key)
}

#[inline]
pub fn nonce_from_u64(sequence: u64) -> Nonce {
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
pub fn generate_nonce() -> Xnonce {
    SafeNonce::generate().0
}

pub struct SafeNonce(pub Xnonce);

impl SafeNonce {
    pub fn generate() -> Self {
        let mut nonce = [0u8; XNONCE];
        crypto_random(&mut nonce);
        SafeNonce(nonce)
    }
    pub fn split(&self, key: &Key) -> (Nonce, Key) {
        let mut input = [0u8; 16];
        let mut npub = [0u8; 12];
        input[..].copy_from_slice(&self.0[..16]);
        npub[4..].copy_from_slice(&self.0[16..]);
        (npub, crate::hchacha20::hchacha20(&input, key, None))
    }
}
