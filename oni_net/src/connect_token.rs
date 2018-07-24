
struct ConnectToken {
    version_info: [u8; VERSION_INFO_BYTES],
    protocol_id: u64,
    create_timestamp: u64,
    expire_timestamp: u64,
    sequence: u64,
    private_data: [u8; CONNECT_TOKEN_PRIVATE_BYTES];
    timeout_seconds: u32,
    server_addresses: Vec<SocketAddr>,
    client_to_server_key: Key,
    server_to_client_key: Key,
}

impl ConnectToken {
    pub fn write(&self, mut buffer: &mut [u8]) -> io::Result<usize> {
        let start_len = buffer.len();

        buffer.write_all(&self.version_info[..]);
        buffer.write_u64(self.protocol_id)?;
        buffer.write_u64(self.create_timestamp)?;
        buffer.write_u64(self.expire_timestamp)?;
        buffer.write_u64(self.sequence)?;
        buffer.write_all(&self.private_data[..])?;
        buffer.write_u32(self.timeout_seconds)?;
        buffer.write_ips(&self.server_addresses)?;

        buffer.write_key(&self.client_to_server_key)?;
        buffer.write_key(&self.server_to_client_key)?;

        let count = CONNECT_TOKEN_BYTES - (start.len() - buffer.len());
        for _ in 0..count {
            buffer.write_u8(0)?;
        }
        Ok(CONNECT_TOKEN_BYTES)
    }

    pub fn read(mut buffer: &mut [u8]) -> io::Result<Self> {
        if buffer.len() != CONNECT_TOKEN_BYTES {
            // TODO
            error!("read connect data has bad buffer length ({})", buffer.len());
            return NETCODE_ERROR;
        }

        let version_info = [0u8; VERSION_INFO_BYTES];
        buffer.read_expect(&version_info[..])?;
        if version_info != VERSION_INFO {
            error!("read connect data has bad version info (got {:?}, expected {:?})", &version_info[..], &VERSION_INFO[..]);
            return NETCODE_ERROR;
        }

        let protocol_id = buffer.read_u64()?;
        let create_timestamp = buffer.read_u64()?;
        let expire_timestamp = buffer.read_u64()?;

        if create_timestamp > expire_timestamp {
            return NETCODE_ERROR;
        }

        let sequence = read_uint64( &buffer );
        read_bytes( &buffer, connect_token.private_data, CONNECT_TOKEN_PRIVATE_BYTES );
        let timeout_seconds = buffer.read_u32()?;

        let server_addresses = buffer.read_ips()?;

        let client_to_server_key = buffer.read_key()?;
        let server_to_client_key = buffer.read_key()?;

        return NETCODE_OK;
    }
}
