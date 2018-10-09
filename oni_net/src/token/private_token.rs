use std::os::raw::c_ulonglong;
use std::mem::{transmute, size_of, uninitialized};

use crate::utils::keygen;

use crate::server::{
    KEY,
    HMAC,
    XNONCE,
    VERSION,
    VERSION_LEN,
};

use super::{USER, DATA, PRIVATE_LEN};

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

#[repr(packed)]
struct PrivateAd {
    _version: [u8; VERSION_LEN],
    _protocol: [u8; 8],
    _expire: [u8; 8],
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

    pub fn encrypt(self, protocol: u64, expire: u64, n: &[u8; XNONCE], k: &[u8; KEY]) -> Result<[u8; PRIVATE_LEN], ()> {
        let ad = PrivateAd {
            _version: VERSION,
            _protocol: protocol.to_le_bytes(),
            _expire: expire.to_le_bytes(),
        };

        let ad_p = (&ad as *const PrivateAd) as *const _;
        let ad_len = size_of::<PrivateAd>() as c_ulonglong;

        let m: [u8; PRIVATE_LEN] = unsafe { transmute(self) };

        let mut c: [u8; PRIVATE_LEN] = unsafe { uninitialized() };
        let mut clen = c.len() as c_ulonglong;

        let ret = unsafe {
            crate::sodium::crypto_aead_xchacha20poly1305_ietf_encrypt(
                c.as_mut_ptr(), &mut clen,
                m.as_ptr(), (m.len() - HMAC) as c_ulonglong,
                ad_p, ad_len,
                0 as *mut _,
                n.as_ptr(), k.as_ptr(),
            )
        };

        if ret != 0 || clen != PRIVATE_LEN as c_ulonglong {
            Err(())
        } else {
            Ok(c)
        }
    }

    pub fn decrypt(c: &[u8; PRIVATE_LEN], protocol: u64, expire: u64, n: &[u8; XNONCE], k: &[u8; KEY]) -> Result<Self, ()> {
        let ad = PrivateAd {
            _version: VERSION,
            _protocol: protocol.to_le_bytes(),
            _expire: expire.to_le_bytes(),
        };

        let ad_p = (&ad as *const PrivateAd) as *const _;
        let ad_len = size_of::<PrivateAd>() as c_ulonglong;

        let mut m: [u8; PRIVATE_LEN] = unsafe { uninitialized() };
        let mut mlen = (m.len() - HMAC) as c_ulonglong;

        // copy hmac
        (&mut m[PRIVATE_LEN - HMAC..]).copy_from_slice(&c[PRIVATE_LEN - HMAC..]);

        let ret = unsafe {
            crate::sodium::crypto_aead_xchacha20poly1305_ietf_decrypt(
                m.as_mut_ptr(), &mut mlen,
                0 as *mut _,
                c.as_ptr(), c.len() as c_ulonglong,
                ad_p, ad_len,
                n.as_ptr(), k.as_ptr(),
            )
        };

        if ret != 0 || mlen != (PRIVATE_LEN - HMAC) as c_ulonglong {
            Err(())
        } else {
            Ok(unsafe { transmute(m) })
        }
    }
}

#[test]
fn private_token() {
    assert_eq!(size_of::<PrivateToken>(), PRIVATE_LEN);

    let k = keygen();
    let n = crate::utils::generate_nonce();
    let protocol = 0x12346789_12346789;
    let expire = 672345;

    let client_id = 0x1122334455667788;
    let timeout = 0x55667788;

    let mut data = [0u8; DATA];
    let mut user = [0u8; USER];
    crate::sodium::crypto_random(&mut data[..]);
    crate::sodium::crypto_random(&mut user[..]);

    let token = PrivateToken::generate(client_id, timeout, data, user);
    let client_key = token.client_key;
    let server_key = token.server_key;

    let token = PrivateToken::encrypt( token, protocol, expire, &n, &k).unwrap();
    let token = PrivateToken::decrypt(&token, protocol, expire, &n, &k).unwrap();

    assert_eq!(token.client_id(), client_id);
    assert_eq!(token.timeout(), timeout);

    assert_eq!(&token.client_key, &client_key);
    assert_eq!(&token.server_key, &server_key);
    assert_eq!(&token.data[..], &data[..]);
    assert_eq!(&token.user[..], &user[..]);
    assert_eq!(&token._reserved[..], &[0u8; 36][..]);
}