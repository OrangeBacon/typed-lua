use crate::{hir::hir_tree::*, utils::SizeOf};

impl SizeOf for Hir {
    fn size(&self) -> usize {
        let Hir {
            instructions,
            strings,
        } = self;
        instructions.size() + strings.size()
    }
}

impl SizeOf for InstructionId {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for Instruction {
    fn size(&self) -> usize {
        let Instruction { id, opcode } = self;
        id.size() + opcode.size()
    }
}

impl SizeOf for Opcode {
    fn size(&self) -> usize {
        match self {
            Opcode::Block { instructions } => instructions.size(),
            Opcode::Return { value } => value.size(),
            Opcode::Tuple { values } => values.size(),
            Opcode::Nil => 0,
            Opcode::Bool(b) => b.size(),
            Opcode::Float(f) => f.size(),
            Opcode::Int(i) => i.size(),
            Opcode::String(s) => s.size(),
            Opcode::Binary { left, op, right } => left.size() + op.size() + right.size(),
            Opcode::Unary(op, expr) => op.size() + expr.size(),
        }
    }
}
