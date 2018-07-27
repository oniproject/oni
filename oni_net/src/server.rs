use std::net::SocketAddr;
use slotmap;
use packet_queue::PacketQueue;
use replay_protection::ReplayProtection;
use utils::UserData;
use Socket;
use MAX_PACKET_BYTES;
use crypto::Key;
use packet::{Allowed, Packet};
use encryption_manager::Mapping;

pub trait Callback: Socket {
    fn connect(&mut self, client: slotmap::Key);
    fn disconnect(&mut self, client: slotmap::Key);

    fn protocol_id(&self) -> u64;

    fn send_packet(&mut self, packet: Packet, packet_key: &Key, addr: SocketAddr) {
        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(&mut data[..], packet_key, self.protocol_id()).unwrap();
        assert!(bytes <= MAX_PACKET_BYTES);
        self.send(addr, &data[..bytes]);
    }
}

    /*
const SERVER_FLAG_IGNORE_CONNECTION_REQUEST_PACKETS       1
const SERVER_FLAG_IGNORE_CONNECTION_RESPONSE_PACKETS      (1<<1)

trait Socket {
    fn send(&mut self, to: SocketAddr, data: &[u8]) -> io::Result<()>;
    //fn receive(to: SocketAddr, data: &[u8]) -> io::Result<()>;
}

void default_config( struct config_t * config )
{
    assert( config );

    config.allocator_context = NULL;
    config.allocate_function = default_allocate_function;
    config.free_function = default_free_function;

    config.network_simulator = NULL;

    config.callback_context = NULL;

    config.connect_disconnect_callback = NULL;
    config.send_loopback_packet_callback = NULL;

    config.override_send_and_receive = 0;
    config.send_packet_override = NULL;
    config.receive_packet_override = NULL;
}
*/

struct Connection {
    connected: bool,
    timeout: u32,
    loopback: bool,
    confirmed: bool,
    encryption_index: usize,
    id: u64,
    sequence: u64,
    last_packet_send_time: f64,
    last_packet_receive_time: f64,
    user_data: UserData,
    replay_protection: ReplayProtection,
    packet_queue: PacketQueue,
    address: SocketAddr,
}

pub struct Server<S: Socket, C: Callback> {
    /*
    struct config_t config;
    struct socket_holder_t socket_holder;
    struct address_t address;

    uint32_t flags;
    */
    time: f64,
    /*
    int running;
    */
    max_clients: u32,
    /*
    int num_connected_clients;
    */
    global_sequence: u64,

    challenge: (u64, Key),

    socket: S,
    callback: C,

    clients: slotmap::SlotMap<Connection>,
    //mapping: HashMap<SocketAddr, slotmap::Key>,

    /*
    int client_connected[MAX_CLIENTS];
    int client_timeout[MAX_CLIENTS];
    int client_loopback[MAX_CLIENTS];
    int client_confirmed[MAX_CLIENTS];
    int client_encryption_index[MAX_CLIENTS];
    uint64_t client_id[MAX_CLIENTS];
    uint64_t client_sequence[MAX_CLIENTS];
    double client_last_packet_send_time[MAX_CLIENTS];
    double client_last_packet_receive_time[MAX_CLIENTS];
    uint8_t client_user_data[MAX_CLIENTS][USER_DATA_BYTES];
    struct replay_protection_t client_replay_protection[MAX_CLIENTS];
    struct packet_queue_t client_packet_queue[MAX_CLIENTS];
    struct address_t client_address[MAX_CLIENTS];
    */

    /*
    struct connect_token_entry_t connect_token_entries[MAX_CONNECT_TOKEN_ENTRIES];
    */
    encryption_manager: Mapping,
    /*
    uint8_t * receive_packet_data[SERVER_MAX_RECEIVE_PACKETS];
    int receive_packet_bytes[SERVER_MAX_RECEIVE_PACKETS];
    struct address_t receive_from[SERVER_MAX_RECEIVE_PACKETS];
    */
}

