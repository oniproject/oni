use std::{
    ptr::{null, null_mut},
    os::raw::{
        c_uchar,
        c_ulonglong,
        c_int,
        c_void,
    },
};

pub const KEY: usize = 32;
pub const HMAC: usize = 16;
pub const NONCE: usize = 12;
pub const XNONCE: usize = 24;

#[link(name = "sodium")]
extern "C" {
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

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[inline]
pub fn nonce_from_u64(sequence: u64) -> [u8; NONCE] {
    let mut n = [0u8; NONCE];
    n[0..8].copy_from_slice(&sequence.to_le_bytes()[..]);
    n
}

#[inline]
pub fn seal_chacha20poly1305(m: &mut [u8], ad: Option<&[u8]>, n: &[u8; NONCE], k: &[u8; KEY]) -> [u8; HMAC] {
    let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((null(), 0));
    let mut tag = [0u8; HMAC];
    let mut maclen = HMAC as c_ulonglong;
    unsafe {
        let _ = crypto_aead_chacha20poly1305_ietf_encrypt_detached(
            m.as_mut_ptr(),
            tag.as_mut_ptr(),
            &mut maclen,
            m.as_ptr(),
            m.len() as c_ulonglong,
            ad_p,
            ad_len,
            null_mut(),
            n.as_ptr(),
            k.as_ptr()
        );
    }
    tag
}

#[inline]
pub fn open_chacha20poly1305(c: &mut [u8], ad: Option<&[u8]>, t: &[u8; HMAC], n: &[u8; NONCE], k: &[u8; KEY]) -> Result<(), ()> {
    let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((null(), 0));
    let ret = unsafe {
        crypto_aead_chacha20poly1305_ietf_decrypt_detached(
            c.as_mut_ptr(),
            null_mut(),
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

#[inline]
pub fn seal_xchacha20poly1305(m: &mut [u8], ad: Option<&[u8]>, n: &[u8; XNONCE], k: &[u8; KEY]) -> [u8; HMAC] {
    let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((null(), 0));
    let mut tag = [0u8; HMAC];
    let mut maclen = HMAC as c_ulonglong;
    unsafe {
        let _ = crypto_aead_xchacha20poly1305_ietf_encrypt_detached(
            m.as_mut_ptr(),
            tag.as_mut_ptr(),
            &mut maclen,
            m.as_ptr(),
            m.len() as c_ulonglong,
            ad_p,
            ad_len,
            null_mut(),
            n.as_ptr(),
            k.as_ptr()
        );
    }
    tag
}

#[inline]
pub fn open_xchacha20poly1305(c: &mut [u8], ad: Option<&[u8]>, t: &[u8; HMAC], n: &[u8; XNONCE], k: &[u8; KEY]) -> Result<(), ()> {
    let (ad_p, ad_len) = ad.map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong)).unwrap_or((null(), 0));
    let ret = unsafe {
        crypto_aead_xchacha20poly1305_ietf_decrypt_detached(
            c.as_mut_ptr(),
            null_mut(),
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

#[inline]
pub fn crypto_random(buf: &mut [u8]) {
    unsafe {
        randombytes_buf(buf.as_mut_ptr() as *mut c_void, buf.len());
    }
}
