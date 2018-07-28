use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::net::SocketAddr;
use std::io::{self, Read, Write};

use crate::{
    crypto::{Key, ReadKey, WriteKey},
    addr::{ReadIps, WriteIps},
    utils::UserData,
    VERSION_BYTES,
    VERSION,
    utils::time,
    token::Private,

    TEST_CLIENT_ID,
    TEST_TIMEOUT_SECONDS,
    TEST_PROTOCOL,
};

pub struct Public {
    pub version: [u8; VERSION_BYTES],
    pub protocol_id: u64,
    pub create_timestamp: u64,
    pub expire_timestamp: u64,
    pub sequence: u64,
    pub private_data: [u8; Private::BYTES],
    pub timeout_seconds: u32,
    pub server_addresses: Vec<SocketAddr>,
    pub client_to_server_key: Key,
    pub server_to_client_key: Key,
}

impl Public {
    pub const BYTES: usize = 2048;

    pub fn new(
        public_server_addresses: Vec<SocketAddr>,
        internal_server_addresses: Vec<SocketAddr>,
        expire_seconds: u32,
        timeout_seconds: u32,
        client_id: u64,
        protocol_id: u64,
        sequence: u64,
        private_key: &Key,
    )
        -> io::Result<Self>
    {
        // generate a connect token
        let user_data = UserData::random();
        let connect_token_private = Private::generate(
            client_id, timeout_seconds, internal_server_addresses, user_data
        );

        // write it to a buffer
        let mut connect_token_data = [0u8; Private::BYTES];
        connect_token_private.write(&mut connect_token_data[..])?;

        // encrypt the buffer
        let create_timestamp = time();
        let expire_timestamp = create_timestamp + expire_seconds as u64;
        Private::encrypt(&mut connect_token_data[..], protocol_id, expire_timestamp, sequence, private_key)?;

        // wrap a connect token around the private connect token data
        Ok(Self {
            version: VERSION,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            private_data: connect_token_data,
            server_addresses: public_server_addresses,
            client_to_server_key: connect_token_private.client_to_server_key,
            server_to_client_key: connect_token_private.server_to_client_key,
            timeout_seconds,
        })
    }

    pub fn generate(
        public_server_addresses: Vec<SocketAddr>,
        internal_server_addresses: Vec<SocketAddr>,
        expire_seconds: u32,
        timeout_seconds: u32,
        client_id: u64,
        protocol_id: u64,
        sequence: u64,
        private_key: &Key,
        output_buffer: &mut [u8],
    )
        -> io::Result<()>
    {
        // generate a connect token
        let user_data = UserData::random();
        let connect_token_private = Private::generate(
            client_id, timeout_seconds, internal_server_addresses, user_data
        );

        // write it to a buffer
        let mut connect_token_data = [0u8; Private::BYTES];
        connect_token_private.write(&mut connect_token_data[..])?;

        // encrypt the buffer
        let create_timestamp = time();
        let expire_timestamp = create_timestamp + expire_seconds as u64;
        Private::encrypt(&mut connect_token_data[..], protocol_id, expire_timestamp, sequence, private_key)?;

        // wrap a connect token around the private connect token data
        let connect_token = Self {
            version: VERSION,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            private_data: connect_token_data,
            server_addresses: public_server_addresses,
            client_to_server_key: connect_token_private.client_to_server_key,
            server_to_client_key: connect_token_private.server_to_client_key,
            timeout_seconds,
        };

        // write the connect token to the output buffer
        connect_token.write(output_buffer)?;
        Ok(())
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<usize> {
        let start_len = buffer.len();

        buffer.write_all(&self.version[..])?;
        buffer.write_u64::<LE>(self.protocol_id)?;
        buffer.write_u64::<LE>(self.create_timestamp)?;
        buffer.write_u64::<LE>(self.expire_timestamp)?;
        buffer.write_u64::<LE>(self.sequence)?;
        buffer.write_all(&self.private_data[..])?;
        buffer.write_u32::<LE>(self.timeout_seconds)?;
        buffer.write_ips(&self.server_addresses)?;

        buffer.write_key(&self.client_to_server_key)?;
        buffer.write_key(&self.server_to_client_key)?;

        let count = Self::BYTES - (start_len - buffer.len());
        for _ in 0..count {
            buffer.write_u8(0)?;
        }
        Ok(Self::BYTES)
    }

    pub fn read(mut buffer: &[u8]) -> Option<Self> {
        if buffer.len() != Self::BYTES {
            error!("read connect data has bad buffer length ({})", buffer.len());
            return None;
        }

        let mut version = [0u8; VERSION_BYTES];
        buffer.read_exact(&mut version[..]).ok()?;
        if version != VERSION {
            error!("read connect data has bad version info (got {:?}, expected {:?})", &version[..], &VERSION[..]);
            return None;
        }

        let protocol_id = buffer.read_u64::<LE>().ok()?;
        let create_timestamp = buffer.read_u64::<LE>().ok()?;
        let expire_timestamp = buffer.read_u64::<LE>().ok()?;

        if create_timestamp > expire_timestamp {
            return None;
        }

        let sequence = buffer.read_u64::<LE>().ok()?;
        let mut private_data = [0u8; Private::BYTES];
        buffer.read_exact(&mut private_data[..]).ok()?;

        let timeout_seconds = buffer.read_u32::<LE>().ok()?;
        let server_addresses = buffer.read_ips().ok()?;
        let client_to_server_key = buffer.read_key().ok()?;
        let server_to_client_key = buffer.read_key().ok()?;

        Some(Self {
            version,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            private_data,
            timeout_seconds,
            server_addresses,
            client_to_server_key,
            server_to_client_key,
        })
    }
}

#[test]
fn connect_token_public() {
    // generate a private connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = UserData::random();
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
    let key = Key::generate();
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
    assert_eq!(output_connect_token.protocol_id, input_connect_token.protocol_id);
    assert_eq!(output_connect_token.create_timestamp, input_connect_token.create_timestamp);
    assert_eq!(output_connect_token.expire_timestamp, input_connect_token.expire_timestamp);
    assert_eq!(output_connect_token.sequence, input_connect_token.sequence);
    assert_eq!(&output_connect_token.private_data[..], &input_connect_token.private_data[..]);
    assert_eq!(&output_connect_token.server_addresses[..], &input_connect_token.server_addresses[..]);
    assert_eq!(output_connect_token.client_to_server_key, input_connect_token.client_to_server_key);
    assert_eq!(output_connect_token.server_to_client_key, input_connect_token.server_to_client_key);
    assert_eq!(output_connect_token.timeout_seconds, input_connect_token.timeout_seconds);
}
