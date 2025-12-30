use std::{
    fmt,
    ops::{BitOr, Shl},
};

use bytes::Buf;

use crate::{
    lua::bytecode::{primitives::read_string, table_item::TableItem},
    utils::{ReadVar, bits::Bits},
};

pub fn read_parts<R, T>(data: &mut R) -> T
where
    R: Buf,
    T: From<u32> + Bits + Shl<u32, Output = T> + BitOr<Output = T>,
{
    let hi = data.read_leb();
    let lo = data.read_leb();
    (T::from(hi) << u32::BITS) | T::from(lo)
}

pub enum Complex {
    /// A reference to a prototype in the dump.
    ///
    /// The argument to this variant is the index of the prototype being referred to.
    Prototype(usize),
    Table {
        array: Vec<TableItem>,
        hash: Vec<(TableItem, TableItem)>,
    },
    Signed(i64),
    Unsigned(u64),
    Complex {
        real: u64,
        imaginary: u64,
    },
    String(String),
}

impl Complex {
    /// Creates a new complex constant.
    ///
    /// This function is an implementation of LuaJIT's `bcread_kgc`.
    ///
    /// # Arguments
    ///
    /// * `data` - The data source.
    /// * `proto` - The index of the `Prototype` this constant belongs to.
    pub fn new<R>(data: &mut R, proto: usize) -> Self
    where
        R: Buf,
    {
        let tp = data.read_leb::<u32>() as usize;

        match tp {
            0 => Self::Prototype(proto - 1),
            1 => {
                let narray = data.read_leb::<u32>() as usize;
                let nhash = data.read_leb::<u32>() as usize;

                let array = (0..narray).map(|_| TableItem::new(data)).collect();

                let entries = (0..nhash)
                    .map(|_| {
                        let key = TableItem::new(data);
                        let value = TableItem::new(data);

                        (key, value)
                    })
                    .collect();

                Self::Table {
                    array,
                    hash: entries,
                }
            }
            2 => {
                let value = read_parts(data);
                Complex::Signed(u64::cast_signed(value))
            }
            3 => Complex::Unsigned(read_parts(data)),
            4 => {
                // Complex
                let real = read_parts(data);
                let imaginary = read_parts(data);

                Complex::Complex { real, imaginary }
            }
            5.. => Complex::String(read_string(data, tp - 5)),
        }
    }
}

pub struct Numeric(pub u64);

impl Numeric {
    pub fn new(data: &mut impl Buf) -> Self {
        let (is_number, lo) = bcread_uleb128_33(data);
        if is_number {
            let hi = data.read_leb::<u32>();
            let value = ((hi as u64) << u32::BITS) | (lo as u64);

            Self(value)
        } else {
            Self(lo as u64)
        }
    }
}

impl fmt::Debug for Numeric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", &self.0)
    }
}

fn bcread_uleb128_33<R: Buf>(pp: &mut R) -> (bool, u32) {
    let mut buffer = pp.get_u8() as u32;
    let is_number_bit = (buffer & 0b01) != 0;

    let mut value = buffer >> 1;
    if (buffer & 0x80) != 0 {
        let mut shift = 6;
        value &= 0x3F;

        loop {
            assert!(shift < u32::BITS, "Parsing too much 33-bits uleb128");
            buffer = pp.get_u8() as u32;
            value |= (buffer & 0x7F) << shift;
            shift += 7;

            if (buffer & 0x80) == 0 {
                break;
            }
        }
    }

    (is_number_bit, value)
}

impl fmt::Debug for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prototype(index) => write!(f, "{{ Prototype: {:#?} }}", index),
            Self::Table { array, hash } => f
                .debug_struct("Table")
                .field("array", array)
                .field("hash", hash)
                .finish(),
            Self::Signed(value) => write!(f, "{{ Signed: {:#?} }}", value),
            Self::Unsigned(value) => write!(f, "{{ Unsigned: {:#?} }}", value),
            Self::Complex { real, imaginary } => f
                .debug_struct("Complex")
                .field("real", real)
                .field("imaginary", imaginary)
                .finish(),
            Self::String(value) => write!(f, "{:#?}", value),
        }
    }
}
