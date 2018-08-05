mod seq;
mod reliable;
mod sequenced;

pub use self::seq::Seq;
pub use self::reliable::Reliable;
pub use self::sequenced::Sequenced;

#[derive(Debug, PartialEq)]
pub enum Error {
    TooLarge,
    TooSmall,
    InvalidHeader,
    Stale { seq: Seq },
}
