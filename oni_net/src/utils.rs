#[macro_export]
macro_rules! read_array {
    ($buffer:expr, $size:expr) => {
        {
            use ::std::io::Read;
            let mut array: [u8; $size] = unsafe { ::std::mem::uninitialized() };
            $buffer.read_exact(&mut array[..]).ok()?;
            array
        }
    }
}

#[macro_export]
macro_rules! make_rw {
    (struct $base:ident; const $c_size:ident = $size:expr; trait $r_type:ident { $r_fn:ident } trait $w_type:ident { $w_fn:ident }) => {
        pub struct $base([u8; $c_size]);

        impl From<[u8; $c_size]> for $base {
            fn from(v: [u8; $c_size]) -> Self {
                $base(v)
            }
        }

        impl ::std::fmt::Debug for $base {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(f, "{}({:?})", stringify!($base), &self.0[..])
            }
        }

        impl Eq for $base {}
        impl PartialEq for $base {
            fn eq(&self, other: &Self) -> bool {
                &self.0[..] == &other.0[..]
            }
        }

        impl Clone for $base {
            fn clone(&self) -> Self {
                $base(self.0.clone())
            }
        }

        impl $base {
            pub fn as_slice(&self) -> &[u8] {
                &self.0[..]
            }
            pub fn as_mut_slice(&mut self) -> &mut [u8] {
                &mut self.0[..]
            }

            pub fn as_ptr(&self) -> *const u8 {
                &self.0 as *const _
            }
        }

        pub const $c_size: usize = $size;
        pub trait $r_type: ::std::io::Read {
            fn $r_fn(&mut self) -> ::std::io::Result<$base> {
                let mut data = [0u8; $c_size];
                self.read_exact(&mut data[..])?;
                Ok($base(data))
            }
        }
        pub trait $w_type: ::std::io::Write {
            fn $w_fn(&mut self, d: &$base) -> ::std::io::Result<()> {
                self.write_all(&d.0[..])?;
                Ok(())
            }
        }
        impl<T: ::std::io::Read> $r_type for T {}
        impl<T: ::std::io::Write> $w_type for T {}
    }
}

make_rw!(
    struct UserData;
    const USER_DATA_BYTES = 256;
    trait ReadUserData { read_user_data }
    trait WriteUserData { write_user_data }
);

impl UserData {
    pub fn random() -> Self {
        let mut data = [0u8; USER_DATA_BYTES];
        ::crypto::random_bytes(&mut data[..]);
        UserData(data)
    }
}

impl Default for UserData {
    fn default() -> Self {
        UserData([0u8; USER_DATA_BYTES])
    }
}

pub fn time() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
