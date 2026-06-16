use std::borrow::Cow;

use crate::parser::ast;

mod literal;
mod name_tree;

use hashbrown::{HashMap, hash_map::EntryRef};
use name_tree as nt;

/// Run the name resolution pass over a parsed AST.
#[derive(Debug, Clone)]
pub struct Resolver<'a> {
    ast: &'a ast::Block<'a>,

    string_table: Vec<Vec<u8>>,
    string_lookup: HashMap<Vec<u8>, nt::StringId>,

    number_table: Vec<nt::Number>,

    variable_table: Vec<nt::Variable>,
    locals: Vec<Local>,
    scope_depth: usize,
}

/// All state for variable resolution
#[derive(Debug, Clone)]
struct Local {
    depth: usize,
    var: nt::VariableId,
}

impl<'a> Resolver<'a> {
    /// Create a new name resolver
    pub fn new(ast: &'a ast::Block<'a>) -> Self {
        Self {
            ast,
            string_table: vec![],
            string_lookup: HashMap::new(),
            number_table: vec![],
            variable_table: vec![],
            locals: vec![],
            scope_depth: 0,
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
        self.scope_enter();

        let mut statements = Vec::with_capacity(block.statements.len());
        for s in &block.statements {
            self.statement(s, &mut statements);
        }
        let ret_stat = block.ret_stat.as_ref().map(|s| self.ret_stat(s));

        self.scope_leave();

        nt::Block {
            statements,
            ret_stat,
        }
    }

    /// Resolve a statement
    fn statement(&mut self, s: &ast::Statement, out: &mut Vec<nt::Statement>) {
        match s {
            ast::Statement::Empty => out.push(nt::Statement::Empty),
            ast::Statement::Assign { vars, exps } => todo!(),
            ast::Statement::Call(function_call) => todo!(),
            ast::Statement::Label(label) => todo!(),
            ast::Statement::Break => todo!(),
            ast::Statement::Goto(token) => todo!(),
            ast::Statement::Block(block) => todo!(),
            ast::Statement::While { expr, block } => todo!(),
            ast::Statement::Repeat { block, expr } => todo!(),
            ast::Statement::If {
                expr,
                block,
                elseif,
                else_block,
            } => todo!(),
            ast::Statement::For {
                name,
                initial,
                limit,
                step,
                block,
            } => todo!(),
            ast::Statement::ForEach {
                names,
                exprs,
                block,
            } => todo!(),
            ast::Statement::Function { name, body, vis } => todo!(),
            ast::Statement::Declare { vis, names, exprs } => self.declare(*vis, names, exprs, out),
            ast::Statement::GlobalCollective { attrib } => todo!(),
        }
    }

    /// Resolve a return statement
    fn ret_stat(&mut self, ret: &ast::ReturnStatement) -> nt::ReturnStatement {
        nt::ReturnStatement {
            exprs: ret.exprs.iter().map(|e| self.expr(e)).collect(),
        }
    }

    /// Resolve a declaration statement
    fn declare(
        &mut self,
        vis: ast::Visibility,
        names: &ast::AttributeNameList,
        exprs: &[ast::Expression],
        out: &mut Vec<nt::Statement>,
    ) {
        match vis {
            ast::Visibility::Local => {
                let value = self.expr(&exprs[0]);

                let name = names.names[0].0;
                let s = self.insert_string(name.value.as_bytes());

                let var = nt::VariableId(
                    self.variable_table
                        .len()
                        .try_into()
                        .expect("Too many variables within module"),
                );
                self.variable_table.push(nt::Variable::Local(nt::Local {
                    line: name.line,
                    name: s,
                    attr_close: false,
                    attr_const: false,
                }));

                self.locals.push(Local {
                    depth: self.scope_depth,
                    var,
                });
                out.push(nt::Statement::ScopeStart(var));
                out.push(nt::Statement::Assign {
                    vars: vec![nt::Var::Name(var)],
                    exps: vec![value],
                });
            }
            ast::Visibility::Global => todo!(),
        }
    }

    /// Resolve an expression
    fn expr(&mut self, expr: &ast::Expression) -> nt::Expression {
        match expr {
            ast::Expression::Nil => nt::Expression::Nil,
            ast::Expression::False => nt::Expression::Bool(false),
            ast::Expression::True => nt::Expression::Bool(true),
            ast::Expression::Number(tok) => nt::Expression::Number(self.number(*tok)),
            ast::Expression::String(tok) => nt::Expression::String(self.string(*tok)),
            ast::Expression::DotDotDot => todo!(),
            ast::Expression::Function(function) => todo!(),
            ast::Expression::Prefix(pre) => nt::Expression::Prefix(self.prefix(pre)),
            ast::Expression::Table(field_list) => todo!(),
            ast::Expression::Binary { left, op, right } => todo!(),
            ast::Expression::Unary { expr, op } => todo!(),
        }
    }

    /// Resolve a prefix expression
    fn prefix(&mut self, pre: &ast::PrefixExpression) -> nt::PrefixExpression {
        match pre {
            ast::PrefixExpression::Var(var) => nt::PrefixExpression::Var(Box::new(self.var(var))),
            ast::PrefixExpression::Call(function_call) => todo!(),
            ast::PrefixExpression::Expr(expression) => todo!(),
        }
    }

    /// Resolve a var expression
    fn var(&mut self, var: &ast::Var) -> nt::Var {
        match var {
            ast::Var::Name(token) => nt::Var::Name(self.resolve(token.value)),
            ast::Var::Index { first, index } => todo!(),
            ast::Var::Dot { first, name } => todo!(),
        }
    }

    /// Get a variable id for a variable name
    fn resolve(&mut self, name: &str) -> nt::VariableId {
        let s = self.insert_string(name.as_bytes());

        for l in self.locals.iter().rev() {
            let id = l.var;
            let var = &self.variable_table[id.0 as usize];
            let name = match var {
                name_tree::Variable::Local(local) => local.name,
                name_tree::Variable::Global(global) => global.name,
            };
            if name == s {
                return id;
            }
        }

        todo!()
    }

    /// Enter a lexical scope
    fn scope_enter(&mut self) {
        self.scope_depth += 1;
    }

    /// Leave a lexical scope
    fn scope_leave(&mut self) {
        self.scope_depth -= 1;

        while let Some(last) = self.locals.last() {
            if last.depth > self.scope_depth {
                self.locals.pop();
            }
        }
    }

    /// Insert a string into the string table.
    fn insert_string<'b>(&mut self, s: impl Into<Cow<'b, [u8]>>) -> nt::StringId {
        let id = nt::StringId(
            self.string_table
                .len()
                .try_into()
                .expect("Too many strings within module"),
        );
        let s = s.into();

        match self.string_lookup.entry_ref(s.as_ref()) {
            EntryRef::Occupied(entry) => *entry.get(),
            EntryRef::Vacant(entry) => {
                self.string_table.push(s.to_vec());
                *entry.insert_entry(id).get()
            }
        }
    }
}
