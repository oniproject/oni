#![feature(
    int_to_from_bytes,
    wrapping_int_impl,
    decl_macro,
)]

pub mod chacha20;
pub mod poly1305;
pub mod verify;

pub mod aead;

mod ncha;

mod hchacha20;

//pub mod subtle;

#[inline(always)]
fn memzero<T: Sized>(p: &mut T) {
    let p: *mut T = p;
    let p: *mut u8 = p as *mut u8;
    for i in 0..std::mem::size_of::<T>() {
        unsafe { p.add(i).write_volatile(0) }
    }
}

#[inline(always)]
fn memzero_slice(p: &mut [u8]) {
    let count = p.len();
    for i in 0..p.len() {
        unsafe { p.as_mut_ptr().add(i).write_volatile(0) }
    }
}

pub fn crypto_random(buf: &mut [u8]) {
    use rand::Rng;
    rand::thread_rng().fill(buf)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
