pub const NUM_DISCONNECT_PACKETS: usize = 10;

use std::net::SocketAddr;

use MAX_PACKET_BYTES;
use MAX_PACKET_SIZE;
use MAX_PAYLOAD_BYTES;
use utils::time;
use packet::{Packet, Allowed, Context};

use connect_token::ConnectToken;
use challenge_token::CHALLENGE_TOKEN_BYTES;
use replay_protection::ReplayProtection;
use packet_queue::PacketQueue;

pub trait Callback {
    fn state_change(&mut self, old: State, new: State);
    fn send(&mut self, addr: SocketAddr, packet: &[u8]);
    fn recv(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddr)>;
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq)]
pub enum State {
    TokenExpired = -6,
    InvalidToken = -5,
    TimedOut = -4,
    ResponseTimedOut = -3,
    RequestTimedOut = -2,
    Denied = -1,

    Disconnected = 0,
    SendingRequest = 1,
    SendingResponse = 2,
    Connected = 3,
}

impl Default for State {
    fn default() -> Self {
        State::Disconnected
    }
}

/*
impl State {
    pub fn is_ok(&self) -> bool {
        match self {
            State::Disconnected |
            State::SendingRequest |
            State::SendingResponse |
            State::Connected => true,
            _ => false,
        }
    }
    pub fn is_err(&self) -> bool {
        match self {
            State::Disconnected |
            State::SendingRequest |
            State::SendingResponse |
            State::Connected => false,
            _ => true,
        }
    }
}

pub struct Client<C: Callback> {
    config: C,

    state: State,

    time: f64,
    connect_start_time: f64,
    last_packet_send_time: f64,
    last_packet_receive_time: f64,

    should_disconnect: Option<State>,

    sequence: u64,

    client_index: u32,
    max_clients: u32,
    server_address_index: usize,

    addr: SocketAddr,
    server_address: SocketAddr,
    connect_token: ConnectToken,
    /*
    socket_holder: SocketHoder,
    */
    context: Context,
    replay_protection: ReplayProtection,
    packet_receive_queue: PacketQueue<[u8; MAX_PAYLOAD_BYTES]>,
    challenge_token_sequence: u64,
    challenge_token_data: [u8; CHALLENGE_TOKEN_BYTES],
    /*
    uint8_t * receive_packet_data[CLIENT_MAX_RECEIVE_PACKETS],
    int receive_packet_bytes[CLIENT_MAX_RECEIVE_PACKETS],
    */
}


impl<C: Callback> Client<C> {
    pub fn new(addr: SocketAddr, config: C, time: f64) -> Self {
        Self {
            addr,
            config: config,
            time,

            client_index: 0,
            max_clients: 0,
            sequence: 0,

            state: State::Disconnected,
            connect_start_time: 0.0,
            last_packet_send_time: -1000.0,
            last_packet_receive_time: -1000.0,
            should_disconnect: None,
            server_address_index: 0,

            replay_protection: ReplayProtection::default(),
            packet_receive_queue: PacketQueue::default(),
            challenge_token_sequence: 0,
            challenge_token_data: [0u8; CHALLENGE_TOKEN_BYTES],
        }
    }

