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
            Opcode::Negate(instruction_id) => instruction_id.size(),
            Opcode::Length(instruction_id) => instruction_id.size(),
            Opcode::Not(instruction_id) => instruction_id.size(),
            Opcode::BitNot(instruction_id) => instruction_id.size(),
            Opcode::Plus { left, right } => left.size() + right.size(),
            Opcode::Minus { left, right } => left.size() + right.size(),
            Opcode::Multiply { left, right } => left.size() + right.size(),
            Opcode::Divide { left, right } => left.size() + right.size(),
            Opcode::FloorDivide { left, right } => left.size() + right.size(),
            Opcode::Exponent { left, right } => left.size() + right.size(),
            Opcode::Modulo { left, right } => left.size() + right.size(),
            Opcode::BitAnd { left, right } => left.size() + right.size(),
            Opcode::BitXor { left, right } => left.size() + right.size(),
            Opcode::BitOr { left, right } => left.size() + right.size(),
            Opcode::RightShift { left, right } => left.size() + right.size(),
            Opcode::LeftShift { left, right } => left.size() + right.size(),
            Opcode::Concat { left, right } => left.size() + right.size(),
            Opcode::Less { left, right } => left.size() + right.size(),
            Opcode::LessEqual { left, right } => left.size() + right.size(),
            Opcode::Greater { left, right } => left.size() + right.size(),
            Opcode::GreaterEqual { left, right } => left.size() + right.size(),
            Opcode::Equal { left, right } => left.size() + right.size(),
            Opcode::NotEqual { left, right } => left.size() + right.size(),
            Opcode::And { left, right } => left.size() + right.size(),
            Opcode::Or { left, right } => left.size() + right.size(),
        }
    }
}
