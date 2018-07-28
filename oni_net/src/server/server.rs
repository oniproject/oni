use std::{
    net::SocketAddr,
    time::Instant,
    io,
};

use crate::{
    Socket,
    utils::time,
    crypto::Key,
    encryption_manager::{Mapping, Keys},
    token::Challenge,
    packet::{
        MAX_PACKET_BYTES,
        MAX_PAYLOAD_BYTES,
        Allowed,
        Request,
        Encrypted,
        NoProtection,
    },
};

use super::{Slot, Clients};

pub trait Callback {
    fn connect(&mut self, slot: Slot);
    fn disconnect(&mut self, slot: Slot);

    fn receive(&mut self, slot: Slot, seq: u64, payload: &[u8]);
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

pub struct Server<S: Socket, C: Callback> {
    protocol_id: u64,
    private_key: Key,


    /*
    struct config_t config;
    struct socket_holder_t socket_holder;
    struct addr_t addr;
    */

    time: Instant,
    max_clients: u32,
    global_sequence: u64,

    challenge_sequence: u64,
    challenge_key: Key,

    socket: S,
    callback: C,

    clients: Clients,
    //mapping: HashMap<SocketAddr, Slot>,

    /*
    int client_connected[MAX_CLIENTS];
    int client_timeout[MAX_CLIENTS];
    int client_loopback[MAX_CLIENTS];
    int client_encryption_index[MAX_CLIENTS];
    uint64_t client_id[MAX_CLIENTS];
    uint64_t client_sequence[MAX_CLIENTS];
    double client_last_packet_send_time[MAX_CLIENTS];
    double client_last_packet_receive_time[MAX_CLIENTS];
    uint8_t client_user_data[MAX_CLIENTS][USER_DATA_BYTES];
    struct replay_protection_t client_replay_protection[MAX_CLIENTS];
    struct packet_queue_t client_packet_queue[MAX_CLIENTS];
    */

