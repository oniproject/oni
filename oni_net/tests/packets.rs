use oni_net::{
    packet::{Request, Encrypted, Allowed},
    packet::{MAX_PACKET_BYTES, MAX_PAYLOAD_BYTES},
    protection::{NoFilter, ReplayProtection, Protection},
    protection::{REPLAY_PROTECTION_BUFFER_SIZE},
    token,
    utils::{time},
    crypto::{keygen, TOKEN_DATA},
};

const CLIENT_ID: u64 = 0x1;
const TIMEOUT_SECONDS: u32 = 15;
const PROTOCOL: u64 = 0x1122334455667788;
const SEQ: u64 = 1000;

fn random_user_data() -> [u8; TOKEN_DATA] {
    [4u8; TOKEN_DATA]
    /* FIXME
    let mut user_data = [0u8; USER_DATA];
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
    // generate private key
    let private_key = keygen();

    // generate a connect token
    let connect_token = token::Public::generate(
        [8u8; TOKEN_DATA],
        5, 45, CLIENT_ID,
        PROTOCOL, &private_key,
    );

    // setup a connection request packet wrapping the encrypted connect token
    // write the connection request packet to a buffer
    let input = Request::write_token(&connect_token);

    // send over network

    // read the connection request packet back in from the buffer
    // (the connect token data is decrypted as part of the read packet validation)
    let output = Request::read(&input[..], time(), PROTOCOL, &private_key).unwrap();

    let Request { expire, token } = output;
    // make sure the read packet matches what was written
    assert_eq!(expire, connect_token.expire);

    let private = token::Private::read(&token).unwrap();
    assert_eq!(&private.data[..], &connect_token.data[..]);
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

    let written = input.write(&mut buffer[..], &packet_key, PROTOCOL, SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output = Encrypted::read(
        &mut buffer[..written],
        &mut NoFilter,
        &packet_key,
        PROTOCOL,
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

    let written = input.write(&mut buffer[..], &packet_key, PROTOCOL, SEQ).unwrap();

    assert!(written > 0);

    // read the packet back in from the buffer
    let output = Encrypted::read(
        &mut buffer[..written],
        &mut NoFilter,
        &packet_key,
        PROTOCOL,
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

    let written = Encrypted::Disconnect.write(&mut buffer[..], &packet_key, PROTOCOL, SEQ).unwrap();
    assert!(written > 0);

    // read the packet back in from the buffer
    let output = Encrypted::read(
        &mut buffer[..written],
        &mut NoFilter,
        &packet_key,
        PROTOCOL,
        Allowed::DISCONNECT,
    ).unwrap();

    // make sure the read packet matches what was written
    match output {
        Encrypted::Disconnect => (),
        _ => panic!("wrong packet"),
    }
}


#[test]
fn replay_protection() {
    let mut replay_protection = ReplayProtection::default();

    for _ in 0..2 {
        replay_protection.reset();

        assert_eq!(replay_protection.most_recent_sequence(), 0);

        const MAX_SEQUENCE: u64 = 4 * REPLAY_PROTECTION_BUFFER_SIZE as u64;

        // the first time we receive packets, they should not be already received
        for sequence in 0..MAX_SEQUENCE {
            assert!(!replay_protection.packet_already_received(sequence));
        }

        // old packets outside buffer should be considered already received
        assert!(replay_protection.packet_already_received(0));

        // packets received a second time should be flagged already received
        for sequence in MAX_SEQUENCE - 10..MAX_SEQUENCE {
            assert!(replay_protection.packet_already_received(sequence));
        }

        // jumping ahead to a much higher sequence should be considered not already received
        assert!(!replay_protection.packet_already_received(MAX_SEQUENCE + REPLAY_PROTECTION_BUFFER_SIZE as u64));

        // old packets should be considered already received
        for sequence in 0..MAX_SEQUENCE {
            assert!(replay_protection.packet_already_received(sequence));
        }
    }
}
