#![feature(integer_atomics)]
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU64, Ordering},
};

use oni_net::{
    token::{Public, Private},
    crypto::{keygen, Key, TOKEN_DATA},
    utils::time,
};

const CLIENT_ID: u64 = 666;
const TIMEOUT_SECS: u32 = 5; // in seconds
const PROTOCOL: u64 = 4321;
const EXPIRY: u64 = 45;

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
    let private_key = keygen();
    let relay = Relay::new(private_key);
    let server = Server::new(private_key);

    return;

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

struct Relay {
    private_key: Key,
    match_nonce: AtomicU64,
}
struct Server {
    private_key: Key,
}
struct Client {
    connect_token: Public,
}

impl Relay {
    fn new(private_key: Key) -> Self {
        Self {
            private_key,
            match_nonce: AtomicU64::new(0),
        }
    }
    pub fn generate_token(&self, client_id: u64, protocol_id: u64) -> ConnectToken {
    /*
        let create = time();
        let expire = create + EXPIRY;
        let match_nonce = self.match_nonce.fetch_add(1, Ordering::Relaxed);

        let mut msg = [0u8; 256];
        (&mut msg[..8]).copy_from_slice(b"fuck you");
        let private = Private::generate(client_id, TIMEOUT_SECS, msg);

        let public = Public {
            version: oni_net::VERSION,
            protocol_id,
            create,
            expire,
            sequence: match_nonce,
            client_key: private.client_key,
            server_key: private.server_key,
            timeout: TIMEOUT_SECS,
            token: private.write_encrypted(
                PROTOCOL,
                expire,
                match_nonce,
                &self.private_key,
            ),
            data: [0u8; TOKEN_DATA],
        };

        //Public::generate();
        */
        unimplemented!()
    }

    /*
    pub fn match_handler(&self, client_id: u64, protocol_id: u64) {
        let match_nonce = self.match_nonce.fetch_add(1, Ordering::Relaxed);

        /*
        vars := mux.Vars( r )
        atomic.AddUint64( &MatchNonce, 1 )
        clientId, _ := strconv.ParseUint( vars["clientId"], 10, 64 )
        protocolId, _ := strconv.ParseUint( vars["protocolId"], 10, 64 )
        serverAddresses := make( []net.UDPAddr, 1 )
        serverAddresses[0] = net.UDPAddr{ IP: net.ParseIP( ServerAddress ), Port: ServerPort }
        userData := make( []byte, UserDataBytes )
        connectToken := GenerateConnectToken( clientId, serverAddresses, protocolId, ConnectTokenExpiry, TimeoutSeconds, MatchNonce, userData, PrivateKey )
        if connectToken == nil {
            log.Printf( "error: failed to generate connect token" )
            return
        }
        connectTokenBase64 := base64.StdEncoding.EncodeToString( connectToken )
        w.Header().Set( "Content-Type", "application/text" )
        if _, err := io.WriteString( w, connectTokenBase64 ); err != nil {
            log.Printf( "error: failed to write string response" )
            return
        }
        fmt.Printf( "matched client %.16x to %s:%d\n", clientId, ServerAddress, ServerPort )
        */
    }

    fn generate_connect_token(&mut self,
        client_id: u64,
        server_addr: SocketAddr,
        protocol_id: u64,
        expiry: u64,
        timeout: u64,
        match_nonce: u64,
        user_data: &[u8],
        //private_key: &[u8],
    ) {
    }
    */
}

impl Server {
    fn new(private_key: Key) -> Self {
        Self {
            private_key,
        }
    }
}

impl Client {
    fn new(mut connect_token: ConnectToken) -> Self {
        let connect_token = Public::read(&mut connect_token[..]).unwrap();
        Self {
            connect_token,
        }
    }
}


    /*
    pub fn _gen_def(
        client_id: u64, protocol_id: u64, match_nonce: u64, user_data) {
        const ConnectTokenExpiry = 45
        const TimeoutSeconds = 5


        Self::gen(
            client_id,
            /*serverAddresses,*/
            shared_data: &[u8],
            protocolId,
            ConnectTokenExpiry = 45,
            TimeoutSeconds = 5,
            MatchNonce,
            private_data: &[u8],
            PrivateKey
        )
    }

    pub fn gen(
        client_id: u64,
        shared_data: &[u8],
        protocol_id u64,
        expire: u64,
        timeout: i32,
        sequence: u64,
        user_data: &[u8],
        private_key: &Key,
    )
        -> [u8; Self::BYTES]
    {
        unimplemtented!()
    }
*/

/*
 *

pub use crate::sock::{Socket, Udp};

pub fn new_server(addr: std::net::SocketAddr) {
    use crate::crypto::keygen;

    let protocol = 1234;

    let pkey = keygen();
    let ckey = keygen();

    let server = crate::server::Server::new(
        1234,
        pkey,
        ckey,
        callback,
        Udp::new(addr).unwrap(),
    );
}
*/

