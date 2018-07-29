use std::{
    net::SocketAddr,
    time::{Instant, Duration},
    io,
};

use crate::{
    Socket,
    PACKET_SEND_DELTA,
    utils::time,
    crypto::{Key, MAC_BYTES},
    encryption_manager::Mapping,
    token::{Challenge, Private},
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

pub struct Server<S: Socket, C: Callback> {
    protocol_id: u64,
    private_key: Key,

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
    pub fn new() -> Self {
        unimplemented!()

            //global_sequence: 1 << 63
    }
    pub fn max_clients(&self) -> u32 {
        self.max_clients
    }

    pub fn update(&mut self) {
        self.advance();
        self.receive_packets();
        self.send_packets();
        self.check_for_timeouts();
    }

    pub fn send_packet(&mut self, slot: Slot, payload: &[u8]) {
        assert!(payload.len() <= MAX_PAYLOAD_BYTES);

        let c = match self.clients.get(slot).map(|c| c.is_confirmed()) {
            Some(c) => c,
            None => return,
        };
        if !c {
            self.send_keep_alive(slot);
        }

        let (data, len) = array_from_slice_uninitialized!(payload, MAX_PAYLOAD_BYTES);
        self.send_client_packet(slot, Encrypted::Payload {
            sequence: 0,
            len,
            data,
        });
    }

    fn advance(&mut self) {
        self.time = Instant::now();
        self.encryption_manager.advance();
    }

    fn send_packets(&mut self) {
        for client in self.clients.iter() {
            if client.last_send() + PACKET_SEND_DELTA <= self.time {
                self.send_keep_alive(client.slot());
            }
        }
    }

    fn check_for_timeouts(&mut self) {
        for client in self.clients.iter() {
            if client.last_recv + client.timeout <= self.time {
                self.disconnect_client_internal(client.slot());
            }
        }
    }

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
            } else if let Some(key) = self.encryption_manager.recv_key(from).cloned() {
                self.process_encrypted(from, packet, key);
            }
        }
    }

    fn process_request(&mut self, addr: SocketAddr, packet: &mut [u8], current_timestamp: u64) -> io::Result<()> {
        let request = Request::read(packet, current_timestamp, self.protocol_id, &self.private_key);
        let request = match request {
            Some(r) => r,
            None => return Ok(()),
        };

        let token = Private::read(&request.private_data[..])?;

        let serv = self.socket.addr();
        if !token.server_addresses.iter().any(|a| a == &serv) {
            return Ok(());
        }

        if self.clients.has_addr(addr) || self.clients.has_id(token.client_id) {
            return Ok(());
        }

        /* TODO
        let mac = &request.private_data[Private::BYTES - MAC_BYTES..];
        if !self.connect_token_entries.find_or_add(addr, mac, self.time) {
            return Ok(());
        }
        */

        if self.clients.len() >= self.max_clients as usize {
            self.send_global_packet(addr, &token.server_to_client_key, Encrypted::Denied);
            return Ok(());
        }

        if !self.encryption_manager.insert(
            addr,
            token.server_to_client_key.clone(),
            token.client_to_server_key,
            token.timeout_seconds,
        ) {
            return Ok(());
        }

        let seq = self.challenge_sequence;
        self.challenge_sequence += 1;

        let challenge_data = Challenge::write_encrypted(
            token.client_id,
            &token.user_data,
            seq,
            &self.challenge_key,
        )?;

        self.send_global_packet(addr, &token.server_to_client_key, Encrypted::Challenge {
            challenge_sequence: seq,
            challenge_data,
        });

        Ok(())
    }
    fn process_encrypted(&mut self, addr: SocketAddr, packet: &mut [u8], recv_key: Key) -> io::Result<()> {
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
                    self.disconnect_client_internal(slot);
                }
                _ => unreachable!(),
            }
        } else {
            let (id, slot) = {
                let packet = Encrypted::read(packet, &mut NoProtection, &recv_key, self.protocol_id, Allowed::RESPONSE);
                let challenge = if let Some(Encrypted::Response { mut challenge_data, challenge_sequence }) = packet {
                    Challenge::decrypt(&mut challenge_data, challenge_sequence, &self.challenge_key)?;
                    Challenge::read(&mut challenge_data)
                } else {
                    return Ok(());
                };

                if self.clients.len() >= self.max_clients as usize {
                    self.send_global_packet(addr, &recv_key, Encrypted::Denied);
                    return Ok(());
                }

                if self.clients.has_id(challenge.client_id) {
                    return Ok(());
                }

                let key = self.encryption_manager.find(addr).unwrap();
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

    fn send_global_packet(&mut self, addr: SocketAddr, recv_key: &Key, packet: Encrypted) {
        let seq = self.global_sequence;
        let protocol = self.protocol_id;
        let mut data = [0u8; MAX_PACKET_BYTES];
        let bytes = packet.write(&mut data[..], recv_key, protocol, seq).unwrap();
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

    fn disconnect_client(&mut self, slot: Slot) {
        if send_disconnect_packets {
            for _ in 0..NUM_DISCONNECT_PACKETS {
                self.send_client_packet(slot, Encrypted::Disconnect);
            }
        }
        self.disconnect_client_internal(slot);
    }

    fn disconnect_client_internal(&mut self, slot: Slot) {
        let client = self.clients.remove(slot);
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
