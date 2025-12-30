use std::fmt;

use bytes::Buf;

use crate::{
    lua::bytecode::{Numeric, primitives::read_string},
    utils::ReadVar,
};

pub enum TableItem {
    Nil,
    False,
    True,
    Integer(i32),
    Numeric(Numeric),
    String(String),
}

impl TableItem {
    // bcread_ktabk
    pub fn new<R: Buf>(data: &mut R) -> Self {
        let tp = data.read_leb::<u32>() as usize;

        match tp {
            0 => Self::Nil,
            1 => Self::False,
            2 => Self::True,
            3 => Self::Integer(u32::cast_signed(data.read_leb::<u32>())),
            4 => {
                // Yes, this is correct. We don't use the constructor here.
                // Don't fucking ask me.

                let lo = data.read_leb::<u32>() as u64;
                let hi = data.read_leb::<u32>() as u64;

                let value = (hi << u32::BITS) | lo;
                Self::Numeric(Numeric(value))
            }
            5.. => Self::String(read_string(data, tp - 5)),
        }
    }
}

impl fmt::Debug for TableItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "Nil"),
            Self::False => write!(f, "False"),
            Self::True => write!(f, "True"),
            Self::Integer(value) => write!(f, "{{ Integer: {:#?} }}", value),
            Self::Numeric(value) => write!(f, "{{ Numeric: {:#?} }}", value.0),
            Self::String(value) => write!(f, "{:#?}", value),
        }
    }
}
