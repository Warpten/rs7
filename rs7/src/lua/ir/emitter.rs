use crate::lua::ir::Insn;

pub struct Emitter {
    pub instructions: Vec<Insn>,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
        }
    }

    pub fn emit(&mut self, insn: Insn) {
        self.instructions.push(insn);
    }
}
