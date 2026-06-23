use std::borrow::Cow;

use crate::parser::{ast, lexer::Token};

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
    locals: Vec<Variable>,
    globals: HashMap<nt::StringId, nt::VariableId>,
    scope_depth: usize,
    function_depth: usize,
}

/// All state for variable resolution
#[derive(Debug, Clone)]
enum Variable {
    Var {
        depth: usize,
        id: nt::VariableId,
        // if the variable is limited to being used in a specified function and should
        // not be resolved in a deeper function
        func_depth: Option<usize>,
    },
    Collective {
        depth: usize,
        attr_const: bool,
    },
    /// Refer to an existing variable with a different name.  Note that this is
    /// only referencing a variable, alias -> alias is not supported.
    Alias {
        depth: usize,
        name: nt::StringId,
        id: nt::VariableId,
    },
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
            globals: HashMap::new(),
            scope_depth: 0,
            function_depth: 0,
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

        let mut close = vec![];
        self.scope_leave(&mut close);

        nt::Block {
            statements,
            ret_stat,
            close,
        }
    }

    /// Resolve a statement
    fn statement(&mut self, s: &ast::Statement, out: &mut Vec<nt::Statement>) {
        match s {
            ast::Statement::Empty => out.push(nt::Statement::Empty),
            ast::Statement::Assign { vars, exps } => out.push(self.assign(vars, exps)),
            ast::Statement::Call(call) => out.push(nt::Statement::Call(self.call(call))),
            ast::Statement::Label(label) => todo!(),
            ast::Statement::Break => todo!(),
            ast::Statement::Goto(token) => todo!(),
            ast::Statement::Block(block) => out.push(nt::Statement::Block(self.block(block))),
            ast::Statement::While { expr, block } => out.push(nt::Statement::While {
                expr: self.expr(expr),
                block: self.block(block),
            }),
            ast::Statement::Repeat { block, expr } => out.push(self.repeat(block, expr)),
            ast::Statement::If {
                expr,
                block,
                elseif,
                else_block,
            } => out.push(nt::Statement::If {
                expr: self.expr(expr),
                block: self.block(block),
                elseif: elseif
                    .iter()
                    .map(|(expr, block)| (self.expr(expr), self.block(block)))
                    .collect(),
                else_block: else_block.as_ref().map(|b| self.block(b)),
            }),
            ast::Statement::For {
                name,
                initial,
                limit,
                step,
                block,
            } => out.push(self.for_loop(*name, initial, limit, step.as_ref(), block)),
            ast::Statement::ForEach {
                names,
                exprs,
                block,
            } => out.push(self.for_each(names, exprs, &block)),
            ast::Statement::Function { name, body, vis } => todo!(),
            ast::Statement::Declare { vis, names, exprs } => self.declare(*vis, names, exprs, out),
            ast::Statement::GlobalCollective { attrib } => {
                if let Some(attr) = attrib
                    && attr.name.value != "const"
                {
                    panic!("Unknown attribute: {}", attr.name.value);
                }

                let attr_const = attrib.is_some();
                self.locals.push(Variable::Collective {
                    depth: self.scope_depth,
                    attr_const,
                });
            }
        }
    }

    /// Resolve an assignment
    fn assign(&mut self, vars: &[ast::Var], exps: &[ast::Expression]) -> nt::Statement {
        let vars = vars
            .iter()
            .map(|v| {
                if let ast::Var::Name(tok) = v {
                    let var = self.resolve(tok.value);
                    let var = &self.variable_table[var.0 as usize];
                    let is_const = match var {
                        name_tree::Variable::Local(local) => local.attr_const,
                        name_tree::Variable::Global(global) => global.attr_const,
                    };
                    if is_const {
                        panic!("Attempt to assign to const variable `{}`", tok.value);
                    }
                }
                self.var(v)
            })
            .collect();
        nt::Statement::Assign {
            vars,
            exps: exps.iter().map(|e| self.expr(e)).collect(),
        }
    }

    /// Resolve a repeat statement
    fn repeat(&mut self, block: &ast::Block, expr: &ast::Expression) -> nt::Statement {
        // this is not using the normal `self.block` method as we need to extend
        // the scope of the repeat block into the controlling expression after
        // the block finishes.  `repeat local a = 5 until a < 3` should be valid.

        self.scope_enter();

        let mut statements = Vec::with_capacity(block.statements.len());
        for s in &block.statements {
            self.statement(s, &mut statements);
        }
        let ret_stat = block.ret_stat.as_ref().map(|s| self.ret_stat(s));

        let expr = self.expr(expr);

        let mut leave = vec![];
        self.scope_leave(&mut leave);

        nt::Statement::Repeat {
            block: nt::Block {
                statements,
                ret_stat,
                close: vec![],
            },
            expr,
            block_end: leave,
        }
    }

    /// Resolve a for loop
    fn for_loop(
        &mut self,
        name: Token,
        initial: &ast::Expression,
        limit: &ast::Expression,
        step: Option<&ast::Expression>,
        block: &ast::Block,
    ) -> nt::Statement {
        let initial = self.expr(initial);
        let limit = self.expr(limit);
        let step = step.map(|s| self.expr(s));

        self.scope_enter();

        let var = nt::VariableId(
            self.variable_table
                .len()
                .try_into()
                .expect("Too many variables within module"),
        );
        let s = self.insert_string(name.value.as_bytes());
        self.variable_table.push(nt::Variable::Local(nt::Local {
            line: Some(name.line),
            name: s,
            attr_close: false,
            attr_const: true,
        }));
        self.locals.push(Variable::Var {
            depth: self.scope_depth,
            id: var,
            func_depth: None,
        });

        let block = self.block(block);

        // leave will only close the loop variable which isn't close
        self.scope_leave(&mut vec![]);

        nt::Statement::For {
            name: var,
            initial,
            limit,
            step,
            block,
        }
    }

    /// Resolve a for each loop
    fn for_each(
        &mut self,
        names: &[Token],
        exprs: &[ast::Expression],
        block: &ast::Block,
    ) -> nt::Statement {
        let exprs = exprs.iter().map(|e| self.expr(e)).collect();

        self.scope_enter();

        let names = names
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let var = nt::VariableId(
                    self.variable_table
                        .len()
                        .try_into()
                        .expect("Too many variables within module"),
                );
                let s = self.insert_string(name.value.as_bytes());
                self.variable_table.push(nt::Variable::Local(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: false,
                    attr_const: idx == 0,
                }));
                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                var
            })
            .collect();

        let block = self.block(block);

        // leave will only close the loop variable which isn't close
        self.scope_leave(&mut vec![]);

        nt::Statement::ForEach {
            names,
            exprs,
            block,
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
            ast::Visibility::Local => self.declare_local(names, exprs, out),
            ast::Visibility::Global => self.declare_global(names, exprs, out),
        }
    }

    /// Resolve a local variable declaration
    fn declare_local(
        &mut self,
        names: &ast::AttributeNameList,
        exprs: &[ast::Expression],
        out: &mut Vec<nt::Statement>,
    ) {
        let values = exprs.iter().map(|e| self.expr(e)).collect();

        let mut all_const = false;
        let mut all_close = false;

        if let Some(all) = &names.attrib {
            let name = all.name.value;
            if name == "const" {
                all_const = true;
            } else if name == "close" {
                all_close = true;
                if names.names.len() > 1 {
                    panic!("The `close` attribute can only be applied to one variable at a time");
                }
            } else {
                panic!("Unknown attribute: {name}");
            }
        }

        let names = names
            .names
            .iter()
            .map(|(name, attr)| {
                let s = self.insert_string(name.value.as_bytes());

                let mut is_close = all_close;
                let mut is_const = all_const;

                if let Some(attr) = attr {
                    let name = attr.name.value;
                    if name == "const" {
                        is_const = true;
                    } else if name == "close" {
                        is_close = true;
                    } else {
                        panic!("Unknown attribute: {name}");
                    }
                }

                let var = nt::VariableId(
                    self.variable_table
                        .len()
                        .try_into()
                        .expect("Too many variables within module"),
                );
                self.variable_table.push(nt::Variable::Local(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: is_close,
                    attr_const: is_const,
                }));

                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                out.push(nt::Statement::ScopeStart(var));
                nt::Var::Name(var)
            })
            .collect();

        if !exprs.is_empty() {
            out.push(nt::Statement::Assign {
                vars: names,
                exps: values,
            });
        }
    }

    /// Resolve a global variable declaration
    fn declare_global(
        &mut self,
        names: &ast::AttributeNameList,
        exprs: &[ast::Expression],
        out: &mut Vec<nt::Statement>,
    ) {
        let values = exprs.iter().map(|e| self.expr(e)).collect();

        let mut all_const = false;

        if let Some(all) = &names.attrib {
            let name = all.name.value;
            if name == "const" {
                all_const = true;
            } else if name == "close" {
                panic!("The `close` attribute can only be applied to local variables");
            } else {
                panic!("Unknown attribute: {name}");
            }
        }

        let names = names
            .names
            .iter()
            .map(|(name, attr)| {
                let s = self.insert_string(name.value.as_bytes());

                let mut is_const = all_const;

                if let Some(attr) = attr {
                    let name = attr.name.value;
                    if name == "const" {
                        is_const = true;
                    } else if name == "close" {
                        panic!("The `close` attribute can only be applied to local variables");
                    } else {
                        panic!("Unknown attribute: {name}");
                    }
                }

                let var = nt::VariableId(
                    self.variable_table
                        .len()
                        .try_into()
                        .expect("Too many variables within module"),
                );
                self.variable_table.push(nt::Variable::Global(nt::Global {
                    line: Some(name.line),
                    name: s,
                    attr_const: is_const,
                }));

                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                nt::Var::Name(var)
            })
            .collect();

        if !exprs.is_empty() {
            out.push(nt::Statement::Assign {
                vars: names,
                exps: values,
            });
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
            ast::Expression::DotDotDot => nt::Expression::DotDotDot(self.resolve("...")),
            ast::Expression::Function(function) => {
                nt::Expression::Function(self.function(function))
            }
            ast::Expression::Prefix(pre) => nt::Expression::Prefix(self.prefix(pre)),
            ast::Expression::Table(field_list) => {
                nt::Expression::Table(self.field_list(field_list))
            }
            ast::Expression::Binary { left, op, right } => nt::Expression::Binary {
                left: Box::new(self.expr(left)),
                op: *op,
                right: Box::new(self.expr(right)),
            },
            ast::Expression::Unary { expr, op } => nt::Expression::Unary {
                expr: Box::new(self.expr(expr)),
                op: *op,
            },
        }
    }

    /// Resolve a function expression (lambda)
    fn function(&mut self, function: &ast::Function) -> nt::Function {
        self.scope_enter();
        self.function_depth += 1;

        let names = function
            .parameters
            .names
            .iter()
            .map(|name| {
                let var = nt::VariableId(
                    self.variable_table
                        .len()
                        .try_into()
                        .expect("Too many variables within module"),
                );
                let s = self.insert_string(name.value.as_bytes());
                self.variable_table.push(nt::Variable::Local(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: false,
                    attr_const: false,
                }));
                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                var
            })
            .collect();

        let var_name = function.parameters.var_name.map(|var_arg| {
            // we have a variadic argument, so create its `...` variable
            let var = nt::VariableId(
                self.variable_table
                    .len()
                    .try_into()
                    .expect("Too many variables within module"),
            );
            let s = self.insert_string(b"...");
            self.variable_table.push(nt::Variable::Local(nt::Local {
                line: None,
                name: s,
                attr_close: false,
                attr_const: false,
            }));
            self.locals.push(Variable::Var {
                depth: self.scope_depth,
                id: var,
                // `...` is only available directly inside a variadic function
                // which implies it cannot be resolved from a deeper function
                func_depth: Some(self.function_depth),
            });

            if let Some(name) = var_arg {
                // named var arg, also allow accessing the SAME variable through
                // a different name

                let s = self.insert_string(name.value.as_bytes());
                self.locals.push(Variable::Alias {
                    depth: self.scope_depth,
                    id: var,
                    name: s,
                });
            }
            var
        });

        let parameters = nt::ParameterList {
            self_var: None,
            names,
            var_name,
        };

        let body = self.block(&function.body);

        // only contains function parameters which cant be <close>
        self.scope_leave(&mut vec![]);
        self.function_depth -= 1;

        nt::Function { parameters, body }
    }

    /// Resolve a prefix expression
    fn prefix(&mut self, pre: &ast::PrefixExpression) -> nt::PrefixExpression {
        match pre {
            ast::PrefixExpression::Var(var) => nt::PrefixExpression::Var(Box::new(self.var(var))),
            ast::PrefixExpression::Call(call) => nt::PrefixExpression::Call(self.call(call)),
            ast::PrefixExpression::Expr(expression) => {
                nt::PrefixExpression::Expr(Box::new(self.expr(expression)))
            }
        }
    }

    /// Resolve a function call
    fn call(&mut self, call: &ast::FunctionCall) -> nt::FunctionCall {
        nt::FunctionCall {
            receiver: Box::new(self.prefix(&call.receiver)),
            method_name: call
                .method_name
                .map(|n| self.insert_string(n.value.as_bytes())),
            args: self.args(&call.args),
        }
    }

    /// Resolve function arguments
    fn args(&mut self, args: &ast::FunctionArgs) -> nt::FunctionArgs {
        match args {
            ast::FunctionArgs::Call { exprs } => nt::FunctionArgs::Call {
                exprs: exprs.iter().map(|e| self.expr(e)).collect(),
            },
            ast::FunctionArgs::Table { table } => nt::FunctionArgs::Table {
                table: self.field_list(table),
            },
            ast::FunctionArgs::String { value } => nt::FunctionArgs::String {
                value: self.insert_string(value.value.as_bytes()),
            },
        }
    }

    /// Resolve a var expression
    fn var(&mut self, var: &ast::Var) -> nt::Var {
        match var {
            ast::Var::Name(token) => nt::Var::Name(self.resolve(token.value)),
            ast::Var::Index { first, index } => nt::Var::Index {
                first: self.prefix(first),
                index: self.expr(index),
            },
            ast::Var::Dot { first, name } => nt::Var::Dot {
                first: self.prefix(first),
                name: self.insert_string(name.value.as_bytes()),
            },
        }
    }

    /// Resolve a table constructor
    fn field_list(&mut self, fields: &ast::FieldList) -> nt::FieldList {
        nt::FieldList {
            fields: fields
                .fields
                .iter()
                .map(|f| match f {
                    ast::Field::Index { index, expr } => nt::Field::Index {
                        index: self.expr(index),
                        expr: self.expr(expr),
                    },
                    ast::Field::Assign { name, expr } => nt::Field::Assign {
                        name: self.insert_string(name.value.as_bytes()),
                        expr: self.expr(expr),
                    },
                    ast::Field::Exp { expr } => nt::Field::Exp {
                        expr: self.expr(expr),
                    },
                })
                .collect(),
        }
    }

    /// Get a variable id for a variable name
    fn resolve(&mut self, name: &str) -> nt::VariableId {
        #[derive(PartialEq)]
        enum ResolveState {
            Global,
            Collective,
            CollectiveConst,
            None,
        }

        let s = self.insert_string(name.as_bytes());

        let mut state = ResolveState::None;

        // iterate through a stack, top first (fifo)
        for l in self.locals.iter().rev() {
            match l {
                Variable::Var { id, func_depth, .. } => {
                    let var = &self.variable_table[id.0 as usize];
                    if state == ResolveState::None && matches!(var, nt::Variable::Global(_)) {
                        state = ResolveState::Global;
                    }

                    let name = match var {
                        nt::Variable::Local(local) => local.name,
                        nt::Variable::Global(global) => global.name,
                    };
                    if name == s
                        && (func_depth.is_none() || *func_depth == Some(self.function_depth))
                    {
                        return *id;
                    }
                }
                Variable::Collective { attr_const, .. } => {
                    // found a `global *`
                    if state != ResolveState::None {
                        continue;
                    }
                    if *attr_const {
                        state = ResolveState::CollectiveConst
                    } else {
                        state = ResolveState::Collective
                    }
                }
                Variable::Alias { name, id, .. } => {
                    let var = &self.variable_table[id.0 as usize];
                    if state == ResolveState::None && matches!(var, nt::Variable::Global(_)) {
                        state = ResolveState::Global;
                    }

                    if *name == s {
                        return *id;
                    }
                }
            }
        }

        // did not find anything already declared, so work out what to do now
        let attr_const = match state {
            ResolveState::Global => {
                // found a `global var_name` declaration first, so error on undef
                panic!("Undefined variable: `{name}`")
            }
            ResolveState::None | ResolveState::Collective => false,
            ResolveState::CollectiveConst => true,
        };

        // try to find a global, if not, create one with the given `const` attr
        if let Some(&g) = self.globals.get(&s) {
            return g;
        }

        let var = nt::VariableId(
            self.variable_table
                .len()
                .try_into()
                .expect("Too many variables within module"),
        );

        // This code creates the global within the `if false`, where is is
        // mutable, so its use outside of the never-executed code still believes
        // it is mutable, as mutability is a property of the global, rather than
        // something that is lexically scoped.  This is considered to be intentional,
        // although we should probably provide a warning somehow? warnings are
        // not at all a priority yet.
        // ```lua
        // global<const>*
        // if false then global a = 5 end
        // a = 6 -- a isn't constant here, due to the previous global declaration
        // ```
        self.variable_table.push(nt::Variable::Global(nt::Global {
            line: None,
            name: s,
            attr_const,
        }));

        self.globals.insert(s, var);
        var
    }

    /// Enter a lexical scope
    fn scope_enter(&mut self) {
        self.scope_depth += 1;
    }

    /// Leave a lexical scope
    fn scope_leave(&mut self, out: &mut Vec<nt::Statement>) {
        self.scope_depth -= 1;

        while let Some(last) = self.locals.last() {
            let depth = match last {
                Variable::Var { depth, .. } => *depth,
                Variable::Collective { depth, .. } => *depth,
                Variable::Alias { depth, .. } => *depth,
            };

            if depth > self.scope_depth {
                if let Variable::Var { id: var, .. } = last
                    && matches!(self.variable_table[var.0 as usize], nt::Variable::Local(_))
                {
                    out.push(nt::Statement::ScopeEnd(*var));
                }

                self.locals.pop();
            } else {
                break;
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
