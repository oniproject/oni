use std::{
    net::SocketAddr,
    time::{Instant, Duration},
    io,
};

use crate::{
    Socket,
    NUM_DISCONNECT_PACKETS,
    PACKET_SEND_DELTA,
    utils::time,
    crypto::{Key, MAC_BYTES, keygen},
    encryption_manager::Mapping,
    token::{Challenge, Private},
    packet::{
        MAX_PACKET_BYTES,
        MAX_PAYLOAD_BYTES,
        is_request_packet,
        Allowed,
        Request,
        Encrypted,
    },
};

use super::{Slot, Clients};

pub trait Callback {
    fn connect(&mut self, slot: Slot);
    fn disconnect(&mut self, slot: Slot);
    fn receive(&mut self, slot: Slot, payload: &[u8]);
}

pub enum Event<'a> {
    Connect(Slot),
    Disconnect(Slot),
    Receive(Slot, &'a [u8]),
}

pub struct Server<S: Socket, C: Callback> {
    protocol_id: u64,
    private_key: Key,

    time: Instant,
    timestamp: u64,
    //max_clients: u32,
    global_sequence: u64,

    challenge: (u64, Key),

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
    uint8_t client_user_data[MAX_CLIENTS][USER_DATA];
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
    pub fn new(protocol_id: u64, pkey: Key, callback: C, socket: S) -> Self {
        Self {
            protocol_id,
            callback,
            socket,
            private_key: pkey,
            global_sequence: 1,
            challenge: (1, keygen()),
            encryption_manager: Mapping::new(),
            clients: Clients::new(),
            time: Instant::now(),
            timestamp: time(),
            //global_sequence: 1 << 63
        }
    }

    pub fn update<F>(&mut self, mut callback: F)
        where F: FnMut(Event)
    {
        let now = Instant::now();
        self.time = now;
        self.timestamp = time();
        self.encryption_manager.advance();


        let mut packet = [0u8; MAX_PACKET_BYTES];
        while let Some((len, from)) = self.socket.recv(&mut packet[..]) {
            if len <= 1 {
                continue;
            }
            let packet = &mut packet[..len];
            if is_request_packet(packet) {
                self.process_request(from, packet);
            } else if let Some(key) = self.encryption_manager.recv_key(from).cloned() {
                self.process_encrypted(from, packet, key);
            }
        }

        // send_packets
        for client in self.clients.filter_last_send(self.time - PACKET_SEND_DELTA) {
            let packet = Encrypted::keep_alive();
            if !self.encryption_manager.touch(client.addr()) {
                //error!("encryption mapping is out of date for client {:?}", slot);
                return;
            }

            let key = self.encryption_manager
                .find(client.addr())
                .map(|e| e.send_key())
                .unwrap();

            let seq = client.send(self.time);
            let mut data: [u8; MAX_PACKET_BYTES] = unsafe { std::mem::uninitialized() };
            let len = packet.write(&mut data, key, self.protocol_id, seq).unwrap();
            self.socket.send(client.addr(), &data[..len]);
        }

        // check_for_timeouts
        let em = &mut self.encryption_manager;
        self.clients.retain(|slot, client| {
            let remove = client.is_timeout(now);
            if remove {
                em.remove(client.addr());
                callback(Event::Disconnect(slot));
            }
            remove
        });
    }

    fn process_request(&mut self, addr: SocketAddr, packet: &mut [u8]) {
        let request = none_ret!(Request::read(
            packet,
            self.timestamp,
            self.protocol_id,
            &self.private_key,
        ));

        let token = err_ret!(Private::read(&request.token[..]));

        /* FIXME
        let serv = self.socket.addr();
        if !token.server_addresses.iter().any(|a| a == &serv) {
            return Ok(());
        }
        */

        if self.clients.has_addr(addr) || self.clients.has_id(token.client_id) {
            return;
        }

        /* TODO
        let mac = &request.private_data[Private::BYTES - MAC_BYTES..];
        if !self.connect_token_entries.find_or_add(addr, mac, self.time) {
            return Ok(());
        }
        */

        /* TODO
        if self.clients.len() >= self.max_clients as usize {
            self.send_global_packet(addr, &token.server_key, Encrypted::Disconnect);
            return;
        }
        */

        if !self.encryption_manager.insert(
            addr,
            token.server_key,
            token.client_key,
            token.timeout,
        ) {
            return;
        }

        let seq = self.challenge.0;
        self.challenge.0 += 1;

        self.send_global_packet(addr, &token.server_key, Encrypted::Challenge {
            seq,
            data: err_ret!(token.challenge(seq, &self.challenge.1)),
        });
    }

