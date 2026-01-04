use std::fmt;

use bytes::Buf;

use crate::{
    lua::bytecode::{Complex, Dump, EndianBuffer, Instruction, Numeric, debug::Debug},
    utils::ReadVar,
};

#[derive(Debug, Copy, Clone)]
pub struct Upvalue(u16);

pub struct Prototype {
    /// Index of this prototype within its dump.
    pub index: usize,

    flags: u8,
    numparams: u8,
    framesize: u8,
    debug: Option<Debug>,

    pub instructions: Vec<Instruction>,
    pub uvs: Vec<Upvalue>,
    pub kgc: Vec<Complex>,
    pub kn: Vec<Numeric>,
}

impl Prototype {
    /// Parses a LuaJIT prototype.
    ///
    /// This function is an implementation of `lj_bcread_proto`.
    ///
    /// # Arguments
    ///
    /// * `dump` - The dump this prototype belongs to.
    /// * `data` - The data to parse.
    /// * `index` - The index of this prototype in the `Dump`.
    /// * `version` - The bytecode version.
    pub fn new<B>(dump: &Dump, data: &mut impl EndianBuffer<B>, index: usize, version: u8) -> Option<Self>
    where
        B: Buf,
    {
        let size = data.read_leb::<u32>();
        if size == 0 {
            return None;
        }

        let flags = data.get_u8();
        let numparams = data.get_u8();
        let framesize = data.get_u8();
        let sizeuv = data.get_u8() as usize;

        let sizekgc = data.read_leb::<u32>();
        let sizekn = data.read_leb::<u32>();
        let sizeinsn = data.read_leb::<u32>() as usize;

        let (sizedbg, _firstline, numline) = if !dump.stripped {
            let sizedbg = data.read_leb::<u32>();
            let (firstline, numline) = if sizedbg != 0 {
                let firstline = data.read_leb::<u32>();
                let numline = data.read_leb::<u32>() as usize;

                (firstline, numline)
            } else {
                (0, 0)
            };

            (sizedbg, firstline, numline)
        } else {
            (0, 0, 0)
        };

        // LuaJIT: prepends FUNCF opcode where A = framesize
        let instructions = (0..sizeinsn).map(|_| Instruction::new(data, version)).collect();

        let upvalues = (0..sizeuv).map(|_| Upvalue(data.read_u16())).collect();

        let complex_constants = (0..sizekgc).map(|_| Complex::new(data.deref_mut(), index)).collect();

        let numeric_constants = (0..sizekn).map(|_| Numeric::new(data.deref_mut())).collect();

        let debug = if sizedbg > 0 {
            Some(Debug::new(data, sizeinsn, numline, sizeuv))
        } else {
            None
        };

        // TODO: Validate that we read `size` bytes.

        Some(Self {
            index,
            flags,
            numparams,
            framesize,
            debug,
            instructions,
            uvs: upvalues,
            kgc: complex_constants,
            kn: numeric_constants,
        })
    }
}

impl fmt::Debug for Prototype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut binding = f.debug_struct(format!("Prototype [{}]", self.index).as_str());
        binding
            .field("flags", &self.flags)
            .field("numparams", &self.numparams)
            .field("framesize", &self.framesize);

        if let Some(dbg) = &self.debug {
            binding.field("debug", &dbg);
        }

        binding
            .field("instructions", &self.instructions)
            .field("uvs", &self.uvs)
            .field("kgc", &self.kgc)
            .field("kn", &self.kn)
            .finish_non_exhaustive()
    }
}
