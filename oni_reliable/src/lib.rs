#![feature(int_to_from_bytes)]

mod buffer;
mod sequence;
mod bitset;

pub mod sequenced;

pub use self::{
    buffer::{Buffer, Entry},
    sequence::{Sequence, SequenceOps, SequenceIO},
};
