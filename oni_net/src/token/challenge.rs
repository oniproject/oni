use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io;

use crate::{
    crypto::{encrypt_aead, decrypt_aead, MAC_BYTES, Key, Nonce},
    utils::{UserData, ReadUserData, WriteUserData, USER_DATA_BYTES},
};

pub struct Challenge {
    pub client_id: u64,
    pub user_data: UserData,
}

impl Challenge {
    pub const BYTES: usize = 300;

    pub fn read(buffer: &[u8; Self::BYTES]) -> Self {
        let mut buffer = &buffer[..];
        let start_len = buffer.len();
        let client_id = buffer.read_u64::<LE>().unwrap();
        let user_data = buffer.read_user_data().unwrap();
        assert!(start_len - buffer.len() == 8 + USER_DATA_BYTES);
        Self { client_id, user_data }
    }

    pub fn write(client_id: u64, user_data: &UserData) -> [u8; Self::BYTES] {
        let mut data = [0u8; Self::BYTES];
        {
            let mut buffer = &mut data[..];
            buffer.write_u64::<LE>(client_id).unwrap();
            buffer.write_user_data(user_data).unwrap();
            assert!(buffer.len() >= MAC_BYTES);
        }
        data
    }

    pub fn write_encrypted(id: u64, user_data: &UserData, seq: u64, key: &Key)
        -> io::Result<[u8; Self::BYTES]>
    {
        let mut buf = Self::write(id, user_data);
        Self::encrypt(&mut buf, seq, key)?;
        Ok(buf)
    }

    pub fn encrypt(buffer: &mut [u8; Self::BYTES], seq: u64, key: &Key)
        -> io::Result<()>
    {
        let nonce = Nonce::from_sequence(seq);
        encrypt_aead(&mut buffer[..Self::BYTES - MAC_BYTES], &[], &nonce, key)
    }

    pub fn decrypt(buffer: &mut [u8; Self::BYTES], seq: u64, key: &Key)
        -> io::Result<()>
    {
        let nonce = Nonce::from_sequence(seq);
        decrypt_aead(&mut buffer[..Self::BYTES], &[], &nonce, key)
    }
}

#[test]
fn challenge_token() {
    // generate a challenge token
    let mut user_data = [0u8; crate::utils::USER_DATA_BYTES];
    crate::crypto::random_bytes(&mut user_data[..]);
    let client_id = 1;
    let user_data: UserData = user_data.into();

    // write it to a buffer
    let mut buffer = Challenge::write(1, &user_data);

    // encrypt/decrypt the buffer
    let seq = 1000u64;
    let key = Key::generate();
    Challenge::encrypt(&mut buffer, seq, &key).unwrap();
    Challenge::decrypt(&mut buffer, seq, &key).unwrap();

    // read the challenge token back in
    let output_token = Challenge::read(&buffer);
    // make sure that everything matches the original challenge token
    assert_eq!(output_token.client_id, client_id);
    assert_eq!(output_token.user_data, user_data);
}
