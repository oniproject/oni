
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::net::SocketAddr;
use std::io::{self, Write};

use crate::{
    crypto::{encrypt_aead, decrypt_aead, MAC_BYTES, Key, Nonce, ReadKey, WriteKey},
    addr::{ReadIps, WriteIps, MAX_SERVERS_PER_CONNECT},
    utils::{UserData, ReadUserData, WriteUserData},
    VERSION_INFO_BYTES,
};

pub struct Private {
    pub client_id: u64,
    pub timeout_seconds: u32,
    pub server_addresses: Vec<SocketAddr>,
    pub client_to_server_key: Key,
    pub server_to_client_key: Key,
    pub user_data: UserData,
}

impl Private {
    pub const BYTES: usize = 1024;

    pub fn generate(client_id: u64, timeout_seconds: u32, addresses: Vec<SocketAddr>, user_data: UserData) -> Self {
        assert!(addresses.len() > 0);
        assert!(addresses.len() <= MAX_SERVERS_PER_CONNECT);
        Self {
            client_id,
            timeout_seconds,
            server_addresses: addresses,
            client_to_server_key: Key::generate(),
            server_to_client_key: Key::generate(),
            user_data,
        }
    }

    pub fn read(mut buffer: &[u8]) -> io::Result<Self> {
        Ok(Self {
            client_id: buffer.read_u64::<LE>()?,
            timeout_seconds: buffer.read_u32::<LE>()?,
            server_addresses: buffer.read_ips()?,
            client_to_server_key: buffer.read_key()?,
            server_to_client_key: buffer.read_key()?,
            user_data: buffer.read_user_data()?,
        })
    }


    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<()> {
        buffer.write_u64::<LE>(self.client_id)?;
        buffer.write_u32::<LE>(self.timeout_seconds)?;
        buffer.write_ips(&self.server_addresses)?;
        buffer.write_key(&self.client_to_server_key)?;
        buffer.write_key(&self.server_to_client_key)?;
        buffer.write_user_data(&self.user_data)
    }

    pub fn encrypt(
        buffer: &mut [u8],
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == Self::BYTES);

        let mut additional = [0u8; VERSION_INFO_BYTES + 8 + 8];
        {
            let mut p = &mut additional[..];
            p.write_all(&::VERSION_INFO[..]).unwrap();
            p.write_u64::<LE>(protocol_id).unwrap();
            p.write_u64::<LE>(expire_timestamp).unwrap();
        }

        let nonce = Nonce::from_sequence(sequence);
        let len = Self::BYTES - MAC_BYTES;
        encrypt_aead(&mut buffer[..len], &additional[..], &nonce, key)
    }

    pub fn decrypt(
        buffer: &mut [u8],
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == Self::BYTES);

        let mut additional = [0u8; VERSION_INFO_BYTES + 8 + 8];
        {
            let mut p = &mut additional[..];
            p.write_all(&::VERSION_INFO[..]).unwrap();
            p.write_u64::<LE>(protocol_id).unwrap();
            p.write_u64::<LE>(expire_timestamp).unwrap();
        }

        let nonce = Nonce::from_sequence(sequence);
        let len = Self::BYTES;
        decrypt_aead(&mut buffer[..len], &additional[..], &nonce, key)
    }
}

#[test]
fn connect_token() {
    use crate::{
        TEST_CLIENT_ID,
        TEST_TIMEOUT_SECONDS,
        TEST_PROTOCOL_ID,
    };

    // generate a connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();

    let mut user_data = [0u8; crate::utils::USER_DATA_BYTES];
    crate::crypto::random_bytes(&mut user_data[..]);
    let user_data: UserData = user_data.into();

    let input_token = Private::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, vec![server_address], user_data.clone());

    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(input_token.server_addresses, &[server_address]);
    assert_eq!(input_token.user_data, user_data);

    // write it to a buffer

    let mut buffer = [0u8; Private::BYTES];
    input_token.write(&mut buffer[..]).unwrap();

    // encrypt/decrypt the buffer

    let sequence = 1000u64;
    let expire_timestamp: u64 = 30 + crate::utils::time();
    let key = Key::generate();

    Private::encrypt(
        &mut buffer[..],
        TEST_PROTOCOL_ID,
        expire_timestamp,
        sequence,
        &key).unwrap();

    Private::decrypt(
        &mut buffer[..],
        TEST_PROTOCOL_ID,
        expire_timestamp,
        sequence,
        &key).unwrap();

    // read the connect token back in

    let output_token = Private::read(&mut buffer[..]).unwrap();

    // make sure that everything matches the original connect token

    assert_eq!(output_token.client_id, input_token.client_id);
    assert_eq!(output_token.timeout_seconds, input_token.timeout_seconds);
    assert_eq!(output_token.client_to_server_key, input_token.client_to_server_key);
    assert_eq!(output_token.server_to_client_key, input_token.server_to_client_key);
    assert_eq!(output_token.user_data, input_token.user_data);
    assert_eq!(&output_token.server_addresses[..], &input_token.server_addresses[..]);
}
