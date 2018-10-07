// FIXME: message length

use byteorder::{LE, WriteBytesExt, ReadBytesExt};
use std::io::{self, Read, Write};
use crate::{
    utils::time,
    sodium::{
        seal, open,
        x_seal, x_open,

        randbuf, generate_nonce,
        KEYBYTES, NPUBBYTES, ABYTES},
    VERSION_BYTES,
    VERSION,
};

pub use crate::sodium::keygen;

pub const TOKEN_DATA: usize = 640;

#[inline]
pub fn map_err(_err: ()) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "chacha20poly1305")
}

pub const MAC_BYTES: usize = ABYTES;

pub type Key = [u8; KEYBYTES];

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
    // 8 + 256 + [20] + 16 = 300
    pub const BYTES: usize = 300;

    pub fn read(mut b: [u8; Self::BYTES], seq: u64, key: &Key)
        -> io::Result<Self>
    {
        open(&mut b[..Self::BYTES], None, &new_nonce(seq), key)
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

        seal(&mut b[..Self::BYTES - MAC_BYTES], None, &new_nonce(seq), key)
            .map_err(map_err)?;

        Ok(b)
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

    pub fn read(buffer: &[u8; Self::BYTES]) -> io::Result<Self> {
        let mut buffer = &buffer[..];
        Ok(Self {
            client_id: buffer.read_u64::<LE>()?,
            timeout: buffer.read_u32::<LE>()?,
            client_key: read_array!(buffer, KEYBYTES),
            server_key: read_array!(buffer, KEYBYTES),
            data: read_array!(buffer, TOKEN_DATA),
            seed: read_array!(buffer, 256),
        })
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<()> {
        buffer.write_u64::<LE>(self.client_id)?;
        buffer.write_u32::<LE>(self.timeout)?;
        buffer.write_all(&self.client_key[..])?;
        buffer.write_all(&self.server_key[..])?;
        buffer.write_all(&self.data[..])?;
        buffer.write_all(&self.seed[..])
    }

    pub fn write_sealed(&self, protocol_id: u64, expire: u64, nonce: &[u8; 24], key: &Key) -> [u8; Self::BYTES] {
        let mut buf = [0u8; Self::BYTES];
        self.write(&mut buf[..]).unwrap();
        Self::seal(&mut buf[..], protocol_id, expire, nonce, key).unwrap();
        buf
    }

    pub fn seal(
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

        x_seal(
            &mut buffer[..Self::BYTES - MAC_BYTES],
            Some(&additional[..]),
            nonce,
            key,
        ).map_err(map_err)
    }

    pub fn open(
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

        x_open(
            &mut buffer[..Self::BYTES],
            Some(&additional[..]),
            nonce,
            key,
        ).map_err(map_err)
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

    /// Generate a connect token.
    pub fn generate(
        data: [u8; TOKEN_DATA],
        expire: u32, // in seconds
        timeout: u32, // in seconds
        client_id: u64,
        protocol_id: u64,
        private_key: &Key,
    ) -> Self {
        let nonce = generate_nonce();

        let create = time();
        let expire = create + expire as u64;

        let private = Private::generate(client_id, timeout, data);
        let private_data = private.write_sealed(protocol_id, expire, &nonce, private_key);

        Self {
            version: VERSION,
            protocol_id,
            create, expire,
            nonce,
            timeout,
            data,
            token: private_data,
            client_key: private.client_key,
            server_key: private.server_key,
        }
    }

    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<usize> {
        let start_len = buffer.len();

        buffer.write_all(&self.version[..])?;
        buffer.write_u64::<LE>(self.protocol_id)?;
        buffer.write_u64::<LE>(self.create)?;
        buffer.write_u64::<LE>(self.expire)?;
        buffer.write_all(&self.nonce[..])?;
        buffer.write_all(&self.token[..])?;
        buffer.write_u32::<LE>(self.timeout)?;
        buffer.write_all(&self.client_key[..])?;
        buffer.write_all(&self.server_key[..])?;

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
        let client_key = read_array_ok!(buffer, KEYBYTES);
        let server_key = read_array_ok!(buffer, KEYBYTES);;
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

/// Generate a connect token.
pub fn generate_connect_token(
    data: [u8; TOKEN_DATA],
    // in seconds
    expire: u32, timeout: u32,
    client_id: u64, protocol_id: u64,
    private_key: &Key,
)
    -> io::Result<[u8; Public::BYTES]>
{
    let mut buf = [0u8; Public::BYTES];
    Public::generate(
        data,
        expire, timeout,
        client_id, protocol_id,
        private_key,
    ).write(&mut buf[..])?;
    Ok(buf)
}
