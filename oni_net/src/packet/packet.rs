use crypto::{encrypt_aead, decrypt_aead, Key, Nonce, MAC_BYTES};
use utils::{UserData, sequence_number_bytes_required};
use token;
use VERSION_INFO;
use VERSION_INFO_BYTES;

use TEST_CLIENT_ID;
use TEST_TIMEOUT_SECONDS;
use TEST_PROTOCOL_ID;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use replay_protection::ReplayProtection;

use packet::{
    Request,
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

impl From<Request> for Packet {
    fn from(r: Request) -> Self {
        let Request {
            sequence, version_info, protocol_id, expire_timestamp, private_data,
        } = r;
        Packet::Request {
            sequence, version_info, protocol_id, expire_timestamp, private_data,
        }
    }
}

pub enum Packet {
    Request {
        sequence: u64,
        version_info: [u8; VERSION_INFO_BYTES],
        protocol_id: u64,
        // connect_token
        expire_timestamp: u64,
        private_data: [u8; token::Private::BYTES],
    },
    Denied {
        sequence: u64,
    },
    Challenge {
        sequence: u64,
        // challenge_token
        token_sequence: u64,
        token_data: [u8; token::Challenge::BYTES],
    },
    Response {
        sequence: u64,
        // challenge_token
        token_sequence: u64,
        token_data: [u8; token::Challenge::BYTES],
    },
    KeepAlive {
        sequence: u64,
        client_index: u32,
        max_clients: u32,
    },
    Payload {
        sequence: u64,
        len: usize,
        data: [u8; MAX_PAYLOAD_BYTES],
    },
    Disconnect {
        sequence: u64,
    },
}

impl Packet {
    pub fn set_sequence(&mut self, seq: u64) {
        match self {
            Packet::Denied    { ref mut sequence, .. } |
            Packet::Challenge { ref mut sequence, .. } |
            Packet::Response  { ref mut sequence, .. } |
            Packet::KeepAlive { ref mut sequence, .. } |
            Packet::Payload   { ref mut sequence, .. } |
            Packet::Disconnect{ ref mut sequence, .. } => *sequence = seq,

            Packet::Request   { .. } => (),
        }
    }

    fn packet_type(&self) -> u8 {
        match self {
            Packet::Request   { .. }=> REQUEST,
            Packet::Denied    { .. }=> DENIED,
            Packet::Challenge { .. }=> CHALLENGE,
            Packet::Response  { .. }=> RESPONSE,
            Packet::KeepAlive { .. }=> KEEP_ALIVE,
            Packet::Payload   { .. }=> PAYLOAD,
            Packet::Disconnect{ .. }=> DISCONNECT,
        }
    }
}

/// encrypt the per-packet packet written with the prefix byte,
/// protocol id and version as the associated data.
/// this must match to decrypt.
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

impl Packet {
    pub fn write(self, mut buffer: &mut [u8], key: &Key, protocol_id: u64) -> io::Result<usize> {
        match self {
        Packet::Request { version_info: _, protocol_id, expire_timestamp, sequence, private_data } => {
            let buf = Request::write_request(
                protocol_id,
                expire_timestamp,
                sequence,
                private_data,
            );
            buffer.write_all(&buf[..])?;
            Ok(Request::BYTES)
        }

        // *** encrypted packets ***
        Packet::Payload { sequence, data, len } =>
            encrypt_packet(buffer, sequence, key, protocol_id, PAYLOAD, |mut buffer| {
                buffer.write(&data[..len])?;
                Ok(len)
            }),

        Packet::Denied { sequence } =>     encrypt_packet(buffer, sequence, key, protocol_id, DENIED,     |_| Ok(0)),
        Packet::Disconnect { sequence } => encrypt_packet(buffer, sequence, key, protocol_id, DISCONNECT, |_| Ok(0)),

        Packet::Challenge { sequence, token_sequence, token_data } =>
            encrypt_packet(buffer, sequence, key, protocol_id, CHALLENGE, |mut buffer| {
                buffer.write_u64::<LE>(token_sequence)?;
                buffer.write_all(&token_data[..])?;
                Ok(CHALLENGE_INNER_SIZE)
            }),
        Packet::Response { sequence, token_sequence, token_data } =>
            encrypt_packet(buffer, sequence, key, protocol_id, RESPONSE, |mut buffer| {
                buffer.write_u64::<LE>(token_sequence)?;
                buffer.write_all(&token_data[..])?;
                Ok(RESPONSE_INNER_SIZE)
            }),

        Packet::KeepAlive { sequence, client_index, max_clients } =>
            encrypt_packet(buffer, sequence, key, protocol_id, KEEP_ALIVE, |mut buffer| {
                buffer.write_u32::<LE>(client_index)?;
                buffer.write_u32::<LE>(max_clients)?;
                Ok(KEEP_ALIVE_INNER_SIZE)
            }),
        }
    }
}

pub fn read_packet(
    buffer: &[u8],
    read_packet_key: Option<&Key>,
    current_protocol_id: u64,
    current_timestamp: u64,
    private_key: Option<&Key>,
    replay_protection : Option<&mut ReplayProtection>,
    allowed: Allowed,
    ) -> Option<Packet>
{
    let kind = buffer[0] & 0xF;
    if !allowed.packet_type(kind) {
        debug!("packet type is not allowed");
        return None;
    }

    // connection request packet: first byte is zero
    if kind == REQUEST {
        let private_key = match private_key {
            Some(key) => key,
            None => {
                debug!("ignored connection request packet. no private key");
                return None;
            }
        };
        Request::read(buffer, current_timestamp, current_protocol_id, private_key)
            .map(|r| r.into())
    } else {
        let read_packet_key = match read_packet_key {
            Some(key) => key,
            None => {
                debug!("ignored encrypted packet. no read packet key for this address");
                return None;
            }
        };
        read_encrypted_packet(buffer, replay_protection, read_packet_key, current_protocol_id)
    }
}

pub fn read_encrypted_packet(
    mut buffer: &[u8],
    replay_protection : Option<&mut ReplayProtection>,
    read_packet_key: &Key,
    current_protocol_id: u64,
) -> Option<Packet> {
    let prefix_byte = buffer.read_u8().ok()?;
    let packet_type = prefix_byte & 0xF;
    let sequence_bytes = prefix_byte >> 4;

    if buffer.len() < 1 + 1 + MAC_BYTES  {
        debug!("ignored encrypted packet. packet is too small to be valid ({} bytes)", buffer.len());
        return None;
    }

    // extract the packet type and number of sequence bytes from the prefix byte
    if packet_type >= PACKET_NUMS {
        debug!("ignored encrypted packet. packet type {} is invalid", packet_type);
        return None;
    }

    if sequence_bytes < 1 || sequence_bytes > 8 {
        debug!("ignored encrypted packet. sequence bytes {} is out of range [1,8]", sequence_bytes);
        return None;
    }

    if buffer.len() < sequence_bytes as usize + MAC_BYTES {
        debug!("ignored encrypted packet. buffer is too small for sequence bytes + encryption mac");
        return None;
    }

    // read variable length sequence number [1,8]
    let mut sequence = 0u64;
    for i in 0..sequence_bytes {
        let value = buffer.read_u8().ok()?;
        sequence |= (value as u64) << (8 * i as u64);
    }

    // replay protection (optional)
    if let Some(replay_protection) = replay_protection {
        if packet_type >= KEEP_ALIVE {
            if replay_protection.packet_already_received(sequence) {
                debug!("ignored connection payload packet. sequence {} already received (replay protection)", sequence);
                return None;
            }
        }
    }

    // decrypt the per-packet type data
    let encrypted_bytes = buffer.len(); //(buffer.len() - (buffer - start));
    if encrypted_bytes < MAC_BYTES {
        debug!("ignored encrypted packet. encrypted payload is too small");
        return None;
    }

    let mut encrypted: [u8; MAX_PACKET_BYTES] = unsafe { ::std::mem::uninitialized() };
    (&mut encrypted[..encrypted_bytes]).copy_from_slice(buffer);
    let buffer = &mut encrypted[..encrypted_bytes];

    let add = associated_data(current_protocol_id, prefix_byte);
    let nonce = Nonce::from_sequence(sequence);
    if decrypt_aead(buffer, &add[..], &nonce, read_packet_key).is_err() {
        debug!("ignored encrypted packet. failed to decrypt");
        return None;
    }

    let decrypted_bytes = encrypted_bytes - MAC_BYTES;
    let mut buffer = &buffer[..decrypted_bytes];

    // process the per-packet type data that was just decrypted
    if packet_type == DENIED {
        if decrypted_bytes != 0 {
            debug!("ignored connection denied packet. decrypted packet data is wrong size");
            return None
        }
        Some(Packet::Denied { sequence })
    } else if packet_type == CHALLENGE {
        if decrypted_bytes != CHALLENGE_INNER_SIZE {
            debug!("ignored connection challenge packet. decrypted packet data is wrong size: {}", decrypted_bytes);
            return None;
        }
        let challenge_token_sequence = buffer.read_u64::<LE>().ok()?;
        let mut challenge_token_data: [u8; token::Challenge::BYTES] = unsafe { ::std::mem::uninitialized() };
        buffer.read_exact(&mut challenge_token_data[..]).ok()?;
        Some(Packet::Challenge {
            sequence,
            token_sequence: challenge_token_sequence,
            token_data: challenge_token_data
        })
    } else if packet_type == RESPONSE {
        if decrypted_bytes != RESPONSE_INNER_SIZE {
            debug!("ignored connection response packet. decrypted packet data is wrong size");
            return None;
        }
        let challenge_token_sequence = buffer.read_u64::<LE>().ok()?;
        let mut challenge_token_data: [u8; token::Challenge::BYTES] = unsafe { ::std::mem::uninitialized() };
        buffer.read_exact(&mut challenge_token_data[..]).ok()?;
        Some(Packet::Response {
            sequence,
            token_sequence: challenge_token_sequence,
            token_data: challenge_token_data
        })
    } else if packet_type == KEEP_ALIVE {
        if decrypted_bytes != KEEP_ALIVE_INNER_SIZE {
            debug!("ignored connection keep alive packet. decrypted packet data is wrong size");
            return None;
        }
        let client_index = buffer.read_u32::<LE>().ok()?;
        let max_clients = buffer.read_u32::<LE>().ok()?;
        Some(Packet::KeepAlive {
            sequence,
            client_index,
            max_clients,
        })
    } else if packet_type == PAYLOAD {
        if decrypted_bytes < 1 || decrypted_bytes > MAX_PAYLOAD_BYTES {
            debug!("ignored connection payload packet. payload packet data is wrong size");
            return None;
        }
        let mut data: [u8; MAX_PAYLOAD_BYTES] = unsafe { ::std::mem::uninitialized() };
        let len = decrypted_bytes;
        (&mut data[..len]).copy_from_slice(&buffer[..]);
        Some(Packet::Payload { sequence, data, len })
    } else if packet_type == DISCONNECT {
        if decrypted_bytes != 0 {
            debug!("ignored connection disconnect packet. decrypted packet data is wrong size");
            return None;
        }
        Some(Packet::Disconnect { sequence })
    } else {
        None
    }
}
