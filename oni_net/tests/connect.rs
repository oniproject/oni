use std::net::SocketAddr;
use oni_net::{
    packet::MAX_PAYLOAD_BYTES,
    token,
    crypto::{keygen, MAC_BYTES, Public},
    UserData,
    USER_DATA_BYTES,

    Socket,
    client::{Client, State, Callback, Error},
};

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;
const TEST_SEQ: u64 = 1000;

fn random_user_data() -> UserData {
    [4u8; USER_DATA_BYTES]
    /* FIXME
    let mut user_data = [0u8; USER_DATA_BYTES];
    random_bytes(&mut user_data[..]);
    user_data.into()
    */
}

fn random_token() -> [u8; token::Challenge::BYTES] {
    [4u8; token::Challenge::BYTES]
    // FIXME: random_bytes(&mut x_data[..]);
}

fn random_payload() -> [u8; MAX_PAYLOAD_BYTES] {
    [4u8; MAX_PAYLOAD_BYTES]

    /*
        let mut input_data = [0u8; MAX_PAYLOAD_BYTES];
        random_bytes(&mut input_data[..]);
    */
}


#[test]
fn client_error_token_expired() {
    struct NoSocket;

    impl Socket for NoSocket {
        fn addr(&self) -> SocketAddr { "0.0.0.0:0".parse().unwrap() }
        fn send(&self, _addr: SocketAddr, _packet: &[u8]) {}
        fn recv(&self, _packet: &mut [u8]) -> Option<(usize, SocketAddr)> { None }
    }

    struct Cb;
    impl Callback for Cb {
        fn state_change(&mut self, old: State, new: State) {
            println!("state: {:?} -> {:?}", old, new);
        }
        fn receive(&mut self, data: &[u8]) {
            println!("receive: {:?}", data);
        }
    }

    let addr = "[::1]:40000".parse().unwrap();
    let client_id = 666;
    let private_key = keygen();
    let public_data = random_user_data();
    let private_data = random_user_data();
    let token = Public::new(
        //vec![addr], vec![addr],
        0, TEST_TIMEOUT_SECONDS, client_id, TEST_PROTOCOL,
        0, public_data, &private_key, private_data,
    ).unwrap();

    let mut client = Client::connect(NoSocket, Cb, addr, token);

    client.update();

    assert_eq!(client.state(), State::Disconnected {
        err: Error::TokenExpired,
    });
}
