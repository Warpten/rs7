/// The IR we emit for LuaJIT is very close to the original bytecode but there are some fundamental
/// differences:
///
/// * In bytecode, instructions encode the logic behind their operands' raw values. We strip away this
///   relationship in IR in an effort to deduplicate instructions. This is the `Slot` enumeration.
/// * Furthermore, some instructions in bitcode are actually not atomic from a "language" point of view,
///   so we introduced `Expr`. `Expr` is a wrapper around complex operations declared in LuaJIT bytecode
///   that relates to instruction operands. It allows us to avoid having to create virtual slots while
///   parsing and translating the IR.
/// * We use a branch register that behaves as a stack: a new value is pushed to it when it is written
///   by IR instructions, and the current value is popped (and returned) when IR instructions read it.
/// * Finally, an `Insn` is an IR instruction for which one or many `Op`s are given. Each `Op` can be:
///   * a `Slot`
///   * an `Expr`
///
pub mod emitter;
pub mod function;
pub mod insn;
pub mod module;
pub mod printer;

pub use emitter::*;
pub use function::*;
pub use insn::*;
pub use module::*;
pub use printer::*;