    /*
    struct t * create_overload(
        CONST char * address1_string,
        CONST char * address2_string,
        CONST struct config_t * config,
        double time )
    {
        assert( config );
        assert( netcode.initialized );

        struct address_t address1;
        struct address_t address2;

        memset( &address1, 0, sizeof( address1 ) );
        memset( &address2, 0, sizeof( address2 ) );

        if ( parse_address( address1_string, &address1 ) != OK )
        {
            printf( LOG_LEVEL_ERROR, "error: failed to parse client address\n" );
            return NULL;
        }

        if ( address2_string != NULL && parse_address( address2_string, &address2 ) != OK )
        {
            printf( LOG_LEVEL_ERROR, "error: failed to parse client address2\n" );
            return NULL;
        }


        struct socket_t socket_ipv4;
        struct socket_t socket_ipv6;

        memset( &socket_ipv4, 0, sizeof( socket_ipv4 ) );
        memset( &socket_ipv6, 0, sizeof( socket_ipv6 ) );

        if ( address1.type == ADDRESS_IPV4 || address2.type == ADDRESS_IPV4 )
        {
            if ( !socket_create( &socket_ipv4, address1.type == ADDRESS_IPV4 ? &address1 : &address2, CLIENT_SOCKET_SNDBUF_SIZE, CLIENT_SOCKET_RCVBUF_SIZE, config ) )
            {
                return NULL;
            }
        }

        if ( address1.type == ADDRESS_IPV6 || address2.type == ADDRESS_IPV6 )
        {
            if ( !socket_create( &socket_ipv6, address1.type == ADDRESS_IPV6 ? &address1 : &address2, CLIENT_SOCKET_SNDBUF_SIZE, CLIENT_SOCKET_RCVBUF_SIZE, config ) )
            {
                return NULL;
            }
        }

        struct t * client = (struct t*) config.allocate_function( config.allocator_context, sizeof( struct t ) );

        if ( !client )
        {
            socket_destroy( &socket_ipv4 );
            socket_destroy( &socket_ipv6 );
            return NULL;
        }

        struct address_t socket_address = address1.type == ADDRESS_IPV4 ? socket_ipv4.address : socket_ipv6.address;

        if ( !config.network_simulator )
        {
            printf( LOG_LEVEL_INFO, "client started on port %d\n", socket_address.port );
        }
        else
        {
            printf( LOG_LEVEL_INFO, "client started on port %d (network simulator)\n", socket_address.port );
        }

        self.config = *config;
        self.socket_holder.ipv4 = socket_ipv4;
        self.socket_holder.ipv6 = socket_ipv6;
        self.address = config.network_simulator ? address1 : socket_address;
        self.state = CLIENT_STATE_DISCONNECTED;
        self.time = time;
        self.connect_start_time = 0.0;
        self.last_packet_send_time = -1000.0;
        self.last_packet_receive_time = -1000.0;
        self.should_disconnect = 0;
        self.should_disconnect_state = CLIENT_STATE_DISCONNECTED;
        self.sequence = 0;
        self.client_index = 0;
        self.max_clients = 0;
        self.server_address_index = 0;
        self.challenge_token_sequence = 0;
        memset( &self.server_address, 0, sizeof( struct address_t ) );
        memset( &self.connect_token, 0, sizeof( struct connect_token_t ) );
        memset( &self.context, 0, sizeof( struct context_t ) );
        memset( self.challenge_token_data, 0, CHALLENGE_TOKEN_BYTES );

        packet_queue_init( &self.packet_receive_queue, config.allocator_context, config.allocate_function, config.free_function );

        replay_protection_reset( &self.replay_protection );

        return client;
    }

    fn destroy(&mut) {
        self.disconnect();
    }
    */

    fn set_state(&mut self, state: State) {
        debug!("client changed state from {:?} to {:?}", self.state, state);
        self.config.state_change(self.state, state);
        self.state = state;
    }

    fn reset_before_next_connect(&mut self) {
        self.connect_start_time = self.time;
        self.last_packet_send_time = self.time - 1.0;
        self.last_packet_receive_time = self.time;
        self.should_disconnect = None;
        self.challenge_token_sequence = 0;
        self.challenge_token_data = [0u8; CHALLENGE_TOKEN_BYTES];
        self.replay_protection.reset();
    }

    /*
    fn reset_connection_data(&mut self, state: State) {
        self.sequence = 0;
        self.client_index = 0;
        self.max_clients = 0;
        self.connect_start_time = 0.0;
        self.server_address_index = 0;
        memset( &self.server_address, 0, sizeof( struct address_t ) );
        memset( &self.connect_token, 0, sizeof( struct connect_token_t ) );
        memset( &self.context, 0, sizeof( struct context_t ) );

        self.set_state(state);
        self.reset_before_next_connect();
        self.packet_receive_queue.clear();
    }
    */

    fn connect(&mut self, connect_token: &[u8]) {
        self.disconnect();

        if ConnectToken::read(connect_token, &self.connect_token).is_err() {
            self.set_state(State::InvalidToken);
            return;
        }

        self.server_address_index = 0;
        self.server_address = self.connect_token.server_addresses[0];

        info!("client connecting to server {} [{}/{}]",
            self.server_address,
            self.server_address_index + 1,
            self.connect_token.num_server_addresses);

        self.context.read_packet_key = self.connect_token.server_to_client_key;
        self.context.write_packet_key = self.connect_token.client_to_server_key;

        self.reset_before_next_connect();
        self.set_state(State::SendingRequest);
    }

