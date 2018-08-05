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
macro_rules! array_from_slice_uninitialized {
    ($buffer:expr, $size:expr) => {
        {
            let len = $buffer.len();
            let mut array: [u8; $size] = unsafe { ::std::mem::uninitialized() };
            (&mut array[..len]).copy_from_slice(&$buffer[..len]);
            (array, len)
        }
    }
}

#[macro_export]
macro_rules! array_from_slice_zeroed {
    ($buffer:expr, $size:expr) => {
        {
            let len = $buffer.len();
            let mut array: [u8; $size] = unsafe { ::std::mem::zeroed() };
            (&mut array[..len]).copy_from_slice(&$buffer[..len]);
            (array, len)
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
        crate::crypto::random_bytes(&mut data[..]);
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

pub struct UncheckedWriter {
    p: *mut u8,
    start: *mut u8,
}
impl UncheckedWriter {
    #[inline(always)]
    pub fn new(p: *mut u8) -> Self {
        Self { p, start: p }
    }
    #[inline(always)]
    pub unsafe fn diff(self) -> usize {
        self.p.offset_from(self.start) as usize
    }
    #[inline(always)]
    pub unsafe fn write_u8(&mut self, v: u8) {
        *self.p = v;
        self.p = self.p.add(1);
    }
    #[inline(always)]
    pub unsafe fn write_u16(&mut self, v: u16) {
        (self.p as *mut u16).write(v.to_le());
        self.p = self.p.add(2);
    }
    #[inline(always)]
    pub unsafe fn write_u32(&mut self, v: u32) {
        (self.p as *mut u32).write(v.to_le());
        self.p = self.p.add(4);
    }
    #[inline(always)]
    pub unsafe fn write_u64(&mut self, v: u64) {
        (self.p as *mut u64).write(v.to_le());
        self.p = self.p.add(8);
    }
    #[inline(always)]
    pub unsafe fn write_u128(&mut self, v: u128) {
        (self.p as *mut u128).write(v.to_le());
        self.p = self.p.add(16);
    }
}

pub struct UncheckedReader {
    p: *const u8,
    start: *const u8,
}
impl UncheckedReader {
    #[inline(always)]
    pub fn new(p: *const u8) -> Self {
        Self { p, start: p }
    }
    #[inline(always)]
    pub unsafe fn diff(self) -> usize {
        self.p.offset_from(self.start) as usize
    }
    #[inline(always)]
    pub unsafe fn read_u8(&mut self) -> u8 {
        let v = self.p.read();
        self.p = self.p.add(1);
        v
    }
    #[inline(always)]
    pub unsafe fn read_u16(&mut self) -> u16 {
        let v = (self.p as *mut u16).read();
        self.p = self.p.add(2);
        if cfg!(target_endian = "big") {
            v.to_be()
        } else {
            v
        }
    }
    #[inline(always)]
    pub unsafe fn read_u32(&mut self) -> u32 {
        let v = (self.p as *mut u32).read();
        self.p = self.p.add(4);
        if cfg!(target_endian = "big") {
            v.to_be()
        } else {
            v
        }
    }

    #[inline(always)]
    pub unsafe fn read_u64(&mut self) -> u64 {
        let v = (self.p as *mut u64).read();
        self.p = self.p.add(8);
        if cfg!(target_endian = "big") {
            v.to_be()
        } else {
            v
        }
    }

    #[inline(always)]
    pub unsafe fn read_u128(&mut self) -> u128 {
        let v = (self.p as *mut u128).read();
        self.p = self.p.add(16);
        if cfg!(target_endian = "big") {
            v.to_be()
        } else {
            v
        }
    }
}