    fn process_encrypted(&mut self, addr: SocketAddr, packet: &mut [u8], recv_key: Key) {
        let slot = self.clients.slot_by_addr(addr);

        if !slot.is_null() {
            let packet =  Encrypted::read(
                packet,
                self.clients.get_mut(slot).unwrap(),
                self.encryption_manager.find(addr).unwrap().recv_key(),
                self.protocol_id,
                Allowed::CONNECTED,
            );

            match if let Some(p) = packet { p } else { return; } {
                Encrypted::Payload { len, data } => {
                    self.clients.get_mut(slot).unwrap().recv(self.time);
                    if len != 0 {
                        self.callback.receive(slot, &data[..len]);
                    }
                }
                Encrypted::Disconnect => self.disconnect_client_internal(slot),
                _ => unreachable!(),
            }
        } else {
            let challenge = none_ret!(Encrypted::read_challenge(
                packet, &recv_key, self.protocol_id, &self.challenge.1,
            ));

            /* XXX
            if self.clients.len() >= self.max_clients as usize {
                self.send_global_packet(addr, &recv_key, Encrypted::Disconnect);
                return Ok(());
            }
            */

            if self.clients.has_id(challenge.id) {
                return;
            }

            let key = self.encryption_manager.find(addr).unwrap();
            key.disable_expire();
            let id = challenge.id;
            let slot = self.clients.insert(addr, challenge, self.time, key.timeout());
            //info!("server accepted client[{}] {:?} in slot {:?}", id, addr, slot);
            self.send_client_packet(slot, Encrypted::keep_alive());
            self.callback.connect(slot);
        }
    }

    pub fn send_packet(&mut self, slot: Slot, payload: &[u8]) {
        let packet = Encrypted::payload(payload)
            .expect("payload length must less or equal MAX_PAYLOAD_BYTES");
        self.send_client_packet(slot, packet);
    }

    fn send_global_packet(&mut self, addr: SocketAddr, recv_key: &Key, packet: Encrypted) {
        let seq = self.global_sequence;
        let protocol = self.protocol_id;
        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(&mut data[..], recv_key, protocol, seq).unwrap();

        debug_assert!(bytes <= MAX_PACKET_BYTES);

        self.socket.send(addr, &data[..bytes]);
        self.global_sequence += 1;
    }

    fn send_client_packet(&mut self, slot: Slot, packet: Encrypted) {
        let client = self.clients.get_mut(slot).unwrap();
        if !self.encryption_manager.touch(client.addr()) {
            //error!("encryption mapping is out of date for client {:?}", slot);
            return;
        }

        let key = self.encryption_manager
            .find(client.addr())
            .map(|e| e.send_key())
            .unwrap();

        let seq = client.send(self.time);
        let mut data: [u8; MAX_PACKET_BYTES] = unsafe { std::mem::uninitialized() };
        let len = packet.write(&mut data, key, self.protocol_id, seq).unwrap();
        self.socket.send(client.addr(), &data[..len]);
    }

    pub fn disconnect(&mut self, slot: Slot) {
        if true { //send_disconnect_packets {
            for _ in 0..NUM_DISCONNECT_PACKETS {
                self.send_client_packet(slot, Encrypted::Disconnect);
            }
        }
        self.disconnect_client_internal(slot);
    }

    fn disconnect_client_internal(&mut self, slot: Slot) {
        let client = self.clients.remove(slot).unwrap();
        self.encryption_manager.remove(client.addr());
        self.callback.disconnect(slot);
    }


    /*
    uint64_t client_id( struct t * server, int client_index )
    {
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

    /*
    pub fn num_connected_clients(&mut self) -> usize { self.num_connected_clients }
    pub fn client_user_data( struct t * server, int client_index ) -> &UserData {
        &self.client_user_data[client_index]
    }
    */
}
