use std::net::SocketAddr;
use oni_net::{
    Socket,
    crypto::{keygen, Public, TOKEN_DATA, generate_connect_token},
    client::{Client, State, Event, Error},
};
const TEST_PROTOCOL: u64 = 0x1122334455667788;

fn random_user_data() -> [u8; TOKEN_DATA] {
    [4u8; TOKEN_DATA]
    /* FIXME
    let mut user_data = [0u8; USER_DATA];
    random_bytes(&mut user_data[..]);
    user_data.into()
    */
}

struct NoSocket;

impl Socket for NoSocket {
    fn addr(&self) -> SocketAddr { "0.0.0.0:0".parse().unwrap() }
    fn send(&self, _addr: SocketAddr, _packet: &[u8]) {}
    fn recv(&self, _packet: &mut [u8]) -> Option<(usize, SocketAddr)> { None }
}

#[test]
fn client_error_token_expired() {
    let addr = "[::1]:40000".parse().unwrap();
    let client_id = 666;
    let private_key = keygen();

    let expire = 0;
    let timeout = 0;

    let token = generate_connect_token(
        random_user_data(), random_user_data(),
        expire, timeout,
        client_id, TEST_PROTOCOL, &private_key).unwrap();

    let token = Public::read(&token[..]).unwrap();

    let mut client = Client::connect(NoSocket, addr, token);

    client.update(|e| match e {
        Event::Packet(data) => println!("receive: {:?}", data),
        Event::Disconnected(err) => println!("disconnected: {:?}", err),
        Event::Connected => println!("connected"),
    });

    assert_eq!(client.state(), State::Disconnected(Error::TokenExpired));
}
