use crypto::{encrypt_aead, decrypt_aead, Key, Nonce, MAC_BYTES};
use utils::{UserData, sequence_number_bytes_required};
use token;
use VERSION_INFO;
use VERSION_INFO_BYTES;

use MAX_PAYLOAD_BYTES;
use MAX_PACKET_BYTES;

use TEST_CLIENT_ID;
use TEST_TIMEOUT_SECONDS;
use TEST_PROTOCOL_ID;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};
use replay_protection::ReplayProtection;

const REQUEST_SIZE: usize = 1 + VERSION_INFO_BYTES + 8 * 3 + token::Private::BYTES;

pub struct Context {
    pub write_packet_key: Key,
    pub read_packet_key: Key,
}

const REQUEST: u8 =     0;
const DENIED: u8 =      1;
const CHALLENGE: u8 =   2;
const RESPONSE: u8 =    3;
const KEEP_ALIVE: u8 =  4;
const PAYLOAD: u8 =     5;
const DISCONNECT: u8 =  6;

const PACKET_NUMS: u8 = 7;

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
        if p == REQUEST { self.contains(Allowed::REQUEST) }
        else if p == DENIED { self.contains(Allowed::DENIED) }
        else if p == CHALLENGE { self.contains(Allowed::CHALLENGE) }
        else if p == RESPONSE { self.contains(Allowed::RESPONSE) }
        else if p == KEEP_ALIVE { self.contains(Allowed::KEEP_ALIVE) }
        else if p == PAYLOAD { self.contains(Allowed::PAYLOAD) }
        else if p == DISCONNECT { self.contains(Allowed::DISCONNECT) }
        else { false }
    }
}

