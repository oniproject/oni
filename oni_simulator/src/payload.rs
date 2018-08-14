use generic_array::{
    ArrayLength,
    GenericArray,
};

#[derive(PartialEq, Debug)]
pub struct Payload<MTU: ArrayLength<u8>> {
    len: usize,
    data: GenericArray<u8, MTU>,
}

impl<MTU: ArrayLength<u8>> Clone for Payload<MTU> {
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            data: self.data.clone(),
        }
    }
}

impl<'a, MTU: ArrayLength<u8>> Payload<MTU> {
    crate fn copy_to(&self, buf: &mut [u8]) -> usize {
        let payload = &self.data[..self.len];
        let len = self.len.min(buf.len());
        (&mut buf[..len]).copy_from_slice(&payload[..len]);
        len
    }
}

impl<'a, MTU: ArrayLength<u8>> From<&'a [u8]> for Payload<MTU> {
    fn from(payload: &'a [u8]) -> Self {
        let mut data: GenericArray<u8, MTU> = unsafe { std::mem::zeroed() };
        let len = payload.len();
        (&mut data[..len]).copy_from_slice(payload);
        Self { data, len }
    }
}
