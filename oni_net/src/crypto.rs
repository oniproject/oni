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

pub fn encrypt_aead(message: &mut [u8], additional: &[u8], nonce: &Nonce, key: &Key) -> io::Result<()> {
    unsafe {
        let mut encrypted_length = 0;
        let result = crypto_aead_chacha20poly1305_ietf_encrypt(
            message.as_mut_ptr(), &mut encrypted_length,
            message.as_mut_ptr(), message.len() as c_ulonglong,
            additional.as_ptr(), additional.len() as c_ulonglong,
            ptr::null(), nonce.as_ptr(), key.as_ptr());
        assert_eq!(encrypted_length as usize, message.len() + MAC_BYTES);
        if result != 0 {
            Err(io::Error::new(io::ErrorKind::InvalidData, "encrypt_aead"))
        } else {
            Ok(())
        }
    }
}
pub fn decrypt_aead(message: &mut [u8], additional: &[u8], nonce: &Nonce, key: &Key) -> io::Result<()> {
    unsafe {
        let mut decrypted_length = 0;
        let result = crypto_aead_chacha20poly1305_ietf_decrypt(
            message.as_mut_ptr(), &mut decrypted_length,
            ptr::null_mut(),
            message.as_mut_ptr(), message.len() as c_ulonglong,
            additional.as_ptr(), additional.len() as c_ulonglong,
            nonce.as_ptr(), key.as_ptr());
        assert_eq!(decrypted_length as usize, message.len() - MAC_BYTES);
        if result != 0 {
            Err(io::Error::new(io::ErrorKind::InvalidData, "encrypt_aead"))
        } else {
            Ok(())
        }
    }
}