impl<S: Socket, C: Callback> Server<S, C> {
/*
    int socket_create(
        struct socket_t * socket,
        struct address_t * address,
        int send_buffer_size,
        int receive_buffer_size,
        CONST struct config_t * config )
    {
        assert( socket );
        assert( address );
        assert( config );

        if ( !config.network_simulator )
        {
            if ( !config.override_send_and_receive )
            {
                if ( socket_create( socket, address, send_buffer_size, receive_buffer_size ) != SOCKET_ERROR_NONE )
                {
                    return 0;
                }
            }
        }

        return 1;
    }

    struct t * create_overload( CONST char * address1_string, CONST char * address2_string, CONST struct config_t * config, double time )
    {
        assert( config );
        assert( netcode.initialized );

        struct address_t address1;
        struct address_t address2;

        memset( &address1, 0, sizeof( address1 ) );
        memset( &address2, 0, sizeof( address2 ) );

        if ( parse_address( address1_string, &address1 ) != OK )
        {
            printf( LOG_LEVEL_ERROR, "error: failed to parse server public address\n" );
            return NULL;
        }

        if ( address2_string != NULL && parse_address( address2_string, &address2 ) != OK )
        {
            printf( LOG_LEVEL_ERROR, "error: failed to parse server public address2\n" );
            return NULL;
        }

        struct address_t bind_address_ipv4;
        struct address_t bind_address_ipv6;

        memset( &bind_address_ipv4, 0, sizeof( bind_address_ipv4 ) );
        memset( &bind_address_ipv6, 0, sizeof( bind_address_ipv6 ) );

        struct socket_t socket_ipv4;
        struct socket_t socket_ipv6;

        memset( &socket_ipv4, 0, sizeof( socket_ipv4 ) );
        memset( &socket_ipv6, 0, sizeof( socket_ipv6 ) );

        if ( address1.type == ADDRESS_IPV4 || address2.type == ADDRESS_IPV4 )
        {
            bind_address_ipv4.type = ADDRESS_IPV4;
            bind_address_ipv4.port = address1.type == ADDRESS_IPV4 ? address1.port : address2.port;

            if ( !socket_create( &socket_ipv4, &bind_address_ipv4, SERVER_SOCKET_SNDBUF_SIZE, SERVER_SOCKET_RCVBUF_SIZE, config ) )
            {
                return NULL;
            }
        }

        if ( address1.type == ADDRESS_IPV6 || address2.type == ADDRESS_IPV6 )
        {
            bind_address_ipv6.type = ADDRESS_IPV6;
            bind_address_ipv6.port = address1.type == ADDRESS_IPV6 ? address1.port : address2.port;

            if ( !socket_create( &socket_ipv6, &bind_address_ipv6, SERVER_SOCKET_SNDBUF_SIZE, SERVER_SOCKET_RCVBUF_SIZE, config ) )
            {
                return NULL;
            }
        }

        struct t * server = (struct t*) config.allocate_function( config.allocator_context, sizeof( struct t ) );
        if ( !server )
        {
            socket_destroy( &socket_ipv4 );
            socket_destroy( &socket_ipv6 );
            return NULL;
        }

        if ( !config.network_simulator )
        {
            printf( LOG_LEVEL_INFO, "server listening on %s\n", address1_string );
        }
        else
        {
            printf( LOG_LEVEL_INFO, "server listening on %s (network simulator)\n", address1_string );
        }

        self.config = *config;
        self.socket_holder.ipv4 = socket_ipv4;
        self.socket_holder.ipv6 = socket_ipv6;
        self.address = address1;
        self.flags = 0;
        self.time = time;
        self.running = 0;
        self.max_clients = 0;
        self.num_connected_clients = 0;
        self.global_sequence = 1 << 63;

        memset( self.client_connected, 0, sizeof( self.client_connected ) );
        memset( self.client_loopback, 0, sizeof( self.client_loopback ) );
        memset( self.client_confirmed, 0, sizeof( self.client_confirmed ) );
        memset( self.client_id, 0, sizeof( self.client_id ) );
        memset( self.client_sequence, 0, sizeof( self.client_sequence ) );
        memset( self.client_last_packet_send_time, 0, sizeof( self.client_last_packet_send_time ) );
        memset( self.client_last_packet_receive_time, 0, sizeof( self.client_last_packet_receive_time ) );
        memset( self.client_address, 0, sizeof( self.client_address ) );
        memset( self.client_user_data, 0, sizeof( self.client_user_data ) );

        int i;
        for ( i = 0; i < MAX_CLIENTS; ++i )
            self.client_encryption_index[i] = -1;

        connect_token_entries_reset( self.connect_token_entries );

        encryption_manager_reset( &self.encryption_manager );

        for ( i = 0; i < MAX_CLIENTS; ++i )
            replay_protection_reset( &self.client_replay_protection[i] );

        memset( &self.client_packet_queue, 0, sizeof( self.client_packet_queue ) );

        return server;
    }

    struct t * create( CONST char * address_string, CONST struct config_t * config, double time )
    {
        return create_overload( address_string, NULL, config, time );
    }

    void destroy( struct t * server )
    {
        stop( server );

        socket_destroy( &self.socket_holder.ipv4 );
        socket_destroy( &self.socket_holder.ipv6 );

        self.config.free_function( self.config.allocator_context, server );
    }

    void start( struct t * server, int max_clients )
    {
        assert( max_clients > 0 );
        assert( max_clients <= MAX_CLIENTS );

        if ( self.running )
            stop( server );

        info!("server started with {} client slots", max_clients);

        self.running = 1;
        self.max_clients = max_clients;
        self.num_connected_clients = 0;
        self.challenge_sequence = 0;
        self.challenge_key = Key::generate();

        for i in 0..self.max_clients {
            packet_queue_init( &self.client_packet_queue[i], self.config.allocator_context, self.config.allocate_function, self.config.free_function );
        }
    }
    */

