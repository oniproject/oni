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
    crate fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }
    crate fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.len]
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
