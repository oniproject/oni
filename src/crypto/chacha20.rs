#![allow(clippy::not_unsafe_ptr_arg_deref)]

use byteorder::{LE, ByteOrder};
use std::ptr;
use super::memzero_slice;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ChaCha20 {
    pub state: [u32; 16],
}

#[inline(always)]
unsafe fn load32_le(src: *const u8) -> u32 {
    u32::from_le_bytes(*(src as *const [u8; 4]))
}

#[inline(always)]
unsafe fn store32_le(dst: *mut u8, w: u32) {
    let bytes = w.to_le_bytes();
    *dst.add(0) = bytes[0];
    *dst.add(1) = bytes[1];
    *dst.add(2) = bytes[2];
    *dst.add(3) = bytes[3];
}

macro qround($a:expr, $b:expr, $c:expr, $d:expr) {
    $a = $a.wrapping_add($b); $d = ($d ^ $a).rotate_left(16);
    $c = $c.wrapping_add($d); $b = ($b ^ $c).rotate_left(12);
    $a = $a.wrapping_add($b); $d = ($d ^ $a).rotate_left( 8);
    $c = $c.wrapping_add($d); $b = ($b ^ $c).rotate_left( 7);
}

//const K16: &[u8; 16] = b"expand 16-byte k";
const K32: &[u8; 16] = b"expand 32-byte k";

impl ChaCha20 {
    pub const KEYBYTES: usize = 32;
    pub const NONCEBYTES: usize = 8;
    pub const IETF_NONCEBYTES: usize = 12;

    pub fn new(exp: &[u8; 16], key: &[u8; Self::KEYBYTES], iv: &[u8; 16]) -> Self {
        let mut state = [0u32; 16];
        LE::read_u32_into(exp, &mut state[ 0.. 4]);
        LE::read_u32_into(key, &mut state[ 4..12]);
        LE::read_u32_into( iv, &mut state[12..16]);
        Self { state }
    }

    pub fn new_basic(key: &[u8; Self::KEYBYTES], iv: [u8; Self::NONCEBYTES], ic: u64) -> Self {
        let ic = &ic.to_le_bytes();
        let mut state = [0u32; 16];
        LE::read_u32_into(K32, &mut state[ 0.. 4]);
        LE::read_u32_into(key, &mut state[ 4..12]);
        LE::read_u32_into( ic, &mut state[12..14]);
        LE::read_u32_into(&iv, &mut state[14..16]);
        Self { state }
    }

    pub fn new_ietf(key: &[u8; Self::KEYBYTES], iv: &[u8; Self::IETF_NONCEBYTES], ic: u32) -> Self {
        let ic = &ic.to_le_bytes();
        let mut state = [0u32; 16];
        LE::read_u32_into(K32, &mut state[ 0.. 4]);
        LE::read_u32_into(key, &mut state[ 4..12]);
        LE::read_u32_into( ic, &mut state[12..13]);
        LE::read_u32_into( iv, &mut state[13..16]);
        Self { state }
    }

    pub fn stream(c: &mut [u8], n: [u8; 8], k: &[u8; Self::KEYBYTES]) {
        memzero_slice(c);
        Self::stream_xor(c.as_mut_ptr(), c.as_ptr(), c.len() as u64, n, 0, k)
    }

    pub fn stream_xor(c: *mut u8, m: *const u8, mlen: u64, n: [u8; 8], ic: u64, k: &[u8; Self::KEYBYTES]) {
        if 0 == mlen { return; }
        let mut ctx = Self::new_basic(k, n, ic);
        unsafe {
            ctx.encrypt_bytes(m, c, mlen);
        }
    }

    pub fn stream_ietf(c: &mut [u8], n: &[u8; 12], k: &[u8; Self::KEYBYTES]) {
        memzero_slice(c);
        Self::stream_ietf_xor(c.as_mut_ptr(), c.as_ptr(), c.len() as u64, n, 0, k)
    }

    pub fn ietf(m: &mut [u8], n: &[u8; 12], ic: u32, k: &[u8; Self::KEYBYTES]) {
        Self::stream_ietf_xor(m.as_mut_ptr(), m.as_ptr(), m.len() as u64, n, ic, k)
    }

    pub fn stream_ietf_xor(c: *mut u8, m: *const u8, mlen: u64, n: &[u8; 12], ic: u32, k: &[u8; Self::KEYBYTES]) {
        if 0 == mlen { return; }

        let mut ctx = Self::new_ietf(k, n, ic);
        unsafe {
            ctx.encrypt_bytes(m, c, mlen);
        }
    }

    pub fn inplace(&mut self, m: &mut [u8]) {
        if m.is_empty() { return; }
        unsafe {
            self.encrypt_bytes(m.as_ptr(), m.as_mut_ptr(), m.len() as u64)
        }
    }


