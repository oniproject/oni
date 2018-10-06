/*
use std::thread::sleep;
use std::time::{Instant, Duration};

use oni_net::{
    packet::{Request, Encrypted, Allowed},
    packet::{MAX_PACKET_BYTES, MAX_PAYLOAD_BYTES},
    protection::NoFilter,
    token,
    utils::{time},
    crypto::{keygen, Key},
    UserData,
    USER_DATA_BYTES,

    client::{self, Client},
    server::{self, Server, Event, Slot},
};


const TEST_PROTOCOL_ID: u64 = 0x1122334455667788;

struct Scallback;

impl server::Callback for Scallback  {
    fn connect(&mut self, slot: Slot) {
        println!("connect[{:?}]", slot);
    }
    fn disconnect(&mut self, slot: Slot) {
        println!("disconnect[{:?}]", slot);
    }
    fn receive(&mut self, slot: Slot, payload: &[u8]) {
        println!("receive[{:?}]: {:?}", slot, payload);
    }
}

/*
fn server(addr: SocketAddr) {
    println!("[server]");

    let mut quit = false;

    let private_key = keygen();
    let time = 0.0;
    let delta_time = ;

    let server = Server::new(TEST_PROTOCOL_ID, private_key: Key, socket);
    //pub fn new(protocol_id: u64, pkey: Key, callback: C, socket: S) -> Self {


    while !quit {
        server.update();

        if server.client_connected(0) {
            server.send(0, packet_data, NETCODE_MAX_PACKET_SIZE);
        }

        for client in server.clients() {
            while let Some(packet) = client.recv() {
                println!("recv packet from {}: {:?}", client.addr(), packet);
            }
        }

        sleep(delta_time);
    }

    println!();
    println!("shutting down");
}
*/

fn generate_connect_token(
    public_data: [u8; USER_DATA_BYTES],
    internal_data: [u8; USER_DATA_BYTES],
    expire: u32,
    timeout: u32,
    client_id: u64,
    protocol_id: u64,
    private_key: &Key,
) -> Result<(), ()>
{
    unimplemented!()
}

#[test]
fn client_server() {
    const CONNECT_TOKEN_EXPIRY: u32 = 30;
    const CONNECT_TOKEN_TIMEOUT: u32 = 5;
    const PROTOCOL_ID: u64 =  0x1122334455667788;
    const DELTA_TIME: Duration = Duration::from_millis(1000 / 60);

    let private_key = keygen();

    println!("[client/server]");

    let client_id = 1345643;
    let connect_token = generate_connect_token(
        public,
        internal,
        CONNECT_TOKEN_EXPIRY,
        CONNECT_TOKEN_TIMEOUT,
        client_id,
        PROTOCOL_ID,
        &private_key,
    ).unwrap();

    let client = Client::connect(PROTOCOL_ID, connect_token, "::".parse().unwrap());
    let server = Server::new    (PROTOCOL_ID, private_key, "[::1]:40000".parse().unwrap());

    println!("client id is {}", client_id);

    let mut server_num_packets_received = 0;
    let mut client_num_packets_received = 0;

    let mut ref_packet = [0u8; MAX_PAYLOAD_BYTES];
    for (i, v) in ref_packet.iter_mut().enumerate() {
        *v = (i & 0xFF) as u8;
    }

    let ref_packet = &ref_packet[..];

    let mut buf = [0u8; MAX_PACKET_BYTES];
    loop {
        client.update();
        server.update();

        if client.state(client) == client::State::Connected {
            client.send(ref_packet);
        }

        let slot = server.clients().take(0);

        if server.client_connected(slot) {
            server.send(client, ref_packet);
        }

        while let Some(len) = client.recv(&mut buf) {
            let payload = &buf[..len];
            assert_eq!(payload, ref_packet, "client packet");
            client_num_packets_received += 1;
        }

        for client in server.clients() {
            while let Some(len) = server.recv(&mut buf, client) {
                let payload = &buf[..len];
                assert_eq!(payload, ref_packet, "server packet");
                client_num_packets_received += 1;
            }
        }

        if client_num_packets_received >= 10 && server_num_packets_received >= 10 {
            if server.client_connected(slot) {
                println!("client and server successfully exchanged packets");
                server.disconnect(slot);
            }
        }

        if client.state().is_err()  {
            println!("client error state: {:?}", client.state());
            break;
        }

        sleep(DELTA_TIME);
    }

    println!("shutting down");
}
*/
