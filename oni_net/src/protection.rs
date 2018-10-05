const REPLAY_PROTECTION_BUFFER_SIZE: usize = 256;
const INVALID_SEQUENCE: u64 = 0xFFFF_FFFF_FFFF_FFFF;

pub trait Protection {
    fn packet_already_received(&mut self, seq: u64) -> bool;
}

pub struct NoProtection;

impl Protection for NoProtection {
    fn packet_already_received(&mut self, _seq: u64) -> bool { false }
}

pub struct ReplayProtection {
    received_packet: Vec<u64>,
    most_recent_sequence: u64,
}

impl Default for ReplayProtection {
    fn default() -> Self {
        Self::new(REPLAY_PROTECTION_BUFFER_SIZE)
    }
}

impl ReplayProtection {
    pub fn new(len: usize) -> Self {
        Self {
            most_recent_sequence: 0,
            received_packet: vec![INVALID_SEQUENCE; len],
        }
    }

    pub fn reset(&mut self) {
        self.most_recent_sequence = 0;
        self.received_packet = vec![INVALID_SEQUENCE; self.received_packet.len()];
    }
}

impl Protection for ReplayProtection {
    fn packet_already_received(&mut self, sequence: u64) -> bool {
        if sequence + self.received_packet.len() as u64  <= self.most_recent_sequence as u64 {
            return true;
        }
        if sequence > self.most_recent_sequence {
            self.most_recent_sequence = sequence;
        }
        let index = (sequence % self.received_packet.len() as u64) as usize;
        if self.received_packet[index] == INVALID_SEQUENCE {
            self.received_packet[index] = sequence;
            return false;
        }
        if self.received_packet[index] >= sequence {
            return true
        }
        self.received_packet[index] = sequence;
        false
    }
}


#[test]
fn replay_protection() {
    let mut replay_protection = ReplayProtection::default();

    for _ in 0..2 {
        replay_protection.reset();

        assert_eq!(replay_protection.most_recent_sequence, 0);

        const MAX_SEQUENCE: u64 = 4 * REPLAY_PROTECTION_BUFFER_SIZE as u64;

        // the first time we receive packets, they should not be already received
        for sequence in 0..MAX_SEQUENCE {
            assert!(!replay_protection.packet_already_received(sequence));
        }

        // old packets outside buffer should be considered already received
        assert!(replay_protection.packet_already_received(0));

        // packets received a second time should be flagged already received
        for sequence in MAX_SEQUENCE - 10..MAX_SEQUENCE {
            assert!(replay_protection.packet_already_received(sequence));
        }

        // jumping ahead to a much higher sequence should be considered not already received
        assert!(!replay_protection.packet_already_received(MAX_SEQUENCE + REPLAY_PROTECTION_BUFFER_SIZE as u64));

        // old packets should be considered already received
        for sequence in 0..MAX_SEQUENCE {
            assert!(replay_protection.packet_already_received(sequence));
        }
    }
}
