mod request;
mod encrypted;

pub use self::request::Request;
pub use self::encrypted::Encrypted;

pub const REQUEST: u8 =     0;
pub const DENIED: u8 =      1;
pub const CHALLENGE: u8 =   2;
pub const RESPONSE: u8 =    3;
pub const KEEP_ALIVE: u8 =  4;
pub const PAYLOAD: u8 =     5;
pub const DISCONNECT: u8 =  6;
const PACKET_NUMS: u8 = 7;

pub const MAX_PACKET_BYTES: usize = 1200;
pub const MAX_PAYLOAD_BYTES: usize = 1100;

use token;
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use VERSION_INFO;
use VERSION_INFO_BYTES;


const CHALLENGE_INNER_SIZE: usize = 8 + token::Challenge::BYTES;
const RESPONSE_INNER_SIZE: usize = 8 + token::Challenge::BYTES;
const KEEP_ALIVE_INNER_SIZE: usize = 4 + 4;

pub fn is_request_packet(buffer: &[u8]) -> bool {
    buffer[0] == 0
}

pub fn is_encrypted_packet(buffer: &[u8]) -> bool {
    buffer[0] != 0
}

const ASSOCIATED_DATA_BYTES: usize = VERSION_INFO_BYTES+8+1;
fn associated_data(protocol_id: u64, prefix_byte: u8) -> [u8; ASSOCIATED_DATA_BYTES] {
    let mut data: [u8; ASSOCIATED_DATA_BYTES] = unsafe { ::std::mem::uninitialized() };
    {
        let mut p = &mut data[..];
        p.write_all(&VERSION_INFO[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u8(prefix_byte).unwrap();
    }
    data
}

bitflags! {
    pub struct Allowed: u8 {
        const REQUEST =     1 << REQUEST;
        const DENIED =      1 << DENIED;
        const CHALLENGE =   1 << CHALLENGE;
        const RESPONSE =    1 << RESPONSE;
        const KEEP_ALIVE =  1 << KEEP_ALIVE;
        const PAYLOAD =     1 << PAYLOAD;
        const DISCONNECT =  1 << DISCONNECT;

        const CLIENT_CONNECTED = Self::PAYLOAD.bits | Self::KEEP_ALIVE.bits | Self::DISCONNECT.bits;
        const CLIENT_SENDING_RESPONSE = Self::DENIED.bits | Self::KEEP_ALIVE.bits;
        const CLIENT_SENDING_REQUEST = Self::DENIED.bits | Self::CHALLENGE.bits;
    }
}

impl Allowed {
    pub fn packet_type(self, p: u8) -> bool {
        if      p == REQUEST    { self.contains(Allowed::REQUEST)   }
        else if p == DENIED     { self.contains(Allowed::DENIED)    }
        else if p == CHALLENGE  { self.contains(Allowed::CHALLENGE) }
        else if p == RESPONSE   { self.contains(Allowed::RESPONSE)  }
        else if p == KEEP_ALIVE { self.contains(Allowed::KEEP_ALIVE)}
        else if p == PAYLOAD    { self.contains(Allowed::PAYLOAD)   }
        else if p == DISCONNECT { self.contains(Allowed::DISCONNECT)}
        else { false }
    }
}

pub fn sequence_number_bytes_required(sequence: u64) -> u8 {
    let mut mask: u64 = 0xFF00_0000_0000_0000;
    for i in 0..7 {
        if sequence & mask != 0 {
            return 8 - i
        }
        mask >>= 8;
    }
    1
}

#[test]
fn sequence() {
    assert_eq!(sequence_number_bytes_required(0_________________ ), 1);
    assert_eq!(sequence_number_bytes_required(0x11______________ ), 1);
    assert_eq!(sequence_number_bytes_required(0x1122____________ ), 2);
    assert_eq!(sequence_number_bytes_required(0x112233__________ ), 3);
    assert_eq!(sequence_number_bytes_required(0x11223344________ ), 4);
    assert_eq!(sequence_number_bytes_required(0x1122334455______ ), 5);
    assert_eq!(sequence_number_bytes_required(0x112233445566____ ), 6);
    assert_eq!(sequence_number_bytes_required(0x11223344556677__ ), 7);
    assert_eq!(sequence_number_bytes_required(0x1122334455667788 ), 8);
}
