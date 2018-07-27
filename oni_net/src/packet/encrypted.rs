use crypto::{encrypt_aead, decrypt_aead, Key, Nonce, MAC_BYTES};
use utils::{UserData, sequence_number_bytes_required};
use token;
use VERSION_INFO;
use VERSION_INFO_BYTES;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use replay_protection::ReplayProtection;

use packet::{
    Allowed,
    associated_data,

    MAX_PAYLOAD_BYTES,
    MAX_PACKET_BYTES,

    CHALLENGE_INNER_SIZE,
    RESPONSE_INNER_SIZE,
    KEEP_ALIVE_INNER_SIZE,

    REQUEST,
    DENIED,
    CHALLENGE,
    RESPONSE,
    KEEP_ALIVE,
    PAYLOAD,
    DISCONNECT,
    PACKET_NUMS,
};

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
    Payload {
        sequence: u64,
        len: usize,
        data: [u8; MAX_PAYLOAD_BYTES],
    },
    Disconnect,
}

impl Encrypted {
    const MIN_PACKET_BYTES: usize = 1 + 1 + MAC_BYTES;

    pub fn read(
        mut buffer: &[u8],
        protection : Option<&mut ReplayProtection>,
        key: &Key,
        current_protocol_id: u64,
        allowed: Allowed,
    ) -> Option<Self> {
        if buffer.len() < Self::MIN_PACKET_BYTES {
            return None;
        }

        let prefix_byte = buffer.read_u8().ok()?;
        let packet_type = prefix_byte & 0xF;
        let sequence_bytes = prefix_byte >> 4;

        // extract the packet type and number of sequence bytes from the prefix byte
        if !allowed.packet_type(packet_type) {
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

        // replay protection (optional)
        if let Some(protection) = protection {
            if packet_type >= KEEP_ALIVE {
                if protection.packet_already_received(sequence) {
                    return None;
                }
            }
        }

        // decrypt the per-packet type data
        let encrypted_bytes = buffer.len(); //(buffer.len() - (buffer - start));
        if encrypted_bytes < MAC_BYTES {
            return None;
        }

        let mut encrypted: [u8; MAX_PACKET_BYTES] = unsafe { ::std::mem::uninitialized() };
        (&mut encrypted[..encrypted_bytes]).copy_from_slice(buffer);
        let buffer = &mut encrypted[..encrypted_bytes];

        let add = associated_data(current_protocol_id, prefix_byte);
        let nonce = Nonce::from_sequence(sequence);
        if decrypt_aead(buffer, &add[..], &nonce, key).is_err() {
            return None;
        }

        let len = encrypted_bytes - MAC_BYTES;
        let mut buffer = &buffer[..len];

        // process the per-packet type data that was just decrypted
        if packet_type == DENIED && len == 0 {
            Some(Encrypted::Denied)
        } else if packet_type == CHALLENGE && len == CHALLENGE_INNER_SIZE {
            Some(Encrypted::Challenge {
                challenge_sequence: buffer.read_u64::<LE>().ok()?,
                challenge_data: read_array!(buffer, token::Challenge::BYTES),
            })
        } else if packet_type == RESPONSE && len == RESPONSE_INNER_SIZE {
            Some(Encrypted::Response {
                challenge_sequence: buffer.read_u64::<LE>().ok()?,
                challenge_data: read_array!(buffer, token::Challenge::BYTES),
            })
        } else if packet_type == KEEP_ALIVE && len == KEEP_ALIVE_INNER_SIZE {
            Some(Encrypted::KeepAlive {
                client_index: buffer.read_u32::<LE>().ok()?,
                max_clients: buffer.read_u32::<LE>().ok()?,
            })
        } else if packet_type == PAYLOAD && len > 1 && len <= MAX_PAYLOAD_BYTES {
            let mut data: [u8; MAX_PAYLOAD_BYTES] = unsafe { ::std::mem::uninitialized() };
            (&mut data[..len]).copy_from_slice(&buffer[..]);
            Some(Encrypted::Payload { sequence, data, len })
        } else if packet_type == DISCONNECT && len == 0 {
            Some(Encrypted::Disconnect)
        } else {
            None
        }
    }

    pub fn write(self, buffer: &mut [u8], key: &Key, protocol_id: u64, sequence: u64) -> io::Result<usize> {
        match self {
        Encrypted::Payload { sequence: _, data, len } =>
            encrypt_packet(buffer, sequence, key, protocol_id, PAYLOAD, |mut buffer| {
                buffer.write(&data[..len])?;
                Ok(len)
            }),

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

fn encrypt_packet<'a, F>(mut buffer: &'a mut [u8], sequence: u64, write_packet_key: &Key, protocol_id: u64, packet_type: u8, f: F)
    -> io::Result<usize>
    where F: FnOnce(&'a mut [u8]) -> io::Result<usize>
{
    // write the prefix byte (this is a combination of the packet type and number of sequence bytes)
    let sequence_bytes = sequence_number_bytes_required(sequence);

    assert!(sequence_bytes >= 1);
    assert!(sequence_bytes <= 8);

    assert!(packet_type <= 0xF);

    let prefix_byte = packet_type | (sequence_bytes << 4);
    buffer.write_u8(prefix_byte)?;

    // write the variable length sequence number [1,8] bytes.
    let mut sequence_temp = sequence;
    for _ in 0..sequence_bytes {
        buffer.write_u8((sequence_temp & 0xFF) as u8)?;
        sequence_temp >>= 8;
    }

    let len = unsafe {
        use ::std::slice::from_raw_parts_mut;
        f(from_raw_parts_mut(buffer.as_mut_ptr(), buffer.len()))?
    };
    let encrypted = &mut buffer[..len];

    let add = associated_data(protocol_id, prefix_byte);
    let nonce = Nonce::from_sequence(sequence);
    encrypt_aead(encrypted, &add[..], &nonce, &write_packet_key)?;
    Ok(1 + sequence_bytes as usize + len + MAC_BYTES)
}

use TEST_CLIENT_ID;
use TEST_TIMEOUT_SECONDS;
use TEST_PROTOCOL_ID;

#[test]
fn connection_denied_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = Encrypted::Denied.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID, 1000).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        None,
        &packet_key,
        TEST_PROTOCOL_ID,
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
    ::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Encrypted::Challenge {
        challenge_sequence: 0,
        challenge_data: x_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID, 1000).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        None,
        &packet_key,
        TEST_PROTOCOL_ID,
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
    ::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Encrypted::Response {
        challenge_sequence: 0,
        challenge_data: x_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID, 1000).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        None,
        &packet_key,
        TEST_PROTOCOL_ID,
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
    ::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Encrypted::KeepAlive {
        client_index: 10,
        max_clients: 16,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID, 1000).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        None,
        &packet_key,
        TEST_PROTOCOL_ID,
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
    // setup a connection payload packet
    let mut input_data = [0u8; MAX_PAYLOAD_BYTES];
    ::crypto::random_bytes(&mut input_data[..]);

    let input_packet = Encrypted::Payload {
        sequence: 1000,
        len: MAX_PAYLOAD_BYTES,
        data: input_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID, 1000).unwrap();

    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        None,
        &packet_key,
        TEST_PROTOCOL_ID,
        Allowed::PAYLOAD,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Encrypted::Payload { sequence, len, data } => {
            assert_eq!(sequence, 1000);
            assert_eq!(len, MAX_PAYLOAD_BYTES);
            assert_eq!(&data[..], &input_data[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_disconnect_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = Encrypted::Disconnect.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID, 1000).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        None,
        &packet_key,
        TEST_PROTOCOL_ID,
        Allowed::DISCONNECT,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Encrypted::Disconnect => (),
        _ => panic!("wrong packet"),
    }
}