    fn send_global_packet(&mut self, mut packet: Packet, to: SocketAddr, packet_key: &Key) {
        packet.set_sequence(self.global_sequence);

        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(
            &mut data[..],
            packet_key,
            self.socket.protocol_id(),
        ).unwrap();
        assert!(bytes <= MAX_PACKET_BYTES);
        self.socket.send(to, &data[..bytes]);
        self.global_sequence += 1;
    }

    fn send_client_packet(&mut self, mut packet: Packet, client_key: slotmap::Key) {
        /*
        assert( packet );
        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        assert( self.client_connected[client_index] );
        assert( !self.client_loopback[client_index] );
        */

        let client = &mut self.clients[client_key];

        if !self.encryption_manager.touch(client.encryption_index, client.address, self.time) {
            error!("encryption mapping is out of date for client {:?}", client_key);
            return;
        }

        let packet_key = self.encryption_manager.get_send_key(client.encryption_index)
            .unwrap();

        packet.set_sequence(client.sequence);
        client.sequence += 1;
        self.socket.send_packet(packet, packet_key, client.address);
        client.last_packet_send_time = self.time;
    }

    fn disconnect_client_internal(&mut self, client_key: slotmap::Key, send_disconnect_packets: bool) {
    /*
        assert( self.running );
        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        assert( self.client_connected[client_index] );
        assert( !self.client_loopback[client_index] );

        printf( LOG_LEVEL_INFO, "server disconnected client %d\n", client_index );

        if ( self.config.connect_disconnect_callback )
        {
            self.config.connect_disconnect_callback( self.config.callback_context, client_index, 0 );
        }

        if send_disconnect_packets {
            debug!("server sent disconnect packets to client {}", client_key);

            int i;
            for ( i = 0; i < NUM_DISCONNECT_PACKETS; ++i )
            {
                printf( LOG_LEVEL_DEBUG, "server sent disconnect packet %d\n", i );

                struct connection_disconnect_packet_t packet;
                packet.packet_type = CONNECTION_DISCONNECT_PACKET;

                send_client_packet( server, &packet, client_index );
            }
        }

        while ( 1 )
        {
            void * packet = packet_queue_pop( &self.client_packet_queue[client_index], NULL );
            if ( !packet )
                break;
            self.config.free_function( self.config.allocator_context, packet );
        }

        client.packet_queue.clear();
        client.replay_protection.reset();

        self.encryption_manager.remove_encryption_mapping(client.address, self.time);

        client.connected = false;
        client.confirmed = false;
        client.id = 0;
        client.sequence = 0;
        client.last_packet_send_time = 0.0;
        client.last_packet_receive_time = 0.0;
        //memset( &self.client.address[client_index], 0, sizeof( struct address_t ) );
        client.encryption_index = -1;
        //memset( self.client_user_data[client_index], 0, USER_DATA_BYTES );

        self.num_connected_clients--;

        assert( self.num_connected_clients >= 0 );
        */
    }

    fn disconnect_client(&mut self, client_key: slotmap::Key) {
        self.disconnect_client_internal(client_key, true);
    }

    fn disconnect_all_clients(&mut self) {
        let keys: Vec<_> = self.clients.keys().collect();
        for key in keys {
            self.disconnect_client_internal(key, true);
        }
    }

    /*
    pub fn stop(&mut self) {
        if !self.running {
            return;
        }

        self.disconnect_all_clients();
        self.running = false;

        self.max_clients = 0;
        self.num_connected_clients = 0;

        self.global_sequence = 0;
        self.challenge_sequence = 0;
        memset( self.challenge_key, 0, KEY_BYTES );
        connect_token_entries_reset( self.connect_token_entries );
        self.encryption_manager.reset( &self.encryption_manager );
        info!("server stopped");
    }
    */

