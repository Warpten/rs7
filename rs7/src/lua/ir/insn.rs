use crate::lua::{bytecode, ir::Emitter};

/// A slot is a primitive bytecode `Instruction` operand.
pub enum Slot {
    /// A variable slot number.
    Var(u32),
    /// An upvalue slot number.
    Upvalue(u32),
    /// A literal.
    UnsignedLiteral(u32),
    /// A signed literal.
    SignedLiteral(i32),
    /// A primitive.
    Pri(Primitive),
    /// A number constant; index into constant table.
    Num(u32),
    /// A string constant; negated index into constant table.
    Str(u32),
    /// A template table; negated index into constant table.
    Table(u32),
    /// A function prototype; negated index into constant table.
    Func(u32),
    /// A data constant, negated index into constant table
    Constant(u32),
    /// A branch target, relative to next instruction, biased with 0x8000
    Branch(u32),
}

impl Slot {
    pub fn len(self) -> Expr {
        Expr::Len(self)
    }
}

impl Into<Op> for Slot {
    fn into(self) -> Op {
        Op::Slot(self)
    }
}

pub enum Primitive {
    Nil,
    True,
    False,
}

pub enum Op {
    Expr(Expr),
    Slot(Slot),
    Cmp { op: CmpOp, lhs: Slot, rhs: Slot },
}

/// An `Expr` is a fragment of a complex instruction.
///
/// # Examples:
/// * `ADDVN a, b, c` would translate to:
/// ```
/// Insn::Add {
///   lhs: Slot::Var(a),
///   rhs: Op::Expr(Expr::Add {
///     lhs: Slot::Var(b),
///     rhs: Slot::Num(c)
///   })
/// }
/// ```
pub enum Expr {
    /// `lhs + rhs`.
    Add(Slot, Slot),
    /// `lhs - rhs`.
    Sub(Slot, Slot),
    /// `lhs * rhs`.
    Mul(Slot, Slot),
    /// `lhs / rhs`.
    Div(Slot, Slot),
    /// `lhs % rhs`.
    Mod(Slot, Slot),
    /// `lhs ^ rhs`.
    Pow(Slot, Slot),
    /// `lhs .. ~ .. rhs`.
    Cat(Slot, Slot),
    /// `lhs[rhs]`.
    Index(Slot, Slot),
    /// `-value`.
    Negate(Slot),
    /// `#value` (object length).
    Len(Slot),
}

impl Into<Op> for Expr {
    fn into(self) -> Op {
        Op::Expr(self)
    }
}

/// IR instructions are thinly lifted bytecode instructions.
///
/// While bytecode instructions are mostly their raw data, IR instructions
/// are able to resolve their operands given a context. Some bytecode
/// instructions are also too granular (e.g. they exist in multiple forms
/// depending on their operands). This first abstraction level unifies
/// instructions so that each instruction is a logical unit of operation
/// independant of its operands.
#[rustfmt::skip]
pub enum Insn {
    Assign { lhs: Op, rhs: Op },
    JumpIf { cond: Op, target: Label },
    Jump { target: Label },
}

#[repr(u8)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub enum Label {
    None,
    Label(u32),
}

#[rustfmt::skip]
macro_rules! op {
    (Var $v:ident) => { Slot::Var($v as u32) };
    (Num $v:ident) => { Slot::Num($v as u32) };
    (Str $v:ident) => { Slot::Str($v as u32) };
    (Uv $v:ident) => { Slot::Upvalue($v as u32) };
    (Pri $v:ident) => {
        Slot::Pri(match $v {
            0 => Primitive::Nil,
            1 => Primitive::True,
            2 => Primitive::False,
            _ => unimplemented!("Unknown primitive type")
        })
    }
}

#[rustfmt::skip]
macro_rules! expr {
    (Add $lhs:expr, $rhs:expr) => { Expr::Add($lhs, $rhs) };
    (Sub $lhs:expr, $rhs:expr) => { Expr::Sub($lhs, $rhs) };
    (Div $lhs:expr, $rhs:expr) => { Expr::Div($lhs, $rhs) };
    (Mul $lhs:expr, $rhs:expr) => { Expr::Mul($lhs, $rhs) };
    (Mod $lhs:expr, $rhs:expr) => { Expr::Mod($lhs, $rhs) };
    (Pow $lhs:expr, $rhs:expr) => { Expr::Pow($lhs, $rhs) };
    (Cat $lhs:expr, $rhs:expr) => { Expr::Cat($lhs, $rhs) };
}

impl Insn {
    #[inline]
    fn emit_cond_branch(emitter: &mut Emitter, op: CmpOp, a: u8, d: u16) {
        emitter.emit(Self::JumpIf {
            cond: Op::Cmp {
                op,
                lhs: Slot::Var(a as u32).into(),
                rhs: Slot::Var(d as u32).into(),
            },
            target: Label::None,
        })
    }

    #[inline]
    fn emit_assignment<L: Into<Op>, R: Into<Op>>(emitter: &mut Emitter, lhs: L, rhs: R) {
        emitter.emit(Self::Assign {
            lhs: lhs.into(),
            rhs: rhs.into(),
        })
    }

