// TODO:: error handing

use byteorder::{LE, WriteBytesExt, ReadBytesExt};
use std::{
    io,
    ptr,
    os::raw::{
        c_uchar,
        c_ulonglong,
        c_int,
        c_void,
    },
};

pub const MAC_BYTES: usize = 16;

pub trait Cipher {
    const MAC: usize;
    const NONCE: usize;
    const KEY: usize;

    type Nonce;
    type Key;

    fn decrypt_aead(m: &mut [u8], add: &[u8], nonce: &Self::Nonce, key: &Self::Key) -> Result<(), ()>;
    fn encrypt_aead(m: &mut [u8], add: &[u8], nonce: &Self::Nonce, key: &Self::Key) -> Result<(), ()>;
}

#[allow(non_camel_case_types)]
pub struct chacha20_poly1305_ietf;

impl Cipher for chacha20_poly1305_ietf {
    const MAC: usize = 16;
    const NONCE: usize = 12;
    const KEY: usize = 32;

    type Nonce = [u8; 12];
    type Key = [u8; 32];

    fn encrypt_aead(m: &mut [u8], add: &[u8], nonce: &Self::Nonce, key: &Self::Key) -> Result<(), ()> {
        let mut len = 0;
        if 0 == unsafe {
            crypto_aead_chacha20poly1305_ietf_encrypt(
                m.as_mut_ptr(), &mut len,
                m.as_mut_ptr(), m.len() as c_ulonglong,
                add.as_ptr(), add.len() as c_ulonglong,
                ptr::null(), nonce.as_ptr(), key.as_ptr())
        } {
            assert_eq!(len as usize, m.len() + Self::MAC);
            Ok(())
        } else {
            Err(())
        }
    }
    fn decrypt_aead(m: &mut [u8], add: &[u8], nonce: &Self::Nonce, key: &Self::Key) -> Result<(), ()> {
        let mut len = 0;
        if 0 == unsafe {
            crypto_aead_chacha20poly1305_ietf_decrypt(
                m.as_mut_ptr(), &mut len,
                ptr::null_mut(),
                m.as_mut_ptr(), m.len() as c_ulonglong,
                add.as_ptr(), add.len() as c_ulonglong,
                nonce.as_ptr(), key.as_ptr())
        } {
            assert_eq!(len as usize, m.len() - Self::MAC);
            Ok(())
        } else {
            Err(())
        }
    }
}

#[link(name = "sodium")]
extern "C" {
    fn crypto_aead_chacha20poly1305_ietf_decrypt(
        m: *mut c_uchar,
        mlen_p: *mut c_ulonglong,
        nsec: *mut c_uchar,
        c: *const c_uchar,
        clen: c_ulonglong,
        ad: *const c_uchar,
        adlen: c_ulonglong,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;
    fn crypto_aead_chacha20poly1305_ietf_encrypt(
        c: *mut c_uchar,
        clen_p: *mut c_ulonglong,
        m: *const c_uchar,
        mlen: c_ulonglong,
        ad: *const c_uchar,
        adlen: c_ulonglong,
        nsec: *const c_uchar,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

make_rw!(
    struct Key;
    const KEY_BYTES = 32;
    trait ReadKey { read_key }
    trait WriteKey { write_key }
);

make_rw!(
    struct Nonce;
    const NONCE_BYTES = 12;
    trait ReadNonce { read_nonce }
    trait WriteNonce { write_nonce }
);

impl Key {
    pub fn generate() -> Self {
        let mut key = [0u8; KEY_BYTES];
        random_bytes(&mut key[..]);
        Key(key)
    }
}

impl Nonce {
    pub fn from_sequence(sequence: u64) -> Self {
        let mut nonce = [0u8; 12];
        {
            let mut p = &mut nonce[..];
            p.write_u32::<LE>(0).unwrap();
            p.write_u64::<LE>(sequence).unwrap();
        }
        Self::from(nonce)
    }
}

pub fn random_u64() -> u64 {
    let mut buf = [0u8; 4];
    random_bytes(&mut buf);
    (&buf[..]).read_u64::<LE>().unwrap()
}

pub fn random_bytes(buf: &mut [u8]) {
    unsafe {
        randombytes_buf(buf.as_mut_ptr() as *mut _, buf.len());
    }
}

pub fn encrypt_aead(m: &mut [u8], add: &[u8], nonce: &Nonce, key: &Key) -> io::Result<()> {
    chacha20_poly1305_ietf::encrypt_aead(m, add, &nonce.0, &key.0)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "encrypt_aead"))
}
pub fn decrypt_aead(m: &mut [u8], add: &[u8], nonce: &Nonce, key: &Key) -> io::Result<()> {
    chacha20_poly1305_ietf::decrypt_aead(m, add, &nonce.0, &key.0)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "decrypt_aead"))
}
