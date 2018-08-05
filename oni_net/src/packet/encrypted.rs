use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Write};

use crate::{
    token,
    crypto::{encrypt_aead, decrypt_aead, Key, Nonce, MAC_BYTES},
    packet::{
        Allowed,
        associated_data,
        sequence_number_bytes_required,

        Protection, NoProtection,

        MAX_PAYLOAD_BYTES,
        MAX_PACKET_BYTES,
        MAX_CHANNEL_ID,

        DENIED,
        CHALLENGE,
        RESPONSE,
        KEEP_ALIVE,
        DISCONNECT,
        _RESERVED_0,
        _RESERVED_1,
        PAYLOAD,
    },
};

const CHALLENGE_INNER_SIZE: usize = 8 + token::Challenge::BYTES;
const RESPONSE_INNER_SIZE: usize = 8 + token::Challenge::BYTES;
const KEEP_ALIVE_INNER_SIZE: usize = 4 + 4;

pub enum Encrypted {
    Denied,
    Challenge {
        challenge_sequence: u64,
        challenge_data: [u8; token::Challenge::BYTES],
    },
    Response {
        challenge_sequence: u64,
        challenge_data: [u8; token::Challenge::BYTES],
    },
    KeepAlive {
        client_index: u32,
        max_clients: u32,
    },
    Disconnect,
    Payload {
        sequence: u64,
        channel: u8,
        len: usize,
        data: [u8; MAX_PAYLOAD_BYTES],
    },
}

impl Encrypted {
    const MIN_PACKET_BYTES: usize = 1 + 1 + MAC_BYTES;

    pub fn read<T>(mut buffer: &[u8], protection: &mut T, key: &Key, protocol: u64, allowed: Allowed) -> Option<Self>
        where T: Protection
    {
        if buffer.len() < Self::MIN_PACKET_BYTES {
            return None;
        }

        let prefix = buffer.read_u8().ok()?;
        let kind = prefix & 0b1_1111;
        let sequence_bytes = (prefix >> 5) + 1;

        // ignore reserved packages
        if kind == _RESERVED_0 || kind == _RESERVED_1 {
            return None;
        }

        // extract the packet type and number of sequence bytes from the prefix byte
        if !allowed.packet_type(kind) {
            return None;
        }
        if sequence_bytes < 1 || sequence_bytes > 8 {
            return None;
        }
        if buffer.len() < sequence_bytes as usize + MAC_BYTES {
            return None;
        }

        // read variable length sequence number [1,8]
        let mut sequence = 0u64;
        for i in 0..sequence_bytes {
            let value = buffer.read_u8().ok()?;
            sequence |= (value as u64) << (8 * i as u64);
        }

        // replay protection
        if kind >= KEEP_ALIVE {
            if protection.packet_already_received(sequence) {
                return None;
            }
        }

        Self::decrypt(buffer, key, sequence, kind, prefix, protocol)
    }

    fn decrypt(buffer: &[u8], key: &Key, sequence: u64, kind: u8, prefix: u8, protocol: u64) -> Option<Self> {
        // decrypt the per-packet type data
        let len = buffer.len();
        if len < MAC_BYTES {
            return None;
        }

        let mut encrypted: [u8; MAX_PACKET_BYTES] = unsafe { std::mem::uninitialized() };
        (&mut encrypted[..len]).copy_from_slice(buffer);
        let buffer = &mut encrypted[..len];

        let add = associated_data(protocol, prefix);
        let nonce = Nonce::from_sequence(sequence);
        if decrypt_aead(buffer, &add[..], &nonce, key).is_err() {
            return None;
        }

        let len = len - MAC_BYTES;
        let mut buffer = &buffer[..len];

        // process the per-packet type data that was just decrypted
        if kind == DENIED && len == 0 {
            Some(Encrypted::Denied)
        } else if kind == CHALLENGE && len == CHALLENGE_INNER_SIZE {
            Some(Encrypted::Challenge {
                challenge_sequence: buffer.read_u64::<LE>().ok()?,
                challenge_data: read_array!(buffer, token::Challenge::BYTES),
            })
        } else if kind == RESPONSE && len == RESPONSE_INNER_SIZE {
            Some(Encrypted::Response {
                challenge_sequence: buffer.read_u64::<LE>().ok()?,
                challenge_data: read_array!(buffer, token::Challenge::BYTES),
            })
        } else if kind == KEEP_ALIVE && len == KEEP_ALIVE_INNER_SIZE {
            Some(Encrypted::KeepAlive {
                client_index: buffer.read_u32::<LE>().ok()?,
                max_clients: buffer.read_u32::<LE>().ok()?,
            })
        } else if kind == DISCONNECT && len == 0 {
            Some(Encrypted::Disconnect)
        } else if kind >= PAYLOAD && len <= MAX_PAYLOAD_BYTES {
            let mut data: [u8; MAX_PAYLOAD_BYTES] = unsafe { std::mem::uninitialized() };
            (&mut data[..len]).copy_from_slice(&buffer[..]);
            Some(Encrypted::Payload { sequence, data, len, channel: kind - PAYLOAD })
        } else {
            None
        }
    }

