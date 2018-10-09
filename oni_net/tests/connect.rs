/*
use oni_net::{
    crypto::{keygen, Public, TOKEN_DATA, generate_connect_token},
    client::{Client, State, Event, Error},
};

#[test]
fn client_error_token_expired() {
    const PROTOCOL: u64 = 0x1122334455667788;

    let addr = "[::1]:40000".parse().unwrap();
    let client_id = 666;
    let private_key = keygen();

    let expire = 0;
    let timeout = 0;

    let token = generate_connect_token(
        [4u8; TOKEN_DATA],
        expire, timeout,
        client_id, PROTOCOL, &private_key).unwrap();

    let token = Public::read(&token[..]).unwrap();

    let mut client = Client::new(PROTOCOL, token, addr).unwrap();

    client.update(|e| match e {
        Event::Packet(data) => println!("receive: {:?}", data),
        Event::Disconnected(err) => println!("disconnected: {:?}", err),
        Event::Connected => println!("connected"),
    });

    assert_eq!(client.state(), State::Disconnected(Error::TokenExpired));
}
*/
