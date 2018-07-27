use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

use crate::{
    token,
    VERSION_INFO_BYTES,
    VERSION_INFO,
    crypto::Key,
    packet::REQUEST,
};

pub struct Request {
    pub sequence: u64,
    pub version_info: [u8; VERSION_INFO_BYTES],
    pub protocol_id: u64,
    pub expire_timestamp: u64,
    pub private_data: [u8; token::Private::BYTES],
}

impl Request {
    pub const BYTES: usize = 1 + VERSION_INFO_BYTES + 8 * 3 + token::Private::BYTES;

    pub fn read(
        mut buffer: &[u8],
        current_timestamp: u64,
        current_protocol_id: u64,
        key: &Key,
    ) -> Option<Self> {
        if buffer.len() != Self::BYTES {
            return None;
        }

        if buffer.read_u8().ok()? != 0 {
            return None;
        }

        let mut version_info = [0u8; VERSION_INFO_BYTES];
        buffer.read_exact(&mut version_info[..]).ok()?;
        if version_info != VERSION_INFO {
            return None;
        }

        let protocol_id = buffer.read_u64::<LE>().ok()?;
        if protocol_id != current_protocol_id {
            return None;
        }

        let expire_timestamp = buffer.read_u64::<LE>().ok()?;
        if expire_timestamp <= current_timestamp {
            return None;
        }

        let sequence = buffer.read_u64::<LE>().ok()?;

        let mut private_data = [0u8; token::Private::BYTES];
        buffer.read_exact(&mut private_data[..]).ok()?;

        if token::Private::decrypt(
            &mut private_data[..], protocol_id, expire_timestamp, sequence, key,
        ).is_err() {
            println!("!!! decrypt !!!");
            return None;
        }

        Some(Self {
            version_info,
            protocol_id,
            expire_timestamp,
            sequence,
            private_data,
        })
    }

    pub fn write(self) -> [u8; Self::BYTES] {
        Self::write_request(
            self.protocol_id,
            self.expire_timestamp,
            self.sequence,
            self.private_data,
        )
    }

    pub fn write_token(token: &token::Public) -> [u8; Self::BYTES] {
        Self::write_request(
            token.protocol_id,
            token.expire_timestamp,
            token.sequence,
            token.private_data,
        )
    }

    pub fn write_request(
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        private_data: [u8; token::Private::BYTES],
    ) -> [u8; Self::BYTES] {
        let mut buffer: [u8; Self::BYTES] = unsafe { ::std::mem::uninitialized() };
        {
            let mut buffer = &mut buffer[..];
            buffer.write_u8(REQUEST).unwrap();
            buffer.write_all(&VERSION_INFO[..]).unwrap();
            buffer.write_u64::<LE>(protocol_id).unwrap();
            buffer.write_u64::<LE>(expire_timestamp).unwrap();
            buffer.write_u64::<LE>(sequence).unwrap();
            buffer.write_all(&private_data[..]).unwrap();
        }
        buffer
    }
}

#[test]
fn connection_request_packet() {
    use crate::{
        TEST_PROTOCOL_ID,
        TEST_TIMEOUT_SECONDS,
        TEST_CLIENT_ID,
        token,
        utils::{UserData, time},
        crypto::MAC_BYTES,
    };

    // generate a connect token
    let server_address = "127.0.0.1:40000".parse().unwrap();
    let user_data = UserData::random();
    let input_token = token::Private::generate(TEST_CLIENT_ID, TEST_TIMEOUT_SECONDS, vec![server_address], user_data.clone());
    assert_eq!(input_token.client_id, TEST_CLIENT_ID);
    assert_eq!(input_token.server_addresses, &[server_address]);
    assert_eq!(input_token.user_data, user_data);

    // write the conect token to a buffer (non-encrypted)
    let mut token_data = [0u8; token::Private::BYTES];
    input_token.write(&mut token_data).unwrap();

    // copy to a second buffer then encrypt it in place (we need the unencrypted token for verification later on)
    let mut encrypted_token_data = token_data.clone();

    let token_sequence = 1000u64;
    let token_expire_timestamp = time() + 30;
    let key = Key::generate();

    token::Private::encrypt(
        &mut encrypted_token_data[..],
        TEST_PROTOCOL_ID,
        token_expire_timestamp,
        token_sequence,
        &key,
    ).unwrap();

    // setup a connection request packet wrapping the encrypted connect token
    let input_packet = Request {
        version_info: VERSION_INFO,
        protocol_id: TEST_PROTOCOL_ID,
        expire_timestamp: token_expire_timestamp,
        sequence: token_sequence,
        private_data: encrypted_token_data,
    };

    // write the connection request packet to a buffer
    let buffer = input_packet.write();

    // read the connection request packet back in from the buffer
    // (the connect token data is decrypted as part of the read packet validation)
    let output_packet = Request::read(
        &buffer[..],
        crate::utils::time(),
        TEST_PROTOCOL_ID,
        &key,
    );

    if let Some(Request { version_info, protocol_id, expire_timestamp, sequence, private_data  }) = output_packet {
        //assert_eq!(sequence, 100);
        // make sure the read packet matches what was written
        assert_eq!(version_info, VERSION_INFO);
        assert_eq!(protocol_id, TEST_PROTOCOL_ID);
        assert_eq!(expire_timestamp, token_expire_timestamp );
        assert_eq!(sequence, token_sequence);
        let len = token::Private::BYTES - MAC_BYTES;
        assert_eq!(&private_data[..len], &token_data[..len]);
    } else {
        panic!("fail packet");
    }
}
