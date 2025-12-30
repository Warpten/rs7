use std::usize;

use bytes::Buf;

use crate::{
    lua::bytecode::{Prototype, primitives::read_string},
    utils::ReadVar,
};

#[derive(Debug)]
pub struct Dump {
    pub stripped: bool,
    pub name: Option<String>,
    protos: Vec<Prototype>,
    main: usize,
}

impl Dump {
    /// Parses a LuaJIT bytecode dump.
    ///
    /// This function is an implementation of `lj_bcread`.
    ///
    /// # Arguments:
    ///
    /// * `data` - The binary data to parse.
    pub fn new(mut data: impl Buf) -> Self {
        let header = [data.get_u8(), data.get_u8(), data.get_u8(), data.get_u8()];
        assert!(header == [0x1B, 0x4C, 0x4A, 2]);

        let flags = data.read_leb::<u32>();

        // TODO: Validate flags; if FFI we need to load ctype_ffi

        let file_name = if (flags & 2) == 0 {
            let len = data.read_leb::<u32>() as usize;
            Some(read_string(&mut data, len))
        } else {
            None
        };

        let mut instance = Self {
            stripped: (flags & 2) != 0,
            name: file_name,
            protos: vec![],
            main: usize::MAX,
        };

        while data.has_remaining() {
            if let Some(p) = Prototype::new(&instance, &mut data, instance.protos.len()) {
                instance.protos.push(p);
            }
        }

        assert!(!instance.protos.is_empty());

        instance.main = instance.protos.len() - 1;
        instance
    }

    /// Returns the main prototype in this bytecode dump.
    pub fn main(&self) -> &Prototype {
        &self.protos[self.main]
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        fs::File,
        io::{BufReader, Read},
    };

    use bytes::Bytes;

    use crate::lua::bytecode::Dump;

    #[test]
    pub fn test_bc() {
        let file = File::open(format!(
            "{}/Downloads/ai.lua.jit",
            env::home_dir().unwrap().to_string_lossy()
        ))
        .unwrap();
        let mut reader = BufReader::new(file);

        // Can i avoid this ?
        let mut data =
            Vec::with_capacity(reader.get_ref().metadata().map_or(0, |m| m.len()) as usize);
        _ = reader.read_to_end(&mut data);
        let bytes = Bytes::from(data);

        let dump = Dump::new(bytes);
        println!("{:#?}", dump);
    }
}
