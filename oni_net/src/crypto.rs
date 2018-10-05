// FIXME: message length

use byteorder::{LE, WriteBytesExt, ReadBytesExt};
use std::{
    net::SocketAddr,
    io::{self, Read, Write},
};
use generic_array::{GenericArray, ArrayLength};
use crate::{
    utils::{time, UserData, ReadUserData, WriteUserData, USER_DATA_BYTES},
    addr::{ReadIps, WriteIps, MAX_SERVERS_PER_CONNECT},
    VERSION_BYTES,
    VERSION,
};

pub mod chacha20poly1305 {
    use std::{
        ptr,
        os::raw::{
            c_uchar,
            c_ulonglong,
            c_int,
            c_void,
        },
    };

    pub const KEYBYTES: usize = 32;
    pub const NPUBBYTES: usize = 12;
    pub const ABYTES: usize = 16;

    #[link(name = "sodium")]
    extern "C" {
        fn crypto_aead_chacha20poly1305_decrypt(
            m: *mut c_uchar,
            mlen_p: *mut c_ulonglong,
            nsec: *mut c_uchar,
            c: *const c_uchar,
            clen: c_ulonglong,
            ad: *const c_uchar,
            adlen: c_ulonglong,
            npub: *const c_uchar,
            k: *const c_uchar,
        ) -> c_int;
        fn crypto_aead_chacha20poly1305_encrypt(
            c: *mut c_uchar,
            clen_p: *mut c_ulonglong,
            m: *const c_uchar,
            mlen: c_ulonglong,
            ad: *const c_uchar,
            adlen: c_ulonglong,
            nsec: *const c_uchar,
            npub: *const c_uchar,
            k: *const c_uchar,
        ) -> c_int;

        fn crypto_aead_chacha20poly1305_keygen(k: *mut c_uchar);
    }

    #[inline]
    pub fn keygen() -> [u8; KEYBYTES] {
        let mut k = [0u8; KEYBYTES];
        unsafe {
            crypto_aead_chacha20poly1305_keygen(k.as_mut_ptr());
        }
        k
    }

    #[inline]
    pub fn encrypt(m: &mut [u8], add: &[u8], nonce: &[u8; NPUBBYTES], key: &[u8; KEYBYTES]) -> Result<(), ()> {
        let mut len = 0;
        if 0 == unsafe {
            crypto_aead_chacha20poly1305_encrypt(
                m.as_mut_ptr(), &mut len,
                m.as_mut_ptr(), m.len() as c_ulonglong,
                add.as_ptr(), add.len() as c_ulonglong,
                ptr::null(), nonce.as_ptr(), key.as_ptr())
        } {
            assert_eq!(len as usize, m.len() + ABYTES);
            Ok(())
        } else {
            Err(())
        }
    }

    #[inline]
    pub fn decrypt(m: &mut [u8], add: &[u8], nonce: &[u8; NPUBBYTES], key: &[u8; KEYBYTES]) -> Result<(), ()> {
        let mut len = 0;
        if 0 == unsafe {
            crypto_aead_chacha20poly1305_decrypt(
                m.as_mut_ptr(), &mut len,
                ptr::null_mut(),
                m.as_mut_ptr(), m.len() as c_ulonglong,
                add.as_ptr(), add.len() as c_ulonglong,
                nonce.as_ptr(), key.as_ptr())
        } {
            assert_eq!(len as usize, m.len() - ABYTES);
            Ok(())
        } else {
            Err(())
        }
    }
}

pub use self::chacha20poly1305::{keygen, encrypt, decrypt, KEYBYTES, NPUBBYTES, ABYTES};

#[inline]
pub fn map_err(_err: ()) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "chacha20poly1305")
}

pub const MAC_BYTES: usize = ABYTES;

/*
make_rw!(
    struct Key;
    const KEY_BYTES = KEYBYTES;
    trait ReadKey { read_key }
    trait WriteKey { write_key }
);
*/

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

