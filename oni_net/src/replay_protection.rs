use generic_array::{GenericArray, typenum::U256};

#[derive(Default)]
pub struct ReplayProtection {
    seq: u64,
    bits: GenericArray<u8, U256>,
}

impl ReplayProtection {
    pub fn new() -> Self {
        Self {
            seq: 0,
            bits: GenericArray::default(),
        }
    }

    pub fn already_received(&mut self, seq: u64) -> bool {
        let len = self.bits.len() as u64;
        if seq.wrapping_add(len) <= self.seq {
            return true;
        }
        if seq > self.seq {
            for bit in self.seq+1..=seq {
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
        assert!(!rp.already_received(sequence),
        "The first time we receive packets, they should not be already received");
    }

    assert!(rp.already_received(0),
    "Old packets outside buffer should be considered already received");

    for sequence in MAX - 10..MAX {
        assert!(rp.already_received(sequence),
        "Packets received a second time should be flagged already received");
    }

    assert!(!rp.already_received(MAX + SIZE),
    "Jumping ahead to a much higher sequence should be considered not already received");


    for sequence in 0..MAX {
        assert!(rp.already_received(sequence),
        "Old packets should be considered already received");
    }
}
