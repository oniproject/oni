use std::{
    ptr,
    os::raw::{
        c_uchar,
        c_ulonglong,
        c_int,
        c_void,
    },
};

pub const KEYBYTES: usize = 32;
pub const NPUBBYTES: usize = 12;
pub const ABYTES: usize = 16;

pub const BIGNONCE: usize = 24;

#[link(name = "sodium")]
extern "C" {
    fn crypto_aead_chacha20poly1305_ietf_decrypt(
        m: *mut c_uchar, mlen_p: *mut c_ulonglong,
        nsec: *mut c_uchar,
        c: *const c_uchar, clen: c_ulonglong,
        ad: *const c_uchar, adlen: c_ulonglong,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;
    fn crypto_aead_chacha20poly1305_ietf_encrypt(
        c: *mut c_uchar, clen_p: *mut c_ulonglong,
        m: *const c_uchar, mlen: c_ulonglong,
        ad: *const c_uchar, adlen: c_ulonglong,
        nsec: *const c_uchar,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;

    fn crypto_aead_xchacha20poly1305_ietf_decrypt(
        m: *mut c_uchar, mlen_p: *mut c_ulonglong,
        nsec: *mut c_uchar,
        c: *const c_uchar, clen: c_ulonglong,
        ad: *const c_uchar, adlen: c_ulonglong,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;
    fn crypto_aead_xchacha20poly1305_ietf_encrypt(
        c: *mut c_uchar, clen_p: *mut c_ulonglong,
        m: *const c_uchar, mlen: c_ulonglong,
        ad: *const c_uchar, adlen: c_ulonglong,
        nsec: *const c_uchar,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;

    fn crypto_aead_chacha20poly1305_keygen(k: *mut c_uchar);

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[inline]
pub fn keygen() -> [u8; KEYBYTES] {
    let mut k = [0u8; KEYBYTES];
    unsafe {
        crypto_aead_chacha20poly1305_keygen(k.as_mut_ptr());
    }
    k
}

#[inline]
pub fn randbuf(buf: &mut [u8]) {
    unsafe {
        randombytes_buf(buf.as_mut_ptr() as *mut c_void, buf.len());
    }
}

#[inline]
pub fn generate_nonce() -> [u8; 24] {
    let mut nonce = [0u8; 24];
    randbuf(&mut nonce[..]);
    nonce
}

#[inline]
pub fn seal(m: &mut [u8], ad: Option<&[u8]>, nonce: &[u8; NPUBBYTES], key: &[u8; KEYBYTES]) -> Result<(), ()> {
    let (ad_p, ad_len) = ad
        .map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong))
        .unwrap_or((ptr::null(), 0));

    let mut len = 0;
    if 0 == unsafe {
        crypto_aead_chacha20poly1305_ietf_encrypt(
            m.as_mut_ptr(), &mut len,
            m.as_mut_ptr(), m.len() as c_ulonglong,
            ad_p, ad_len,
            ptr::null(), nonce.as_ptr(), key.as_ptr())
    } {
        assert_eq!(len as usize, m.len() + ABYTES);
        Ok(())
    } else {
        Err(())
    }
}

#[inline]
pub fn open(m: &mut [u8], ad: Option<&[u8]>, nonce: &[u8; NPUBBYTES], key: &[u8; KEYBYTES]) -> Result<(), ()> {
    let (ad_p, ad_len) = ad
        .map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong))
        .unwrap_or((ptr::null(), 0));

    let mut len = 0;
    if 0 == unsafe {
        crypto_aead_chacha20poly1305_ietf_decrypt(
            m.as_mut_ptr(), &mut len,
            ptr::null_mut(),
            m.as_mut_ptr(), m.len() as c_ulonglong,
            ad_p, ad_len,
            nonce.as_ptr(), key.as_ptr())
    } {
        assert_eq!(len as usize, m.len() - ABYTES);
        Ok(())
    } else {
        Err(())
    }
}

#[inline]
pub fn x_seal(m: &mut [u8], ad: Option<&[u8]>, nonce: &[u8; BIGNONCE], key: &[u8; KEYBYTES]) -> Result<(), ()> {
    let (ad_p, ad_len) = ad
        .map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong))
        .unwrap_or((ptr::null(), 0));

    let mut len = 0;
    if 0 == unsafe {
        crypto_aead_xchacha20poly1305_ietf_encrypt(
            m.as_mut_ptr(), &mut len,
            m.as_mut_ptr(), m.len() as c_ulonglong,
            ad_p, ad_len,
            ptr::null(), nonce.as_ptr(), key.as_ptr())
    } {
        assert_eq!(len as usize, m.len() + ABYTES);
        Ok(())
    } else {
        Err(())
    }
}

#[inline]
pub fn x_open(m: &mut [u8], ad: Option<&[u8]>, nonce: &[u8; BIGNONCE], key: &[u8; KEYBYTES]) -> Result<(), ()> {
    let (ad_p, ad_len) = ad
        .map(|ad| (ad.as_ptr(), ad.len() as c_ulonglong))
        .unwrap_or((ptr::null(), 0));

    let mut len = 0;
    if 0 == unsafe {
        crypto_aead_xchacha20poly1305_ietf_decrypt(
            m.as_mut_ptr(), &mut len,
            ptr::null_mut(),
            m.as_mut_ptr(), m.len() as c_ulonglong,
            ad_p, ad_len,
            nonce.as_ptr(), key.as_ptr())
    } {
        assert_eq!(len as usize, m.len() - ABYTES);
        Ok(())
    } else {
        Err(())
    }
}
