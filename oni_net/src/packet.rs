use crypto::{encrypt_aead, decrypt_aead, Key};
use utils::{UserData, sequence_number_bytes_required};
use challenge_token::CHALLENGE_TOKEN_BYTES;
use connect_token_private::CONNECT_TOKEN_PRIVATE_BYTES;
use VERSION_INFO;
use VERSION_INFO_BYTES;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

pub const MAX_PAYLOAD_BYTES: usize = 1100;

const REQUEST_PACKET_SIZE: usize = 1 + VERSION_INFO_BYTES + 4 + 4 + 4 + CONNECT_TOKEN_PRIVATE_BYTES;

pub struct Payload {
    len: usize,
    data: [u8; MAX_PAYLOAD_BYTES],
}

impl Payload {
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    fn read<'a>(mut buffer: &'a [u8]) -> io::Result<(Self, &'a [u8])> {
        let mut data = [0u8; MAX_PAYLOAD_BYTES];
        let len = buffer.read(&mut data)?;
        Ok((Self { len, data }, buffer))
    }
}


const REQUEST_PACKET: u8 =           0;
const DENIED_PACKET: u8 =            1;
const CHALLENGE_PACKET: u8 =         2;
const RESPONSE_PACKET: u8 =          3;
const KEEP_ALIVE_PACKET: u8 =        4;
const PAYLOAD_PACKET: u8 =           5;
const DISCONNECT_PACKET: u8 =        6;
const NUM_PACKETS: u8 =              7;

enum Packet {
    Request(RequestPacket),
    Denied,
    Challenge(ChallengePacket),
    Response(ResponsePacket),
    KeepAlive(KeepAlivePacket),
    Payload(Payload),
    Disconnect,
}

impl Packet {
    fn packet_type(&self) -> u8 {
        match self {
            Packet::Request(_) => 0,
            Packet::Denied => 1,
            Packet::Challenge(_) => 2,
            Packet::Response(_) => 3,
            Packet::KeepAlive(_) => 4,
            Packet::Payload(_) => 5,
            Packet::Disconnect => 6,
        }
    }
}

struct RequestPacket {
    version_info: [u8; VERSION_INFO_BYTES],
    protocol_id: u64,
    connect_token_expire_timestamp: u64,
    connect_token_sequence: u64,
    connect_token_data: [u8; CONNECT_TOKEN_PRIVATE_BYTES],
}

struct ChallengePacket {
    challenge_token_sequence: u64,
    challenge_token_data: [u8; CHALLENGE_TOKEN_BYTES],
}

struct ResponsePacket {
    packet_type: u8,
    challenge_token_sequence: u64,
    challenge_token_data: [u8; CHALLENGE_TOKEN_BYTES],
}

struct KeepAlivePacket {
    client_index: u32,
    max_clients: u32,
}

struct PayloadPacket {
    payload_bytes: u32,
    payload_data: [u8],
}

/*
struct connection_payload_packet_t * create_payload_packet( int payload_bytes, void * allocator_context, void* (*allocate_function)(void*,uint64_t) ) {
    assert!( payload_bytes >= 0 );
    assert!( payload_bytes <= MAX_PAYLOAD_BYTES );

    if ( allocate_function == NULL )
    {
        allocate_function = default_allocate_function;
    }

    struct connection_payload_packet_t * packet = (struct connection_payload_packet_t*) 
        allocate_function( allocator_context, sizeof( struct connection_payload_packet_t ) + payload_bytes );

    if ( !packet )
        return NULL;

    packet.packet_type = CONNECTION_PAYLOAD_PACKET;
    packet.payload_bytes = payload_bytes;

    return packet;
}
*/

pub struct Context {
    pub write_packet_key: Key,
    pub read_packet_key: Key,
    pub allowed_packets: [bool; 8],
    pub current_timestamp: u64,
    pub protocol_id: u64,
}

