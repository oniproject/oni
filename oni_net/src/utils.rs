#![allow(dead_code)]

#[macro_export]
macro_rules! err_ret {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(_) => return,
        }
    }
}

#[macro_export]
macro_rules! none_ret {
    ($v:expr) => {
        match $v {
            Some(v) => v,
            None => return,
        }
    }
}

#[macro_export]
macro_rules! read_array {
    ($buffer:expr, $size:expr) => {{
        use std::io::Read;
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        $buffer.read_exact(&mut array[..])?;
        array
    }}
}

#[macro_export]
macro_rules! read_array_ok {
    ($buffer:expr, $size:expr) => {{
        use std::io::Read;
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        $buffer.read_exact(&mut array[..]).ok()?;
        array
    }}
}

#[macro_export]
macro_rules! read_array_unwrap {
    ($buffer:expr, $size:expr) => {{
        use std::io::Read;
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        $buffer.read_exact(&mut array[..]).unwrap();
        array
    }}
}

pub fn time() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/*
use std::convert::AsMut;
use std::convert::{TryInto, TryFrom};

fn clone_into_array<A, T>(slice: &[T]) -> A
    where A: Default + AsMut<[T]>, T: Clone,
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

/*
fn try_into_array(barry: &[u8]) -> &[u8; 3] {
    barry.try_into().expect("slice with incorrect length")
}
*/

pub macro slice_to_array($slice:expr, $len:expr) {
    if slice.len() == $len {
        let ptr = slice.as_ptr() as *const [u8; $len];
        unsafe { Some(&*ptr) }
    } else {
        None
    }
}

#[test]
fn test_clone_into_array() {
    #[derive(Debug, PartialEq)]
    struct Example {
        a: [u8; 4],
        b: [u8; 6],
    }

    let original = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let e = Example {
        a: clone_into_array(&original[0..4]),
        b: clone_into_array(&original[4..10]),
    };

    assert_eq!(e, Example {
        a: [1, 2, 3, 4],
        b: [5, 6, 7, 8, 9, 10],
    });
}
*/
