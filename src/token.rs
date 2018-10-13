use byteorder::{LE, ByteOrder};
use std::{
    slice::{from_raw_parts, from_raw_parts_mut},
    mem::size_of,
};
use crate::{
    protocol::{VERSION, VERSION_LEN},
    crypto::{
        keygen, generate_nonce,
        nonce_from_u64,
        seal_chacha20poly1305,
        open_chacha20poly1305,
        seal_xchacha20poly1305,
        open_xchacha20poly1305,
        KEY, HMAC, XNONCE,
    },
    unix_time,
};

pub const DATA: usize = 624;
pub const USER: usize = 256;

pub const CHALLENGE_LEN: usize = 300;
pub const PRIVATE_LEN: usize = 1024;
pub const PUBLIC_LEN: usize = 2048;

const CHALLENGE_RESERVED: usize = 20;
const PRIVATE_RESERVED: usize = 52;
const PUBLIC_RESERVED: usize = 268 - VERSION_LEN;

#[repr(C)]
#[derive(Clone)]
pub struct ChallengeToken {
    client_id: [u8; 8],
    _reserved: [u8; CHALLENGE_RESERVED],
    user: [u8; USER],
    hmac: [u8; HMAC],
}

impl ChallengeToken {
    pub fn new(client_id: u64, user: [u8; USER]) -> Self {
        Self {
            client_id: client_id.to_le_bytes(),
            user,
            _reserved: [0u8; CHALLENGE_RESERVED],
            hmac: [0u8; HMAC],
        }
    }

    pub fn client_id(&self) -> u64 {
        u64::from_le_bytes(self.client_id)
    }
    pub fn user(&self) -> &[u8; USER] { &self.user }

    pub fn encode_packet(mut self, seq: u64, k: &[u8; KEY]) -> [u8; 8+CHALLENGE_LEN] {
        let mut buffer = [0u8; 8+CHALLENGE_LEN];
        buffer[..8].copy_from_slice(&seq.to_le_bytes()[..]);
        buffer[8..].copy_from_slice(self.seal(seq, k));
        buffer
    }

    pub fn decode_packet<'a>(buf: &'a mut [u8; 8 + CHALLENGE_LEN], k: &[u8; KEY]) -> Result<&'a Self, ()> {
        let (seq, buf) = buf.split_at_mut(8);
        let seq = LE::read_u64(seq);
        let token = unsafe { &mut *(buf.as_mut_ptr() as *mut [u8; CHALLENGE_LEN]) };
        ChallengeToken::open(token, seq, k)
    }

    pub fn seal<'a>(&'a mut self, seq: u64, k: &[u8; KEY]) -> &'a mut [u8; CHALLENGE_LEN] {
        assert_eq!(size_of::<Self>(), CHALLENGE_LEN);
        let p: *mut Self = self;
        let m = unsafe { from_raw_parts_mut(p as *mut u8, CHALLENGE_LEN-HMAC) };
        self.hmac = seal_chacha20poly1305(m, None, &nonce_from_u64(seq), k);
        unsafe { &mut *(p as *mut [u8; CHALLENGE_LEN]) }
    }

    pub fn open<'a>(buf: &'a mut [u8; CHALLENGE_LEN], seq: u64, k: &[u8; KEY]) -> Result<&'a Self, ()> {
        assert_eq!(size_of::<Self>(), CHALLENGE_LEN);
        let (c, t) = &mut buf[..].split_at_mut(CHALLENGE_LEN-HMAC);
        let t = unsafe { &*(t.as_ptr() as *const [u8; HMAC]) };
        open_chacha20poly1305(c, None, t, &nonce_from_u64(seq), k)?;
        let buf: *mut [u8; CHALLENGE_LEN] = buf;
        Ok(unsafe { &*(buf as *const Self) })
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct PrivateToken {
    client_id: [u8; 8],
    timeout: [u8; 4],
    _reserved: [u8; PRIVATE_RESERVED],

    client_key: [u8; KEY],
    server_key: [u8; KEY],
    data: [u8; DATA],
    user: [u8; USER],
    hmac: [u8; HMAC],
}