impl Context {
    /// encrypt the per-packet packet written with the prefix byte,
    /// protocol id and version as the associated data.
    /// this must match to decrypt.
    fn encrypt_packet<'a, F>(&self, mut buffer: &'a mut [u8], sequence: u64, packet_type: u8, f: F)
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
            p.write_all(VERSION_INFO).unwrap();
            p.write_u64::<LE>(self.protocol_id).unwrap();
            p.write_u8(prefix_byte).unwrap();
        }
        let nonce = Nonce::from_sequence(sequence);
        encrypt_aead(encrypted, &additional[..], &nonce, &self.write_packet_key)?;
        Ok(1 + sequence_bytes as usize + len)
    }

    fn write_packet(&self, packet: &Packet, mut buffer: &mut [u8], sequence: u64) -> io::Result<usize> {
        match packet {
        Packet::Request(p) => {
            buffer.write_u8(REQUEST_PACKET)?;

            buffer.write_all(&p.version_info[..])?;
            buffer.write_u64::<LE>(p.protocol_id)?;
            buffer.write_u64::<LE>(p.connect_token_expire_timestamp)?;
            buffer.write_u64::<LE>(p.connect_token_sequence)?;
            buffer.write_all(&p.connect_token_data[..])?;
            Ok(REQUEST_PACKET_SIZE)
        }
        // *** encrypted packets ***
        // write packet data according to type.
        // this data will be encrypted.
        Packet::Payload(p) =>
            self.encrypt_packet(buffer, sequence, PAYLOAD_PACKET, |mut buffer| {
                buffer.write(&p.data[..p.len])?;
                Ok(p.len)
            }),
        Packet::Denied =>
            self.encrypt_packet(buffer, sequence, DENIED_PACKET, |_| Ok(0)),
        Packet::Disconnect =>
            self.encrypt_packet(buffer, sequence, DISCONNECT_PACKET, |_| Ok(0)),
        Packet::Challenge(p) =>
            self.encrypt_packet(buffer, sequence, CHALLENGE_PACKET, |mut buffer| {
                buffer.write_u64::<LE>(p.challenge_token_sequence)?;
                buffer.write_all(&p.challenge_token_data[..])?;
                Ok(8 + CONNECT_TOKEN_PRIVATE_BYTES)
            }),
        Packet::Response(p) =>
            self.encrypt_packet(buffer, sequence, RESPONSE_PACKET, |mut buffer| {
                buffer.write_u64::<LE>(p.challenge_token_sequence)?;
                buffer.write_all(&p.challenge_token_data[..])?;
                Ok(8 + CONNECT_TOKEN_PRIVATE_BYTES)
            }),
        Packet::KeepAlive(p) =>
            self.encrypt_packet(buffer, sequence, KEEP_ALIVE_PACKET, |mut buffer| {
                buffer.write_u32::<LE>(p.client_index)?;
                buffer.write_u32::<LE>(p.max_clients)?;
                Ok(4 + 4)
            }),
        }
    }

    /*
    pub fn read_packet(mut buffer: &[u8]) -> 


    void * read_packet(
        uint8_t * buffer,
        int buffer_length,
        uint8_t * read_packet_key,
        uint64_t protocol_id,
        uint64_t current_timestamp,
        uint8_t * private_key,
        struct replay_protection_t * replay_protection,
        void * allocator_context,
        void* (*allocate_function)(void*,uint64_t) )
    {
        let sequence = 0u64;

        if buffer.len() < 1 {
            debug!("ignored packet. buffer length is less than 1");
            return NULL;
        }

        uint8_t * start = buffer;

        let prefix_byte = &buffer[..].read_uint8();

        if prefix_byte == REQUEST_PACKET {
            // connection request packet: first byte is zero
            if !self.allowed_packets[REQUEST_PACKET] {
                debug!("ignored connection request packet. packet type is not allowed");
                return NULL;
            }

            if buffer.len() != /*1 +*/ VERSION_INFO_BYTES + 8 + 8 + 8 + CONNECT_TOKEN_PRIVATE_BYTES {
                debug!("ignored connection request packet. bad packet length (expected {}, got {})",
                    /*1 +*/ VERSION_INFO_BYTES + 8 + 8 + 8 + CONNECT_TOKEN_PRIVATE_BYTES, buffer.len());
                return NULL;
            }

            if !private_key {
                debug!("ignored connection request packet. no private key");
                return NULL;
            }

            let mut version_info = [0u8; VERSION_INFO_BYTES];
            buffer.read_exact(&mut version_info);
            if version_info != VERSION_INFO {
                debug!("ignored connection request packet. bad version info");
                return NULL;
            }

            let protocol_id = buffer.read_u64()?;
            if protocol_id != self.protocol_id {
                debug!("ignored connection request packet. wrong protocol id. expected {}, got {}",
                    self.protocol_id, protocol_id);
                return NULL;
            }

            let connect_token_expire_timestamp = buffer.read_uint64()?;
            if connect_token_expire_timestamp <= self.current_timestamp {
                debug!("ignored connection request packet. connect token expired");
                return NULL;
            }

            let connect_token_sequence = buffer.read_uint64();

            //assert( buffer - start == 1 + VERSION_INFO_BYTES + 8 + 8 + 8 );

            if ConnectTokenPrivate::decrypt(buffer,
                    CONNECT_TOKEN_PRIVATE_BYTES,
                    version_info,
                    protocol_id,
                    connect_token_expire_timestamp,
                    connect_token_sequence,
                    private_key ) != OK )
            {
                debug!("ignored connection request packet. connect token failed to decrypt");
                return NULL;
            }

            let mut connect_token_data = [0u8; CONNECT_TOKEN_PRIVATE_BYTES];
            buffer.read_exact(&connect_token_data[..])?;

            //assert( buffer - start == 1 + VERSION_INFO_BYTES + 8 + 8 + 8 + CONNECT_TOKEN_PRIVATE_BYTES );

            return Ok(Packet::Request(RequestPacket {
                version_info,
                protocol_id,
                connect_token_expire_timestamp,
                connect_token_sequence,
            })
        } else {
            // *** encrypted packets ***
            if !read_packet_key {
                debug!("ignored encrypted packet. no read packet key for this address");
                return NULL;
            }

            if buffer_length < 1 + 1 + MAC_BYTES  {
                debug!("ignored encrypted packet. packet is too small to be valid ({} bytes)\n", buffer_length);
                return NULL;
            }

            // extract the packet type and number of sequence bytes from the prefix byte

            let packet_type = prefix_byte & 0xF;

            if packet_type >= CONNECTION_NUM_PACKETS {
                debug!("ignored encrypted packet. packet type {} is invalid", packet_type);
                return NULL;
            }

            if !allowed_packets[packet_type] {
                debug!("ignored encrypted packet. packet type {} is not allowed", packet_type);
                return NULL;
            }

            let sequence_bytes = prefix_byte >> 4;

            if sequence_bytes < 1 || sequence_bytes > 8 {
                debug!("ignored encrypted packet. sequence bytes %d is out of range [1,8]", sequence_bytes);
                return NULL;
            }

            if buffer_length < 1 + sequence_bytes + MAC_BYTES {
                debug!("ignored encrypted packet. buffer is too small for sequence bytes + encryption mac");
                return NULL;
            }

            // read variable length sequence number [1,8]
            for i in 0..sequence_bytes {
                let value = buffer.read_uint8()?;
                *sequence |= ( uint64_t) ( value ) << ( 8 * i );
            }

            // replay protection (optional)

            if replay_protection && packet_type >= CONNECTION_KEEP_ALIVE_PACKET {
                if replay_protection_packet_already_received( replay_protection, *sequence ){
                    debug!("ignored connection payload packet. sequence {} already received (replay protection)", *sequence);
                    return NULL;
                }
            }

            // decrypt the per-packet type data

            let additional_data = [u8; VERSION_INFO_BYTES+8+1];
            {
                let mut p = &mut additional_data[..];
                p.write_all(&VERSION_INFO[..]).unwrap();
                p.write_u64(protocol_id).unwrap();
                p.write_u8(prefix_byte).unwrap();
            }

            let nonce = [u8; 12];
            {
                let p = &nonce[..];
                p.write_u32(0);
                p.write_u64(*sequence);
            }

            let encrypted_bytes = (int) ( buffer_length - ( buffer - start ) );

            if encrypted_bytes < MAC_BYTES {
                debug!("ignored encrypted packet. encrypted payload is too small");
                return NULL;
            }

            if ( decrypt_aead( buffer, encrypted_bytes, additional_data, sizeof( additional_data ), nonce, read_packet_key ) != OK )
            {
                printf( LOG_LEVEL_DEBUG, "ignored encrypted packet. failed to decrypt\n" );
                return NULL;
            }

            int decrypted_bytes = encrypted_bytes - MAC_BYTES;

            // process the per-packet type data that was just decrypted
            match packet_type {
                CONNECTION_DENIED_PACKET => {
                    if decrypted_bytes != 0 {
                        debug!("ignored connection denied packet. decrypted packet data is wrong size");
                        return NULL;
                    }
                    return Ok(Packet::Denied)
                }
                CONNECTION_CHALLENGE_PACKET => {
                    if decrypted_bytes != 8 + CHALLENGE_TOKEN_BYTES {
                        debug!("ignored connection challenge packet. decrypted packet data is wrong size");
                        return NULL;
                    }

                    struct connection_challenge_packet_t * packet = (struct connection_challenge_packet_t*) 
                        allocate_function( allocator_context, sizeof( struct connection_challenge_packet_t ) );

                    if ( !packet )
                    {
                        printf( LOG_LEVEL_DEBUG, "ignored connection challenge packet. could not allocate packet struct\n" );
                        return NULL;
                    }
                    packet.packet_type = CONNECTION_CHALLENGE_PACKET;
                    packet.challenge_token_sequence = read_uint64( &buffer );
                    read_bytes( &buffer, packet.challenge_token_data, CHALLENGE_TOKEN_BYTES );
                    return packet;
                }
                CONNECTION_RESPONSE_PACKET => {
                    if ( decrypted_bytes != 8 + CHALLENGE_TOKEN_BYTES )
                    {
                        printf( LOG_LEVEL_DEBUG, "ignored connection response packet. decrypted packet data is wrong size\n" );
                        return NULL;
                    }

                    struct connection_response_packet_t * packet = (struct connection_response_packet_t*) 
                        allocate_function( allocator_context, sizeof( struct connection_response_packet_t ) );

                    if ( !packet )
                    {
                        printf( LOG_LEVEL_DEBUG, "ignored connection response packet. could not allocate packet struct\n" );
                        return NULL;
                    }
                    
                    packet.packet_type = CONNECTION_RESPONSE_PACKET;
                    packet.challenge_token_sequence = read_uint64( &buffer );
                    read_bytes( &buffer, packet.challenge_token_data, CHALLENGE_TOKEN_BYTES );
                    
                    return packet;
                }
                CONNECTION_KEEP_ALIVE_PACKET => {
                    if ( decrypted_bytes != 8 )
                    {
                        printf( LOG_LEVEL_DEBUG, "ignored connection keep alive packet. decrypted packet data is wrong size\n" );
                        return NULL;
                    }

                    struct connection_keep_alive_packet_t * packet = (struct connection_keep_alive_packet_t*) 
                        allocate_function( allocator_context, sizeof( struct connection_keep_alive_packet_t ) );

                    if ( !packet )
                    {
                        printf( LOG_LEVEL_DEBUG, "ignored connection keep alive packet. could not allocate packet struct\n" );
                        return NULL;
                    }
                    
                    packet.packet_type = CONNECTION_KEEP_ALIVE_PACKET;
                    packet.client_index = read_uint32( &buffer );
                    packet.max_clients = read_uint32( &buffer );
                    
                    return packet;
                }
                PAYLOAD_PACKET => {
                    if decrypted_bytes < 1 || decrypted_bytes > MAX_PAYLOAD_BYTES {
                        debug!("ignored connection payload packet. payload packet data is wrong size");
                        return NULL;
                    }
                    struct connection_payload_packet_t * packet = create_payload_packet( decrypted_bytes, allocator_context, allocate_function );

                    if !packet  {
                        printf( LOG_LEVEL_DEBUG, "ignored connection payload packet. could not allocate packet struct\n" );
                        return NULL;
                    }
                    memcpy( packet.payload_data, buffer, decrypted_bytes );
                    return packet;
                }
                DISCONNECT_PACKET => {
                    if decrypted_bytes != 0 {
                        debug!("ignored connection disconnect packet. decrypted packet data is wrong size");
                        return NULL;
                    }
                    return Ok(Packet::Disconnect);
                }
                default:
                    return NULL;
            }
        }
    }
    */
}
