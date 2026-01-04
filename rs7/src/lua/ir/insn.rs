use crate::lua::{bytecode, ir::Emitter};

/// A slot is a primitive bytecode `Instruction` operand.
///
/// LuaJIT instructions have one to three operands. Each operand is an integer
/// that has a meaning tied to the instruction. In our IR, this relation is stripped,
/// so the operands acquire metadata to retain this information instead. As a consequence,
/// we chose to wrap them in a lightweight enumeration type, effectively encoding the
/// information in the type system.
pub enum BasicOperand {
    /// A variable slot number.
    Var(u32),
    /// An upvalue slot number.
    Upvalue(u32),
    /// A literal value.
    UnsignedLiteral(u32),
    /// A signed literal value.
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

impl BasicOperand {
    pub fn len(self) -> Expr {
        Expr::Len(self)
    }

    pub fn neg(self) -> Expr {
        Expr::Negate(self)
    }

    pub fn not(self) -> Expr {
        Expr::Not(self)
    }
}

macro_rules! define_binop {
    ($v:ident, $fn:ident) => {
        impl ::core::ops::$v for BasicOperand {
            type Output = Expr;

            fn $fn(self, rhs: Self) -> Self::Output {
                Expr::$v(self, rhs)
            }
        }
    };
}

// Define helpers to simplify combining slots in operands
define_binop!(Rem, rem);
define_binop!(Mul, mul);
define_binop!(Div, div);
define_binop!(Add, add);
define_binop!(Sub, sub);
impl BasicOperand {
    pub fn pow(self, exp: Self) -> Expr {
        Expr::Pow(self, exp)
    }
}

impl Into<Operand> for BasicOperand {
    fn into(self) -> Operand {
        Operand::Basic(self)
    }
}

pub enum Primitive {
    Nil,
    True,
    False,
}

pub enum Operand {
    Expr(Expr),
    Basic(BasicOperand),
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
    /// A binary comparison operation. This should only be used by the branch register.
    Binary(CmpOp, BasicOperand, BasicOperand),
    /// `lhs + rhs`.
    Add(BasicOperand, BasicOperand),
    /// `lhs - rhs`.
    Sub(BasicOperand, BasicOperand),
    /// `lhs * rhs`.
    Mul(BasicOperand, BasicOperand),
    /// `lhs / rhs`.
    Div(BasicOperand, BasicOperand),
    /// `lhs % rhs`.
    Rem(BasicOperand, BasicOperand),
    /// `lhs ^ rhs`.
    Pow(BasicOperand, BasicOperand),
    /// `lhs .. ~ .. rhs`.
    Cat(BasicOperand, BasicOperand),
    /// `lhs[rhs]`.
    Index(BasicOperand, BasicOperand),
    /// `!value`.
    Not(BasicOperand),
    /// `-value`.
    Negate(BasicOperand),
    /// `#value` (object length).
    Len(BasicOperand),
}

impl Into<Operand> for Expr {
    fn into(self) -> Operand {
        Operand::Expr(self)
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
    Assign { lhs: Operand, rhs: Operand },
    /// Follows the given label if `cond` evals to `true`.
    ConditionalBranch { cond: Operand, target: Label },
    /// Unconditionally jumps to the target label.
    Branch { target: Label },
    /// Returns control flow to the caller.
    Return {
        base: BasicOperand,
        /// The amount of return values, starting at the base `Slot`.
        count: u16
    }
}

/// The comparison opcode used by `Expr::Binary`.
#[repr(u8)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// The destination of a branch instruction.
pub enum Label {
    None,
    Label { ir: usize, bc: usize },
}

#[rustfmt::skip]
macro_rules! op {
    (Var $v:ident) => { BasicOperand::Var($v as u32) };
    (Num $v:ident) => { BasicOperand::Num($v as u32) };
    (Str $v:ident) => { BasicOperand::Str($v as u32) };
    (Lit $v:ident) => { BasicOperand::UnsignedLiteral($v as u32) };
    (Uv $v:ident) => { BasicOperand::Upvalue($v as u32) };
    (Pri $v:ident) => {
        BasicOperand::Pri(match $v {
            0 => Primitive::Nil,
            1 => Primitive::True,
            2 => Primitive::False,
            _ => unimplemented!("Unknown primitive type")
        })
    }
}

#[rustfmt::skip]
macro_rules! expr {
    (Add $lhs:expr, $rhs:expr) => { $lhs + $rhs };
    (Sub $lhs:expr, $rhs:expr) => { $lhs - $rhs };
    (Div $lhs:expr, $rhs:expr) => { $lhs / $rhs };
    (Mul $lhs:expr, $rhs:expr) => { $lhs * $rhs };
    (Mod $lhs:expr, $rhs:expr) => { $lhs % $rhs };
    (Pow $lhs:expr, $rhs:expr) => { Expr::Pow($lhs, $rhs) };
    (Cat $lhs:expr, $rhs:expr) => { Expr::Cat($lhs, $rhs) };
    (Idx $lhs:expr, $rhs:expr) => { Expr::Index($lhs, $rhs) };
}

impl Insn {
    #[inline]
    fn emit_cond_branch(emitter: &mut Emitter, op: CmpOp, a: u8, d: u16) {
        let op = Expr::Binary(op, op!(Var a), op!(Var d));

        // Some instructions are followed by explicit branches; others inline the branch label
        // in their operands. To account for this, we do not set the branch label here; explicit
        // branching instructions will instead acquire the last emitted branch instruction and
        // fixup the branch label. See `Emitter::fixup_branches`.

        emitter.emit(Self::ConditionalBranch {
            cond: op.into(),
            target: Label::None,
        });
    }