pub fn new_nonce(sequence: u64) -> [u8; chacha20poly1305::NPUBBYTES] {
    let mut nonce = [0u8; chacha20poly1305::NPUBBYTES];
    {
        let mut p = &mut nonce[..];
        p.write_u32::<LE>(0).unwrap();
        p.write_u64::<LE>(sequence).unwrap();
    }
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

    pub fn encrypt(buffer: &mut [u8; Self::BYTES], seq: u64, key: &Key) -> io::Result<()> {
        let m = &mut buffer[..Self::BYTES - MAC_BYTES];
        chacha20poly1305::encrypt(m, &[], &new_nonce(seq), key).map_err(map_err)
    }

    pub fn decrypt(buffer: &mut [u8; Self::BYTES], seq: u64, key: &Key) -> io::Result<()> {
        let m = &mut buffer[..Self::BYTES];
        chacha20poly1305::decrypt(m, &[], &new_nonce(seq), key).map_err(map_err)
    }
}

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
        let user_data = UserData::default();
        let private = Private::generate(
            client_id, timeout_seconds, internal_server_addresses, user_data
        );

        // write it to a buffer
        let mut private_data = [0u8; Private::BYTES];
        private.write(&mut private_data[..])?;

        // encrypt the buffer
        let create_timestamp = time();
        let expire_timestamp = create_timestamp + expire_seconds as u64;
        Private::encrypt(&mut private_data[..], protocol_id,
                         expire_timestamp, sequence, private_key)?;

        // wrap a connect token around the private connect token data
        Ok(Self {
            version: VERSION,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            private_data,
            server_addresses: public_server_addresses,
            client_to_server_key: private.client_to_server_key,
            server_to_client_key: private.server_to_client_key,
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
        let user_data = UserData::default();
        let private = Private::generate(
            client_id, timeout_seconds, internal_server_addresses, user_data
        );

        // write it to a buffer
        let mut private_data = [0u8; Private::BYTES];
        private.write(&mut private_data[..])?;

        // encrypt the buffer
        let create_timestamp = time();
        let expire_timestamp = create_timestamp + expire_seconds as u64;
        Private::encrypt(&mut private_data[..], protocol_id,
                         expire_timestamp, sequence, private_key)?;

        // wrap a connect token around the private connect token data
        let connect_token = Self {
            version: VERSION,
            protocol_id,
            create_timestamp,
            expire_timestamp,
            sequence,
            private_data,
            server_addresses: public_server_addresses,
            client_to_server_key: private.client_to_server_key,
            server_to_client_key: private.server_to_client_key,
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

    pub fn generate(
        client_id: u64,
        timeout_seconds: u32,
        addresses: Vec<SocketAddr>,
        user_data: UserData) -> Self
    {
        assert!(addresses.len() > 0);
        assert!(addresses.len() <= MAX_SERVERS_PER_CONNECT);
        Self {
            client_id,
            timeout_seconds,
            server_addresses: addresses,
            client_to_server_key: keygen(),
            server_to_client_key: keygen(),
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

        let mut additional = [0u8; VERSION_BYTES + 8 + 8];
        {
            let mut p = &mut additional[..];
            p.write_all(&VERSION[..]).unwrap();
            p.write_u64::<LE>(protocol_id).unwrap();
            p.write_u64::<LE>(expire_timestamp).unwrap();
        }

        chacha20poly1305::encrypt(
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
        {
            let mut p = &mut additional[..];
            p.write_all(&VERSION[..]).unwrap();
            p.write_u64::<LE>(protocol_id).unwrap();
            p.write_u64::<LE>(expire_timestamp).unwrap();
        }

        chacha20poly1305::decrypt(
            &mut buffer[..Self::BYTES],
            &additional[..],
            &new_nonce(sequence),
            key,
        ).map_err(map_err)
    }
}