    /*
    struct connect_token_entry_t connect_token_entries[MAX_CONNECT_TOKEN_ENTRIES];
    */
    encryption_manager: Mapping,
    /*
    uint8_t * receive_packet_data[SERVER_MAX_RECEIVE_PACKETS];
    int receive_packet_bytes[SERVER_MAX_RECEIVE_PACKETS];
    struct addr_t receive_from[SERVER_MAX_RECEIVE_PACKETS];
    */
}

impl<S: Socket, C: Callback> Server<S, C> {
    pub fn max_clients(&self) -> u32 { self.max_clients }
    pub fn update(&mut self) {
        self.time = Instant::now();
        self.receive_packets();
        self.send_packets();
        self.check_for_timeouts();
    }
/*
    int socket_create(
        struct socket_t * socket,
        struct addr_t * addr,
        int send_buffer_size,
        int receive_buffer_size,
        CONST struct config_t * config )
    {
        assert( socket );
        assert( addr );
        assert( config );

        if ( !config.network_simulator )
        {
            if ( !config.override_send_and_receive )
            {
                if ( socket_create( socket, addr, send_buffer_size, receive_buffer_size ) != SOCKET_ERROR_NONE )
                {
                    return 0;
                }
            }
        }

        return 1;
    }

    struct t * create_overload( CONST char * addr1_string, CONST char * addr2_string, CONST struct config_t * config, double time )
    {
        assert( config );
        assert( netcode.initialized );

        struct addr_t addr1;
        struct addr_t addr2;

        memset( &addr1, 0, sizeof( addr1 ) );
        memset( &addr2, 0, sizeof( addr2 ) );

        if ( parse_addr( addr1_string, &addr1 ) != OK )
        {
            printf( LOG_LEVEL_ERROR, "error: failed to parse server public addr\n" );
            return NULL;
        }

        if ( addr2_string != NULL && parse_addr( addr2_string, &addr2 ) != OK )
        {
            printf( LOG_LEVEL_ERROR, "error: failed to parse server public addr2\n" );
            return NULL;
        }

        struct addr_t bind_addr_ipv4;
        struct addr_t bind_addr_ipv6;

        memset( &bind_addr_ipv4, 0, sizeof( bind_addr_ipv4 ) );
        memset( &bind_addr_ipv6, 0, sizeof( bind_addr_ipv6 ) );

        struct socket_t socket_ipv4;
        struct socket_t socket_ipv6;

        memset( &socket_ipv4, 0, sizeof( socket_ipv4 ) );
        memset( &socket_ipv6, 0, sizeof( socket_ipv6 ) );

        if ( addr1.type == ADDRESS_IPV4 || addr2.type == ADDRESS_IPV4 )
        {
            bind_addr_ipv4.type = ADDRESS_IPV4;
            bind_addr_ipv4.port = addr1.type == ADDRESS_IPV4 ? addr1.port : addr2.port;

            if ( !socket_create( &socket_ipv4, &bind_addr_ipv4, SERVER_SOCKET_SNDBUF_SIZE, SERVER_SOCKET_RCVBUF_SIZE, config ) )
            {
                return NULL;
            }
        }

        if ( addr1.type == ADDRESS_IPV6 || addr2.type == ADDRESS_IPV6 )
        {
            bind_addr_ipv6.type = ADDRESS_IPV6;
            bind_addr_ipv6.port = addr1.type == ADDRESS_IPV6 ? addr1.port : addr2.port;

            if ( !socket_create( &socket_ipv6, &bind_addr_ipv6, SERVER_SOCKET_SNDBUF_SIZE, SERVER_SOCKET_RCVBUF_SIZE, config ) )
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
            printf( LOG_LEVEL_INFO, "server listening on %s\n", addr1_string );
        }
        else
        {
            printf( LOG_LEVEL_INFO, "server listening on %s (network simulator)\n", addr1_string );
        }

        self.config = *config;
        self.socket_holder.ipv4 = socket_ipv4;
        self.socket_holder.ipv6 = socket_ipv6;
        self.addr = addr1;
        self.flags = 0;
        self.time = time;
        self.max_clients = 0;
        self.num_connected_clients = 0;
        self.global_sequence = 1 << 63;

        memset( self.client_connected, 0, sizeof( self.client_connected ) );
        memset( self.client_loopback, 0, sizeof( self.client_loopback ) );
        memset( self.client_id, 0, sizeof( self.client_id ) );
        memset( self.client_sequence, 0, sizeof( self.client_sequence ) );
        memset( self.client_last_packet_send_time, 0, sizeof( self.client_last_packet_send_time ) );
        memset( self.client_last_packet_receive_time, 0, sizeof( self.client_last_packet_receive_time ) );
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

    struct t * create( CONST char * addr_string, CONST struct config_t * config, double time )
    {
        return create_overload( addr_string, NULL, config, time );
    }
    */

    fn send_global_packet(&mut self, addr: SocketAddr, packet: Encrypted) {
        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(
            &mut data[..],
            self.encryption_manager.find(addr).unwrap().recv_key(),
            self.protocol_id,
            self.global_sequence,
        ).unwrap();
        assert!(bytes <= MAX_PACKET_BYTES);
        self.socket.send(addr, &data[..bytes]);
        self.global_sequence += 1;
    }

    fn send_client_packet(&mut self, slot: Slot, packet: Encrypted) {
        let client = self.clients.get_mut(slot).unwrap();
        if !self.encryption_manager.touch(client.addr()) {
            error!("encryption mapping is out of date for client {:?}", slot);
            return;
        }

        let key = self.encryption_manager.find(client.addr())
            .map(|e| e.send_key())
            .unwrap();

        let seq = client.send(self.time);
        let mut data: [u8; MAX_PACKET_BYTES] = unsafe { std::mem::uninitialized() };
        let len = packet.write(&mut data, key, self.protocol_id, seq).unwrap();
        self.socket.send(client.addr(), &data[..len]);
    }

