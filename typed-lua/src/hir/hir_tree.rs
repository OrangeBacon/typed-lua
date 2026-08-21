//! HIR (High-level Intermediate Representation)
//! This is a less-complex representation of a source program.  It is a de-sugaring
//! of the name-tree into something half-way between an AST and a CFG.  The aim
//! is for this to be utilised for type checking.
//!
//! Each opcode in here is relatively high-level, and might end up getting lowered
//! into a lot of smaller, lower level instructions, e.g. add turns into type checks,
//! an actual add, metatable lookup, calling one of the possible metatables, etc.

use crate::utils::OrderedFloat;

/// Root of the High-level IR
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hir {
    /// All instructions in the root of a module
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionId(pub u32);

/// A HIR instruction with its ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instruction {
    pub id: InstructionId,
    pub opcode: Opcode,
}

/// All possible HIR instructions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Opcode {
    // statements
    Block {
        instructions: Vec<Instruction>,
    },
    Return {
        value: Option<InstructionId>,
    },

    // expressions
    Tuple {
        values: Vec<InstructionId>,
    },
    Nil,
    Bool(bool),
    Float(OrderedFloat),
    Int(i64),

    /// numerical negative, Negate(a) = -a
    Negate(InstructionId),

    /// table length, Length(a) = #a
    Length(InstructionId),

    /// logical not, Not(a) = not a
    Not(InstructionId),

    /// bitwise not, BitNot(a) = ~a
    BitNot(InstructionId),
}
