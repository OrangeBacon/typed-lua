use crate::{
    hir::hir_tree::*,
    name_resolution::name_tree as nt,
};

pub mod hir_print;
mod hir_size;
mod hir_tree;

pub struct HirBuilder<'a> {
    tree: &'a nt::NameContainer<nt::Block>,

    instruction_count: u32,
}

impl<'a> HirBuilder<'a> {
    /// Create a builder to convert the name tree into HIR.
    pub fn new(tree: &'a nt::NameContainer<nt::Block>) -> Self {
        Self {
            tree,
            instruction_count: 0,
        }
    }

    /// Get the HIR from the builder.
    pub fn run(mut self) -> Hir {
        Hir {
            instructions: self.block(&self.tree.tree),
            strings: self.tree.string_table.clone(),
        }
    }

    /// convert a block statement
    fn block(&mut self, block: &nt::Block) -> Vec<Instruction> {
        let mut out = vec![];

        for stmt in &block.statements {
            todo!()
        }

        if let Some(ret) = &block.ret_stat {
            let exprs: Vec<_> = ret.exprs.iter().map(|e| self.expr(&mut out, e)).collect();
            let multi = self.tuple(&mut out, &exprs);
            self.inst(&mut out, Opcode::Return { value: multi });
        }

        for close in &block.close {
            todo!()
        }

        out
    }

    /// convert an expression
    fn expr(&mut self, out: &mut Vec<Instruction>, expr: &nt::Expression) -> InstructionId {
        match expr {
            nt::Expression::Nil => self.inst(out, Opcode::Nil),
            nt::Expression::Bool(b) => self.inst(out, Opcode::Bool(*b)),
            nt::Expression::Number(nt::Number::Float(f)) => self.inst(out, Opcode::Float(*f)),
            nt::Expression::Number(nt::Number::Integer(i)) => self.inst(out, Opcode::Int(*i)),
            nt::Expression::String(s) => self.inst(out, Opcode::String(*s)),
            nt::Expression::Function(function) => todo!(),
            nt::Expression::Prefix(prefix_expression) => todo!(),
            nt::Expression::Table(field_list) => todo!(),
            nt::Expression::Binary { left, op, right } => {
                let left = self.expr(out, &left);
                let right = self.expr(out, &right);

                self.inst(
                    out,
                    Opcode::Binary {
                        left,
                        op: *op,
                        right,
                    },
                )
            }
            nt::Expression::Unary { expr, op } => {
                let inner = self.expr(out, expr);
                self.inst(out, Opcode::Unary(*op, inner))
            }
        }
    }

    /// Create a tuple contiaining all provided values.  If only one value
    /// is provided, return that single value.  Used for returning multiple values.
    /// Returns None if no input is provided.
    fn tuple(
        &mut self,
        out: &mut Vec<Instruction>,
        values: &[InstructionId],
    ) -> Option<InstructionId> {
        match values {
            [] => None,
            [val] => Some(*val),
            _ => Some(self.inst(
                out,
                Opcode::Tuple {
                    values: values.to_vec(),
                },
            )),
        }
    }

    /// Create an instruction, add it to the list and return its ID.
    fn inst(&mut self, out: &mut Vec<Instruction>, op: Opcode) -> InstructionId {
        let id = InstructionId(self.instruction_count);
        self.instruction_count += 1;

        out.push(Instruction { id, opcode: op });

        id
    }
}