    fn find_client_index_by_id(&self, client_id: u64) -> slotmap::Key {
        self.clients.iter()
            .find_map(|(k, c)| if c.connected && c.id == client_id {
                Some(k)
            } else {
                None
            })
            .unwrap_or_default()
    }

    fn find_client_index_by_address(&self, addr: SocketAddr) -> slotmap::Key {
        self.clients.iter()
            .find_map(|(k, c)| if c.connected && c.address == addr {
                Some(k)
            } else {
                None
            })
            .unwrap_or_default()
    }

        /*
    fn process_connection_request_packet(&mut self, from: SocketAddr, packet: RequestPacket) {
        (void) from;

        struct connect_token_private_t connect_token_private;
        if ConnectTokenPrivate::read( packet.connect_token_data, CONNECT_TOKEN_PRIVATE_BYTES, &connect_token_private ) != OK {
            debug!("server ignored connection request. failed to read connect token");
            return;
        }

        int found_address = 0;
        int i;
        for ( i = 0; i < connect_token_private.num_addresses; ++i )
        {
            if self.address == connect_token_private.addresses[i] {
                found_address = 1;
            }
        }
        if !connect_token_private.addresses.iter().any(|a| a == self.address) {
            debug!("server ignored connection request. server address not in connect token whitelist");
            return;
        }

        if self.find_client_index_by_address(from).is_none() {
            debug!("server ignored connection request. a client with this address is already connected");
            return;
        }

        if self.find_client_index_by_id(connect_token_private.client_id).is_none() {
            debug!("server ignored connection request. a client with this id is already connected");
            return;
        }

        if !connect_token_entries_find_or_add(
            self.connect_token_entries,
            from,
            packet.connect_token_data + CONNECT_TOKEN_PRIVATE_BYTES - MAC_BYTES,
            self.time )
        {
            debug!("server ignored connection request. connect token has already been used");
            return;
        }

        if self.num_connected_clients == self.max_clients {
            debug!("server denied connection request. server is full");
            self.send_global_packet(Packet::Denied { sequence: self.global_sequence }, from, connect_token_private.to_client_key);
            return;
        }

        let expire_time = if connect_token_private.timeout_seconds >= 0 {
            self.time + connect_token_private.timeout_seconds
        } else {
            -1.0
        };

        if !self.encryption_manager.add_encryption_mapping(
            from,
            connect_token_private.to_client_key,
            connect_token_private.client_to_key,
            self.time,
            expire_time,
            connect_token_private.timeout_seconds)
        {
            debug!("server ignored connection request. failed to add encryption mapping");
            return;
        }

        struct challenge_token_t challenge_token;
        challenge_token.client_id = connect_token_private.client_id;
        memcpy( challenge_token.user_data, connect_token_private.user_data, USER_DATA_BYTES );

        struct connection_challenge_packet_t challenge_packet;
        challenge_packet.packet_type = CONNECTION_CHALLENGE_PACKET;
        challenge_packet.challenge_token_sequence = self.challenge_sequence;
        write_challenge_token( &challenge_token, challenge_packet.challenge_token_data, CHALLENGE_TOKEN_BYTES );
        if ( encrypt_challenge_token(
                challenge_packet.challenge_token_data,
                CHALLENGE_TOKEN_BYTES,
                self.challenge_sequence,
                self.challenge_key ) != OK )
        {
            debug!("server ignored connection request. failed to encrypt challenge token");
            return;
        }

        self.challenge_sequence += 1;

        debug!("server sent connection challenge packet");
        self.send_global_packet(&challenge_packet, from, connect_token_private.to_client_key);
    }
        */

    /*
    int find_free_client_index( struct t * server )
    {
        int i;
        for ( i = 0; i < self.max_clients; ++i )
        {
            if ( !self.client_connected[i] )
                return i;
        }

        return -1;
    }
    */

