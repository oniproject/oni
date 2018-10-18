#![allow(
    dead_code, mutable_transmutes, non_camel_case_types, non_snake_case, non_upper_case_globals,
    unused_mut
)]

use byteorder::{LE, ByteOrder};
use std::num::Wrapping;
use crate::{memzero, memzero_slice};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ChaCha20 {
    pub state: [u32; 16],
}

#[inline(always)]
unsafe fn load32_le(src: *const u8) -> Wrapping<u32> {
    Wrapping(u32::from_le_bytes(*(src as *const [u8; 4])))
}

#[inline(always)]
unsafe fn store32_le(dst: *mut u8, w: Wrapping<u32>) {
    let bytes = w.0.to_le_bytes();
    *dst.add(0) = bytes[0];
    *dst.add(1) = bytes[1];
    *dst.add(2) = bytes[2];
    *dst.add(3) = bytes[3];
}

macro qround($a:expr, $b:expr, $c:expr, $d:expr) {
    $a = $a + $b; $d = ($d ^ $a).rotate_left(16);
    $c = $c + $d; $b = ($b ^ $c).rotate_left(12);
    $a = $a + $b; $d = ($d ^ $a).rotate_left( 8);
    $c = $c + $d; $b = ($b ^ $c).rotate_left( 7);
}

const EXPAND16: &[u8; 16] = b"expand 16-byte k";
const EXPAND32: &[u8; 16] = b"expand 32-byte k";

impl ChaCha20 {
    pub const KEYBYTES: usize = 32;
    pub const NONCEBYTES: usize = 8;
    pub const IETF_NONCEBYTES: usize = 12;

    pub fn new() -> Self {
        Self { state: [0; 16] }
    }

    pub fn keysetup16(&mut self, mut k: &[u8; Self::KEYBYTES / 2]) {
        LE::read_u32_into(EXPAND16, &mut self.state[0..4]);
        LE::read_u32_into(k, &mut self.state[4..8]);
        LE::read_u32_into(k, &mut self.state[8..12]);
    }

    pub fn keysetup32(&mut self, mut k: &[u8; Self::KEYBYTES]) {
        LE::read_u32_into(EXPAND32, &mut self.state[0..4]);
        LE::read_u32_into(k, &mut self.state[4..12]);
    }

    pub fn ivsetup(&mut self, mut iv: &[u8; 8], counter: [u8; 8]) {
        LE::read_u32_into(&counter, &mut self.state[12..14]);
        LE::read_u32_into(iv, &mut self.state[14..16]);
    }

    pub fn ietf_ivsetup(&mut self, iv: &[u8; 12], counter: [u8; 4]) {
        LE::read_u32_into(&counter, &mut self.state[12..13]);
        LE::read_u32_into(iv, &mut self.state[13..16]);
    }

    pub fn x_ietf_ivsetup(&mut self, iv: &[u8; 16]) {
        LE::read_u32_into(iv, &mut self.state[12..16]);
    }

    pub unsafe fn encrypt_bytes(&mut self, mut m: *const u8, mut c: *mut u8, mut bytes: u64) {
        let mut ctarget: *mut u8 = 0 as *mut u8;
        if 0 == bytes { return; }

        let mut  j0 = Wrapping(self.state[ 0]);
        let mut  j1 = Wrapping(self.state[ 1]);
        let mut  j2 = Wrapping(self.state[ 2]);
        let mut  j3 = Wrapping(self.state[ 3]);
        let mut  j4 = Wrapping(self.state[ 4]);
        let mut  j5 = Wrapping(self.state[ 5]);
        let mut  j6 = Wrapping(self.state[ 6]);
        let mut  j7 = Wrapping(self.state[ 7]);
        let mut  j8 = Wrapping(self.state[ 8]);
        let mut  j9 = Wrapping(self.state[ 9]);
        let mut j10 = Wrapping(self.state[10]);
        let mut j11 = Wrapping(self.state[11]);
        let mut j12 = Wrapping(self.state[12]);
        let mut j13 = Wrapping(self.state[13]);
        let mut j14 = Wrapping(self.state[14]);
        let mut j15 = Wrapping(self.state[15]);
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

            x0  += j0;
            x1  += j1;
            x2  += j2;
            x3  += j3;
            x4  += j4;
            x5  += j5;
            x6  += j6;
            x7  += j7;
            x8  += j8;
            x9  += j9;
            x10 += j10;
            x11 += j11;
            x12 += j12;
            x13 += j13;
            x14 += j14;
            x15 += j15;

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

            j12 += Wrapping(1);
            if Wrapping(0) == j12 {
                j13 += Wrapping(1);
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
                self.state[12] = j12.0;
                self.state[13] = j13.0;
                return;
            } else {
                bytes = bytes.wrapping_sub(64);
                c = c.offset(64);
                m = m.offset(64)
            }
        }
    }

    pub fn stream(c: &mut [u8], n: &[u8; 8], k: &[u8; Self::KEYBYTES]) {
        memzero_slice(c);
        let clen = c.len() as u64;
        let c = c.as_mut_ptr();
        if 0 == clen {
            return;
        }
        let mut ctx = Self::new();
        ctx.keysetup32(k);
        ctx.ivsetup(n, [0; 8]);
        unsafe {
            ctx.encrypt_bytes(c, c, clen);
        }
        memzero(&mut ctx)
    }

    pub fn stream_xor_ic(mut c: *mut u8, mut m: *const u8, mut mlen: u64, n: &[u8; 8], mut ic: u64, k: &[u8; Self::KEYBYTES]) {
        if 0 == mlen { return; }

        let mut ctx = Self::new();
        let mut ic_bytes: [u8; 8] = [0; 8];
        LE::write_u64(&mut ic_bytes, ic);
        ctx.keysetup32(k);
        ctx.ivsetup(n, ic_bytes);
        unsafe {
            ctx.encrypt_bytes(m, c, mlen);
        }
        memzero(&mut ctx)
    }

    pub fn stream_ietf(c: &mut [u8], n: &[u8; 12], k: &[u8; Self::KEYBYTES]) {
        memzero_slice(c);
        let clen = c.len() as u64;
        let c = c.as_mut_ptr();
        if 0 == clen {
            return;
        }
        let mut ctx = Self::new();
        ctx.keysetup32(k);
        ctx.ietf_ivsetup(n, [0; 4]);
        unsafe {
            ctx.encrypt_bytes(c, c, clen);
        }
        memzero(&mut ctx)
    }

    pub fn stream_ietf_xor_ic(mut c: *mut u8, mut m: *const u8, mut mlen: u64, n: &[u8; 12], mut ic: u32, k: &[u8; Self::KEYBYTES]) {
        if 0 == mlen { return; }

        let mut ctx = Self::new();
        ctx.keysetup32(k);
        ctx.ietf_ivsetup(n, ic.to_le_bytes());
        unsafe {
            ctx.encrypt_bytes(m, c, mlen);
        }
        memzero(&mut ctx)
    }

    pub fn stream_ietf_xor(mut c: *mut u8, mut m: *const u8, mut mlen: u64, n: &[u8; 12], k: &[u8; Self::KEYBYTES]) {
        Self::stream_ietf_xor_ic(c, m, mlen, n, 0, k);
    }

    pub fn stream_xor(mut c: *mut u8, mut m: *const u8, mut mlen: u64, n: &[u8; 8], k: &[u8; Self::KEYBYTES]) {
        Self::stream_xor_ic(c, m, mlen, n, 0, k);
    }
}
