#![allow(clippy::cast_lossless)]

use byteorder::{LE, ByteOrder};
use std::mem::size_of;
use crate::memzero;

#[must_use]
fn verify16(x: &[u8; 16], y: &[u8; 16]) -> bool {
    let mut d = 0u16;
    for i in 0..16 {
        d |= u16::from(x[i] ^ y[i])
    }
    (1 & ((d.wrapping_sub(1)) >> 8)).wrapping_sub(1) == 0
}

struct DonnaState64 {
    r: [u64; 3],
    h: [u64; 3],
}

const BLOCK_SIZE: usize = 16;
static PAD_ZEROS: [u8; 16] = [0u8; 16];

#[repr(align(64))]
pub struct Poly1305 {
    // TODO 32-bits version
    // 17 + sizeof(u64) + 8 * sizeof(u64)
    state: DonnaState64,
    pad: [u64; 2],
    leftover: usize,
    buffer: [u8; BLOCK_SIZE],
}

impl Poly1305 {
    pub const BYTES: usize = 16;
    pub const KEYBYTES: usize = 32;

    pub const fn statebytes() -> usize { size_of::<Self>() }
    pub const fn bytes() -> usize { Self::BYTES }
    pub const fn keybytes() -> usize { Self::KEYBYTES }

    pub fn new() -> Self {
        unsafe { std::mem::zeroed() }
    }

    pub fn with_key(key: &[u8]) -> Self {
        assert!(key.len() == 32);
        let key = unsafe { &*(key.as_ptr() as *const [u8; Self::KEYBYTES]) };
        let mut poly1305 = Self::new();
        poly1305.init(key);
        poly1305
    }

    pub fn sum(m: &[u8], k: &[u8; Self::KEYBYTES]) -> [u8; Self::BYTES] {
        let mut state = Self::new();
        state.init(k);
        state.update_donna(m);
        state.finish()
    }

    #[must_use]
    pub fn verify(h: &[u8; Self::BYTES], m: &[u8], k: &[u8; Self::KEYBYTES]) -> bool {
        verify16(h, &Self::sum(m, k))
    }

    pub fn init(&mut self, key: &[u8; Self::KEYBYTES]) {
        self.leftover = 0;
        self.init64(key)
    }

    pub fn finish(mut self) -> [u8; Self::BYTES] {
        let mut out = [0u8; 16];
        self.finish64(&mut out, self.leftover);
        memzero(&mut self);
        out
    }

    #[must_use]
    pub fn finish_verify(self, mac: &[u8; Self::BYTES]) -> bool {
        let mut computed_mac = self.finish();
        let ret = verify16(&computed_mac, &mac);
        memzero(&mut computed_mac);
        ret
    }

    pub fn update(&mut self, input: &[u8]) {
        self.update_donna(input)
    }

    pub fn update_pad(&mut self, input: &[u8]) {
        let n = 0x10usize.wrapping_sub(input.len()) & 0xf;
        self.update(input);
        self.update(&PAD_ZEROS[..n]);
    }

    pub fn update_u64(&mut self, input: u64) {
        self.update(&input.to_le_bytes());
    }

    pub fn keygen(k: &mut [u8; Self::KEYBYTES]) {
        unimplemented!("Poly1305::keygen")
    }

    pub fn update_donna(&mut self, mut m: &[u8]) {
        // handle leftover
        if self.leftover != 0 {
            let want = (BLOCK_SIZE - self.leftover).min(m.len());

            let p = self.leftover;
            self.buffer[p..p+want].copy_from_slice(&m[..want]);
            m = &m[want..];
            self.leftover += want;

            if self.leftover < BLOCK_SIZE {
                return;
            }
            self.state.blocks(&self.buffer[..BLOCK_SIZE], false);
            self.leftover = 0;
        }

        // process full blocks
        if m.len() >= BLOCK_SIZE {
            let want = m.len() & !(BLOCK_SIZE - 1);
            self.state.blocks(&m[..want], false);
            m = &m[want..];
        }

        // store leftover
        if !m.is_empty() {
            let p = self.leftover;
            self.buffer[p..p+m.len()].copy_from_slice(m);
            self.leftover += m.len();
        }
    }

    pub fn init64(&mut self, key: &[u8; 32]) {
        // r &= 0xffffffc0ffffffc0ffffffc0fffffff
        let t0 = LE::read_u64(&key[0..8]);
        let t1 = LE::read_u64(&key[8..16]);
        // wiped after finalization
        self.state.r[0] = t0 & 0x0ffc_0fff_ffff;
        self.state.r[1] = (t0 >> 44 | t1 << 20) & 0x0fff_ffc0_ffff;
        self.state.r[2] = t1 >> 24 & 0x000f_ffff_fc0f;
        // h = 0
        self.state.h[0] = 0;
        self.state.h[1] = 0;
        self.state.h[2] = 0;
        // save pad for later
        self.pad[0] = LE::read_u64(&key[16..24]);;
        self.pad[1] = LE::read_u64(&key[24..32]);
    }

