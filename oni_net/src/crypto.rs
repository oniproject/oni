// FIXME: message length

use byteorder::{LE, WriteBytesExt, ReadBytesExt};
use std::io::{self, Read, Write};
use crate::{
    utils::time,
    chacha20poly1305::{
        encrypt, decrypt,
        encrypt_bignonce, decrypt_bignonce,
        randbuf, generate_nonce,
        KEYBYTES, NPUBBYTES, ABYTES},
    VERSION_BYTES,
    VERSION,
};

pub use crate::chacha20poly1305::keygen;

pub const TOKEN_DATA: usize = 640;

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
    /// client id
    pub id: u64,
    /// user data
    pub data: [u8; 256],
}

impl Challenge {
    pub const BYTES: usize = 300;

    pub fn read(mut b: [u8; Self::BYTES], seq: u64, key: &Key)
        -> io::Result<Self>
    {

        decrypt(&mut b[..Self::BYTES], &[], &new_nonce(seq), key)
            .map_err(map_err)?;

        let mut b = &b[..];
        let id = b.read_u64::<LE>().unwrap();
        let data = read_array_unwrap!(b, 256);

        //debug_assert!(start_len - b.len() == 8 + USER_DATA);
        Ok(Self { id, data })
    }

    pub fn write(self, seq: u64, key: &Key)
        -> io::Result<[u8; Self::BYTES]>
    {
        let mut b = [0u8; Self::BYTES];
        let mut w = &mut b[..];
        w.write_u64::<LE>(self.id).unwrap();
        w.write_all(&self.data[..]).unwrap();
        //debug_assert!(w.len() >= MAC_BYTES);

        encrypt(&mut b[..Self::BYTES - MAC_BYTES], &[], &new_nonce(seq), key)
            .map_err(map_err)?;

        Ok(b)
    }
}

pub struct KeyPair {
    /// Client to Server key
    pub client_key: Key,
    /// Server to Client key
    pub server_key: Key,
}

impl KeyPair {
    pub fn keygen() -> Self {
        Self {
            client_key: keygen(),
            server_key: keygen(),
        }
    }
}

pub struct Public {
    pub version: [u8; VERSION_BYTES],

    /// Protocol ID
    pub protocol_id: u64,

    /// Create timestamp
    pub create: u64,
    /// Expire timestamp
    pub expire: u64,

    /// Connect token nonce
    pub nonce: [u8; 24],
    /// Timeout in seconds
    pub timeout: u32,

    /// Client to Server key
    pub client_key: Key,
    /// Server to Client key
    pub server_key: Key,

    /// Encrypted private connect token data
    pub token: [u8; Private::BYTES],

    /// User data
    pub data: [u8; TOKEN_DATA],
}

impl Public {
    pub const BYTES: usize = 2048;

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<usize> {
        let start_len = buffer.len();

        buffer.write_all(&self.version[..])?;
        buffer.write_u64::<LE>(self.protocol_id)?;
        buffer.write_u64::<LE>(self.create)?;
        buffer.write_u64::<LE>(self.expire)?;
        buffer.write_all(&self.nonce[..])?;
        buffer.write_all(&self.token[..])?;
        buffer.write_u32::<LE>(self.timeout)?;

        buffer.write_key(&self.client_key)?;
        buffer.write_key(&self.server_key)?;

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
        let create = buffer.read_u64::<LE>().ok()?;
        let expire = buffer.read_u64::<LE>().ok()?;

        if create > expire {
            return None;
        }

        let mut nonce = [0u8; 24];
        buffer.read_exact(&mut nonce[..]).ok()?;

        let mut token = [0u8; Private::BYTES];
        buffer.read_exact(&mut token[..]).ok()?;

        let timeout = buffer.read_u32::<LE>().ok()?;
        let client_key = buffer.read_key().ok()?;
        let server_key = buffer.read_key().ok()?;
        let mut data = [0u8; TOKEN_DATA];
        buffer.read_exact(&mut data).ok()?;

        Some(Self {
            version,
            protocol_id,
            create,
            expire,
            nonce,
            token,
            timeout,
            client_key,
            server_key,
            data,
        })
    }
}

pub struct Private {
    pub client_id: u64,
    pub timeout: u32,

