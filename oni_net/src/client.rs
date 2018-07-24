enum ConnectionError {
    ConnectTokenExpired,
    InvalidConnectToken,
    ConnectionTimedOut,
    ConnectionResponseTimedOut,
    ConnectionRequestTimedOut,
    ConnectionDenied,
}

enum ClientState {
    ConnectTokenExpired = -6,
    InvalidConnectToken = -5,
    ConnectionTimedOut  = -4,
    ConnectionResponseTimedOut = -3,
    ConnectionRequestTimedOut = -2,
    ConnectionDenied = -1,

    Disconnected = 0,
    SendingConnectionRequest = 1,
    SendingConnectionResponse = 2,
    Connected = 3,
}

impl Default for ClientState {
    fn default() -> Self {
        ClientState::Disconnected
    }
}

#[test]
fn connect() {
    let server_addr = "127.0.0.0:5000".parse().unwrap();
    let matcher = Matcher {
        ips: vec![server_addr],
    };

    let client = Matcher::new();

    // A client authenticates with the web backend
    // The authenticated client requests to play a game via REST call to the web backend

    // The web backend generates a connect token and returns it to that client over HTTPS
    let token = matcher.generate_token();

    // The client uses the connect token to establish a connection with a dedicated server over UDP
    client.connecto_to(token.ips[0], token);

    // The dedicated server runs logic to ensure that only clients with a valid connect token can connect to it

    // Once a connection is established the client and server exchange encrypted and signed UDP packets
}

pub struct ChallengeToken {
    client_id: u64,
    user_data: [u8; 256],
    _zero_pad_to_300_bytes: [u8; 36],
    //<zero pad to 300 bytes>
}
