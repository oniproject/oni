use std::mem::transmute;
use crate::protocol::{VERSION, VERSION_LEN, KEY, XNONCE};
use crate::utils::{generate_nonce, time_secs};
use super::{USER, DATA, PUBLIC_LEN, PrivateToken, PRIVATE_LEN};

#[repr(C)]
pub struct PublicToken {
    version: [u8; VERSION_LEN],
    protocol: [u8; 8],
    create: [u8; 8],
    expire: [u8; 8],
    timeout: [u8; 4],
    _reserved: [u8; 268 - VERSION_LEN],

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

        let create = time_secs();
        let expire = create + expire as u64;

        let private = PrivateToken::generate(client_id, timeout, data, user);
        let client_key = *private.client_key();
        let server_key = *private.server_key();

        let token = PrivateToken::encrypt(private, protocol, expire, &nonce, private_key)
            .unwrap();

        Self {
            version: VERSION,
            protocol: protocol.to_le_bytes(),
            create: create.to_le_bytes(),
            expire: expire.to_le_bytes(),
            timeout: timeout.to_le_bytes(),
            _reserved: [0u8; 268 - VERSION_LEN],

            nonce,
            client_key,
            server_key,
            token,
            data,
        }
    }

    pub fn read(b: [u8; PUBLIC_LEN]) -> Result<Self, ()> {
        let tok: Self = unsafe { transmute(b) };
        if tok.version == VERSION {
            Ok(tok)
        } else {
            Err(())
        }
    }

    pub fn write(self) -> [u8; PUBLIC_LEN] {
        unsafe { transmute(self) }
    }
}
