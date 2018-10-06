#![feature(test)]
extern crate test;
use test::{Bencher, black_box};

use oni_net::{
    packet::{Request, Encrypted, Allowed},
    packet::{MAX_PACKET_BYTES, MAX_PAYLOAD_BYTES},
    protection::NoFilter,
    token,
    utils::{time},
    crypto::{keygen, MAC_BYTES, TOKEN_DATA},
};

const TEST_CLIENT_ID: u64 = 0x1;
const TEST_TIMEOUT_SECONDS: u32 = 15;
const TEST_PROTOCOL: u64 = 0x1122334455667788;
const TEST_SEQ: u64 = 1000;

fn random_user_data() -> [u8; TOKEN_DATA] {
    [4u8; TOKEN_DATA]
}

fn random_token() -> [u8; token::Challenge::BYTES] {
    [4u8; token::Challenge::BYTES]
}

fn random_payload() -> [u8; MAX_PAYLOAD_BYTES] {
    [4u8; MAX_PAYLOAD_BYTES]
}

#[bench]
fn bench_request(b: &mut Bencher) {
    b.iter(|| {
        // generate a connect token
        //let server_address = "127.0.0.1:40000".parse().unwrap();
        let user_data = black_box(random_user_data());
        let input_token = token::Private::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, user_data.clone());

        // write the conect token to a buffer (non-encrypted)
        let mut token_data = [0u8; token::Private::BYTES];
        input_token.write(&mut token_data).unwrap();

        // copy to a second buffer then encrypt it in place (we need the unencrypted token for verification later on)
        let mut encrypted_token_data = token_data.clone();

        let token_nonce = [0; 24];
        let token_expire_timestamp = time() + 30;
        let key = keygen();

        token::Private::encrypt(
            &mut encrypted_token_data[..],
            TEST_PROTOCOL,
            token_expire_timestamp,
            &token_nonce,
            &key,
        ).unwrap();

        // setup a connection request packet wrapping the encrypted connect token
        let input = Request {
            expire: token_expire_timestamp,
            nonce: token_nonce,
            token: encrypted_token_data,
        };

        let input = black_box(input);

        // write the connection request packet to a buffer
        let buffer = input.write(TEST_PROTOCOL);

        let buffer = black_box(buffer);

        // read the connection request packet back in from the buffer
        // (the connect token data is decrypted as part of the read packet validation)
        let output = Request::read(
            &buffer[..],
            time(),
            TEST_PROTOCOL,
            &key,
        );

        let output = black_box(output);

        /*
        if let Some(Request { expire, sequence, token }) = output {
            //assert_eq!(sequence, 100);
            // make sure the read packet matches what was written
            assert_eq!(expire, token_expire_timestamp);
            assert_eq!(sequence, token_sequence);
            let len = token::Private::BYTES - MAC_BYTES;
            assert_eq!(&token[..len], &token_data[..len]);
        } else {
            panic!("fail packet");
        }
        */
    });
}