    fn disconnect_client_internal(&mut self, client_key: Slot, send_disconnect_packets: bool) {
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

        self.encryption_manager.remove_encryption_mapping(client.addr, self.time);

        self.num_connected_clients--;

        assert( self.num_connected_clients >= 0 );
        */
    }

    fn disconnect_client(&mut self, slot: Slot) {
        self.disconnect_client_internal(slot, true);
    }

    fn disconnect_all_clients(&mut self) {
        let keys: Vec<_> = self.clients.keys().collect();
        for slot in keys {
            self.disconnect_client_internal(slot, true);
        }
    }

    /*
    fn process_connection_request_packet(&mut self, from: SocketAddr, packet: RequestPacket) {
        (void) from;

        struct connect_token_private_t connect_token_private;
        if ConnectTokenPrivate::read( packet.connect_token_data, CONNECT_TOKEN_PRIVATE_BYTES, &connect_token_private ) != OK {
            return;
        }

        int found_addr = 0;
        int i;
        for ( i = 0; i < connect_token_private.num_addres; ++i )
        {
            if self.addr == connect_token_private.addres[i] {
                found_addr = 1;
            }
        }
        if !connect_token_private.addres.iter().any(|a| a == self.addr) {
            return;
        }

        if self.find_client_index_by_addr(from).is_none() {
            return;
        }

        if self.find_client_index_by_id(connect_token_private.client_id).is_none() {
            return;
        }

        if !connect_token_entries_find_or_add(
            self.connect_token_entries,
            from,
            packet.connect_token_data + CONNECT_TOKEN_PRIVATE_BYTES - MAC_BYTES,
            self.time )
        {
            return;
        }

        if self.num_connected_clients == self.max_clients {
            self.send_global_packet(Packet::Denied, from, connect_token_private.to_client_key, self.global_sequence );
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
            return;
        }

        self.challenge_sequence += 1;

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

        /*
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

        let client_key = self.find_client_index_by_addr(from);
        let encryption_index = if let Some(client) = self.clients.get(client_key) {
            client.encryption_index
        } else {
            self.encryption_manager.find_encryption_mapping(from, self.time)
        };
        */

        /*
        let read_packet_key = self.encryption_manager.get_receive_key(encryption_index);
        if !read_packet_key && packet_data[0] != 0 {
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
    */


    fn receive_packets(&mut self) {
        let current_timestamp = time();

        let mut packet = [0u8; MAX_PACKET_BYTES];
        while let Some((len, from)) = self.socket.recv(&mut packet[..]) {
            if len <= 1 {
                continue;
            }
            let packet = &mut packet[..len];
            if packet[0] == 0 {
                self.process_request(from, packet, current_timestamp);
            } else if self.encryption_manager.contains(from) {
                self.process_encrypted(from, packet);
            }
        }
    }