pub enum Packet {
    Request {
        sequence: u64,
        version_info: [u8; VERSION_INFO_BYTES],
        protocol_id: u64,
        // connect_token
        expire_timestamp: u64,
        data: [u8; token::Private::BYTES],
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
            Packet::Request   { ref mut sequence, .. } |
            Packet::Denied    { ref mut sequence, .. } |
            Packet::Challenge { ref mut sequence, .. } |
            Packet::Response  { ref mut sequence, .. } |
            Packet::KeepAlive { ref mut sequence, .. } |
            Packet::Payload   { ref mut sequence, .. } |
            Packet::Disconnect{ ref mut sequence, .. } => *sequence = seq,
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

    let mut additional = [0u8; VERSION_INFO_BYTES+8+1];
    {
        let mut p = &mut additional[..];
        p.write_all(&VERSION_INFO[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u8(prefix_byte).unwrap();
    }

    let nonce = Nonce::from_sequence(sequence);
    encrypt_aead(encrypted, &additional[..], &nonce, &write_packet_key)?;
    Ok(1 + sequence_bytes as usize + len + MAC_BYTES)
}

pub fn write_request_token(token: &::token::Public) -> [u8; REQUEST_SIZE] {
    write_request(
        token.protocol_id,
        token.expire_timestamp,
        token.sequence,
        token.private_data,
    )
}

pub fn write_request(
    protocol_id: u64,
    expire_timestamp: u64,
    sequence: u64,
    data: [u8; ::token::Private::BYTES],
) -> [u8; REQUEST_SIZE] {
    let mut buffer: [u8; REQUEST_SIZE] = unsafe { ::std::mem::uninitialized() };
    {
        let mut buffer = &mut buffer[..];
        buffer.write_u8(REQUEST).unwrap();
        buffer.write_all(&VERSION_INFO[..]).unwrap();
        buffer.write_u64::<LE>(protocol_id).unwrap();
        buffer.write_u64::<LE>(expire_timestamp).unwrap();
        buffer.write_u64::<LE>(sequence).unwrap();
        buffer.write_all(&data[..]).unwrap();
    }
    buffer
}

impl Packet {
    pub fn write(self, mut buffer: &mut [u8], key: &Key, protocol_id: u64) -> io::Result<usize> {
        match self {
        Packet::Request { version_info, protocol_id, expire_timestamp, sequence, data } => {
            buffer.write_u8(REQUEST)?;
            buffer.write_all(&version_info[..])?;
            buffer.write_u64::<LE>(protocol_id)?;
            buffer.write_u64::<LE>(expire_timestamp)?;
            buffer.write_u64::<LE>(sequence)?;
            buffer.write_all(&data[..])?;
            Ok(REQUEST_SIZE)
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
                Ok(8 + token::Challenge::BYTES)
            }),
        Packet::Response { sequence, token_sequence, token_data } =>
            encrypt_packet(buffer, sequence, key, protocol_id, RESPONSE, |mut buffer| {
                buffer.write_u64::<LE>(token_sequence)?;
                buffer.write_all(&token_data[..])?;
                Ok(8 + token::Challenge::BYTES)
            }),

        Packet::KeepAlive { sequence, client_index, max_clients } =>
            encrypt_packet(buffer, sequence, key, protocol_id, KEEP_ALIVE, |mut buffer| {
                buffer.write_u32::<LE>(client_index)?;
                buffer.write_u32::<LE>(max_clients)?;
                Ok(4 + 4)
            }),
        }
    }
}

pub fn read_packet(
    mut buffer: &[u8],
    read_packet_key: Option<&Key>,
    current_protocol_id: u64,
    current_timestamp: u64,
    private_key: Option<&Key>,
    replay_protection : Option<&mut ReplayProtection>,
    allowed: Allowed,
    ) -> Option<Packet>
{
    let prefix_byte = buffer.read_u8().ok()?;

    let packet_type = prefix_byte & 0xF;
    let sequence_bytes = prefix_byte >> 4;

    if !allowed.packet_type(packet_type) {
        debug!("packet type is not allowed");
        return None;
    }

    // connection request packet: first byte is zero
    if prefix_byte == REQUEST {
        if buffer.len() != VERSION_INFO_BYTES + 8 + 8 + 8 + token::Private::BYTES {
            debug!("ignored connection request packet. bad packet length (expected {}, got {})",
                VERSION_INFO_BYTES + 8 + 8 + 8 + token::Private::BYTES, buffer.len());
            return None;
        }

        let private_key = match private_key {
            Some(key) => key,
            None => {
                debug!("ignored connection request packet. no private key");
                return None;
            }
        };

        let mut version_info = [0u8; VERSION_INFO_BYTES];
        buffer.read_exact(&mut version_info[..]).ok()?;
        if version_info != VERSION_INFO {
            debug!("ignored connection request packet. bad version info");
            return None;
        }

        let protocol_id = buffer.read_u64::<LE>().ok()?;
        if protocol_id != current_protocol_id {
            debug!("ignored connection request packet. wrong protocol id. expected {}, got {}",
                current_protocol_id, protocol_id);
            return None;
        }

        let expire_timestamp = buffer.read_u64::<LE>().ok()?;
        if expire_timestamp <= current_timestamp {
            debug!("ignored connection request packet. connect token expired");
            return None;
        }

        let sequence = buffer.read_u64::<LE>().ok()?;

        //assert( buffer - start == 1 + VERSION_INFO_BYTES + 8 + 8 + 8 );

        let mut data = [0u8; token::Private::BYTES];
        buffer.read_exact(&mut data[..]).ok()?;

        if token::Private::decrypt(
            &mut data[..],
            protocol_id,
            expire_timestamp,
            sequence,
            private_key).is_err()
        {
            debug!("ignored connection request packet. connect token failed to decrypt");
            return None;
        }

        //assert( buffer - start == 1 + VERSION_INFO_BYTES + 8 + 8 + 8 + token::Private::BYTES );

        Some(Packet::Request {
            version_info,
            protocol_id,
            expire_timestamp,
            sequence,
            data,
        })
    } else {
        // *** encrypted packets ***
        let read_packet_key = match read_packet_key {
            Some(key) => key,
            None => {
                debug!("ignored encrypted packet. no read packet key for this address");
                return None;
            }
        };

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
        let mut additional = [0u8; VERSION_INFO_BYTES+8+1];
        {
            let mut p = &mut additional[..];
            p.write_all(&VERSION_INFO[..]).unwrap();
            p.write_u64::<LE>(current_protocol_id).unwrap();
            p.write_u8(prefix_byte).unwrap();
        }

        let nonce = Nonce::from_sequence(sequence);

        let encrypted_bytes = buffer.len(); //(buffer.len() - (buffer - start));
        if encrypted_bytes < MAC_BYTES {
            debug!("ignored encrypted packet. encrypted payload is too small");
            return None;
        }

        let mut encrypted: [u8; MAX_PACKET_BYTES] = unsafe { ::std::mem::uninitialized() };
        (&mut encrypted[..encrypted_bytes]).copy_from_slice(buffer);
        let buffer = &mut encrypted[..encrypted_bytes];

        if decrypt_aead(buffer, &additional, &nonce, read_packet_key).is_err() {
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
            if decrypted_bytes != 8 + token::Challenge::BYTES {
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
            if decrypted_bytes != 8 + token::Challenge::BYTES {
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
            if decrypted_bytes != 8 {
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
}

#[test]
fn connection_request_packet() {
    // generate a connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = UserData::random();
    let input_token = token::Private::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, vec![server_address], user_data.clone());
    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(input_token.server_addresses, &[server_address]);
    assert_eq!(input_token.user_data, user_data);

    // write the conect token to a buffer (non-encrypted)
    let mut connect_token_data = [0u8; token::Private::BYTES];
    input_token.write(&mut connect_token_data).unwrap();

    // copy to a second buffer then encrypt it in place (we need the unencrypted token for verification later on)
    let mut encrypted_connect_token_data = connect_token_data.clone();

    let connect_token_sequence = 1000u64;
    let connect_token_expire_timestamp = ::utils::time() + 30;
    let connect_token_key = Key::generate();

    token::Private::encrypt(
        &mut encrypted_connect_token_data[..],
        TEST_PROTOCOL_ID,
        connect_token_expire_timestamp,
        connect_token_sequence,
        &connect_token_key,
    ).unwrap();

    // setup a connection request packet wrapping the encrypted connect token
    let input_packet = Packet::Request {
        version_info: VERSION_INFO,
        protocol_id: TEST_PROTOCOL_ID,
        expire_timestamp: connect_token_expire_timestamp,
        sequence: connect_token_sequence,
        data: encrypted_connect_token_data,
    };

    // write the connection request packet to a buffer
    //let sequence = 1000u64;
    let mut buffer = [0u8; 2048];
    let packet_key = Key::generate();
    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();

    assert!(written > 0);

    // read the connection request packet back in from the buffer
    // (the connect token data is decrypted as part of the read packet validation)
    let allowed_packets = Allowed::all();

    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        Some(&connect_token_key),
        None,
        allowed_packets,
    ).unwrap();

    if let Packet::Request { version_info, protocol_id, expire_timestamp, sequence, data  } = output_packet {
        //assert_eq!(sequence, 100);
        // make sure the read packet matches what was written
        assert_eq!(version_info, VERSION_INFO);
        assert_eq!(protocol_id, TEST_PROTOCOL_ID);
        assert_eq!(expire_timestamp, connect_token_expire_timestamp );
        assert_eq!(sequence, connect_token_sequence);
        let len = token::Private::BYTES - MAC_BYTES;
        assert_eq!(&data[..len], &connect_token_data[..len]);
    } else {
        panic!("fail packet");
    }
}

#[test]
fn connection_denied_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = Packet::Denied {
        sequence: 1000,
    }.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let allowed_packet_types = Allowed::all();

    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        None,
        None,
        allowed_packet_types,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Packet::Denied { sequence } => assert_eq!(sequence, 1000),
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_challenge_packet() {
    // setup a connection challenge packet
    let mut x_data = [0u8; token::Challenge::BYTES];
    ::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Packet::Challenge {
        sequence: 1000,
        token_sequence: 0,
        token_data: x_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let allowed = Allowed::all();
    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        None,
        None,
        allowed,
    ).unwrap();

    match output_packet {
        Packet::Challenge { sequence, token_sequence, token_data } => {
            assert_eq!(sequence, 1000);
            assert_eq!(token_sequence, 0);
            assert_eq!(&token_data[..], &x_data[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_response_packet() {
    // setup a connection challenge packet
    let mut x_data = [0u8; token::Challenge::BYTES];
    ::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Packet::Response {
        sequence: 1000,
        token_sequence: 0,
        token_data: x_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let allowed = Allowed::all();
    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        None,
        None,
        allowed,
    ).unwrap();

    match output_packet {
        Packet::Response { sequence, token_sequence, token_data } => {
            assert_eq!(sequence, 1000);
            assert_eq!(token_sequence, 0);
            assert_eq!(&token_data[..], &x_data[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_keep_alive_packet() {
    // setup a connection challenge packet
    let mut x_data = [0u8; token::Challenge::BYTES];
    ::crypto::random_bytes(&mut x_data[..]);
    let input_packet = Packet::KeepAlive {
        sequence: 1000,
        client_index: 10,
        max_clients: 16,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let allowed = Allowed::all();
    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        None,
        None,
        allowed,
    ).unwrap();

    match output_packet {
        Packet::KeepAlive { sequence, client_index, max_clients } => {
            assert_eq!(sequence, 1000);
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

    let input_packet = Packet::Payload {
        sequence: 1000,
        len: MAX_PAYLOAD_BYTES,
        data: input_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = Key::generate();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();

    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        None,
        None,
        Allowed::all(),
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Packet::Payload { sequence, len, data } => {
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

    let written = Packet::Disconnect {
        sequence: 1000,
    }.write(&mut buffer[..], &packet_key, TEST_PROTOCOL_ID).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let allowed_packet_types = Allowed::all();

    let output_packet = read_packet(
        &mut buffer[..written],
        Some(&packet_key),
        TEST_PROTOCOL_ID,
        ::utils::time(),
        None,
        None,
        allowed_packet_types,
    ).unwrap();


    // make sure the read packet matches what was written
    match output_packet {
        Packet::Disconnect { sequence } => assert_eq!(sequence, 1000),
        _ => panic!("wrong packet"),
    }
}