    unsafe fn encrypt_bytes(&mut self, mut m: *const u8, mut c: *mut u8, mut bytes: u64) {
        let mut ctarget: *mut u8 = ptr::null_mut();
        if 0 == bytes { return; }

        let      j0 = self.state[ 0];
        let      j1 = self.state[ 1];
        let      j2 = self.state[ 2];
        let      j3 = self.state[ 3];
        let      j4 = self.state[ 4];
        let      j5 = self.state[ 5];
        let      j6 = self.state[ 6];
        let      j7 = self.state[ 7];
        let      j8 = self.state[ 8];
        let      j9 = self.state[ 9];
        let     j10 = self.state[10];
        let     j11 = self.state[11];
        let mut j12 = self.state[12];
        let mut j13 = self.state[13];
        let     j14 = self.state[14];
        let     j15 = self.state[15];

        let mut tmp = [0u8; 64];
        loop {
            if bytes < 64 {
                for i in 0..bytes {
                    tmp[i as usize] = *m.offset(i as isize);
                }
                m = tmp.as_mut_ptr();
                ctarget = c;
                c = tmp.as_mut_ptr()
            }

            let mut x0 = j0;
            let mut x1 = j1;
            let mut x2 = j2;
            let mut x3 = j3;
            let mut x4 = j4;
            let mut x5 = j5;
            let mut x6 = j6;
            let mut x7 = j7;
            let mut x8 = j8;
            let mut x9 = j9;
            let mut x10 = j10;
            let mut x11 = j11;
            let mut x12 = j12;
            let mut x13 = j13;
            let mut x14 = j14;
            let mut x15 = j15;

            // 10 loops × 2 rounds/loop = 20 rounds
            for _ in 0..10 {
                // odd round
                qround!(x0, x4,  x8, x12); // column 0
                qround!(x1, x5,  x9, x13); // column 1
                qround!(x2, x6, x10, x14); // column 2
                qround!(x3, x7, x11, x15); // column 3
                // even round
                qround!(x0, x5, x10, x15); // diagonal 1 (main diagonal)
                qround!(x1, x6, x11, x12); // diagonal 2
                qround!(x2, x7,  x8, x13); // diagonal 3
                qround!(x3, x4,  x9, x14); // diagonal 4
            }

            x0  = x0 .wrapping_add(j0 );
            x1  = x1 .wrapping_add(j1 );
            x2  = x2 .wrapping_add(j2 );
            x3  = x3 .wrapping_add(j3 );
            x4  = x4 .wrapping_add(j4 );
            x5  = x5 .wrapping_add(j5 );
            x6  = x6 .wrapping_add(j6 );
            x7  = x7 .wrapping_add(j7 );
            x8  = x8 .wrapping_add(j8 );
            x9  = x9 .wrapping_add(j9 );
            x10 = x10.wrapping_add(j10);
            x11 = x11.wrapping_add(j11);
            x12 = x12.wrapping_add(j12);
            x13 = x13.wrapping_add(j13);
            x14 = x14.wrapping_add(j14);
            x15 = x15.wrapping_add(j15);

            x0  ^= load32_le(m.add( 0));
            x1  ^= load32_le(m.add( 4));
            x2  ^= load32_le(m.add( 8));
            x3  ^= load32_le(m.add(12));
            x4  ^= load32_le(m.add(16));
            x5  ^= load32_le(m.add(20));
            x6  ^= load32_le(m.add(24));
            x7  ^= load32_le(m.add(28));
            x8  ^= load32_le(m.add(32));
            x9  ^= load32_le(m.add(36));
            x10 ^= load32_le(m.add(40));
            x11 ^= load32_le(m.add(44));
            x12 ^= load32_le(m.add(48));
            x13 ^= load32_le(m.add(52));
            x14 ^= load32_le(m.add(56));
            x15 ^= load32_le(m.add(60));

            j12 = j12.wrapping_add(1);
            if 0 == j12 {
                j13 = j13.wrapping_add(1);
            }

            store32_le(c.add( 0), x0);
            store32_le(c.add( 4), x1);
            store32_le(c.add( 8), x2);
            store32_le(c.add(12), x3);
            store32_le(c.add(16), x4);
            store32_le(c.add(20), x5);
            store32_le(c.add(24), x6);
            store32_le(c.add(28), x7);
            store32_le(c.add(32), x8);
            store32_le(c.add(36), x9);
            store32_le(c.add(40), x10);
            store32_le(c.add(44), x11);
            store32_le(c.add(48), x12);
            store32_le(c.add(52), x13);
            store32_le(c.add(56), x14);
            store32_le(c.add(60), x15);

            if bytes <= 64 {
                if bytes < 64 {
                    for i in 0..bytes {
                        // ctarget cannot be NULL
                        *ctarget.offset(i as isize) = *c.offset(i as isize);
                    }
                }
                self.state[12] = j12;
                self.state[13] = j13;
                return;
            } else {
                bytes = bytes.wrapping_sub(64);
                c = c.offset(64);
                m = m.offset(64)
            }
        }
    }
}
