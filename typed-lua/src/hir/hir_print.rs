use std::fmt::{self, Display};

use crate::{
    hir::hir_tree::*,
    utils::{Size, SizeOf, fmt_bstr},
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
                self.block(f, instructions, instructions)
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
            Opcode::String(s) => write!(
                f,
                "string \"{}\"",
                fmt_bstr(&self.hir.strings[s.0 as usize])
            ),
            Opcode::Binary { left, op, right } => write!(f, "{op:?} {left} {right}"),
            Opcode::Unary(op, expr) => write!(f, "{op:?} {expr}"),
        }
    }

    /// Print the contents of a block, assuming that its name has already been printed
    fn block(
        &mut self,
        f: &mut fmt::Formatter<'_>,
        ops: &Vec<Instruction>,
        size: impl SizeOf,
    ) -> fmt::Result {
        write!(f, "[{}]", ops.len())?;

        let size = size.size();
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
        this.block(f, &self.hir.instructions, self.hir)
    }
}

impl Display for InstructionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}
