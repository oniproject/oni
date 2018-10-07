#![feature(integer_atomics)]
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU64, Ordering},
};

#[macro_use]
extern crate arrayvec;

use bincode::{serialize, deserialize};
use arrayvec::ArrayVec;

use oni_net::{
    token::{Public, Private, generate_connect_token, TOKEN_DATA},
    crypto::{keygen, Key},
    utils::time,
};

const CLIENT_ID: u64 = 666;
const PROTOCOL: u64 = 4321;
const TIMEOUT: u32 = 5; // in seconds
const EXPIRY: u32 = 45;

/*
#[test]
fn simple() {
    let handler = Server::run();
    let clients: HashSet<Conn> = HashSet::new();

    for _ in 0..10 {
        handler.update(|event| {
            match event {
            Event::Connect(client) => map.insert(client),
            Event::Disconnect(client) => map.remove(client),
            }
        });
    }
}
*/

#[test]
fn common() {
    let (relay, server) = {
        let private_key = keygen();

        let addr = "[::1]:10000".parse().unwrap();

        let relay = Relay::new(private_key, vec![addr].into_iter().collect());
        let server = Server::new(private_key, addr);
        (relay, server)
    };

    // 1. A client authenticates with the web backend
    // 2. The authenticated client requests to play a game via REST call to the web backend
    // 3. The web backend generates a connect token and returns it to that client over HTTPS
    let connect_token = relay.generate_token(CLIENT_ID, PROTOCOL);

    let client = Client::new(connect_token);
    // 4. The client uses the connect token to establish a connection with a dedicated server over UDP
    // 5. The dedicated server runs logic to ensure that only clients with a valid connect token can connect to it
    // 6. Once a connection is established the client and server exchange encrypted and signed UDP packets

    /*
    let saddr = "127.0.0.1:10000".parse().unwrap();
    let caddr = "127.0.0.1:40000".parse().unwrap();

    let client = Client::new(caddr);
        .build();
    let server = Server::new(saddr)
        .build();

    while client.not_connected() {
        client.update();
        server.update();
    }

    client.send(b"fuck");

    client.update(|_| {});
    server.update(|msg| {
        println!("msg: ");
    });
    */
}

type ConnectToken = [u8; Public::BYTES];

struct Server {
    private_key: Key,
    addr: SocketAddr,
}

impl Server {
    fn new(private_key: Key, addr: SocketAddr) -> Self {
        Self {
            private_key,
            addr,
        }
    }
}

struct Client {
    connect_token: Public,
    servers: ArrayVec<[SocketAddr; 32]>,
}
impl Client {
    fn new(mut connect_token: ConnectToken) -> Self {
        let connect_token = Public::read(&mut connect_token[..]).unwrap();
        let servers = deserialize(&connect_token.data[..]).unwrap();
        Self {
            connect_token,
            servers,
        }
    }
}

struct Relay {
    private_key: Key,
    servers: ArrayVec<[SocketAddr; 32]>,
}
impl Relay {
    fn new(private_key: Key, servers: ArrayVec<[SocketAddr; 32]>) -> Self {
        Self {
            private_key,
            servers,
        }
    }
    pub fn generate_token(&self, client_id: u64, protocol_id: u64) -> ConnectToken {
        let data = [0; TOKEN_DATA];
        let servers: Vec<u8> = serialize(&self.servers).unwrap();

        generate_connect_token(
            data,
            TIMEOUT, EXPIRY,
            client_id, protocol_id,
            &self.private_key,
        ).unwrap()
    }
}
