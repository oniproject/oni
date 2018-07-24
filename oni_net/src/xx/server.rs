struct Server {
    protocol_id: u64,
    current_timestamp: u64,
    private_key: &[u8],
    used_tokens: Vec<tok>,
}

impl Server {
    fn processing_connection_request_packet(&mut self, packet: Request) {
        // The server takes the following steps, in this exact order, when processing a connection request packet:

        // If the packet is not the expected size of 1062 bytes,
        if !packet.check_size() {
            return; // ignore the packet.
        }

        // If the version info in the packet doesn't match "NETCODE 1.01" (13 bytes, with null terminator),
        if packet.version() != VERSION {
            return; // ignore the packet.
        }

        // If the protocol id in the packet doesn't match the expected protocol id of the dedicated server,
        if packet.protocol_id() != self.protocol_id {
            return; // ignore the packet.
        }

        // If the connect token expire timestamp is <= the current timestamp,
        if packet.expire_timestamp() <= self.current_timestamp {
            return; // ignore the packet.
        }

        // If the encrypted private connect token data doesn't decrypt with the private key,
        // using the associated data constructed from: version info, protocol id and expire timestamp,
        let data = match self.decrypt(self.private_key, packet.encrypted_private_connect_token_data(), packet.associated()) {
            Err(_) => return, // ignore the packet.
            Ok(data) => data,
        };

        // If the decrypted private connect token fails to be read for any reason,
        // for example, having a number of server addresses outside of the expected range of [1,32],
        // or having an address type value outside of range [0,1],
        let token = match self.parse_c(data) {
            Err(_) => return, // ignore the packet.
            Ok(token) => token,
        };

        // TODO If the dedicated server public address is not in the list of server addresses in the private connect token,
        return; // ignore the packet.

        // TODO If a client from the packet IP source address and port is already connected,
        return; // ignore the packet.

        // TODO If a client with the client id contained in the private connect token data is already connected,
        return; // ignore the packet.

        // TODO If the connect token has already been used by a different packet source IP address and port,
        return; // ignore the packet.

        // TODO Otherwise, add the private connect token hmac + packet source IP address and port to the history of connect tokens already used.

        // TODO If no client slots are available, then the server is full.
        // Respond with a connection denied packet.

        // TODO Add an encryption mapping for the packet source IP address and port
        // so that packets read from that address and port are decrypted with the client to server key in the private connect token,
        // and packets sent to that address and port are encrypted with the server to client key in the private connect token.
        // This encryption mapping expires in timeout seconds of no packets being sent to or received from that address and port,
        // or if a client fails to establish a connection with the server within timeout seconds.

        // TODO If for some reason this encryption mapping cannot be added,
        return; // ignore the packet.

        // TODO Otherwise, respond with a connection challenge packet and increment the connection challenge sequence number.
    }

    fn respond_denied_packet() {
    }
}
