#![allow(unused_variables)]
use crate::packet::{MIN_PACKET, Kind, CHALLENGE_PACKET_BYTES};

pub trait Protection {
    fn packet_already_received(&mut self, seq: u64) -> bool {
        false
    }
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        true
    }
}

pub struct NoFilter;
impl Protection for NoFilter {}

pub struct ChallengeFilter;
impl Protection for ChallengeFilter {
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        if kind == Kind::Challenge && len == CHALLENGE_PACKET_BYTES {
            return true;
        }
        false
    }
}

pub struct ChallengeOrDisconnectFilter;
impl Protection for ChallengeOrDisconnectFilter {
    fn is_allowed(&mut self, kind: Kind, len: usize, seq: u64) -> bool {
        if kind == Kind::Challenge && len == CHALLENGE_PACKET_BYTES {
            return true;
        }
        if kind == Kind::Disconnect && len == MIN_PACKET {
            return true;
        }
        false
    }
}

pub struct NoProtection;

impl Protection for NoProtection {
    fn packet_already_received(&mut self, _seq: u64) -> bool { false }
}

use generic_array::GenericArray;
use generic_array::typenum::U256;

pub struct ReplayProtection {
    received: GenericArray<u8, U256>,
    seq: u64,
}

impl Default for ReplayProtection {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayProtection {
    pub fn new() -> Self {
        Self {
            seq: 0,
            received: GenericArray::default(),
        }
    }

    pub fn most_recent_sequence(&self) -> u64 {
        self.seq
    }

    pub fn reset(&mut self) {
        self.seq = 0;
        self.received = GenericArray::default();
    }

    #[inline(always)]
    fn index(bit: usize) -> usize { bit >> 3 }

    #[inline(always)]
    fn mask(bit: usize) -> u8 { 1 << (bit & 0b111) }

    #[inline(always)]
    unsafe fn get_unchecked(&self, bit: usize) -> bool {
        *self.received.get_unchecked(Self::index(bit)) & Self::mask(bit) != 0
    }
    #[inline(always)]
    unsafe fn set_unchecked(&mut self, bit: usize) {
        *self.received.get_unchecked_mut(Self::index(bit)) |= Self::mask(bit)
    }
    #[inline(always)]
    unsafe fn clear_unchecked(&mut self, bit: usize) {
        *self.received.get_unchecked_mut(Self::index(bit)) &= !Self::mask(bit)
    }

    #[inline(always)]
    fn get_wrapped(&self, bit: usize) -> bool {
        unsafe { self.get_unchecked(bit % self.received.len()) }
    }
    #[inline(always)]
    fn set_wrapped(&mut self, bit: usize) {
        unsafe { self.set_unchecked(bit % self.received.len()) }
    }
    #[inline(always)]
    fn clear_wrapped(&mut self, bit: usize) {
        unsafe { self.clear_unchecked(bit % self.received.len()) }
    }
}

impl Protection for ReplayProtection {
    fn packet_already_received(&mut self, seq: u64) -> bool {
        let len = self.received.len() as u64;
        if seq + len <= self.seq {
            return true;
        }
        if seq > self.seq {
            for bit in self.seq+1..seq+1 {
                let bit = bit % len;
                unsafe { self.clear_unchecked(bit as usize); }
            }
            if seq >= self.seq + len {
                self.received = GenericArray::default();
            }
            self.seq = seq;
        }
        let index = (seq % self.received.len() as u64) as usize;
        unsafe {
            if self.get_unchecked(index) {
                true
            } else {
                self.set_unchecked(index);
                false
            }
        }
    }
}

#[test]
fn replay_protection() {
    pub const REPLAY_PROTECTION_BUFFER_SIZE: usize = 256;
    let mut replay_protection = ReplayProtection::default();

    for _ in 0..2 {
        replay_protection.reset();

        assert_eq!(replay_protection.most_recent_sequence(), 0);

        const MAX_SEQUENCE: u64 = 4 * REPLAY_PROTECTION_BUFFER_SIZE as u64;

        for sequence in 0..MAX_SEQUENCE {
            assert!(!replay_protection.packet_already_received(sequence),
                "The first time we receive packets, they should not be already received");
        }

        assert!(replay_protection.packet_already_received(0),
            "Old packets outside buffer should be considered already received");

        for sequence in MAX_SEQUENCE - 10..MAX_SEQUENCE {
            assert!(replay_protection.packet_already_received(sequence),
                "Packets received a second time should be flagged already received");
        }

        assert!(!replay_protection.packet_already_received(MAX_SEQUENCE + REPLAY_PROTECTION_BUFFER_SIZE as u64),
            "Jumping ahead to a much higher sequence should be considered not already received");


        for sequence in 0..MAX_SEQUENCE {
            assert!(replay_protection.packet_already_received(sequence),
            "Old packets should be considered already received");
        }
    }
}
