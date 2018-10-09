/*
use oni_net::{
    token::{
        Challenge, Private, Public,
        TOKEN_DATA,
        keygen,
    },
    utils::time_secs,
    VERSION,
};

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;
const TEST_SEQ: u64 = 1000;

fn random_user_data() -> [u8; TOKEN_DATA] {
    [0u8; TOKEN_DATA]
        /* FIXME
    let mut user_data = [0u8; USER_DATA];
    random_bytes(&mut user_data[..]);
    user_data.into()
    */
}

#[test]
fn challenge_token() {
    // generate a challenge token
    let data = [0; 256];
    let id = 1234;
    let seq = TEST_SEQ;
    let key = keygen();

    // seal/open the buffer

    // write it to a buffer
    let input = Challenge { id, data }.write(seq, &key).unwrap();

    // send...

    // read the challenge token back in
    let output = Challenge::read(input, seq, &key).unwrap();

    // make sure that everything matches the original challenge token
    assert_eq!(output.id, id);
    assert_eq!(&output.data[..], &data[..]);
}

#[test]
fn connect_token_private() {
    // generate a connect token
    //let server_address = "127.0.0.1:40000".parse().unwrap();

    let user_data = random_user_data();

    let expire_timestamp: u64 = 30 + time_secs();
    let key = keygen();

    let input = Private::generate(
        TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, user_data.clone());

    assert_eq!(input.client_id, TEST_CLIENT_ID);
    assert_eq!(&input.data[..], &user_data[..]);

    // write it to a buffer

    let mut buffer = [0u8; Private::BYTES];
    input.write(&mut buffer[..]).unwrap();

    // seal/open the buffer

    Private::seal(
        &mut buffer[..],
        TEST_PROTOCOL,
        expire_timestamp,
        &[0; 24],
        &key).unwrap();

    Private::open(
        &mut buffer[..],
        TEST_PROTOCOL,
        expire_timestamp,
        &[0; 24],
        &key).unwrap();

    // read the connect token back in

    let output = Private::read(&mut buffer).unwrap();

    // make sure that everything matches the original connect token

    assert_eq!(output.client_id, input.client_id);
    assert_eq!(output.timeout, input.timeout);
    assert_eq!(output.client_key, input.client_key);
    assert_eq!(output.server_key, input.server_key);
    assert_eq!(&output.data[..], &input.data[..]);
}

#[test]
fn connect_token_public() {
    // generate a private connect token
    // let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = random_user_data();
    let key = keygen();
    let create = time_secs();
    let expire = create + 30;

    // write it to a buffer
    let private = Private::generate(
        TEST_CLIENT_ID,
        TEST_TIMEOUT_SECONDS,
        user_data.clone(),
    );

    // seal the buffer
    let token = private.write_sealed(
        TEST_PROTOCOL,
        expire,
        &[0; 24],
        &key,
    );

    // wrap a public connect token around the private connect token data
    let input = Public {
        version: VERSION,
        protocol_id: TEST_PROTOCOL,
        create,
        expire,
        nonce: [0; 24],
        client_key: private.client_key,
        server_key: private.server_key,
        timeout: TEST_TIMEOUT_SECONDS,

        token,
        data: random_user_data(),
    };

    // write the connect token to a buffer
    let mut buffer = [0u8; Public::BYTES];
    input.write(&mut buffer[..]).unwrap();

    // read the buffer back in
    let output = Public::read(&mut buffer).unwrap();

    // make sure the public connect token matches what was written
    assert_eq!(output.version, input.version);
    assert_eq!(output.protocol_id, input.protocol_id);
    assert_eq!(output.create, input.create);
    assert_eq!(output.expire, input.expire);
    assert_eq!(output.nonce, input.nonce);
    assert_eq!(&output.token[..], &input.token[..]);
    assert_eq!(output.client_key, input.client_key);
    assert_eq!(output.server_key, input.server_key);
    assert_eq!(output.timeout, input.timeout);
}
*/
