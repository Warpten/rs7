use std::ops::{BitOr, BitOrAssign, Shl};

use bytes::Buf;
use num::Zero;

pub trait ReadVar: Buf {
    fn read_leb<T: ReadVarImpl<T>>(&mut self) -> T;
}

pub(crate) trait ReadVarImpl<T>: Zero + BitOrAssign<Self> {
    fn read(data: &mut impl Buf) -> T;
}

// https://github.com/rust-lang/rust/blob/30f74ff0dc4d66debc8b50724c446f817e5f75f4/compiler/rustc_serialize/src/leb128.rs
macro_rules! impl_unsigned {
    ($($t:ty),*) => {
        $(
            impl ReadVarImpl<$t> for $t {
                fn read(data: &mut impl Buf) -> $t {
                    let byte = data.get_u8();
                    if (byte & 0x80) == 0 {
                        return byte as $t;
                    }

                    let mut result = (byte & 0x7F) as $t;
                    let mut shift = 7;
                    loop {
                        let byte = data.get_u8();

                        if (byte & 0x80) == 0 {
                            return result | (byte as $t) << shift;
                        } else {
                            result |= ((byte & 0x7F) as $t) << shift;
                        }
                        shift += 7;
                    }
                }
            }
        )*
    };
}

macro_rules! impl_signed {
    ($($t:ty),*) => {
        $(
            impl ReadVarImpl<$t> for $t {
                fn read(data: &mut impl Buf) -> $t {
                    let mut result = 0;
                    let mut shift = 0;
                    let mut byte;

                    loop {
                        byte = data.get_u8();
                        result |= ((byte & 0x7F) as $t) << shift;
                        shift += 7;

                        if (byte & 0x80) == 0 {
                            break;
                        }
                    }

                    if (shift < <$t>::BITS) && ((byte & 0x40) != 0) {
                        // sign extend
                        result |= (!0 << shift);
                    }

                    result
                }
            }
        )*
    };
}

impl<S: Buf> ReadVar for S {
    fn read_leb<T: ReadVarImpl<T>>(&mut self) -> T {
        T::read(self)
    }
}

impl_unsigned!(u8, u16, u32, u64, u128, usize);
impl_signed!(i8, i16, i32, i64, i128, isize);
