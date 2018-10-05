#![allow(dead_code)]

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

#[macro_export]
macro_rules! array_from_slice_uninitialized {
    ($buffer:expr, $size:expr) => {{
        let len = $buffer.len();
        let mut array: [u8; $size] = unsafe { std::mem::uninitialized() };
        (&mut array[..len]).copy_from_slice(&$buffer[..len]);
        (array, len)
    }}
}

#[macro_export]
macro_rules! array_from_slice_zeroed {
    ($buffer:expr, $size:expr) => {{
        let len = $buffer.len();
        let mut array: [u8; $size] = unsafe { std::mem::zeroed() };
        (&mut array[..len]).copy_from_slice(&$buffer[..len]);
        (array, len)
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

mod no_panic {
    /// A wrapper around a slice that exposes no functions that can panic.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Slice<'a> {
        bytes: &'a [u8]
    }

    impl<'a> Slice<'a> {
        #[inline]
        pub fn new(bytes: &'a [u8]) -> Self {
            Slice { bytes }
        }

        #[inline]
        pub fn get<I>(&self, i: I) -> Option<&I::Output>
            where I: std::slice::SliceIndex<[u8]>
        {
            self.bytes.get(i)
        }

        #[inline]
        // TODO: https://github.com/rust-lang/rust/issues/35729#issuecomment-280872145
        //      pub fn get<I>(&self, i: I) -> Option<&I::Output>
        //          where I: core::slice::SliceIndex<u8>
        pub fn get_i(&self, i: usize) -> Option<&u8> { self.bytes.get(i) }

        // TODO: This will be replaced with `get()` once `get()` is made
        // generic over `SliceIndex`.
        #[inline]
        pub fn get_slice(&self, r: std::ops::Range<usize>) -> Option<Self> {
            self.bytes.get(r).map(|bytes| Self { bytes })
        }

        #[inline]
        pub fn into_iter(&self) -> <&'a [u8] as IntoIterator>::IntoIter {
            self.bytes.into_iter()
        }

        #[inline]
        pub fn is_empty(&self) -> bool { self.bytes.is_empty() }

        #[inline]
        pub fn len(&self) -> usize { self.bytes.len() }

        #[inline]
        pub fn as_slice_less_safe(&self) -> &'a [u8] { self.bytes }
    }
}
*/
