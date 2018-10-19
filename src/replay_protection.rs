use crate::bitset::BitSet256;

#[derive(Default)]
pub struct ReplayProtection {
    seq: u64,
    bits: BitSet256,
}

impl ReplayProtection {
    pub fn new() -> Self {
        Self {
            seq: 0,
            bits: BitSet256::default(),
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
                unsafe { self.bits.clear_unchecked(bit); }
            }
            if seq >= self.seq + len {
                self.bits = BitSet256::default();
            }
            self.seq = seq;
        }
        unsafe {
            let bit = (seq % len) as usize;
            let ret = self.bits.get_unchecked(bit);
            self.bits.set_unchecked(bit);
            ret
        }
    }
}

#[test]
fn replay_protection() {
    let mut rp = ReplayProtection::new();

    let size = rp.bits.len() as u64;
    let max = size * 4;

    assert_eq!(rp.seq, 0);

    for sequence in 0..max {
        assert!(!rp.already_received(sequence),
        "The first time we receive packets, they should not be already received");
    }

    assert!(rp.already_received(0),
    "Old packets outside buffer should be considered already received");

    for sequence in max-10..max {
        assert!(rp.already_received(sequence),
        "Packets received a second time should be flagged already received");
    }

    assert!(!rp.already_received(max + size),
    "Jumping ahead to a much higher sequence should be considered not already received");


    for sequence in 0..max {
        assert!(rp.already_received(sequence),
        "Old packets should be considered already received");
    }
}