    fn connect_client(
        &mut self,
        client_key: slotmap::Key,
        address: SocketAddr,
        client_id: u64,
        encryption_index: usize,
        timeout_seconds: u32,
        user_data: UserData
    )
    {
        /*
        assert( self.running );
        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        assert( address );
        assert( encryption_index != -1 );
        assert( user_data );

        self.num_connected_clients += 1;

        assert!(self.num_connected_clients <= self.max_clients);
        assert!(self.client_connected[client_index] == 0);

        self.encryption_manager.set_expire_time(encryption_index, -1.0);

        self.client_connected[client_index] = 1;
        self.client_timeout[client_index] = timeout_seconds;
        self.client_encryption_index[client_index] = encryption_index;
        self.client_id[client_index] = client_id;
        self.client_sequence[client_index] = 0;
        self.client_address[client_index] = *address;
        self.client_last_packet_send_time[client_index] = self.time;
        self.client_last_packet_receive_time[client_index] = self.time;
        memcpy( self.client_user_data[client_index], user_data, USER_DATA_BYTES );

        char address_string[MAX_ADDRESS_STRING_LENGTH];

        info!("server accepted client {} {} in slot {}",
            address, client_id, client_index);

        self.send_client_packet(Packet::KeepAlive {
            client_index,
            max_clients: self.max_clients,
        }, client_key);
        */

        self.callback.connect(client_key);
    }

    /*
    void process_connection_response_packet(
        &mut self,
        struct address_t * from, 
        struct connection_response_packet_t * packet, 
        int encryption_index )
    {
        if ( decrypt_challenge_token( packet.challenge_token_data, 
                                            CHALLENGE_TOKEN_BYTES, 
                                            packet.challenge_token_sequence, 
                                            self.challenge_key ) != OK )
        {
            printf( LOG_LEVEL_DEBUG, "server ignored connection response. failed to decrypt challenge token\n" );
            return;
        }

        struct challenge_token_t challenge_token;
        if ( read_challenge_token( packet.challenge_token_data, CHALLENGE_TOKEN_BYTES, &challenge_token ) != OK )
        {
            printf( LOG_LEVEL_DEBUG, "server ignored connection response. failed to read challenge token\n" );
            return;
        }

        let packet_send_key = self.encryption_manager.get_send_key(encryption_index );

        if !packet_send_key {
            debug!("server ignored connection response. no packet send key");
            return;
        }

        if self.find_client_index_by_address(from) != -1 {
            debug!("server ignored connection response. a client with this address is already connected");
            return;
        }

        if find_client_index_by_id( server, challenge_token.client_id ) != -1 {
            debug!("server ignored connection response. a client with this id is already connected");
            return;
        }

        if self.num_connected_clients == self.max_clients {
            debug!("server denied connection response. server is full");
            self.send_global_packet(Packet::Denied, from, packet_send_key );
            return;
        }

        let client_index = self.find_free_client_index();
        assert!(client_index != -1);
        let timeout_seconds = self.encryption_manager.get_timeout(encryption_index);
        self.connect_client(client_index, from, challenge_token.client_id, encryption_index, timeout_seconds, challenge_token.user_data);
    }
    */

    fn process_packet_internal(
        &mut self,
        from: SocketAddr,
        packet: Packet,
        sequence: u64,
        encryption_index: usize,
        client_key: slotmap::Key,
        )
    {
        match packet {
        Packet::Request { .. } => {
            /*
            self.process_connection_request_packet(from, packet);
            */
        }
        Packet::Response { .. } => {
            /*
            self.process_connection_response_packet(from, packet, encryption_index);
            */
        }
        Packet::KeepAlive { .. } => {
            if let Some(client) = self.clients.get(client_key) {
                /*
                debug!("server received connection keep alive packet from client {}", client_key);
                client.last_packet_receive_time = self.time;
                if !client_confirmed {
                    debug!("server confirmed connection with client {}", client_key);
                    client.confirmed = true;
                }
                */
            }
        }
        Packet::Payload { .. } => {
            if let Some(client) = self.clients.get(client_key) {
                /*
                debug!("server received connection payload packet from client {:?}", client_key);
                client.last_packet_receive_time = self.time;
                if !client.confirmed {
                    debug!("server confirmed connection with client {}", client_key);
                    client.confirmed = true
                }
                client.packet_queue.push(packet, sequence);
                */
            }
        }
        Packet::Disconnect { .. } => {
            if self.clients.contains_key(client_key) {
                /*
                debug!("server received disconnect packet from client {}", client_key);
                self.disconnect_client_internal(client_key, false);
                */
            }
        }

        _ => (),
        }
    }

