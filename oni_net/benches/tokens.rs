#![feature(test)]
extern crate test;
use test::{Bencher, black_box};

use oni_net::{
    challenge_token::{ChallengeToken, CHALLENGE_LEN},
    token::{Challenge, Private, Public},
    crypto::{keygen, TOKEN_DATA},
    utils::time,
    VERSION,
};

#[bench]
fn challenge_token(b: &mut Bencher) {
    let key = keygen();

    b.iter(|| {
        // encrypt/decrypt the buffer
        let id = black_box(1);
        let data = black_box([0u8; 256]);
        let seq = black_box(123);

        let buffer = Challenge { id, data }.write(seq, &key).unwrap();

        let mut buffer = black_box(buffer); // send...

        let seq = black_box(123);
        let v = Challenge::read(buffer, seq, &key).unwrap();
        black_box(v.data);
        black_box(v.id);
    });
}

#[bench]
fn fast_challenge_token(b: &mut Bencher) {
    let key = keygen();

    b.iter(|| {
        // encrypt/decrypt the buffer
        let id = black_box(1);
        let data = black_box([0u8; 256]);
        let seq = black_box(123);

        let tok = ChallengeToken::new(id, data);
        let buffer = ChallengeToken::encrypt(tok, seq, &key);
        let buffer = black_box(buffer); // send...
        let seq = black_box(123);
        let tok = ChallengeToken::decrypt(buffer, seq, &key).unwrap();

        black_box(tok.user());
        black_box(tok.client_id());
    });
}