    pub fn write(self, buffer: &mut [u8], key: &Key, protocol_id: u64, sequence: u64) -> io::Result<usize> {
        match self {
        Encrypted::Payload { sequence: _, data, channel, len } => {
            assert!(channel <= MAX_CHANNEL_ID);
            encrypt_packet(buffer, sequence, key, protocol_id, PAYLOAD + channel, |mut buffer| {
                buffer.write(&data[..len])?;
                Ok(len)
            })
        }
        Encrypted::Denied =>     encrypt_packet(buffer, sequence, key, protocol_id, DENIED,     |_| Ok(0)),
        Encrypted::Disconnect => encrypt_packet(buffer, sequence, key, protocol_id, DISCONNECT, |_| Ok(0)),

        Encrypted::Challenge { challenge_sequence, challenge_data } =>
            encrypt_packet(buffer, sequence, key, protocol_id, CHALLENGE, |mut buffer| {
                buffer.write_u64::<LE>(challenge_sequence)?;
                buffer.write_all(&challenge_data[..])?;
                Ok(CHALLENGE_INNER_SIZE)
            }),
        Encrypted::Response { challenge_sequence, challenge_data } =>
            encrypt_packet(buffer, sequence, key, protocol_id, RESPONSE, |mut buffer| {
                buffer.write_u64::<LE>(challenge_sequence)?;
                buffer.write_all(&challenge_data[..])?;
                Ok(RESPONSE_INNER_SIZE)
            }),

        Encrypted::KeepAlive { client_index, max_clients } =>
            encrypt_packet(buffer, sequence, key, protocol_id, KEEP_ALIVE, |mut buffer| {
                buffer.write_u32::<LE>(client_index)?;
                buffer.write_u32::<LE>(max_clients)?;
                Ok(KEEP_ALIVE_INNER_SIZE)
            }),
        }
    }
}

fn encrypt_packet<'a, F>(mut buffer: &'a mut [u8], sequence: u64, write_packet_key: &Key, protocol_id: u64, kind: u8, f: F)
    -> io::Result<usize>
    where F: FnOnce(&'a mut [u8]) -> io::Result<usize>
{
    // write the prefix byte (this is a combination of the packet type and number of sequence bytes)
    let sequence_bytes = sequence_number_bytes_required(sequence);

    assert!(sequence_bytes >= 1);
    assert!(sequence_bytes <= 8);

    assert!(kind <= 0b1_1111);

    let prefix = kind | ((sequence_bytes - 1) << 5);
    buffer.write_u8(prefix)?;

    // write the variable length sequence number [1,8] bytes.
    let mut sequence_temp = sequence;
    for _ in 0..sequence_bytes {
        buffer.write_u8((sequence_temp & 0xFF) as u8)?;
        sequence_temp >>= 8;
    }

    let len = unsafe {
        use std::slice::from_raw_parts_mut;
        f(from_raw_parts_mut(buffer.as_mut_ptr(), buffer.len()))?
    };
    let encrypted = &mut buffer[..len];

    let add = associated_data(protocol_id, prefix);
    let nonce = Nonce::from_sequence(sequence);
    encrypt_aead(encrypted, &add[..], &nonce, &write_packet_key)?;
    Ok(1 + sequence_bytes as usize + len + MAC_BYTES)
}


use crate::{
    TEST_PROTOCOL,
    TEST_SEQ,
};

#[test]
fn connection_denied_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = Encrypted::Denied.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::DENIED,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Encrypted::Denied => (),
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_challenge_packet() {
    // setup a connection challenge packet
    let mut x_data = [0u8; token::Challenge::BYTES];
    crate::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Encrypted::Challenge {
        challenge_sequence: 0,
        challenge_data: x_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::CHALLENGE,
    ).unwrap();

    match output_packet {
        Encrypted::Challenge { challenge_sequence, challenge_data } => {
            assert_eq!(challenge_sequence, 0);
            assert_eq!(&challenge_data[..], &x_data[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_response_packet() {
    // setup a connection challenge packet
    let mut x_data = [0u8; token::Challenge::BYTES];
    crate::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Encrypted::Response {
        challenge_sequence: 0,
        challenge_data: x_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::RESPONSE,
    ).unwrap();

    match output_packet {
        Encrypted::Response { challenge_sequence, challenge_data } => {
            assert_eq!(challenge_sequence, 0);
            assert_eq!(&challenge_data[..], &x_data[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_keep_alive_packet() {
    // setup a connection challenge packet
    let mut x_data = [0u8; token::Challenge::BYTES];
    crate::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Encrypted::KeepAlive {
        client_index: 10,
        max_clients: 16,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::KEEP_ALIVE,
    ).unwrap();

    match output_packet {
        Encrypted::KeepAlive { client_index, max_clients } => {
            assert_eq!(client_index, 10);
            assert_eq!(max_clients, 16);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_payload_packet() {
    for chan in 0..=MAX_CHANNEL_ID {
        // setup a connection payload packet
        let mut input_data = [0u8; MAX_PAYLOAD_BYTES];
        crate::crypto::random_bytes(&mut input_data[..]);

        let input_packet = Encrypted::Payload {
            sequence: TEST_SEQ,
            len: MAX_PAYLOAD_BYTES,
            data: input_data,
            channel: chan,
        };

        // write the packet to a buffer
        let mut buffer = [0u8; MAX_PACKET_BYTES];
        let packet_key = Key::generate();

        let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();

        assert!(written > 0);

        // read the packet back in from the buffer
        let output_packet = Encrypted::read(
            &mut buffer[..written],
            &mut NoProtection,
            &packet_key,
            TEST_PROTOCOL,
            Allowed::PAYLOAD,
        ).unwrap();

        // make sure the read packet matches what was written
        match output_packet {
            Encrypted::Payload { sequence, len, data, channel } => {
                assert_eq!(channel, chan);
                assert_eq!(sequence, TEST_SEQ);
                assert_eq!(len, MAX_PAYLOAD_BYTES);
                assert_eq!(&data[..], &input_data[..]);
            }
            _ => panic!("wrong packet"),
        }
    }
}

#[test]
fn connection_disconnect_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = Encrypted::Disconnect.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::DISCONNECT,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Encrypted::Disconnect => (),
        _ => panic!("wrong packet"),
    }
}
