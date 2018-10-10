//! Public token:
//!
//! ```txt
//! [version]
//! [protocol id] u64
//! [create timestamp] u64
//! [expire timestamp] u64
//! [timeout in seconds] u32
//! [reserved bytes] (268 - VERSION_LEN)
//! [nonce] (24 bytes)
//! [client to server key] (32 bytes)
//! [server to client key] (32 bytes)
//! [encrypted private token] (1024 bytes)
//! [open data] (640 bytes)
//! ```
//!

mod public_token;
mod private_token;
mod challenge_token;

pub const DATA: usize = 640;
pub const USER: usize = 256;

pub const CHALLENGE_LEN: usize = 300;
pub const PRIVATE_LEN: usize = 1024;
pub const PUBLIC_LEN: usize = 2048;

pub use self::public_token::PublicToken;
pub use self::private_token::PrivateToken;
pub use self::challenge_token::ChallengeToken;
