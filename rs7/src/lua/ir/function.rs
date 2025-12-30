use crate::lua::{bytecode::Prototype, ir::Module};

pub struct Function {}

impl Function {
    pub fn new(module: &Module, proto: &Prototype) -> Self {
        Self {}
    }
}
