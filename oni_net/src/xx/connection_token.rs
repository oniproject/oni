pub const CONNECT_TOKEN_BYTES: usize = 2048;
// NETCODE_KEY_BYTES );

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use std::{
    io::{self, prelude::*, Error, ErrorKind},
    net::{
        Ipv4Addr,
        Ipv6Addr,
        SocketAddr,
        SocketAddrV4,
        SocketAddrV6,
    },
};

pub struct Token {
    /// globally unique identifier for an authenticated client
    pub client_id: u64,
    /// timeout in seconds.
    /// negative values disable timeout (dev only)
    pub timeout_seconds: u32,

    pub ips: Vec<SocketAddr>,
    pub client_to_server_key: [u8; 32],
    pub server_to_client_key: [u8; 32],
    /// user defined data specific to this protocol id
    pub user_data: [u8; 256],
    //<zero pad to 1024 bytes>
}

impl Token {
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let client_id = r.read_u64::<LE>()?;
        let timeout_seconds = r.read_u32::<LE>()?;

        let mut client_to_server_key = [0u8; 32];
        r.read_exact(&mut client_to_server_key[..])?;
        let mut server_to_client_key = [0u8; 32];
        r.read_exact(&mut server_to_client_key[..])?;

        let ips = read_ips(r)?;

        let mut user_data = [0u8; 256];
        r.read_exact(&mut user_data[..])?;

        Ok(Self {
            client_id,
            timeout_seconds,
            ips,
            client_to_server_key,
            server_to_client_key,
            user_data,
        })
    }

    pub fn write_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_u64::<LE>(self.client_id)?;
        w.write_u32::<LE>(self.timeout_seconds)?;

        write_ips(w, &self.ips)?;

        w.write_all(&self.client_to_server_key[..])?;
        w.write_all(&self.server_to_client_key[..])?;
        w.write_all(&self.user_data[..])?;

        Ok(())
    }
}

int generate_connect_token(
    public_server_addresses: Vec<addr>,
    internal_server_addresses: Vec<addr>,
    expire_seconds: int,
    timeout_seconds: int,
    client_id: u64,
    protocol_id: u64,
    sequence: u64,
    private_key: &[u8],
    output_buffer: &[u8]
    )
{
    assert!(public_server_addresses.len() > 0);
    assert!(public_server_addresses.len() <= MAX_SERVERS_PER_CONNECT);
    assert!(public_server_addresses.len() == internal_server_addresses.len());
    assert!(private_key);
    assert!(output_buffer);

    // generate a connect token

    let mut user_data: [u8; USER_DATA_BYTES];
    random_bytes(user_data, USER_DATA_BYTES);

    let connect_token_private = ConnectTokenPrivate::generate(
        client_id,
        timeout_seconds,
        num_server_addresses,
        parsed_internal_server_addresses,
        user_data);

    // write it to a buffer

    let private_data: [NETCODE_CONNECT_TOKEN_PRIVATE_BYTES];
    private_data.write(connect_token_private, CONNECT_TOKEN_PRIVATE_BYTES);

    // encrypt the buffer

    let create_timestamp: u64 = time(NULL);
    let expire_timestamp: u64 = if expire_seconds >= 0 {
        create_timestamp + expire_seconds
    } else {
        0xFFFF_FFFF_FFFF_FFFF
    };

    encrypt_connect_token_private(
            private_data, CONNECT_TOKEN_PRIVATE_BYTES,
            VERSION_INFO, protocol_id,
            expire_timestamp,
            sequence, private_key)?;

    // wrap a connect token around the private connect token data
    let connect_token = ConnectToken {
        version_info: VERSION_INFO,
        protocol_id,
        create_timestamp,
        expire_timestamp,
        sequence,
        private_data: connect_token_data,

        connect_token.num_server_addresses = num_server_addresses;
        for ( i = 0; i < num_server_addresses; ++i )
            connect_token.server_addresses[i] = parsed_public_server_addresses[i];

        client_to_server_key: connect_token_private.client_to_server_key,
        server_to_client_key: connect_token_private.server_to_client_key,

        timeout_seconds,
    }

    // write the connect token to the output buffer

    netcode_write_connect_token( &connect_token, output_buffer, NETCODE_CONNECT_TOKEN_BYTES );

    return NETCODE_OK;
}
