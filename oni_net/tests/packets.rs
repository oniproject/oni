use oni_net::{
    packet::{Request, Encrypted, NoProtection, Allowed},
    packet::{MAX_PACKET_BYTES, MAX_PAYLOAD_BYTES, MAX_CHANNEL_ID},
    token,
    utils::{UserData, time},
    crypto::{Key, keygen, MAC_BYTES},
    VERSION,
};

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;
const TEST_SEQ: u64 = 1000;

fn random_user_data() -> UserData {
    UserData::default()
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
    let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = random_user_data();
    let input_token = token::Private::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, vec![server_address], user_data.clone());
    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(input_token.server_addresses, &[server_address]);
    assert_eq!(input_token.user_data, user_data);

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
    let input_packet = Request {
        version: VERSION,
        protocol_id: TEST_PROTOCOL,
        expire_timestamp: token_expire_timestamp,
        sequence: token_sequence,
        private_data: encrypted_token_data,
    };

    // write the connection request packet to a buffer
    let buffer = input_packet.write();

    // read the connection request packet back in from the buffer
    // (the connect token data is decrypted as part of the read packet validation)
    let output_packet = Request::read(
        &buffer[..],
        time(),
        TEST_PROTOCOL,
        &key,
    );

    if let Some(Request { version, protocol_id, expire_timestamp, sequence, private_data  }) = output_packet {
        //assert_eq!(sequence, 100);
        // make sure the read packet matches what was written
        assert_eq!(version, VERSION);
        assert_eq!(protocol_id, TEST_PROTOCOL);
        assert_eq!(expire_timestamp, token_expire_timestamp );
        assert_eq!(sequence, token_sequence);
        let len = token::Private::BYTES - MAC_BYTES;
        assert_eq!(&private_data[..len], &token_data[..len]);
    } else {
        panic!("fail packet");
    }
}

#[test]
fn connection_denied_packet() {
    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = Encrypted::Denied.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::DENIED,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Encrypted::Denied => (),
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_challenge_packet() {
    // setup a connection challenge packet
    let token = random_token();
    let input_packet = Encrypted::Challenge {
        challenge_sequence: 0,
        challenge_data: token,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::CHALLENGE,
    ).unwrap();

    match output_packet {
        Encrypted::Challenge { challenge_sequence, challenge_data } => {
            assert_eq!(challenge_sequence, 0);
            assert_eq!(&challenge_data[..], &token[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_response_packet() {
    // setup a connection challenge packet
    let token = random_token();
    let input_packet = Encrypted::Response {
        challenge_sequence: 0,
        challenge_data: token,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::RESPONSE,
    ).unwrap();

    match output_packet {
        Encrypted::Response { challenge_sequence, challenge_data } => {
            assert_eq!(challenge_sequence, 0);
            assert_eq!(&challenge_data[..], &token[..]);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_keep_alive_packet() {
    // setup a connection challenge packet
    let input_packet = Encrypted::KeepAlive {
        client_index: 10,
        max_clients: 16,
    };

    // write the packet to a buffer
    let mut buffer = [0u8; MAX_PACKET_BYTES];
    let packet_key = keygen();

    let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::KEEP_ALIVE,
    ).unwrap();

    match output_packet {
        Encrypted::KeepAlive { client_index, max_clients } => {
            assert_eq!(client_index, 10);
            assert_eq!(max_clients, 16);
        }
        _ => panic!("wrong packet"),
    }
}

#[test]
fn connection_payload_packet() {
    for chan in 0..=MAX_CHANNEL_ID {
        // setup a connection payload packet
        let input_data = random_payload();

        let input_packet = Encrypted::Payload {
            sequence: TEST_SEQ,
            len: MAX_PAYLOAD_BYTES,
            data: input_data,
            channel: chan,
        };

        // write the packet to a buffer
        let mut buffer = [0u8; MAX_PACKET_BYTES];
        let packet_key = keygen();

        let written = input_packet.write(&mut buffer[..], &packet_key, TEST_PROTOCOL, TEST_SEQ).unwrap();

        assert!(written > 0);

        // read the packet back in from the buffer
        let output_packet = Encrypted::read(
            &mut buffer[..written],
            &mut NoProtection,
            &packet_key,
            TEST_PROTOCOL,
            Allowed::PAYLOAD,
        ).unwrap();

        // make sure the read packet matches what was written
        match output_packet {
            Encrypted::Payload { sequence, len, data, channel } => {
                assert_eq!(channel, chan);
                assert_eq!(sequence, TEST_SEQ);
                assert_eq!(len, MAX_PAYLOAD_BYTES);
                assert_eq!(&data[..], &input_data[..]);
            }
            _ => panic!("wrong packet"),
        }
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
    let output_packet = Encrypted::read(
        &mut buffer[..written],
        &mut NoProtection,
        &packet_key,
        TEST_PROTOCOL,
        Allowed::DISCONNECT,
    ).unwrap();

    // make sure the read packet matches what was written
    match output_packet {
        Encrypted::Disconnect => (),
        _ => panic!("wrong packet"),
    }
}
