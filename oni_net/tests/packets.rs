use oni_net::{
    packet::{Request, Encrypted, Allowed},
    packet::{MAX_PACKET_BYTES, MAX_PAYLOAD_BYTES},
    protection::NoFilter,
    token,
    utils::{time},
    crypto::{keygen, MAC_BYTES},
    UserData,
    USER_DATA_BYTES,
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
fn connection_request_packet() {
    // generate a connect token
    //let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = random_user_data();
    let input_token = token::Private::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, user_data.clone());
    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(&input_token.user_data[..], &user_data[..]);

    // write the conect token to a buffer (non-encrypted)
    let mut token_data = [0u8; token::Private::BYTES];
    input_token.write(&mut token_data).unwrap();

    // copy to a second buffer then encrypt it in place (we need the unencrypted token for verification later on)
    let mut encrypted_token_data = token_data.clone();

    let token_sequence = 1000u64;
    let token_expire_timestamp = time() + 30;
    let key = keygen();

    token::Private::encrypt(
        &mut encrypted_token_data[..],
        TEST_PROTOCOL,
        token_expire_timestamp,
        token_sequence,
        &key,
    ).unwrap();

    // setup a connection request packet wrapping the encrypted connect token
    let input = Request {
        expire: token_expire_timestamp,
        sequence: token_sequence,
        token: encrypted_token_data,
    };

    // write the connection request packet to a buffer
    let buffer = input.write(TEST_PROTOCOL);

    // read the connection request packet back in from the buffer
    // (the connect token data is decrypted as part of the read packet validation)
    let output = Request::read(
        &buffer[..],
        time(),
        TEST_PROTOCOL,
        &key,
    );

    if let Some(Request { expire, sequence, token }) = output {
        //assert_eq!(sequence, 100);
        // make sure the read packet matches what was written
        assert_eq!(expire, token_expire_timestamp );
        assert_eq!(sequence, token_sequence);
        let len = token::Private::BYTES - MAC_BYTES;
        assert_eq!(&token[..len], &token_data[..len]);
    } else {
        panic!("fail packet");
    }
}

#[test]
fn connection_challenge_packet() {
    // setup a connection challenge packet
    let token = random_token();
    let input = Encrypted::Challenge {
        seq: 0,
        data: token,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = input.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output = Encrypted::read(
        &mut buffer[..written],
        &mut NoFilter,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::CHALLENGE,
    ).unwrap();

    match output {
        Encrypted::Challenge { seq, data } => {
            assert_eq!(seq, 0);
            assert_eq!(&data[..], &token[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_payload_packet() {
    // setup a connection payload packet
    let input_data = random_payload();
    let input = Encrypted::Payload {
        len: MAX_PAYLOAD_BYTES,
        data: input_data,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = input.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();

    assert!(written > 0);

    // read the packet back in from the buffer
    let output = Encrypted::read(
        &mut buffer[..written],
        &mut NoFilter,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::PAYLOAD,
    ).unwrap();

    // make sure the read packet matches what was written
    match output {
        Encrypted::Payload { len, data } => {
            assert_eq!(len, MAX_PAYLOAD_BYTES);
            assert_eq!(&data[..], &input_data[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_disconnect_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = Encrypted::Disconnect.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output = Encrypted::read(
        &mut buffer[..written],
        &mut NoFilter,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::DISCONNECT,
    ).unwrap();

    // make sure the read packet matches what was written
    match output {
        Encrypted::Disconnect => (),
        _ => panic!("wrong packet"),
    }
}