    pub fn finish64(&mut self, mac: &mut [u8; 16], leftover: usize) {
        let mut h0: u64;
        let mut h1: u64;
        let mut h2: u64;
        let mut  c: u64;
        let mut g0: u64;
        let mut g1: u64;
        let mut g2: u64;

        // process the remaining block
        if 0 != leftover {
            let mut i = leftover;
            self.buffer[i] = 1;
            i += 1;
            for i in i..16 {
                self.buffer[i] = 0;
            }
            self.state.blocks(&self.buffer[..16], true);
        }

        // fully carry h
        h0 = self.state.h[0];
        h1 = self.state.h[1];
        h2 = self.state.h[2];
        c = h1 >> 44;
        h1 &= 0x0fff_ffff_ffff;
        h2 = h2.wrapping_add(c);
        c = h2 >> 42;
        h2 &= 0x03ff_ffff_ffff;
        h0 = h0.wrapping_add(c.wrapping_mul(5));
        c = h0 >> 44;
        h0 &= 0x0fff_ffff_ffff;
        h1 = h1.wrapping_add(c);
        c = h1 >> 44;
        h1 &= 0x0fff_ffff_ffff;
        h2 = h2.wrapping_add(c);
        c = h2 >> 42;
        h2 &= 0x03ff_ffff_ffff;
        h0 = h0.wrapping_add(c.wrapping_mul(5));
        c = h0 >> 44;
        h0 &= 0x0fff_ffff_ffff;
        h1 = h1.wrapping_add(c);

        // compute h + -p
        g0 = h0.wrapping_add(5);
        c = g0 >> 44;
        g0 &= 0x0fff_ffff_ffff;
        g1 = h1.wrapping_add(c);
        c = g1 >> 44;
        g1 &= 0x0fff_ffff_ffff;
        g2 = h2.wrapping_add(c).wrapping_sub(1 << 42);

        // select h if h < p, or h + -p if h >= p
        c = (g2 >> (size_of::<u64>() as u32).wrapping_mul(8).wrapping_sub(1)).wrapping_sub(1);
        g0 &= c;
        g1 &= c;
        g2 &= c;
        c = !c;
        h0 = h0 & c | g0;
        h1 = h1 & c | g1;
        h2 = h2 & c | g2;

        // h = (h + pad)
        let (t0, t1) = (self.pad[0], self.pad[1]);
        h0 = h0.wrapping_add(t0 & 0x0fff_ffff_ffff);
        c = h0 >> 44;
        h0 &= 0x0fff_ffff_ffff;
        h1 = h1.wrapping_add(((t0 >> 44 | t1 << 20) & 0x0fff_ffff_ffff).wrapping_add(c));
        c = h1 >> 44;
        h1 &= 0x0fff_ffff_ffff;
        h2 = h2.wrapping_add((t1 >> 24 & 0x03ff_ffff_ffff).wrapping_add(c));
        h2 &= 0x03ff_ffff_ffff;

        // mac = h % (2^128)
        h0 |= h1 << 44;
        h1 = h1 >> 20 | h2 << 24;

        LE::write_u64(&mut mac[..8], h0);
        LE::write_u64(&mut mac[8..], h1);
    }
}

impl DonnaState64 {
    fn blocks(&mut self, mut m: &[u8], final_0: bool) {
        let hibit: u64 = if final_0 { 0 } else { 1 << 40 };

        // 1 << 128
        let r0: u64 = self.r[0];
        let r1: u64 = self.r[1];
        let r2: u64 = self.r[2];
        let s1: u64 = r1.wrapping_mul(5 << 2);
        let s2: u64 = r2.wrapping_mul(5 << 2);
        let mut h0: u64 = self.h[0];
        let mut h1: u64 = self.h[1];
        let mut h2: u64 = self.h[2];
        let mut c : u64;
        let mut d0: u128;
        let mut d1: u128;
        let mut d2: u128;
        let mut d : u128;

        while m.len() >= 16 {
            // h += m[i]
            let t0 = LE::read_u64(&m[..8]);
            let t1 = LE::read_u64(&m[8..]);
            h0 = h0.wrapping_add(t0 & 0x0fff_ffff_ffff);
            h1 = h1.wrapping_add((t0 >> 44 | t1 << 20) & 0x0fff_ffff_ffff);
            h2 = h2.wrapping_add(t1 >> 24 & 0x03ff_ffff_ffff | hibit);

            // h *= r
            d0 = (h0 as u128).wrapping_mul(r0 as u128);
            d  = (h1 as u128).wrapping_mul(s2 as u128);
            d0 = (d0 as u128).wrapping_add(d) as u128;
            d  = (h2 as u128).wrapping_mul(s1 as u128);
            d0 = (d0 as u128).wrapping_add(d) as u128;
            d1 = (h0 as u128).wrapping_mul(r1 as u128);
            d  = (h1 as u128).wrapping_mul(r0 as u128);
            d1 = (d1 as u128).wrapping_add(d) as u128;
            d  = (h2 as u128).wrapping_mul(s2 as u128);
            d1 = (d1 as u128).wrapping_add(d) as u128;
            d2 = (h0 as u128).wrapping_mul(r2 as u128);
            d  = (h1 as u128).wrapping_mul(r1 as u128);
            d2 = (d2 as u128).wrapping_add(d) as u128;
            d  = (h2 as u128).wrapping_mul(r0 as u128);
            d2 = (d2 as u128).wrapping_add(d) as u128;

            // (partial) h %= p
            c  = (d0 >> 44) as u64;
            h0 = d0 as u64 & 0x0fff_ffff_ffff;
            d1 = (d1 as u128).wrapping_add(c as u128);
            c  = (d1 >> 44) as u64;
            h1 = d1 as u64 & 0x0fff_ffff_ffff;
            d2 = (d2 as u128).wrapping_add(c as u128);
            c  = (d2 >> 42) as u64;
            h2 = d2 as u64 & 0x03ff_ffff_ffff;
            h0 = h0.wrapping_add(c.wrapping_mul(5));
            c  = h0 >> 44;
            h0 &= 0x0fff_ffff_ffff;
            h1 = h1.wrapping_add(c);

            m = &m[16..];
        }

        self.h[0] = h0;
        self.h[1] = h1;
        self.h[2] = h2;
    }
}
