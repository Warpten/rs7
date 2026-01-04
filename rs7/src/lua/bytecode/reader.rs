use std::ops::{Deref, DerefMut};

use bytes::Buf;

/// Provides read operations on a buffer.
pub trait EndianBuffer<B: Buf>: DerefMut<Target = B> {
    fn read_u16<R: Buf>(&mut self) -> u16;
    fn read_u32<R: Buf>(&mut self) -> u32;
    fn read_u64<R: Buf>(&mut self) -> u64;
    fn read_i16<R: Buf>(&mut self) -> i16;
    fn read_i32<R: Buf>(&mut self) -> i32;
    fn read_i64<R: Buf>(&mut self) -> i64;
}

pub struct NativeEndianBuffer<B: Buf>(pub B);
pub struct LittleEndianBuffer<B: Buf>(pub B);
pub struct BigEndianBuffer<B: Buf>(pub B);

impl<B: Buf> EndianBuffer<B> for NativeEndianBuffer<B> {
    fn read_u16<R: Buf>(&mut self) -> u16 {
        self.get_u16_ne()
    }

    fn read_u32<R: Buf>(&mut self) -> u32 {
        self.get_u32_ne()
    }

    fn read_u64<R: Buf>(&mut self) -> u64 {
        self.get_u64_ne()
    }

    fn read_i16<R: Buf>(&mut self) -> i16 {
        self.get_i16_ne()
    }

    fn read_i32<R: Buf>(&mut self) -> i32 {
        self.get_i32_ne()
    }

    fn read_i64<R: Buf>(&mut self) -> i64 {
        self.get_i64_ne()
    }
}

impl<B: Buf> EndianBuffer<B> for LittleEndianBuffer<B> {
    fn read_u16<R: Buf>(&mut self) -> u16 {
        self.get_u16_le()
    }

    fn read_u32<R: Buf>(&mut self) -> u32 {
        self.get_u32_le()
    }

    fn read_u64<R: Buf>(&mut self) -> u64 {
        self.get_u64_le()
    }

    fn read_i16<R: Buf>(&mut self) -> i16 {
        self.get_i16_le()
    }

    fn read_i32<R: Buf>(&mut self) -> i32 {
        self.get_i32_le()
    }

    fn read_i64<R: Buf>(&mut self) -> i64 {
        self.get_i64_le()
    }
}

impl<B: Buf> EndianBuffer<B> for BigEndianBuffer<B> {
    fn read_u16<R: Buf>(&mut self) -> u16 {
        self.get_u16()
    }

    fn read_u32<R: Buf>(&mut self) -> u32 {
        self.get_u32()
    }

    fn read_u64<R: Buf>(&mut self) -> u64 {
        self.get_u64()
    }

    fn read_i16<R: Buf>(&mut self) -> i16 {
        self.get_i16()
    }

    fn read_i32<R: Buf>(&mut self) -> i32 {
        self.get_i32()
    }

    fn read_i64<R: Buf>(&mut self) -> i64 {
        self.get_i64()
    }
}

macro_rules! impl_deref {
    ($t:tt) => {
        impl<B: Buf> Deref for $t<B> {
            type Target = B;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<B: Buf> DerefMut for $t<B> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

impl_deref!(NativeEndianBuffer);
impl_deref!(LittleEndianBuffer);
impl_deref!(BigEndianBuffer);
