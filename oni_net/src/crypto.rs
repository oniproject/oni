// FIXME: message length

use byteorder::{LE, WriteBytesExt, ReadBytesExt};
use std::io::{self, Read, Write};
use crate::{
    utils::time,
    chacha20poly1305::{encrypt, decrypt, KEYBYTES, NPUBBYTES, ABYTES},
    UserData,
    USER_DATA_BYTES,
    VERSION_BYTES,
    VERSION,
};

pub use crate::chacha20poly1305::keygen;


#[inline]
pub fn map_err(_err: ()) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "chacha20poly1305")
}

pub const MAC_BYTES: usize = ABYTES;

pub type Key = [u8; KEYBYTES];

pub trait ReadKey: std::io::Read {
    fn read_key(&mut self) -> std::io::Result<Key> {
        let mut key = [0u8; KEYBYTES];
        self.read_exact(&mut key[..])?;
        Ok(key)
    }
}
pub trait WriteKey: std::io::Write {
    fn write_key(&mut self, key: &Key) -> std::io::Result<()> {
        self.write_all(&key[..])?;
        Ok(())
    }
}
impl<T: std::io::Read> ReadKey for T {}
impl<T: std::io::Write> WriteKey for T {}

pub fn new_nonce(sequence: u64) -> [u8; NPUBBYTES] {
    let mut nonce = [0u8; NPUBBYTES];
    let mut p = &mut nonce[..];
    p.write_u32::<LE>(0).unwrap();
    p.write_u64::<LE>(sequence).unwrap();
    nonce
}

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
        let user_data = read_array_unwrap!(buffer, USER_DATA_BYTES);
        assert!(start_len - buffer.len() == 8 + USER_DATA_BYTES);
        Self { client_id, user_data: user_data.into() }
    }

    pub fn write(client_id: u64, user_data: &UserData) -> [u8; Self::BYTES] {
        let mut data = [0u8; Self::BYTES];
        let mut buffer = &mut data[..];
        buffer.write_u64::<LE>(client_id).unwrap();
        buffer.write_all(&user_data[..]).unwrap();
        assert!(buffer.len() >= MAC_BYTES);
        data
    }

    pub fn decrypt_and_read(data: &mut [u8; Self::BYTES], seq: u64, key: &Key) -> io::Result<Self> {
        Self::decrypt(data, seq, key)?;
        Ok(Self::read(data))
    }

    pub fn write_encrypted(id: u64, user_data: &UserData, seq: u64, key: &Key)
        -> io::Result<[u8; Self::BYTES]>
    {
        let mut buf = Self::write(id, user_data);
        Self::encrypt(&mut buf, seq, key)?;
        Ok(buf)
    }

    pub fn encrypt(buffer: &mut [u8; Self::BYTES], seq: u64, key: &Key) -> io::Result<()> {
        let m = &mut buffer[..Self::BYTES - MAC_BYTES];
        encrypt(m, &[], &new_nonce(seq), key).map_err(map_err)
    }

    pub fn decrypt(buffer: &mut [u8; Self::BYTES], seq: u64, key: &Key) -> io::Result<()> {
        let m = &mut buffer[..Self::BYTES];
        decrypt(m, &[], &new_nonce(seq), key).map_err(map_err)
    }
}

pub struct Public {
    pub version: [u8; VERSION_BYTES],
    pub protocol_id: u64,

    pub create_timestamp: u64,
    pub expire_timestamp: u64,

    pub sequence: u64,
    pub timeout_seconds: u32,

    pub client_to_server_key: Key,
    pub server_to_client_key: Key,

    pub token: [u8; Private::BYTES],
    pub user_data: UserData,
}

impl Public {
    pub const BYTES: usize = 2048;

    pub fn new(
        expire_seconds: u32,
        timeout_seconds: u32,

        client_id: u64,
        protocol_id: u64,
        sequence: u64,

        user_data: UserData,

        private_key: &Key,
        private_data: UserData,
    )
        -> io::Result<Self>
    {
        // generate a connect token
        let private = Private::generate(client_id, timeout_seconds, private_data);

        // write it to a buffer
        let mut token = [0u8; Private::BYTES];
        private.write(&mut token[..])?;

        // encrypt the buffer
        let create_timestamp = time();
        let expire_timestamp = create_timestamp + expire_seconds as u64;
        Private::encrypt(&mut token[..], protocol_id,
                         expire_timestamp, sequence, private_key)?;

        // wrap a connect token around the private connect token data
        Ok(Self {
            version: VERSION,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            token,
            client_to_server_key: private.client_to_server_key,
            server_to_client_key: private.server_to_client_key,
            timeout_seconds,
            user_data,
        })
    }

