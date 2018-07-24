use connection_token::Token;

pub struct Matcher {
    pub ips: Vec<SocketAddr>,
}

impl Matcher {
    pub fn generate_token(&mut self) -> Token {
        Token {
            client_id: 1,
            timeout_seconds: 0xFFFF_FFFF,

            ips: self.ips.clone(),
            client_to_server_key: [0u8; 32],
            server_to_client_key: [0u8; 32],
            user_data: [0u8; 256],
        }
    }
}