#[repr(C)]
struct PrivateAd {
    _version: [u8; VERSION_LEN],
    _protocol: [u8; 8],
    _expire: [u8; 8],
}

impl PrivateAd {
    fn new(protocol: u64, expire: u64) -> Self {
        Self {
            _version: VERSION,
            _protocol: protocol.to_le_bytes(),
            _expire: expire.to_le_bytes(),
        }
    }
    fn as_slice(&self) -> &[u8] {
        let p: *const Self = self;
        unsafe { from_raw_parts(p as *const u8, size_of::<Self>()) }
    }
}

impl PrivateToken {
    pub fn generate(client_id: u64, timeout: u32, data: [u8; DATA], user: [u8; USER]) -> Self {
        Self {
            client_id: u64::to_le_bytes(client_id),
            timeout: u32::to_le_bytes(timeout),
            client_key: keygen(),
            server_key: keygen(),
            data,
            user,
            _reserved: [0u8; PRIVATE_RESERVED],
            hmac: [0u8; HMAC],
        }
    }

    pub fn hmac(&self) -> &[u8; HMAC] { &self.hmac }

    pub fn client_id(&self) -> u64 {
        u64::from_le_bytes(self.client_id)
    }
    pub fn timeout(&self) -> u32 {
        u32::from_le_bytes(self.timeout)
    }

    pub fn client_key(&self) -> &[u8; KEY] { &self.client_key }
    pub fn server_key(&self) -> &[u8; KEY] { &self.server_key }

    pub fn data(&self) -> &[u8; DATA] { &self.data }
    pub fn user(&self) -> &[u8; USER] { &self.user }

    pub fn seal<'a>(&'a mut self, protocol: u64, expire: u64, n: &[u8; XNONCE], k: &[u8; KEY]) -> &'a mut [u8; PRIVATE_LEN] {
        assert_eq!(size_of::<Self>(), PRIVATE_LEN);
        let ad = PrivateAd::new(protocol, expire);
        let p: *mut Self = self;
        let m = unsafe { from_raw_parts_mut(p as *mut u8, PRIVATE_LEN-HMAC) };
        self.hmac = seal_xchacha20poly1305(m, Some(ad.as_slice()), n, k);
        unsafe { &mut *(p as *mut [u8; PRIVATE_LEN]) }
    }

    pub fn open<'a>(buf: &'a mut [u8; PRIVATE_LEN], protocol: u64, expire: u64, n: &[u8; XNONCE], k: &[u8; KEY]) -> Result<&'a Self, ()> {
        assert_eq!(size_of::<Self>(), PRIVATE_LEN);
        let ad = PrivateAd::new(protocol, expire);
        let (c, t) = &mut buf[..].split_at_mut(PRIVATE_LEN-HMAC);
        let t = unsafe { &*(t.as_ptr() as *const [u8; HMAC]) };
        open_xchacha20poly1305(c, Some(ad.as_slice()), t, n, k)?;
        let buf: *mut [u8; PRIVATE_LEN] = buf;
        Ok(unsafe { &*(buf as *const Self) })
    }
}

/// Format:
///
/// ```txt
/// [version]
/// [protocol id] u64
/// [create timestamp] u64
/// [expire timestamp] u64
/// [timeout in seconds] u32
/// [reserved bytes] (268 - VERSION_LEN)
/// [nonce] (24 bytes)
/// [client to server key] (32 bytes)
/// [server to client key] (32 bytes)
/// [encrypted private token] (1024 bytes)
/// [open data] (640 bytes)
/// ```
#[repr(C)]
#[derive(Clone)]
pub struct PublicToken {
    version: [u8; VERSION_LEN],
    protocol: [u8; 8],
    create: [u8; 8],
    expire: [u8; 8],
    timeout: [u8; 4],
    _reserved: [u8; PUBLIC_RESERVED],

