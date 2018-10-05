use oni_net::{
    token::{Challenge, Private, Public},
    crypto::keygen,
    utils::time,
    UserData,
    USER_DATA_BYTES,
    VERSION,
};

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;
const TEST_SEQ: u64 = 1000;

fn random_user_data() -> UserData {
    [0u8; USER_DATA_BYTES]
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
        let key = keygen();

        Challenge::encrypt(&mut buffer, TEST_SEQ, &key).unwrap();
        // send...
        Challenge::decrypt(&mut buffer, TEST_SEQ, &key).unwrap();
    }

    // read the challenge token back in
    let output_token = Challenge::read(&buffer);
    // make sure that everything matches the original challenge token
    assert_eq!(output_token.client_id, TEST_CLIENT_ID);
    assert_eq!(&output_token.user_data[..], &user_data[..]);
}

#[test]
fn connect_token_private() {
    // generate a connect token
    //let server_address = "127.0.0.1:40000".parse().unwrap();

    let user_data = random_user_data();

    let input_token = Private::generate(
        TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, user_data.clone());

    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(&input_token.user_data[..], &user_data[..]);

    // write it to a buffer

    let mut buffer = [0u8; Private::BYTES];
    input_token.write(&mut buffer[..]).unwrap();

    // encrypt/decrypt the buffer

    let expire_timestamp: u64 = 30 + time();
    let key = keygen();

    Private::encrypt(
        &mut buffer[..],
        TEST_PROTOCOL,
        expire_timestamp,
        TEST_SEQ,
        &key).unwrap();

    Private::decrypt(
        &mut buffer[..],
        TEST_PROTOCOL,
        expire_timestamp,
        TEST_SEQ,
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
    assert_eq!(&output_token.user_data[..], &input_token.user_data[..]);
}

#[test]
fn connect_token_public() {
    // generate a private connect token
    // let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = random_user_data();

    // write it to a buffer
    let private = Private::generate(
        TEST_CLIENT_ID,
        TEST_TIMEOUT_SECONDS,
        user_data.clone(),
    );

    let mut token = [0u8; Private::BYTES];
    private.write(&mut token[..]).unwrap();

    // encrypt the buffer
    let create_timestamp = time();
    let expire_timestamp = create_timestamp + 30;
    let key = keygen();
    Private::encrypt(
        &mut token[..],
        TEST_PROTOCOL,
        expire_timestamp,
        TEST_SEQ,
        &key,
    ).unwrap();

    // wrap a public connect token around the private connect token data
    let input = Public {
        version: VERSION,
        protocol_id: TEST_PROTOCOL,
        create_timestamp,
        expire_timestamp,
        sequence: TEST_SEQ,
        client_to_server_key: private.client_to_server_key,
        server_to_client_key: private.server_to_client_key,
        timeout_seconds: TEST_TIMEOUT_SECONDS,

        token,
        user_data: random_user_data(),
    };

    // write the connect token to a buffer
    let mut buffer = [0u8; Public::BYTES];
    input.write(&mut buffer[..]).unwrap();

    // read the buffer back in
    let output = Public::read(&mut buffer).unwrap();

    // make sure the public connect token matches what was written
    assert_eq!(output.version, input.version);
    assert_eq!(output.protocol_id, input.protocol_id);
    assert_eq!(output.create_timestamp, input.create_timestamp);
    assert_eq!(output.expire_timestamp, input.expire_timestamp);
    assert_eq!(output.sequence, input.sequence);
    assert_eq!(&output.token[..], &input.token[..]);
    assert_eq!(output.client_to_server_key, input.client_to_server_key);
    assert_eq!(output.server_to_client_key, input.server_to_client_key);
    assert_eq!(output.timeout_seconds, input.timeout_seconds);
}