    fn process_request(&mut self, addr: SocketAddr, packet: &mut [u8], current_timestamp: u64) {
        let slot = self.clients.slot_by_addr(addr);
        let request = Request::read(packet, current_timestamp, self.protocol_id, &self.private_key);
        let request = match request {
            Some(r) => r,
            None => return,
        };
        // TODO
    }
    fn process_encrypted(&mut self, addr: SocketAddr, packet: &mut [u8]) -> io::Result<()> {
        let slot = self.clients.slot_by_addr(addr);

        if !slot.is_null() {
            let packet = {
                Encrypted::read(
                    packet,
                    self.clients.get_mut(slot).unwrap(),
                    self.encryption_manager.find(addr).unwrap().recv_key(),
                    self.protocol_id,
                    Allowed::KEEP_ALIVE | Allowed::DISCONNECT | Allowed::PAYLOAD,
                )
            };

            match if let Some(p) = packet { p } else { return Ok(()) } {
                Encrypted::KeepAlive { .. } => {
                    self.clients.get_mut(slot).unwrap().recv(self.time);
                }
                Encrypted::Payload { sequence, len, data } => {
                    self.clients.get_mut(slot).unwrap().recv(self.time);
                    self.callback.receive(slot, sequence, &data[..len]);
                }
                Encrypted::Disconnect { .. } => {
                    self.disconnect_client_internal(slot, false);
                }
                _ => unreachable!(),
            }
        } else {
            let (id, slot) = {
                if self.clients.len() >= self.max_clients as usize {
                    self.send_global_packet(addr, Encrypted::Denied);
                    return Ok(());
                }

                let key = self.encryption_manager.find(addr).unwrap();
                let packet = Encrypted::read(packet, &mut NoProtection, key.recv_key(), self.protocol_id, Allowed::RESPONSE);
                let challenge = if let Some(Encrypted::Response { mut challenge_data, challenge_sequence }) = packet {
                    Challenge::decrypt(&mut challenge_data[..], challenge_sequence, &self.challenge_key)?;
                    Challenge::read(&mut challenge_data[..])?
                } else {
                    return Ok(());
                };

                if self.clients.has_id(challenge.client_id) {
                    return Ok(());
                }

                key.disable_expire();
                (challenge.client_id, self.clients.insert(addr, challenge, self.time, key.timeout()))
            };
            info!("server accepted client[{}] {:?} in slot {:?}", id, addr, slot);
            self.send_keep_alive(slot);
            self.callback.connect(slot);
        }
        Ok(())
    }

    fn send_keep_alive(&mut self, slot: Slot) {
        let max_clients = self.max_clients;
        self.send_client_packet(slot, Encrypted::KeepAlive {
            client_index: slot.index(),
            max_clients,
        });
    }

    fn send_packets(&mut self) {
        /*
        if !self.running {
            return;
        }
        for i in 0..self.max_clients {
            if self.client_connected[i] && !self.client_loopback[i] &&
                (self.client_last_packet_send_time[i] + (1.0 / PACKET_SEND_RATE) <= self.time)
            {
                self.send_client_packet(Packet::KeepAlive {
                    client_index: i,
                    max_clients: self.max_clients,
                }, i);
            }
        }
        */
    }

    fn check_for_timeouts(&mut self) {
        /*
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
        */
    }

    /*
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
    */

    pub fn send_packet(&mut self, slot: Slot, payload: &[u8]) {
        assert!(payload.len() <= MAX_PAYLOAD_BYTES);

        let c = match self.clients.get(slot).map(|c| c.is_confirmed()) {
            Some(c) => c,
            None => return,
        };
        if !c {
            let max_clients = self.max_clients;
            self.send_client_packet(slot, Encrypted::KeepAlive {
                client_index: slot.index(),
                max_clients,
            });
        }



        let (data, len) = array_from_slice_uninitialized!(payload, MAX_PAYLOAD_BYTES);
        self.send_client_packet(slot, Encrypted::Payload {
            sequence: 0,
            len,
            data,
        });
    }

    /*
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
    pub fn client_user_data( struct t * server, int client_index ) -> &UserData {
        &self.client_user_data[client_index]
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
        self.client_encryption_index[client_index] = -1;
        self.client_id[client_index] = client_id;
        self.client_sequence[client_index] = 0;
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
        self.client_id[client_index] = 0;
        self.client_sequence[client_index] = 0;
        self.client_last_packet_send_time[client_index] = 0.0;
        self.client_last_packet_receive_time[client_index] = 0.0;
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

        let packet = create_payload_packet( packet_bytes, self.config.allocator_context, self.config.allocate_function );
        if ( !packet )
            return;

        memcpy( packet.payload_data, packet_data, packet_bytes );

        self.client_last_packet_receive_time[client_index] = self.time;

        self.client_packet_queue[client_index].packet_queue_push(packet, packet_sequence);
    }
*/

    //pub fn get_port(&self) -> u16 { self.addr.port() }
}
