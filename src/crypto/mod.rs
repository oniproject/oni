pub mod aead;
mod chacha20;
mod poly1305;
mod hchacha20;

#[inline(always)]
fn memzero_slice(p: &mut [u8]) {
    for i in 0..p.len() {
        unsafe { p.as_mut_ptr().add(i).write_volatile(0) }
    }
}

/// Size of Key.
pub const KEY: usize = 32;

pub const HMAC: usize = 16;

/// Nonce size for ChaCha20Poly1305 IETF in bytes.
pub const NONCE: usize = 12;

/// Nonce size for XChaCha20Poly1305 IETF in bytes.
pub const XNONCE: usize = 24;

pub type Nonce = [u8; NONCE];
pub type Xnonce = [u8; XNONCE];
pub type Key = [u8; KEY];
pub type Tag = [u8; HMAC];

pub use self::chacha20::ChaCha20;
pub use self::poly1305::Poly1305;
pub use self::hchacha20::hchacha20;

/// Performs inplace encryption using ChaCha20Poly1305 IETF.
#[inline]
#[cfg(not(feature = "sodium"))]
pub fn seal(m: &mut [u8], ad: Option<&[u8]>, npub: &Nonce, key: &Key) -> Tag {
    let ad = ad.unwrap_or(&[]);
    let z = &mut [0u8; 64][..];
    ChaCha20::new_ietf(key, npub, 0).inplace(z);
    ChaCha20::new_ietf(key, npub, 1).inplace(m);
    let mut poly1305 = Poly1305::with_key(&z[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(m);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(m.len() as u64);
    poly1305.finish()
}

/// Performs inplace decryption using ChaCha20Poly1305 IETF.
#[inline]
#[cfg(not(feature = "sodium"))]
pub fn open(c: &mut [u8], ad: Option<&[u8]>, tag: &Tag, npub: &Nonce, key: &Key)
    -> Result<(), ()>
{
    let ad = ad.unwrap_or(&[]);
    let z = &mut [0u8; 64][..];
    ChaCha20::new_ietf(key, npub, 0).inplace(z);
    let mut poly1305 = Poly1305::with_key(&z[..32]);
    poly1305.update_pad(ad);
    poly1305.update_pad(c);
    poly1305.update_u64(ad.len() as u64);
    poly1305.update_u64(c.len() as u64);
    if poly1305.finish_verify(tag) {
        ChaCha20::new_ietf(key, npub, 1).inplace(c);
        Ok(())
    } else {
        c.iter_mut().for_each(|v| *v = 0);
        Err(())
    }
}

#[inline]
#[cfg(not(feature = "sodium"))]
pub fn xopen(c: &mut [u8], ad: &[u8], t: &Tag, n: &[u8; XNONCE], k: &[u8; KEY]) -> Result<(), ()> {
    let (n, k) = AutoNonce(*n).split(k);
    open(c, Some(ad), &t, &n, &k)
}

#[inline]
#[cfg(not(feature = "sodium"))]
pub fn xseal(m: &mut [u8], ad: &[u8], n: &[u8; XNONCE], k: &[u8; KEY]) -> Tag {
    let (n, k) = AutoNonce(*n).split(k);
    seal(m, Some(ad), &n, &k)
}

#[inline]
pub fn nonce_from_u64(sequence: u64) -> Nonce {
    let mut n = [0u8; NONCE];
    n[0..8].copy_from_slice(&sequence.to_le_bytes()[..]);
    n
}

pub fn crypto_random(buf: &mut [u8]) {
    use rand::Rng;
    rand::thread_rng().fill(buf)
}

#[inline]
pub fn keygen() -> [u8; KEY] {
    let mut k = [0u8; KEY];
    crypto_random(&mut k);
    k
}

pub struct AutoNonce(pub Xnonce);

impl AutoNonce {
    pub fn generate() -> Self {
        let mut nonce = [0u8; XNONCE];
        crypto_random(&mut nonce);
        AutoNonce(nonce)
    }
    pub fn split(&self, key: &Key) -> (Nonce, Key) {
        let mut input = [0u8; 16];
        let mut npub = [0u8; 12];
        input[..].copy_from_slice(&self.0[..16]);
        npub[4..].copy_from_slice(&self.0[16..]);
        (npub, hchacha20(&input, key, None))
    }
}

#[cfg(feature = "sodium")]
pub use self::sodium::{open, xopen, seal, xseal};

#[cfg(feature = "sodium")]
mod sodium {
    use std::os::raw::{c_int, c_uchar, c_ulonglong};
    use super::{HMAC, KEY, NONCE, XNONCE};

    #[link(name = "sodium")]
    extern "C" {
        fn crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
            c: *mut c_uchar,
            mac: *mut c_uchar,
            maclen_p: *mut c_ulonglong,
            m: *const c_uchar,
            mlen: c_ulonglong,
            ad: *const c_uchar,
            adlen: c_ulonglong,
            nsec: *const c_uchar,
            npub: *const c_uchar,
            k: *const c_uchar
        ) -> c_int;
        fn crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
            m: *mut c_uchar,
            nsec: *mut c_uchar,
            c: *const c_uchar,
            clen: c_ulonglong,
            mac: *const c_uchar,
            ad: *const c_uchar,
            adlen: c_ulonglong,
            npub: *const c_uchar,
            k: *const c_uchar
        ) -> c_int;
        fn crypto_aead_chacha20poly1305_ietf_encrypt_detached(
            c: *mut c_uchar,
            mac: *mut c_uchar,
            maclen_p: *mut c_ulonglong,
            m: *const c_uchar,
            mlen: c_ulonglong,
            ad: *const c_uchar,
            adlen: c_ulonglong,
            nsec: *const c_uchar,
            npub: *const c_uchar,
            k: *const c_uchar
        ) -> c_int;
        fn crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            m: *mut c_uchar,
            nsec: *mut c_uchar,
            c: *const c_uchar,
            clen: c_ulonglong,
            mac: *const c_uchar,
            ad: *const c_uchar,
            adlen: c_ulonglong,
            npub: *const c_uchar,
            k: *const c_uchar
        ) -> c_int;
    }

    pub fn open(c: &mut [u8], ad: Option<&[u8]>, t: &[u8; HMAC], n: &[u8; NONCE], k: &[u8; KEY]) -> Result<(), ()> {
        let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((0 as *const _, 0));
        let ret = unsafe {
            crypto_aead_chacha20poly1305_ietf_decrypt_detached(
                c.as_mut_ptr(),
                0 as *mut _,
                c.as_ptr(),
                c.len() as c_ulonglong,
                t.as_ptr(),
                ad_p,
                ad_len,
                n.as_ptr(),
                k.as_ptr()
            )
        };
        if ret == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn seal(m: &mut [u8], ad: Option<&[u8]>, n: &[u8; NONCE], k: &[u8; KEY]) -> [u8; HMAC] {
        let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((0 as *const _, 0));
        let mut tag = [0u8; HMAC];
        let mut maclen = HMAC as c_ulonglong;
        unsafe {
            crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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
            );
        }
        tag
    }

    pub fn xopen(c: &mut [u8], ad: &[u8], t: &[u8; HMAC], n: &[u8; XNONCE], k: &[u8; KEY]) -> Result<(), ()> {
        let (ad_p, ad_len) = (ad.as_ptr(), ad.len() as c_ulonglong);
        let ret = unsafe {
            crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
                c.as_mut_ptr(),
                0 as *mut _,
                c.as_ptr(),
                c.len() as c_ulonglong,
                t.as_ptr(),
                ad_p,
                ad_len,
                n.as_ptr(),
                k.as_ptr()
            )
        };
        if ret == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn xseal(m: &mut [u8], ad: &[u8], n: &[u8; XNONCE], k: &[u8; KEY]) -> [u8; HMAC] {
        let (ad_p, ad_len) = (ad.as_ptr(), ad.len() as c_ulonglong);
        let mut tag = [0u8; HMAC];
        let mut maclen = HMAC as c_ulonglong;
        unsafe {
            crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
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
            );
        }
        tag
    }
}
