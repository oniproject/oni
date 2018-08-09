pub struct Shim32;
impl crate::Shim for Shim32 {
    type Index = u32;
    type Scalar = f32;
    type Vector = [Self::Scalar; 2];
}

pub struct Shim64;
impl crate::Shim for Shim64 {
    type Index = u32;
    type Scalar = f64;
    type Vector = [Self::Scalar; 2];
}