    /*
    fn process_packet(&mut self, from: SocketAddr, packet_data: &mut [u8]) {
        let allowed_packets = Allowed::DENIED | Allowed::CHALLENGE |
            Allowed::KEEP_ALIVE | Allowed::PAYLOAD | Allowed::DISCONNECT;

        let current_timestamp = time();

        if let Ok((packet, sequence)) = read_packet(
            packet_data,
            self.context.read_packet_key,
            self.connect_token.protocol_id,
            current_timestamp,
            NULL,
            allowed_packets,
            &self.replay_protection,
            self.config.allocator_context,
            self.config.allocate_function )
        {
            self.process_packet_internal(from, packet, sequence);
        }
    }

    fn process_packet_internal(&mut self, from: SocketAddr, packet: &Packet, sequence: u64) {
        if from != self.server_address {
            return;
        }
        match packet {
            Packet::Denied => {
                if self.state == State::SendingConnectionRequest || self.state == State::SendingConnectionResponse {
                    self.should_disconnect = Some(State::ConnectionDenied);
                    self.last_packet_receive_time = self.time;
                }
            }
            Packet::Challenge { sequence, data }=> {
                if self.state == State::SendingConnectionRequest {
                    debug!("client received connection challenge packet from server");
                    self.challenge_token_sequence = sequence;
                    self.challenge_token_data = data;
                    self.last_packet_receive_time = self.time;
                    self.set_state(State::SendingConnectionResponse);
                }
            }
            Packet::KeepAlive { client_index, max_clients } => {
                if self.state == State::Connected {
                    debug!("client received connection keep alive packet from server");
                    self.last_packet_receive_time = self.time;
                } else if self.state == State::SendingConnectionResponse {
                    debug!("client received connection keep alive packet from server");
                    self.last_packet_receive_time = self.time;
                    self.client_index = client_index;
                    self.max_clients = max_clients;
                    self.set_state(State::Connected);
                    info!("client connected to server");
                }
            }
            Packet::Payload { len, data } => {
                if self.state == State::Connected {
                    debug!("client received connection payload packet from server");
                    self.packet_receive_queue.push(data, sequence);
                    self.last_packet_receive_time = self.time;
                }
            }
            Packet::Disconnect => {
                if self.state == State::Connected {
                    debug!("client received disconnect packet from server");
                    self.should_disconnect = Some(State::Disconnected);
                    self.last_packet_receive_time = self.time;
                }
            }
            _ => (),
        }
    }

    fn receive_packets(&self) {
        let mut allowed_packets =
            Allowed::DENIED |
            Allowed::CHALLENGE |
            Allowed::KEEP_ALIVE |
            Allowed::PAYLOAD |
            Allowed::DISCONNECT;

        let current_timestamp = time();

        let mut buf = [0u8; MAX_PACKET_BYTES];
        while let Some((bytes, from)) = self.cb.receive_packet(&mut buf) {
            if bytes == 0 {
                break;
            }

            if let Some((packet, sequence)) = read_packet(
                &mut buf[..bytes],
                self.context.read_packet_key,
                self.connect_token.protocol_id,
                current_timestamp,
                None,
                None,
                allowed_packets,
                &self.replay_protection)
            {
                self.process_packet_internal(from, packet, sequence);
            }
        }
    }

    fn send_packet_to_server_internal(&mut self, packet: Packet) {
        let mut data: [0u8; MAX_PACKET_BYTES];
        let bytes = write_packet(
            packet,
            &mut data[..],
            self.sequence,
            self.context.write_packet_key,
            self.connect_token.protocol_id,
        );
        assert!(bytes <= MAX_PACKET_BYTES);
        self.cb.send_packet(self.address, &mut data[..bytes]);
        self.last_packet_send_time = self.time;

        self.sequence += 1;
    }

    fn send_packets(&mut self) {
        if self.last_packet_send_time + (1.0 / PACKET_SEND_RATE) >= self.time {
            return;
        }

        match self.state {
            State::SendingRequest => {
                debug!("client sent connection request packet to server");
                self.send_packet_to_server_internal(Packet::Request {
                    version_info: ;:VERSION_INFO,
                    protocol_id: self.connect_token.protocol_id,
                    connect_token_expire_timestamp: self.connect_token.expire_timestamp,
                    connect_token_sequence: self.connect_token.sequence,
                    connect_token_data: self.connect_token.private_data,
                });
            }
            State::SendingResponse => {
                debug!("client sent connection response packet to server");
                self.send_packet_to_server_internal(Packet::Response {
                    challenge_token_sequence: self.challenge_token_sequence,
                    challenge_token_data: self.challenge_token_data,
                });
            }
            State::Connected => {
                debug!("client sent connection keep-alive packet to server");
                self.send_packet_to_server_internal(Packet::KeepAlive {
                    client_index: 0,
                    max_clients: 0,
                });
            }
            _ => (),
        }
    }

    fn connect_to_next_server(&mut self) -> bool {
        if self.server_address_index + 1 >= self.connect_token.num_server_addresses {
            debug!("client has no more servers to connect to");
            return 0;
        }

        self.server_address_index += 1;
        self.server_address = self.connect_token.server_addresses[self.server_address_index];

        self.reset_before_next_connect();

        info!("client connecting to next server {} [{}/{}]",
            self.server_address,
            self.server_address_index + 1,
            self.connect_token.num_server_addresses);

        self.set_state(State::SendingConnectionRequest);
        true
    }

    fn update(&mut self, time: f64) {
        self.time = time;

        self.receive_packets();
        self.send_packets();

        if self.state > State::Disconnected && self.state < State::Connected {
            let connect_token_expire_seconds = self.connect_token.expire_timestamp - self.connect_token.create_timestamp;
            if self.time - self.connect_start_time >= connect_token_expire_seconds {
                info!("client connect failed. connect token expired");
                self.disconnect_internal(State::TokenExpired, false);
                return;
            }
        }

        if let Some(state) = self.should_disconnect {
            debug!("client should disconnect . {:?}", state);
            if self.connect_to_next_server() { return }
            self.disconnect_internal(state, false);
            return;
        }

        if !(self.connect_token.timeout_seconds > 0 && self.last_packet_receive_time + self.connect_token.timeout_seconds < time) {
            return;
        }

        match self.state {
            State::SendingConnectionRequest => {
                info!("client connect failed. connection request timed out");
                if self.connect_to_next_server() { return }
                self.disconnect_internal(State::RequestTimedOut, false);
            }
            State::SendingConnectionResponse => {
                info!("client connect failed. connection response timed out");
                if self.connect_to_next_server() { return }
                self.disconnect_internal(State::ResponseTimedOut, false);
            }
            State::Connected => {
                info!("client connection timed out");
                self.disconnect_internal(State::TimedOut, false);
            }
            _ => (),
        }
    }

    pub fn send_packet(&mut self, data: &[u8]) {
        assert!(data.len() <= MAX_PACKET_SIZE);
        if self.state != State::Connected {
            return;
        }
        self.send_packet_to_server_internal(Packet::Payload(data.into());
    }

    fn receive_packet(&mut self) -> Option<(Vec<u8>, u64)> {
        self.packet_receive_queue.pop()
    }

    fn disconnect(&mut self) {
        self.disconnect_internal(State::Disconnected, true);
    }

    fn disconnect_internal(&mut self, destination_state: State, send_disconnect_packets: bool)  {
        assert!(destination_state <= State::Disconnected);

        if self.state <= State::Disconnected || self.state == destination_state {
            return;
        }

        info!("client disconnected");

        if send_disconnect_packets && self.state > State::Disconnected {
            debug!("client sent disconnect packets to server");

            for i in 0..NUM_DISCONNECT_PACKETS {
                debug!("client sent disconnect packet {}", i);
                self.send_packet_to_server_internal(Packet::Disconnect);
            }
        }

        self.reset_connection_data(destination_state);
    }
    */

