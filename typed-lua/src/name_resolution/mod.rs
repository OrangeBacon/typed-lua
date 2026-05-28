use crate::parser::ast;

mod name_tree;

use name_tree as nt;

/// Run the name resolution pass over a parsed AST.
#[derive(Debug, Clone)]
pub struct Resolver<'a> {
    ast: &'a ast::Block<'a>,

    string_table: Vec<String>,
    number_table: Vec<nt::Number>,
    variable_table: Vec<nt::Variable>,
}

impl<'a> Resolver<'a> {
    /// Create a new name resolver
    pub fn new(ast: &'a ast::Block<'a>) -> Self {
        Self {
            ast,
            string_table: vec![],
            number_table: vec![],
            variable_table: vec![],
        }
    }

    /// Get the resolved tree for the input ast.
    pub fn run(mut self) -> nt::NameContainer<nt::Block> {
        nt::NameContainer {
            tree: self.block(self.ast),
            string_table: self.string_table,
            number_table: self.number_table,
            variable_table: self.variable_table,
        }
    }

    /// Resolve a block.
    fn block(&mut self, block: &ast::Block) -> nt::Block {
        let statements = Vec::with_capacity(block.statements.len());

        for s in &block.statements {
            self.statement(s);
        }

        let ret_stat = block.ret_stat.as_ref().map(|s| self.ret_stat(s));

        nt::Block {
            statements,
            ret_stat,
        }
    }

    /// Resolve a statement
    fn statement(&mut self, _s: &ast::Statement) {
        todo!()
    }

    /// Resolve a return statement
    fn ret_stat(&mut self, ret: &ast::ReturnStatement) -> nt::ReturnStatement {
        nt::ReturnStatement {
            exprs: ret.exprs.iter().map(|e| self.expr(e)).collect(),
        }
    }

    /// Resolve an expression
    fn expr(&mut self, expr: &ast::Expression) -> nt::Expression {
        match expr {
            ast::Expression::Nil => nt::Expression::Nil,
            ast::Expression::False => nt::Expression::Bool(false),
            ast::Expression::True => nt::Expression::Bool(true),
            ast::Expression::Number(token) => todo!(),
            ast::Expression::String(token) => todo!(),
            ast::Expression::DotDotDot => todo!(),
            ast::Expression::Function(function) => todo!(),
            ast::Expression::Prefix(prefix_expression) => todo!(),
            ast::Expression::Table(field_list) => todo!(),
            ast::Expression::Binary { left, op, right } => todo!(),
            ast::Expression::Unary { expr, op } => todo!(),
        }
    }
}
