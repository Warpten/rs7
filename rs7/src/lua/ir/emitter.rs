use crate::lua::ir::{Insn, Label};

pub struct Emitter {
    pub instructions: Vec<Insn>,
}

impl Emitter {
    pub fn new() -> Self {
        Self { instructions: vec![] }
    }

    pub fn emit(&mut self, insn: Insn) {
        self.instructions.push(insn);
    }

    pub fn fixup_branch(&mut self, tgt: Label) {
        let idx = self.instructions.len() - 1;

        //   ISLT lhs, rgs
        //   JMP label1
        //   JMP label2
        // label1:
        //   ..
        // label2:
        //   ..
        // In this bytecode, instructions 1 and 2 constitute a conditional jump that
        // jumps to `label1` if `lhs < rhs`, but otherwise fallsthrough to instruction
        // 3, which jumps to `label2`.
        //
        // It is guaranteed that all compare-and-test instructions are immediately
        // followed by a branching instruction.
        if let Insn::ConditionalBranch { target, .. } = &mut self.instructions[idx] {
            if let Label::None = target {
                *target = tgt;
                return;
            }
        }

        self.emit(Insn::Branch { target: tgt });
    }
}