    pub fn next_packet_sequence(&self) -> u64 { self.sequence }
    pub fn port(&self) -> u16 { self.addr.port() }
    //pub fn server_address(&self) -> SocketAddr { self.server_address }

    pub fn state(&self) -> State { self.state }
    pub fn index(&self) -> u32 { self.client_index }
    pub fn max_clients(&self) -> u32 { self.max_clients }
}

#[test]
fn client_error_connect_token_expired() {
    let simulator = Simulator::builder()
        .latency_milliseconds(250)
        .jitter_milliseconds(250)
        .packet_loss_percent(5.0)
        .duplicate_packet_percent(10.0)
        .build();

    let client_addr = "[::]:50000".parse().unwrap();
    let server_addr = "[::1]:40000".parse().unwrap();
    let connect_token = [0u8; CONNECT_TOKEN_BYTES];
    let client_id = random_u64();

    let time = 0.0f64;

    struct netcode_client_config_t client_config;
    netcode_default_client_config( &client_config );
    client_config.network_simulator = network_simulator;
    let client = Client::new(client_addr, &client_config, time);

    ConnectToken::generate(
        vec![server_addr], vec![server_addr],
        0, TEST_TIMEOUT_SECONDS, client_id, TEST_PROTOCOL_ID,
        0, &private_key, &mut connect_token[..],
    ).unwrap();

    client.connect(connect_token);
    client.update(time);
    assert!(client.state(), State::TokenExpired);
}
*/
