use oni_net::{
    token::{Challenge, Private, Public},
    crypto::{Key, keygen},
    utils::{UserData, USER_DATA_BYTES, time},

    VERSION,
};

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;

fn random_user_data() -> UserData {
    UserData::default()
        /* FIXME
    let mut user_data = [0u8; USER_DATA_BYTES];
    random_bytes(&mut user_data[..]);
    user_data.into()
    */
}

#[test]
fn challenge_token() {
    // generate a challenge token
    let user_data = random_user_data();

    // write it to a buffer
    let mut buffer = Challenge::write(TEST_CLIENT_ID, &user_data);

    {
        // encrypt/decrypt the buffer
        let seq = 1000u64;
        let key = keygen();

        Challenge::encrypt(&mut buffer, seq, &key).unwrap();
        // send...
        Challenge::decrypt(&mut buffer, seq, &key).unwrap();
    }

    // read the challenge token back in
    let output_token = Challenge::read(&buffer);
    // make sure that everything matches the original challenge token
    assert_eq!(output_token.client_id, TEST_CLIENT_ID);
    assert_eq!(output_token.user_data, user_data);
}

#[test]
fn connect_token_private() {
    // generate a connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();

    let user_data = random_user_data();

    let input_token = Private::generate(
        TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS,
        vec![server_address], user_data.clone());

    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(input_token.server_addresses, &[server_address]);
    assert_eq!(input_token.user_data, user_data);

    // write it to a buffer

    let mut buffer = [0u8; Private::BYTES];
    input_token.write(&mut buffer[..]).unwrap();

    // encrypt/decrypt the buffer

    let sequence = 1000u64;
    let expire_timestamp: u64 = 30 + time();
    let key = keygen();

    Private::encrypt(
        &mut buffer[..],
        TEST_PROTOCOL,
        expire_timestamp,
        sequence,
        &key).unwrap();

    Private::decrypt(
        &mut buffer[..],
        TEST_PROTOCOL,
        expire_timestamp,
        sequence,
        &key).unwrap();

    // read the connect token back in

    let output_token = Private::read(&mut buffer[..]).unwrap();

    // make sure that everything matches the original connect token

    assert_eq!(output_token.client_id, input_token.client_id);
    assert_eq!(output_token.timeout_seconds, input_token.timeout_seconds);
    assert_eq!(output_token.client_to_server_key,
               input_token.client_to_server_key);
    assert_eq!(output_token.server_to_client_key,
               input_token.server_to_client_key);
    assert_eq!(output_token.user_data, input_token.user_data);
    assert_eq!(&output_token.server_addresses[..],
               &input_token.server_addresses[..]);
}

#[test]
fn connect_token_public() {
    // generate a private connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = random_user_data();
    let connect_token_private = Private::generate(
        TEST_CLIENT_ID,
        TEST_TIMEOUT_SECONDS,
        vec![server_address],
        user_data.clone(),
    );

    assert_eq!(connect_token_private.client_id, TEST_CLIENT_ID);
    assert_eq!(connect_token_private.server_addresses, &[server_address]);
    assert_eq!(connect_token_private.user_data, user_data);

    // write it to a buffer
    let mut connect_token_private_data = [0u8; Private::BYTES];
    connect_token_private.write(&mut connect_token_private_data[..]).unwrap();

    // encrypt the buffer
    let sequence = 1000;
    let create_timestamp = time();
    let expire_timestamp = create_timestamp + 30;
    let key = keygen();
    Private::encrypt(
        &mut connect_token_private_data[..],
        TEST_PROTOCOL,
        expire_timestamp,
        sequence,
        &key,
    ).unwrap();

    // wrap a public connect token around the private connect token data
    let input_connect_token = Public {
        version: VERSION,
        protocol_id: TEST_PROTOCOL,
        create_timestamp,
        expire_timestamp,
        sequence,
        private_data: connect_token_private_data,
        server_addresses: vec![server_address],
        client_to_server_key: connect_token_private.client_to_server_key,
        server_to_client_key: connect_token_private.server_to_client_key,
        timeout_seconds: TEST_TIMEOUT_SECONDS,
    };

    // write the connect token to a buffer
    let mut buffer = [0u8; Public::BYTES];
    input_connect_token.write(&mut buffer[..]).unwrap();

    // read the buffer back in
    let output_connect_token = Public::read(&mut buffer).unwrap();

    // make sure the public connect token matches what was written
    assert_eq!(output_connect_token.version, input_connect_token.version);
    assert_eq!(output_connect_token.protocol_id,
               input_connect_token.protocol_id);
    assert_eq!(output_connect_token.create_timestamp,
               input_connect_token.create_timestamp);
    assert_eq!(output_connect_token.expire_timestamp,
               input_connect_token.expire_timestamp);
    assert_eq!(output_connect_token.sequence, input_connect_token.sequence);
    assert_eq!(&output_connect_token.private_data[..],
               &input_connect_token.private_data[..]);
    assert_eq!(&output_connect_token.server_addresses[..],
               &input_connect_token.server_addresses[..]);
    assert_eq!(output_connect_token.client_to_server_key,
               input_connect_token.client_to_server_key);
    assert_eq!(output_connect_token.server_to_client_key,
               input_connect_token.server_to_client_key);
    assert_eq!(output_connect_token.timeout_seconds,
               input_connect_token.timeout_seconds);
}
