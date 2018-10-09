/*
use std::net::SocketAddr;

use oni_net::{
    token::{Public, Private, generate_connect_token, TOKEN_DATA},
    crypto::{keygen, Key},
    utils::time_secs,
    server_list::ServerList,
};

const CLIENT_ID: u64 = 666;
const PROTOCOL: u64 = 4321;
const TIMEOUT: u32 = 5; // in seconds
const EXPIRY: u32 = 45;

#[test]
fn common() {
    let (relay, server) = {
        let private_key = keygen();

        let addr = "[::1]:10000".parse().unwrap();

        let mut servers = ServerList::new();
        servers.push(addr).unwrap();
        let relay = Relay::new(private_key, servers);
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

type ConnectTokenData = [u8; Public::BYTES];

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
    servers: ServerList,
}
impl Client {
    fn new(mut connect_token: ConnectTokenData) -> Self {
        let connect_token = Public::read(&mut connect_token[..]).unwrap();
        let servers = ServerList::deserialize(&connect_token.data).unwrap();
        Self {
            connect_token,
            servers,
        }
    }
}

struct Relay {
    private_key: Key,
    servers: ServerList,
}

impl Relay {
    fn new(private_key: Key, servers: ServerList) -> Self {
        Self {
            private_key,
            servers,
        }
    }
    pub fn generate_token(&self, client_id: u64, protocol_id: u64) -> ConnectTokenData {
        generate_connect_token(
            self.servers.serialize().unwrap(),
            TIMEOUT, EXPIRY,
            client_id, protocol_id,
            &self.private_key,
        ).unwrap()
    }
}
*/