    /// Client to Server key
    pub client_key: Key,
    /// Server to Client key
    pub server_key: Key,

    pub data: [u8; TOKEN_DATA],
    pub seed: [u8; 256],
}

impl Private {
    pub const BYTES: usize = 1024;

    pub fn challenge(&self, seq: u64, key: &Key)
        -> io::Result<[u8; Challenge::BYTES]>
    {
        Challenge {
            id: self.client_id,
            data: self.seed,
        }.write(seq, key)
    }

    pub fn generate(client_id: u64, timeout: u32, data: [u8; TOKEN_DATA]) -> Self {
        let mut seed = [0u8; 256];
        randbuf(&mut seed);
        Self {
            client_id,
            timeout,
            seed,
            data,
            client_key: keygen(),
            server_key: keygen(),
        }
    }

    pub fn read(mut buffer: &[u8]) -> io::Result<Self> {
        Ok(Self {
            client_id: buffer.read_u64::<LE>()?,
            timeout: buffer.read_u32::<LE>()?,
            client_key: buffer.read_key()?,
            server_key: buffer.read_key()?,
            data: read_array!(buffer, TOKEN_DATA),
            seed: read_array!(buffer, 256),
        })
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<()> {
        buffer.write_u64::<LE>(self.client_id)?;
        buffer.write_u32::<LE>(self.timeout)?;
        buffer.write_key(&self.client_key)?;
        buffer.write_key(&self.server_key)?;
        buffer.write_all(&self.data[..])?;
        buffer.write_all(&self.seed[..])
    }

    pub fn write_encrypted(&self, protocol_id: u64, expire: u64, nonce: &[u8; 24], key: &Key) -> [u8; Self::BYTES] {
        let mut buf = [0u8; Self::BYTES];
        self.write(&mut buf[..]).unwrap();
        Self::encrypt(&mut buf[..], protocol_id, expire, nonce, key).unwrap();
        buf
    }

    pub fn encrypt(
        buffer: &mut [u8],
        protocol_id: u64,
        expire_timestamp: u64,
        nonce: &[u8; 24],
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == Self::BYTES);

        let mut additional = [0u8; VERSION_BYTES + 8 + 8];
        let mut p = &mut additional[..];
        p.write_all(&VERSION[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u64::<LE>(expire_timestamp).unwrap();

        encrypt_bignonce(
            &mut buffer[..Self::BYTES - MAC_BYTES],
            &additional[..],
            nonce,
            key,
        ).map_err(map_err)
    }

    pub fn decrypt(
        buffer: &mut [u8],
        protocol_id: u64,
        expire_timestamp: u64,
        nonce: &[u8; 24],
        key: &Key) -> io::Result<()>
    {
        assert!(buffer.len() == Self::BYTES);

        let mut additional = [0u8; VERSION_BYTES + 8 + 8];
        let mut p = &mut additional[..];
        p.write_all(&VERSION[..]).unwrap();
        p.write_u64::<LE>(protocol_id).unwrap();
        p.write_u64::<LE>(expire_timestamp).unwrap();

        decrypt_bignonce(
            &mut buffer[..Self::BYTES],
            &additional[..],
            nonce,
            key,
        ).map_err(map_err)
    }
}

/// Generate a connect token.
pub fn generate_connect_token(
    public_data: [u8; TOKEN_DATA],
    private_data: [u8; TOKEN_DATA],
    expire: u32, // in seconds
    timeout: u32, // in seconds
    client_id: u64,
    protocol_id: u64,
    private_key: &Key,
)
-> io::Result<[u8; 2048]>
{
    let nonce = generate_nonce();

    let create = time();
    let expire = create + expire as u64;

    let private = Private::generate(client_id, timeout, private_data);
    // write it to a buffer and encrypt the buffer
    let private_data = private.write_encrypted(protocol_id, expire, &nonce, private_key);

    // wrap a connect token around the private connect token data
    let tok = Public {
        version: VERSION,
        protocol_id,
        create,
        expire,
        nonce,
        timeout,
        data: public_data,
        token: private_data,
        client_key: private.client_key,
        server_key: private.server_key,
    };
    let mut buf = [0u8; Public::BYTES];
    tok.write(&mut buf[..])?;
    Ok(buf)
}