    #[inline]
    fn emit_assignment<L: Into<Operand>, R: Into<Operand>>(emitter: &mut Emitter, lhs: L, rhs: R) {
        emitter.emit(Self::Assign {
            lhs: lhs.into(),
            rhs: rhs.into(),
        });
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
            I::ISTYPE { a, d } => todo!(),
            I::ISNUM { a, d } => todo!(),
            I::MOV { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Var d)),
            I::NOT { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Var d).not()),
            I::UNM { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Var d).neg()),
            I::LEN { a, d } => Self::emit_assignment(emitter, op!(Var a), op!(Var d).len()),
            I::ADDVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) + op!(Num c)),
            I::SUBVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) - op!(Num c)),
            I::MULVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) * op!(Num c)),
            I::DIVVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) / op!(Num c)),
            I::MODVN { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) * op!(Num c)),
            I::ADDNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Num b) + op!(Var c)),
            I::SUBNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Num b) - op!(Var c)),
            I::MULNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Num b) * op!(Var c)),
            I::DIVNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Num b) / op!(Var c)),
            I::MODNV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Num b) % op!(Var c)),
            I::ADDVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) + op!(Var c)),
            I::SUBVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) - op!(Var c)),
            I::MULVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) * op!(Var c)),
            I::DIVVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) / op!(Var c)),
            I::MODVV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b) % op!(Var c)),
            I::POW { a, b, c } => Self::emit_assignment(emitter, op!(Var a), op!(Var b).pow(op!(Var c))),
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
            I::TGETV { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Idx op!(Var b), op!(Var c))),
            I::TGETS { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Idx op!(Var b), op!(Str c))),
            I::TGETB { a, b, c } => Self::emit_assignment(emitter, op!(Var a), expr!(Idx op!(Var b), op!(Lit c))),
            I::TGETR { a, b, c } => todo!(),
            I::TSETV { a, b, c } => Self::emit_assignment(emitter, expr!(Idx op!(Var b), op!(Var c)), op!(Var a)),
            I::TSETS { a, b, c } => Self::emit_assignment(emitter, expr!(Idx op!(Var b), op!(Var c)), op!(Str a)),
            I::TSETB { a, b, c } => Self::emit_assignment(emitter, expr!(Idx op!(Var b), op!(Var c)), op!(Lit a)),
            I::TSETR { a, b, c } => todo!(),
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
            I::RET { a, d } => emitter.emit(Insn::Return {
                base: op!(Var a),
                count: d - 1,
            }),
            I::RET0 { a, .. } => emitter.emit(Insn::Return {
                base: op!(Var a),
                count: 0,
            }),
            I::RET1 { a, .. } => emitter.emit(Insn::Return {
                base: op!(Var a),
                count: 1,
            }),
            I::FORI { a, d } => todo!(),
            I::JFORI { a, d } => todo!(),
            I::FORL { a, d } => todo!(),
            I::IFORL { a, d } => todo!(),
            I::JFORL { a, d } => todo!(),
            I::ITERL { a, d } => todo!(),
            I::IITERL { a, d } => todo!(),
            I::JITERL { a, d } => todo!(),
            I::LOOP { a, d } => todo!(),
            I::ILOOP { a, d } => todo!(),
            I::JLOOP { a, d } => todo!(),
            I::JMP { a, d } => emitter.fixup_branch(Label::Label { ir: 0, bc: d as usize }),
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