    fn process_packet(&mut self, from: SocketAddr, packet: &[u8]) {
        let allowed =
            Allowed::REQUEST |
            Allowed::RESPONSE |
            Allowed::KEEP_ALIVE |
            Allowed::PAYLOAD |
            Allowed::DISCONNECT;

        /*
        let current_timestamp = time();

        //uint64_t sequence;

        let client_key = self.find_client_index_by_address(from);
        let encryption_index = if let Some(client) = self.clients.get(client_key) {
            client.encryption_index
        } else {
            self.encryption_manager.find_encryption_mapping(from, self.time)
        };
        */

        /*
        let read_packet_key = self.encryption_manager.get_receive_key(encryption_index);
        if !read_packet_key && packet_data[0] != 0 {
            debug!("server could not process packet because no encryption mapping exists for {}", from);
            return;
        }

        void * packet = read_packet(
            packet_data, 
            packet_bytes, 
            &sequence, 
            read_packet_key, 
            self.config.protocol_id, 
            current_timestamp, 
            self.config.private_key, 
            allowed_packets, 
            ( client_index != -1 ) ? &self.client_replay_protection[client_index] : NULL, 
            );

        if ( !packet )
            return;

        self.process_packet_internal(from, packet, sequence, encryption_index, client_index);
        */
    }