    pub fn parse(insn: bytecode::Instruction, emitter: &mut Emitter) {
        use bytecode::Instruction as I;

        match insn {
            I::ISLT { a, d } => Self::emit_cond_branch(emitter, CmpOp::Lt, a, d),
            I::ISGE { a, d } => Self::emit_cond_branch(emitter, CmpOp::Ge, a, d),
            I::ISLE { a, d } => Self::emit_cond_branch(emitter, CmpOp::Le, a, d),
            I::ISGT { a, d } => Self::emit_cond_branch(emitter, CmpOp::Gt, a, d),
            I::ISEQV { a, d } => Self::emit_cond_branch(emitter, CmpOp::Eq, a, d),
            I::ISNEV { a, d } => Self::emit_cond_branch(emitter, CmpOp::Ne, a, d),
            I::ISEQS { a, d } => Self::emit_cond_branch(emitter, CmpOp::Eq, a, d),
            I::ISNES { a, d } => Self::emit_cond_branch(emitter, CmpOp::Ne, a, d),
            I::ISEQN { a, d } => Self::emit_cond_branch(emitter, CmpOp::Eq, a, d),
            I::ISNEN { a, d } => Self::emit_cond_branch(emitter, CmpOp::Ne, a, d),
            I::ISEQP { a, d } => Self::emit_cond_branch(emitter, CmpOp::Eq, a, d),
            I::ISNEP { a, d } => Self::emit_cond_branch(emitter, CmpOp::Ne, a, d),
            I::ISTC { a, d } => todo!(),
            I::ISFC { a, d } => todo!(),
            I::IST { d } => todo!(),
            I::ISF { d } => todo!(),
            I::MOV { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Var d)),
            I::NOT { a, d } => todo!(),
            I::UNM { a, d } => todo!(),
            I::LEN { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Var d).len()),
            I::ADDVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Add op!(Var b), op!(Num c))),
            I::SUBVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Sub op!(Var b), op!(Num c))),
            I::MULVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Mul op!(Var b), op!(Num c))),
            I::DIVVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Div op!(Var b), op!(Num c))),
            I::MODVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Mod op!(Var b), op!(Num c))),
            I::ADDNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Add op!(Num b), op!(Var c))),
            I::SUBNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Sub op!(Num b), op!(Var c))),
            I::MULNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Mul op!(Num b), op!(Var c))),
            I::DIVNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Div op!(Num b), op!(Var c))),
            I::MODNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Mod op!(Num b), op!(Var c))),
            I::ADDVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Add op!(Var b), op!(Var c))),
            I::SUBVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Sub op!(Var b), op!(Var c))),
            I::MULVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Mul op!(Var b), op!(Var c))),
            I::DIVVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Div op!(Var b), op!(Var c))),
            I::MODVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Mod op!(Var b), op!(Var c))),
            I::POW { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Pow op!(Var b), op!(Var c))),
            I::CAT { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Cat op!(Var b), op!(Var c))),
            I::KSTR { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Str d)),
            I::KCDATA { a, d } => todo!(),
            I::KSHORT { a, d } => todo!(),
            I::KNUM { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Num d)),
            I::KPRI { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Pri d)),
            I::KNIL { a, d } => todo!(),
            I::UGET { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Uv d)),
            I::USETV { a, d } => Self::emit_assignment(emitter, op!(Uv a), op!(Var d)),
            I::USETS { a, d } => Self::emit_assignment(emitter, op!(Uv a), op!(Str d)),
            I::USETN { a, d } => Self::emit_assignment(emitter, op!(Uv a), op!(Num d)),
            I::USETP { a, d } => Self::emit_assignment(emitter, op!(Uv a), op!(Pri d)),
            I::UCLO { a, d } => todo!(),
            I::FNEW { a, d } => todo!(),
            I::TNEW { a, d } => todo!(),
            I::TDUP { a, d } => todo!(),
            I::GGET { a, d } => todo!(),
            I::GSET { a, d } => todo!(),
            I::TGETV { a, b, c } => todo!(),
            I::TGETS { a, b, c } => todo!(),
            I::TGETB { a, b, c } => todo!(),
            I::TSETV { a, b, c } => todo!(),
            I::TSETS { a, b, c } => todo!(),
            I::TSETB { a, b, c } => todo!(),
            I::TSETM { a, d } => todo!(),
            I::CALLM { a, b, c } => todo!(),
            I::CALL { a, b, c } => todo!(),
            I::CALLMT { a, d } => todo!(),
            I::CALLT { a, d } => todo!(),
            I::ITERC { a, b, c } => todo!(),
            I::ITERN { a, b, c } => todo!(),
            I::VARG { a, b, c } => todo!(),
            I::ISNEXT { a, d } => todo!(),
            I::RETM { a, d } => todo!(),
            I::RET { a, d } => todo!(),
            I::RET0 { a, d } => todo!(),
            I::RET1 { a, d } => todo!(),
            I::FORI { a, d } => todo!(),
            I::JFORI { a, d } => todo!(),
            I::FORL { a, d } => todo!(),
            I::IFORL { a, d } => todo!(),
            I::ITERL { a, d } => todo!(),
            I::IITERL { a, d } => todo!(),
            I::JITERL { a, d } => todo!(),
            I::LOOP { a, d } => todo!(),
            I::ILOOP { a, d } => todo!(),
            I::JLOOP { a, d } => todo!(),
            I::JMP { a, d } => todo!(),
            I::FUNCF { a } => todo!(),
            I::IFUNCF { a } => todo!(),
            I::JFUNCF { a, d } => todo!(),
            I::FUNCV { a } => todo!(),
            I::IFUNCV { a } => todo!(),
            I::JFUNCV { a, d } => todo!(),
            I::FUNCC { a } => todo!(),
            I::FUNCCW { a } => todo!(),
            I::FUNC { a } => todo!(),
        }
    }
}