    pub fn generate(
        expire_seconds: u32,
        timeout_seconds: u32,
        client_id: u64,
        protocol_id: u64,
        sequence: u64,
        private_key: &Key,
        output_buffer: &mut [u8],
        user_data: UserData,
        private_data: UserData,
    )
        -> io::Result<()>
    {
        // generate a connect token
        let private = Private::generate(client_id, timeout_seconds, private_data);

        // write it to a buffer
        let mut token = [0u8; Private::BYTES];
        private.write(&mut token[..])?;

        // encrypt the buffer
        let create_timestamp = time();
        let expire_timestamp = create_timestamp + expire_seconds as u64;
        Private::encrypt(&mut token[..], protocol_id,
                         expire_timestamp, sequence, private_key)?;

        // wrap a connect token around the private connect token data
        let connect_token = Self {
            version: VERSION,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            token,
            client_to_server_key: private.client_to_server_key,
            server_to_client_key: private.server_to_client_key,
            timeout_seconds,
            user_data,
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
        buffer.write_all(&self.token[..])?;
        buffer.write_u32::<LE>(self.timeout_seconds)?;

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
            //error!("read connect data has bad buffer length ({})",
            //buffer.len());
            return None;
        }

        let mut version = [0u8; VERSION_BYTES];
        buffer.read_exact(&mut version[..]).ok()?;
        if version != VERSION {
            //error!("read connect data has bad version info (got {:?}, expected {:?})", &version[..], &VERSION[..]);
            return None;
        }

        let protocol_id = buffer.read_u64::<LE>().ok()?;
        let create_timestamp = buffer.read_u64::<LE>().ok()?;
        let expire_timestamp = buffer.read_u64::<LE>().ok()?;

        if create_timestamp > expire_timestamp {
            return None;
        }

        let sequence = buffer.read_u64::<LE>().ok()?;
        let mut token = [0u8; Private::BYTES];
        buffer.read_exact(&mut token[..]).ok()?;

        let timeout_seconds = buffer.read_u32::<LE>().ok()?;
        let client_to_server_key = buffer.read_key().ok()?;
        let server_to_client_key = buffer.read_key().ok()?;
        let mut user_data = [0u8; USER_DATA_BYTES];
        buffer.read_exact(&mut user_data).ok()?;

        Some(Self {
            version,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            token,
            timeout_seconds,
            client_to_server_key,
            server_to_client_key,
            user_data,
        })
    }
}

pub struct Private {
    pub client_id: u64,
    pub timeout_seconds: u32,
    pub client_to_server_key: Key,
    pub server_to_client_key: Key,
    pub user_data: UserData,
}

impl Private {
    pub const BYTES: usize = 256 + 76;

    pub fn generate(client_id: u64, timeout_seconds: u32, user_data: UserData) -> Self {
        Self {
            client_id,
            timeout_seconds,
            client_to_server_key: keygen(),
            server_to_client_key: keygen(),
            user_data,
        }
    }

    pub fn read(mut buffer: &[u8]) -> io::Result<Self> {
        Ok(Self {
            client_id: buffer.read_u64::<LE>()?,
            timeout_seconds: buffer.read_u32::<LE>()?,
            client_to_server_key: buffer.read_key()?,
            server_to_client_key: buffer.read_key()?,
            user_data: read_array!(buffer, USER_DATA_BYTES),
        })
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<()> {
        buffer.write_u64::<LE>(self.client_id)?;
        buffer.write_u32::<LE>(self.timeout_seconds)?;
        buffer.write_key(&self.client_to_server_key)?;
        buffer.write_key(&self.server_to_client_key)?;
        buffer.write_all(&self.user_data[..])
    }

    pub fn encrypt(
        buffer: &mut [u8],
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == Self::BYTES);

        let mut additional = [0u8; VERSION_BYTES + 8 + 8];
        let mut p = &mut additional[..];
        p.write_all(&VERSION[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u64::<LE>(expire_timestamp).unwrap();

        encrypt(
            &mut buffer[..Self::BYTES - MAC_BYTES],
            &additional[..],
            &new_nonce(sequence),
            key,
        ).map_err(map_err)
    }

    pub fn decrypt(
        buffer: &mut [u8],
        protocol_id: u64,
        expire_timestamp: u64,
        sequence: u64,
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == Self::BYTES);

        let mut additional = [0u8; VERSION_BYTES + 8 + 8];
        let mut p = &mut additional[..];
        p.write_all(&VERSION[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u64::<LE>(expire_timestamp).unwrap();

        decrypt(
            &mut buffer[..Self::BYTES],
            &additional[..],
            &new_nonce(sequence),
            key,
        ).map_err(map_err)
    }
}