    nonce: [u8; XNONCE],
    client_key: [u8; KEY],
    server_key: [u8; KEY],
    token: [u8; PRIVATE_LEN],
    data: [u8; DATA],
}

impl PublicToken {
    pub fn protocol_id(&self) -> u64 { u64::from_le_bytes(self.protocol) }
    pub fn create_timestamp(&self) -> u64 { u64::from_le_bytes(self.create) }
    pub fn expire_timestamp(&self) -> u64 { u64::from_le_bytes(self.expire) }
    pub fn timeout_seconds(&self) -> u32 { u32::from_le_bytes(self.timeout) }
    pub fn nonce(&self) -> [u8; XNONCE] { self.nonce }

    pub fn client_key(&self) -> [u8; KEY] { self.client_key }
    pub fn server_key(&self) -> [u8; KEY] { self.server_key }

    pub fn token(&self) -> &[u8; PRIVATE_LEN] { &self.token }
    pub fn data(&self) -> &[u8; DATA] { &self.data }

    pub fn check_version(&self) -> bool {
        self.version == VERSION
    }

    pub fn generate(
        data: [u8; DATA],
        user: [u8; USER],
        expire: u32, // in seconds
        timeout: u32, // in seconds
        client_id: u64,
        protocol: u64,
        private_key: &[u8; KEY],
    ) -> Self {
        let nonce = generate_nonce();

        let create = unix_time();
        let expire = create + u64::from(expire);

        let mut token = PrivateToken::generate(client_id, timeout, data, user);
        let client_key = *token.client_key();
        let server_key = *token.server_key();

        let token = PrivateToken::seal(&mut token, protocol, expire, &nonce, private_key).clone();

        Self {
            version: VERSION,
            protocol: protocol.to_le_bytes(),
            create: create.to_le_bytes(),
            expire: expire.to_le_bytes(),
            timeout: timeout.to_le_bytes(),
            _reserved: [0u8; PUBLIC_RESERVED],

            nonce,
            client_key,
            server_key,
            token,
            data,
        }
    }
}

#[test]
fn challenge_token() {
    use crate::crypto::crypto_random;

    let client_id = 0x1122334455667788;
    let seq = 0x1122334455667799;
    let key = keygen();
    let mut user = [0u8; USER];
    crypto_random(&mut user[..]);

    let tok = &mut ChallengeToken::new(client_id, user);
    let tok = ChallengeToken::seal(tok, seq, &key);
    let tok = ChallengeToken::open(tok, seq, &key).unwrap();

    assert_eq!(tok.client_id(), client_id);
    assert_eq!(&tok.user()[..], &user[..]);
    assert_eq!(tok._reserved, [0u8; CHALLENGE_RESERVED]);
}

#[test]
fn private_token() {
    use crate::crypto::crypto_random;

    let k = keygen();
    let n = generate_nonce();
    let protocol = 0x12346789_12346789;
    let expire = 672345;

    let client_id = 0x1122334455667788;
    let timeout = 0x55667788;

    let mut data = [0u8; DATA];
    let mut user = [0u8; USER];
    crypto_random(&mut data[..]);
    crypto_random(&mut user[..]);

    let tok = &mut PrivateToken::generate(client_id, timeout, data, user);
    let client_key = tok.client_key;
    let server_key = tok.server_key;

    let tok = PrivateToken::seal(tok, protocol, expire, &n, &k);
    let tok = PrivateToken::open(tok, protocol, expire, &n, &k).unwrap();

    assert_eq!(tok.client_id(), client_id);
    assert_eq!(tok.timeout(), timeout);

    assert_eq!(&tok.client_key, &client_key);
    assert_eq!(&tok.server_key, &server_key);
    assert_eq!(&tok.data[..], &data[..]);
    assert_eq!(&tok.user[..], &user[..]);
    assert_eq!(&tok._reserved[..], &[0u8; PRIVATE_RESERVED][..]);
}
