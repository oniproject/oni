use std::{
    ptr,
    os::raw::{
        c_uchar,
        c_ulonglong,
        c_int,
        c_void,
    },
};

pub use crate::utils::*;


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
