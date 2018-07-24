use byteorder::{LE, ByteOrder};

pub const PLAYLOAD_MAX_SIZE: usize = 1200;
pub const CHALLENGE_SIZE: usize = 308;
pub const RESPONSE_SIZE: usize = 308;
pub const KEEP_ALIVE_SIZE: usize = 8;

pub struct RequestAssociatedData {
    /// "NETCODE 1.01" ASCII with null terminator.
    pub version_info: [u8; 13],
    /// 64 bit value unique to this particular game/application
    pub protocol_id: u64,
    /// 64 bit unix timestamp when this connect token expires
    pub expire_timestamp: u64,
}

pub enum Packet<'a> {
    Request(Request<'a>),
    Denied(Denied<'a>),
    Challenge(Challenge<'a>),
    Response(Response<'a>),
    KeepAlive(KeepAlive<'a>),
    Payload(Payload<'a>),
    Disconnect(Disconnect<'a>),

    Invalid(&'a [u8]),
}

const REQUEST: u8 = 0;
const DENIED: u8 = 1;
const CHALLENGE: u8 = 2;
const RESPONSE: u8 = 3;
const KEEP_ALIVE: u8 = 4;
const PAYLOAD: u8 = 5;
const DISCONNECT: u8 = 6;

impl<'a> Packet<'a> {
    fn parse(packet: &'a [u8]) -> Self {
        let id = packet[0];
        match id {
            0 => Packet::request(packet),
            1 => Packet::denied(packet),
            2 => Packet::challenge(packet),
            3 => Packet::response(packet),
            4 => Packet::keep_alive(packet),
            5 => Packet::payload(packet),
            6 => Packet::disconnect(packet),

            _ => Packet::Invalid(packet),
        }
    }

    fn request(packet: &'a [u8])    -> Self { Packet::Request(Request(packet)) }
    fn denied(packet: &'a [u8])     -> Self { Packet::Denied(Denied(packet)) }
    fn challenge(packet: &'a [u8])  -> Self { Packet::Challenge(Challenge(packet)) }
    fn response(packet: &'a [u8])   -> Self { Packet::Response(Response(packet)) }
    fn keep_alive(packet: &'a [u8]) -> Self { Packet::KeepAlive(KeepAlive(packet)) }
    fn payload(packet: &'a [u8])    -> Self { Packet::Payload(Payload(packet)) }
    fn disconnect(packet: &'a [u8]) -> Self { Packet::Disconnect(Disconnect(packet)) }
}

pub struct Request<'a>(&'a [u8]);
pub struct Denied<'a>(&'a [u8]);
pub struct Challenge<'a>(&'a [u8]);
pub struct Response<'a>(&'a [u8]);
pub struct KeepAlive<'a>(&'a [u8]);
pub struct Payload<'a>(&'a [u8]);
pub struct Disconnect<'a>(&'a [u8]);

impl<'a> From<&'a [u8]> for Request<'a> { fn from(p: &'a [u8]) -> Self { Request(p) } }
//impl<'a> From<&'a [u8]> for Challenge<'a> { fn from(p: &'a [u8]) -> Self { Challenge(p) } }
//impl<'a> From<&'a [u8]> for KeepAlive<'a> { fn from(p: &'a [u8]) -> Self { KeepAlive(p) } }
//impl<'a> From<&'a [u8]> for Response<'a> { fn from(p: &'a [u8]) -> Self { Response(p) } }

impl<'a> Request<'a> {
    fn check_size(&self) -> bool { self.0.len() == 1062 }

    pub fn version_info(&self) -> &[u8] { &self.0[1..14] }

    fn associated(&self) -> &[u8] { &self.0[1..30] }

    pub fn protocol_id(&self) -> u64 { LE::read_u64(&self.0[14..22]) }
    pub fn expire_timestamp(&self) -> u64 { LE::read_u64(&self.0[22..30]) }
    pub fn sequence(&self) -> u64 { LE::read_u64(&self.0[30..38]) }
    pub fn token(&self) -> &[u8] { &self.0[38..] }
}

fn reading_encrypted_packet(is_server: bool, packet: &[u8]) {
    // The following steps are taken when reading an encrypted packet, in this exact order:

    // If the packet size is less than 18 bytes then it is too small to possibly be valid,
    if packet.len() < 18 {
        return; // ignore the packet.
    }

    let prefix_byte = packet[0];
    let packet_type = prefix_byte & 0x0F;
    let sequence_bytes = (prefix_byte & 0xF0) >> 4;

    // If the low 4 bits of the prefix byte are greater than or equal to 7,
    if packet_type >= 7 {
        return; // the packet type is invalid, ignore the packet.
    }

    if is_server {
        // The server ignores packets with type connection challenge packet.
        if packet_type == CHALLENGE {
            return;
        }
    } else {
        // The client ignores packets with type connection request packet and connection response packet.
        if packet_type == REQUEST || packet_type == RESPONSE {
            return;
        }
    }

    // If the high 4 bits of the prefix byte (sequence bytes) are outside the range [1,8],
    if sequence_bytes == 0 || sequence_bytes > 8 {
        return; // ignore the packet.
    }

    // If the packet size is less than 1 + sequence bytes + 16,
    if packet.len() < 1 + sequence_bytes as usize + 16 {
        return; // it cannot possibly be valid, ignore the packet.
    }

    // If the packet type fails the replay protection test, ignore the packet.
    // See the section on replay protection below for details.
    if packet_type == PAYLOAD || packet_type == KEEP_ALIVE || packet_type == DISCONNECT {
        unimplemented!("replay protection test");
    }

    // If the per-packet type data fails to decrypt, ignore the packet.
    unimplemented!("decrypt");

    // If the per-packet type data size does not match the expected size for the packet type, ignore the packet.
    match packet_type {
        //  0 bytes for connection denied packet
        DENIED if packet.len() != 0 => return,
        //  308 bytes for connection challenge packet
        CHALLENGE if packet.len() != CHALLENGE_SIZE => return,
        //  308 bytes for connection response packet
        RESPONSE if packet.len() != RESPONSE_SIZE => return,
        //  8 bytes for connection keep-alive packet
        KEEP_ALIVE if packet.len() != KEEP_ALIVE_SIZE => return,
        //  [1,1200] bytes for connection payload packet
        PAYLOAD if packet.len() == 0 || packet.len() > PLAYLOAD_MAX_SIZE => return,
        //  0 bytes for connection disconnect packet
        DISCONNECT if packet.len() != 0 => return,
        _ => (),
    }

    // If all the above checks pass, the packet is processed.
}

pub struct KeepAlivePacket {
    pub client_index: u32,
    pub max_clients: u32,
}

pub struct ChallengePacket {
    pub challenge_token_sequence: u64,
    pub encrypted_challenge_token_data: [u8; 300],
}

pub struct ResponsePacket {
    pub challenge_token_sequence: u64,
    pub encrypted_challenge_token_data: [u8; 300],
}
