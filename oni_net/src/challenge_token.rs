use crypto::{encrypt_aead, decrypt_aead, MAC_BYTES, Key, Nonce};
use utils::{UserData, ReadUserData, WriteUserData, USER_DATA_BYTES};

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io;

pub const CHALLENGE_TOKEN_BYTES: usize = 300;

pub struct ChallengeToken {
    pub client_id: u64,
    pub user_data: UserData,
}

impl ChallengeToken {
    pub fn read(mut buffer: &[u8]) -> io::Result<Self> {
        let start_len = buffer.len();
        let client_id = buffer.read_u64::<LE>()?;
        let user_data = buffer.read_user_data()?;
        assert!(start_len - buffer.len() == 8 + USER_DATA_BYTES);
        Ok(Self { client_id, user_data })
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<()> {
        let start_len = buffer.len();
        buffer.write_u64::<LE>(self.client_id)?;
        buffer.write_user_data(&self.user_data)?;
        assert!(start_len - buffer.len() <= CHALLENGE_TOKEN_BYTES - MAC_BYTES);
        Ok(())
    }

    pub fn encrypt(buffer: &mut [u8], sequence: u64, key: &Key) -> io::Result<()> {
        let nonce = Nonce::from_sequence(sequence);
        encrypt_aead(&mut buffer[..CHALLENGE_TOKEN_BYTES - MAC_BYTES], &[], &nonce, key)
    }

    pub fn decrypt(buffer: &mut [u8], sequence: u64, key: &Key) -> io::Result<()> {
        let nonce = Nonce::from_sequence(sequence);
        decrypt_aead(&mut buffer[..CHALLENGE_TOKEN_BYTES], &[], &nonce, key)
    }
}

#[test]
fn challenge_token() {
    // generate a challenge token
    let mut user_data = [0u8; ::utils::USER_DATA_BYTES];
    ::crypto::random_bytes(&mut user_data[..]);
    let input_token = ChallengeToken {
        client_id: 1,
        user_data: user_data.into(),
    };

    // write it to a buffer
    let mut buffer = [0u8; CHALLENGE_TOKEN_BYTES];
    input_token.write(&mut buffer[..]).unwrap();

    // encrypt/decrypt the buffer
    let sequence = 1000u64;
    let key = Key::generate();
    ChallengeToken::encrypt(&mut buffer[..], sequence, &key).unwrap();
    ChallengeToken::decrypt(&mut buffer[..], sequence, &key).unwrap();

    // read the challenge token back in
    let output_token = ChallengeToken::read(&buffer[..]).unwrap();
    // make sure that everything matches the original challenge token
    assert_eq!(output_token.client_id, input_token.client_id);
    assert_eq!(output_token.user_data, input_token.user_data);
}
