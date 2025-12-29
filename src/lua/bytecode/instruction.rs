use std::fmt;

use bytes::Buf;

pub struct Instruction {
    data: [u8; 4],
}

impl Instruction {
    pub fn new<R: Buf>(data: &mut R) -> Self {
        Self {
            data: data.get_u32_ne().to_ne_bytes(),
        }
    }

    pub fn opcode(&self) -> u8 {
        self.data[0]
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Instruction [{}]", u32::from_ne_bytes(self.data))
    }
}
