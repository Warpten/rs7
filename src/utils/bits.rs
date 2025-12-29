pub trait Bits {
    fn bits() -> u32;
}
impl Bits for u8 {
    fn bits() -> u32 {
        Self::BITS
    }
}
impl Bits for u16 {
    fn bits() -> u32 {
        Self::BITS
    }
}
impl Bits for u32 {
    fn bits() -> u32 {
        Self::BITS
    }
}
impl Bits for u64 {
    fn bits() -> u32 {
        Self::BITS
    }
}
impl Bits for u128 {
    fn bits() -> u32 {
        Self::BITS
    }
}
