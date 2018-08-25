mod request;
mod encrypted;
mod allowed;
mod protection;

pub use self::request::Request;
pub use self::encrypted::Encrypted;
pub use self::allowed::Allowed;
pub use self::protection::{Protection, NoProtection, ReplayProtection};

pub const REQUEST: u8 =     0;
pub const DENIED: u8 =      1;
pub const CHALLENGE: u8 =   2;
pub const RESPONSE: u8 =    3;
pub const KEEP_ALIVE: u8 =  4;
pub const DISCONNECT: u8 =  5;
pub const _RESERVED_0: u8 = 6;
pub const _RESERVED_1: u8 = 7;
pub const PAYLOAD: u8 =     8;

pub const MAX_PACKET_BYTES: usize = 1200;
pub const MAX_PAYLOAD_BYTES: usize = 1100;

pub const MAX_CHANNEL_ID: u8 = 23;

use crate::{
    VERSION,
    VERSION_BYTES,
};

pub fn is_request_packet(buffer: &[u8]) -> bool {
    buffer[0] == 0
}

pub fn is_encrypted_packet(buffer: &[u8]) -> bool {
    buffer[0] != 0
}

const ASSOCIATED_DATA_BYTES: usize = VERSION_BYTES+8+1;
fn associated_data(protocol_id: u64, prefix_byte: u8) -> [u8; ASSOCIATED_DATA_BYTES] {
    let mut data: [u8; ASSOCIATED_DATA_BYTES] = unsafe { std::mem::uninitialized() };
    {
        let p = &mut data[..];
        p[0..VERSION_BYTES].copy_from_slice(&VERSION[..]);
        for i in 0..8 {
            p[VERSION_BYTES + i] = (protocol_id >> i * 8 & 0xFF) as u8;
        }
        p[ASSOCIATED_DATA_BYTES - 1] = prefix_byte;
    }
    data
}

pub const fn sequence_number_bytes_required(sequence: u64) -> u8 {
    let leading_bytes = sequence.leading_zeros() / 8;
    8 - leading_bytes as u8 + (sequence == 0) as u8
}

#[test]
fn sequence() {
    assert_eq!(sequence_number_bytes_required(0x11______________), 1);
    assert_eq!(sequence_number_bytes_required(0x1122____________), 2);
    assert_eq!(sequence_number_bytes_required(0x112233__________), 3);
    assert_eq!(sequence_number_bytes_required(0x11223344________), 4);
    assert_eq!(sequence_number_bytes_required(0x1122334455______), 5);
    assert_eq!(sequence_number_bytes_required(0x112233445566____), 6);
    assert_eq!(sequence_number_bytes_required(0x11223344556677__), 7);
    assert_eq!(sequence_number_bytes_required(0x1122334455667788), 8);

    for i in 0..=0xFF {
        assert_eq!(sequence_number_bytes_required(i), 1);
        assert_eq!(sequence_number_bytes_required(i | 0x100), 2);
    }

    assert_eq!(sequence_number_bytes_required(0xFFFFFFFFFFFFFFFF), 8);
}
