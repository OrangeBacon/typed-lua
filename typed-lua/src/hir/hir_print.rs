use std::fmt::{self, Display};

use crate::{
    hir::hir_tree::*,
    utils::{Size, SizeOf},
};

#[derive(Debug, Clone, Copy)]
pub struct HirPrint<'a> {
    hir: &'a Hir,
    depth: usize,
    top_level: bool,
    top_size: usize,
}

impl<'a> HirPrint<'a> {
    pub fn new(hir: &'a Hir) -> Self {
        HirPrint {
            hir,
            depth: 0,
            top_level: true,
            top_size: hir.size(),
        }
    }

    /// Print a single instruction
    fn instruction(&mut self, f: &mut fmt::Formatter<'_>, op: &'_ Instruction) -> fmt::Result {
        self.depth(f)?;
        write!(f, "{} = ", op.id)?;
        self.opcode(f, &op.opcode)?;

        Ok(())
    }

    /// Print a single opcode
    fn opcode(&mut self, f: &mut fmt::Formatter<'_>, op: &'_ Opcode) -> fmt::Result {
        match op {
            Opcode::Block { instructions } => {
                f.write_str("block")?;
                self.block(f, instructions)
            }
            Opcode::Return { value: Some(id) } => write!(f, "return {id}"),
            Opcode::Return { value: None } => f.write_str("return"),
            Opcode::Tuple { values } => write!(
                f,
                "tuple [{}]",
                values
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Opcode::Nil => f.write_str("nil"),
            Opcode::Bool(b) => write!(f, "bool {b}"),
            Opcode::Float(float) => write!(f, "float {float}"),
            Opcode::Int(i) => write!(f, "int {i}"),
            Opcode::Negate(id) => write!(f, "negate {id}"),
            Opcode::Length(id) => write!(f, "len {id}"),
            Opcode::Not(id) => write!(f, "not {id}"),
            Opcode::BitNot(id) => write!(f, "bit_not {id}"),
            Opcode::Plus { left, right } => write!(f, "plus {left} {right}"),
            Opcode::Minus { left, right } => write!(f, "minus {left} {right}"),
            Opcode::Multiply { left, right } => write!(f, "multiply {left} {right}"),
            Opcode::Divide { left, right } => write!(f, "divide {left} {right}"),
            Opcode::FloorDivide { left, right } => write!(f, "floor_divide {left} {right}"),
            Opcode::Exponent { left, right } => write!(f, "exponent {left} {right}"),
            Opcode::Modulo { left, right } => write!(f, "modulo {left} {right}"),
            Opcode::BitAnd { left, right } => write!(f, "bit_and {left} {right}"),
            Opcode::BitXor { left, right } => write!(f, "bit_xor {left} {right}"),
            Opcode::BitOr { left, right } => write!(f, "bit_or {left} {right}"),
            Opcode::RightShift { left, right } => write!(f, "right_shift {left} {right}"),
            Opcode::LeftShift { left, right } => write!(f, "left_shift {left} {right}"),
            Opcode::Concat { left, right } => write!(f, "concat {left} {right}"),
            Opcode::Less { left, right } => write!(f, "less {left} {right}"),
            Opcode::LessEqual { left, right } => write!(f, "less_equal {left} {right}"),
            Opcode::Greater { left, right } => write!(f, "greater {left} {right}"),
            Opcode::GreaterEqual { left, right } => write!(f, "greater_equal {left} {right}"),
            Opcode::Equal { left, right } => write!(f, "equal {left} {right}"),
            Opcode::NotEqual { left, right } => write!(f, "not_Equal {left} {right}"),
            Opcode::And { left, right } => write!(f, "and {left} {right}"),
            Opcode::Or { left, right } => write!(f, "or {left} {right}"),
        }
    }

    /// Print the contents of a block, assuming that its name has already been printed
    fn block(&mut self, f: &mut fmt::Formatter<'_>, ops: &Vec<Instruction>) -> fmt::Result {
        write!(f, "[{}]", ops.len())?;

        let size = ops.size() + std::mem::size_of::<Instruction>();
        if self.top_level || (size as f64) > (self.top_size as f64) * 0.2 {
            write!(f, " <{}>", Size(size))?;
        }

        f.write_str(" {")?;
        if ops.is_empty() {
            f.write_str(" }")?;
            return Ok(());
        }

        writeln!(f)?;
        self.depth += 1;
        for op in ops {
            self.instruction(f, op)?;
            writeln!(f)?;
        }

        self.depth -= 1;
        self.depth(f)?;
        f.write_str("}")?;

        Ok(())
    }

    /// Print the indentation at the start of a line
    fn depth(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.depth {
            f.write_str("    ")?;
        }

        Ok(())
    }
}

impl Display for HirPrint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hir")?;
        let mut this = *self;
        this.block(f, &self.hir.instructions)
    }
}

impl Display for InstructionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}
