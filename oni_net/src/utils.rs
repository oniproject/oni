use generic_array::GenericArray;
use generic_array::typenum::U256;
use std::{
    time::SystemTime,
    os::raw::{
        c_uchar,
        c_ulonglong,
        c_int,
        c_void,
    },
};

crate fn time_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[doc(hidden)]
crate macro err_ret {
    ($e:expr) => {
        match $e {
            Ok(inner) => inner,
            Err(_) => return,
        }
    },
    ($e:expr, $r:expr) => {
        match $e {
            Ok(inner) => inner,
            Err(_) => return $r,
        }
    }
}

/*
#[doc(hidden)]
crate macro none_ret {
    ($e:expr) => {
        match $e {
            Some(inner) => inner,
            None => return,
        }
    },
    ($e:expr, $r:expr) => {
        match $e {
            Some(inner) => inner,
            None => return $r,
        }
    }
}
*/

#[doc(hidden)]
#[macro_export]
macro_rules! read_array {
    ($buffer:expr, $size:expr) => {{
        use std::io::Read;
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        $buffer.read_exact(&mut array[..])?;
        array
    }}
}

#[doc(hidden)]
#[macro_export]
macro_rules! read_array_ok {
    ($buffer:expr, $size:expr) => {{
        use std::io::Read;
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        $buffer.read_exact(&mut array[..]).ok()?;
        array
    }}
}

#[doc(hidden)]
#[macro_export]
macro_rules! read_array_unwrap {
    ($buffer:expr, $size:expr) => {{
        use std::io::Read;
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        $buffer.read_exact(&mut array[..]).unwrap();
        array
    }}
}

/*
#[doc(hidden)]
crate macro slice_to_array($slice:expr, $len:expr) {
    if $slice.len() == $len {
        let ptr = $slice.as_ptr() as *const [u8; $len];
        unsafe { Ok(*ptr) }
    } else {
        Err(())
    }
}

#[doc(hidden)]
crate macro cast_slice_to_array($slice:expr, $len:expr) {
    &*($slice.as_ptr() as *const [u8; $len])
}
*/

use crate::protocol::{
    KEY,
    XNONCE,
};

#[link(name = "sodium")]
extern "C" {
    crate fn crypto_aead_chacha20poly1305_ietf_encrypt_detached(
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

    crate fn crypto_aead_chacha20poly1305_ietf_decrypt_detached(
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


    crate fn crypto_aead_xchacha20poly1305_ietf_decrypt(
        m: *mut c_uchar, mlen_p: *mut c_ulonglong,
        nsec: *mut c_uchar,
        c: *const c_uchar, clen: c_ulonglong,
        ad: *const c_uchar, adlen: c_ulonglong,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;
    crate fn crypto_aead_xchacha20poly1305_ietf_encrypt(
        c: *mut c_uchar, clen_p: *mut c_ulonglong,
        m: *const c_uchar, mlen: c_ulonglong,
        ad: *const c_uchar, adlen: c_ulonglong,
        nsec: *const c_uchar,
        npub: *const c_uchar,
        k: *const c_uchar,
    ) -> c_int;

    crate fn randombytes_buf(buf: *mut c_void, size: usize);
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

crate struct ReplayProtection {
    seq: u64,
    bits: GenericArray<u8, U256>,
}

impl ReplayProtection {
    crate fn new() -> Self {
        Self {
            seq: 0,
            bits: GenericArray::default(),
        }
    }

    crate fn packet_already_received(&mut self, seq: u64) -> bool {
        let len = self.bits.len() as u64;
        if seq.wrapping_add(len) <= self.seq {
            return true;
        }
        if seq > self.seq {
            for bit in self.seq+1..seq+1 {
                let bit = (bit % len) as usize;
                unsafe { self.clear_unchecked(bit); }
            }
            if seq >= self.seq + len {
                self.bits = GenericArray::default();
            }
            self.seq = seq;
        }
        unsafe {
            let bit = (seq % len) as usize;
            let ret = self.get_unchecked(bit);
            self.set_unchecked(bit);
            ret
        }
    }

    #[inline(always)] unsafe fn get_unchecked(&self, bit: usize) -> bool {
        *self.bits.get_unchecked(bit >> 3) & (1 << (bit & 0b111)) != 0
    }
    #[inline(always)] unsafe fn set_unchecked(&mut self, bit: usize) {
        *self.bits.get_unchecked_mut(bit >> 3) |= 1 << (bit & 0b111);
    }
    #[inline(always)] unsafe fn clear_unchecked(&mut self, bit: usize) {
        *self.bits.get_unchecked_mut(bit >> 3) &= !(1 << (bit & 0b111));
    }
}

#[test]
fn replay_protection() {
    const SIZE: u64 = 256;
    const MAX: u64 = 4 * SIZE;


    let mut rp = ReplayProtection::new();

    assert_eq!(rp.seq, 0);

    for sequence in 0..MAX {
        assert!(!rp.packet_already_received(sequence),
        "The first time we receive packets, they should not be already received");
    }

    assert!(rp.packet_already_received(0),
    "Old packets outside buffer should be considered already received");

    for sequence in MAX - 10..MAX {
        assert!(rp.packet_already_received(sequence),
        "Packets received a second time should be flagged already received");
    }

    assert!(!rp.packet_already_received(MAX + SIZE),
    "Jumping ahead to a much higher sequence should be considered not already received");


    for sequence in 0..MAX {
        assert!(rp.packet_already_received(sequence),
        "Old packets should be considered already received");
    }
}