    /*
    void read_and_process_packet(
        struct t * server,
        struct address_t * from,
        uint8_t * packet_data,
        int packet_bytes,
        uint64_t current_timestamp,
        uint8_t * allowed_packets )
    {
        if ( !self.running )
            return;

        if ( packet_bytes <= 1 )
            return;

        uint64_t sequence;

        let client_index = self.find_client_index_by_address(from);
        let encryption_index = if client_index != -1 {
            assert( client_index >= 0 );
            assert( client_index < self.max_clients );
            self.client_encryption_index[client_index]
        } else {
            self.encryption_manager.find_encryption_mapping(from, self.time);
        }

        let read_packet_key = encryption_manager.get_receive_key( &self.encryption_manager, encryption_index );

        if !read_packet_key && packet_data[0] != 0 {
            debug!("server could not process packet because no encryption mapping exists for {}", from);
            return;
        }

        void * packet = read_packet( packet_data,
            packet_bytes,
            &sequence,
            read_packet_key,
            self.config.protocol_id,
            current_timestamp,
            self.config.private_key,
            allowed_packets,
            ( client_index != -1 ) ? &self.client_replay_protection[client_index] : NULL, 
            self.config.allocator_context, 
            self.config.allocate_function );

        if !packet {
            return;
        }

        self.process_packet_internal(from, packet, sequence, encryption_index, client_index);
    }

    void receive_packets( struct t * server )
    {
        uint8_t allowed_packets[CONNECTION_NUM_PACKETS];
        memset( allowed_packets, 0, sizeof( allowed_packets ) );
        allowed_packets[CONNECTION_REQUEST_PACKET] = 1;
        allowed_packets[CONNECTION_RESPONSE_PACKET] = 1;
        allowed_packets[CONNECTION_KEEP_ALIVE_PACKET] = 1;
        allowed_packets[CONNECTION_PAYLOAD_PACKET] = 1;
        allowed_packets[CONNECTION_DISCONNECT_PACKET] = 1;

        let current_timestamp = time();

        if ( !self.config.network_simulator )
        {
            // process packets received from socket

            while ( 1 )
            {
                struct address_t from;
                
                uint8_t packet_data[MAX_PACKET_BYTES];
                
                int packet_bytes = 0;
                
                if ( self.config.override_send_and_receive )
                {
                    packet_bytes = self.config.receive_packet_override( self.config.callback_context, &from, packet_data, MAX_PACKET_BYTES );
                }
                else
                {
                    if (self.socket_holder.ipv4.handle != 0)
                        packet_bytes = socket_receive_packet( &self.socket_holder.ipv4, &from, packet_data, MAX_PACKET_BYTES );

                    if ( packet_bytes == 0 && self.socket_holder.ipv6.handle != 0)
                        packet_bytes = socket_receive_packet( &self.socket_holder.ipv6, &from, packet_data, MAX_PACKET_BYTES );
                }

                if ( packet_bytes == 0 )
                    break;

                read_and_process_packet( server, &from, packet_data, packet_bytes, current_timestamp, allowed_packets );
            }
        }
        else
        {
            // process packets received from network simulator

            int num_packets_received = network_simulator_receive_packets( self.config.network_simulator, 
                                                                                &self.address, 
                                                                                SERVER_MAX_RECEIVE_PACKETS, 
                                                                                self.receive_packet_data, 
                                                                                self.receive_packet_bytes, 
                                                                                self.receive_from );

            int i;
            for ( i = 0; i < num_packets_received; ++i )
            {
                read_and_process_packet( server, 
                                                        &self.receive_from[i], 
                                                        self.receive_packet_data[i], 
                                                        self.receive_packet_bytes[i], 
                                                        current_timestamp, 
                                                        allowed_packets );

                self.config.free_function( self.config.allocator_context, self.receive_packet_data[i] );
            }
        }
    }

    fn send_packets(&mut self) {
        if !self.running {
            return;
        }
        for i in 0..self.max_clients {
            if self.client_connected[i] && !self.client_loopback[i] &&
                (self.client_last_packet_send_time[i] + (1.0 / PACKET_SEND_RATE) <= self.time)
            {
                debug!("server sent connection keep alive packet to client {}", i);
                self.send_client_packet(Packet::KeepAlive {
                    client_index: i,
                    max_clients: self.max_clients,
                }, i);
            }
        }
    }

    void check_for_timeouts( struct t * server )
    {
        if ( !self.running )
            return;

        int i;
        for ( i = 0; i < self.max_clients; ++i )
        {
            if ( self.client_connected[i] && self.client_timeout[i] > 0 && !self.client_loopback[i] &&
                ( self.client_last_packet_receive_time[i] + self.client_timeout[i] <= self.time ) )
            {
                printf( LOG_LEVEL_INFO, "server timed out client %d\n", i );
                disconnect_client_internal( server, i, 0 );
                return;
            }
        }
    }

    int client_connected( struct t * server, int client_index )
    {
        if ( !self.running )
            return 0;

        if ( client_index < 0 || client_index >= self.max_clients )
            return 0;

        return self.client_connected[client_index];
    }

    uint64_t client_id( struct t * server, int client_index )
    {
        if ( !self.running )
            return 0;

        if ( client_index < 0 || client_index >= self.max_clients )
            return 0;

        return self.client_id[client_index];
    }

    fn next_packet_sequence(&mut self, client_index: usize) -> u64 {
        if !self.client_connected[client_index] {
            return 0;
        }
        self.client_sequence[client_index]
    }

    pub fn send_packet(&mut self, int client_index, CONST uint8_t * packet_data, int packet_bytes )
    {
        assert( packet_data );
        assert( packet_bytes >= 0 );
        assert( packet_bytes <= MAX_PACKET_SIZE );

        if ( !self.running )
            return;

        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        if ( !self.client_connected[client_index] )
            return;

        if ( !self.client_loopback[client_index] )
        {
            uint8_t buffer[MAX_PAYLOAD_BYTES*2];

            struct connection_payload_packet_t * packet = (struct connection_payload_packet_t*) buffer;

            packet.packet_type = CONNECTION_PAYLOAD_PACKET;
            packet.payload_bytes = packet_bytes;
            memcpy( packet.payload_data, packet_data, packet_bytes );

            if ( !self.client_confirmed[client_index] )
            {
                struct connection_keep_alive_packet_t keep_alive_packet;
                keep_alive_packet.packet_type = CONNECTION_KEEP_ALIVE_PACKET;
                keep_alive_packet.client_index = client_index;
                keep_alive_packet.max_clients = self.max_clients;
                send_client_packet( server, &keep_alive_packet, client_index );
            }

            send_client_packet( server, packet, client_index );
        }
        else
        {
            assert( self.config.send_loopback_packet_callback );

            self.config.send_loopback_packet_callback( self.config.callback_context,
                                                        client_index, 
                                                        packet_data, 
                                                        packet_bytes, 
                                                        self.client_sequence[client_index]++ );

            self.client_last_packet_send_time[client_index] = self.time;
        }
    }

    uint8_t * receive_packet(&mut self, client_index: Key, int * packet_bytes, uint64_t * packet_sequence) {
        if ( !self.running )
            return NULL;

        if !self.client_connected[client_index]
            return NULL;

        struct connection_payload_packet_t * packet = (struct connection_payload_packet_t*)
            packet_queue_pop( &self.client_packet_queue[client_index], packet_sequence );

        if packet {
            assert( packet.packet_type == CONNECTION_PAYLOAD_PACKET );
            *packet_bytes = packet.payload_bytes;
            assert( *packet_bytes >= 0 );
            assert( *packet_bytes <= MAX_PACKET_BYTES );
            return (uint8_t*) &packet.payload_data;
        } else {
            return NULL;
        }
    }

    void free_packet( struct t * server, void * packet )
    {
        assert( packet );
        (void) server;
        int offset = offsetof( struct connection_payload_packet_t, payload_data );
        self.config.free_function( self.config.allocator_context, ( (uint8_t*) packet ) - offset );
    }

    pub fn num_connected_clients(&mut self) -> usize { self.num_connected_clients }
    pub fn  client_user_data( struct t * server, int client_index ) -> &UserData {
        &self.client_user_data[client_index]
    }

    pub fn running(&self) -> bool { self.running }
    pub fn max_clients(&self) -> usize { self.max_clients }
    pub fn update(&mut self, time: f64) {
        self.time = time;
        self.receive_packets();
        self.send_packets();
        self.check_for_timeouts();
    }
    */

/*
    pub fn connect_loopback_client(&mut self, client_index: isize, client_id: u64, CONST uint8_t * user_data ) {
        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        assert( self.running );
        assert( !self.client_connected[client_index] );

        self.num_connected_clients++;

        assert( self.num_connected_clients <= self.max_clients );

        self.client_loopback[client_index] = 1;
        self.client_connected[client_index] = 1;
        self.client_confirmed[client_index] = 1;
        self.client_encryption_index[client_index] = -1;
        self.client_id[client_index] = client_id;
        self.client_sequence[client_index] = 0;
        memset( &self.client_address[client_index], 0, sizeof( struct address_t ) );
        self.client_last_packet_send_time[client_index] = self.time;
        self.client_last_packet_receive_time[client_index] = self.time;

        if ( user_data )
        {
            memcpy( self.client_user_data[client_index], user_data, USER_DATA_BYTES );
        }
        else
        {
            memset( self.client_user_data[client_index], 0, USER_DATA_BYTES );
        }

        printf( LOG_LEVEL_INFO, "server connected loopback client %.16" PRIx64 " in slot %d\n", client_id, client_index );

        if ( self.config.connect_disconnect_callback )
        {
            self.config.connect_disconnect_callback( self.config.callback_context, client_index, 1 );
        }
    }

    void disconnect_loopback_client( struct t * server, int client_index )
    {
        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        assert( self.running );
        assert( self.client_connected[client_index] );
        assert( self.client_loopback[client_index] );

        printf( LOG_LEVEL_INFO, "server disconnected loopback client %d\n", client_index );

        if ( self.config.connect_disconnect_callback )
        {
            self.config.connect_disconnect_callback( self.config.callback_context, client_index, 0 );
        }

        while ( 1 )
        {
            void * packet = packet_queue_pop( &self.client_packet_queue[client_index], NULL );
            if ( !packet )
                break;
            self.config.free_function( self.config.allocator_context, packet );
        }

        packet_queue_clear( &self.client_packet_queue[client_index] );

        self.client_connected[client_index] = 0;
        self.client_loopback[client_index] = 0;
        self.client_confirmed[client_index] = 0;
        self.client_id[client_index] = 0;
        self.client_sequence[client_index] = 0;
        self.client_last_packet_send_time[client_index] = 0.0;
        self.client_last_packet_receive_time[client_index] = 0.0;
        memset( &self.client_address[client_index], 0, sizeof( struct address_t ) );
        self.client_encryption_index[client_index] = -1;
        memset( self.client_user_data[client_index], 0, USER_DATA_BYTES );

        self.num_connected_clients--;

        assert( self.num_connected_clients >= 0 );
    }

    int client_loopback( struct t * server, int client_index ) {
        assert( self.running );
        self.client_loopback[client_index];
    }

    void process_loopback_packet( struct t * server, int client_index, CONST uint8_t * packet_data, int packet_bytes, uint64_t packet_sequence )
    {
        assert( client_index >= 0 );
        assert( client_index < self.max_clients );
        assert( packet_data );
        assert( packet_bytes >= 0 );
        assert( packet_bytes <= MAX_PACKET_SIZE );
        assert( self.client_connected[client_index] );
        assert( self.client_loopback[client_index] );
        assert( self.running );

        struct connection_payload_packet_t * packet = create_payload_packet( packet_bytes, self.config.allocator_context, self.config.allocate_function );
        if ( !packet )
            return;

        memcpy( packet.payload_data, packet_data, packet_bytes );

        debug!("server processing loopback packet from client {}", client_index);

        self.client_last_packet_receive_time[client_index] = self.time;

        self.client_packet_queue[client_index].packet_queue_push(packet, packet_sequence);
    }
*/

    //pub fn get_port(&self) -> u16 { self.address.port() }
}
