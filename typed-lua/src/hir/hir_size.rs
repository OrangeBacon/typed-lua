use crate::{hir::hir_tree::*, utils::SizeOf};

impl SizeOf for Hir {
    fn size(&self) -> usize {
        let Hir { instructions } = self;
        instructions.size()
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
            Opcode::Negate(instruction_id) => instruction_id.size(),
            Opcode::Length(instruction_id) => instruction_id.size(),
            Opcode::Not(instruction_id) => instruction_id.size(),
            Opcode::BitNot(instruction_id) => instruction_id.size(),
        }
    }
}
