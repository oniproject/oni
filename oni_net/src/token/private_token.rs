use std::{slice::{from_raw_parts, from_raw_parts_mut}, mem::size_of};
use crate::protocol::{VERSION, VERSION_LEN, KEY, HMAC, XNONCE};
use crate::utils::{
    keygen,
    seal_xchacha20poly1305,
    open_xchacha20poly1305,
};
use super::{USER, DATA, PRIVATE_LEN};

#[repr(C)]
#[derive(Clone)]
pub struct PrivateToken {
    client_id: [u8; 8],
    timeout: [u8; 4],
    _reserved: [u8; 36],

    client_key: [u8; KEY],
    server_key: [u8; KEY],
    data: [u8; 640],
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
            _reserved: [0u8; 36],
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

#[test]
fn private_token() {
    let k = keygen();
    let n = crate::utils::generate_nonce();
    let protocol = 0x12346789_12346789;
    let expire = 672345;

    let client_id = 0x1122334455667788;
    let timeout = 0x55667788;

    let mut data = [0u8; DATA];
    let mut user = [0u8; USER];
    crate::utils::crypto_random(&mut data[..]);
    crate::utils::crypto_random(&mut user[..]);

    let token = &mut PrivateToken::generate(client_id, timeout, data, user);
    let client_key = token.client_key;
    let server_key = token.server_key;

    let token = PrivateToken::seal(token, protocol, expire, &n, &k);
    let token = PrivateToken::open(token, protocol, expire, &n, &k).unwrap();

    assert_eq!(token.client_id(), client_id);
    assert_eq!(token.timeout(), timeout);

    assert_eq!(&token.client_key, &client_key);
    assert_eq!(&token.server_key, &server_key);
    assert_eq!(&token.data[..], &data[..]);
    assert_eq!(&token.user[..], &user[..]);
    assert_eq!(&token._reserved[..], &[0u8; 36][..]);
}
