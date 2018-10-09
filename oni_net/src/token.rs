mod public_token;
mod private_token;
mod challenge_token;

/*
pub use crate::crypto::{
    Private, Public, Challenge,
    TOKEN_DATA,
    generate_connect_token,
    keygen,
};
*/

pub const DATA: usize = 640;
pub const USER: usize = 256;

pub const CHALLENGE_LEN: usize = 300;
pub const PRIVATE_LEN: usize = 1024;
pub const PUBLIC_LEN: usize = 2048;

pub use self::public_token::PublicToken;
pub use self::private_token::PrivateToken;
pub use self::challenge_token::ChallengeToken;
