use std::borrow::Cow;

use crate::parser::{ast, lexer::Token};

mod literal;
mod name_tree;
pub mod name_tree_print;
mod name_tree_size;

use hashbrown::{HashMap, hash_map::EntryRef};
use name_tree as nt;

/// Name of the `_ENV` variable in lua source code
const ENVIRONMENT: &str = "_ENV";

/// Run the name resolution pass over a parsed AST.
#[derive(Debug, Clone)]
pub struct Resolver<'a> {
    ast: &'a ast::Block<'a>,

    string_table: Vec<Vec<u8>>,
    string_lookup: HashMap<Vec<u8>, nt::StringId>,

    variable_table: Vec<nt::Local>,
    locals: Vec<Variable>,
    scope_depth: usize,
    function_depth: usize,

    label_table: Vec<Label>,
    labels: Vec<nt::LabelId>,
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
    /// Refer to an existing variable with a different name.  Note that this is
    /// only referencing a variable, alias -> alias is not supported.
    Alias {
        depth: usize,
        name: nt::StringId,
        id: nt::VariableId,
    },
}

/// A potentially resolved label
#[derive(Debug, Clone)]
struct Label {
    name: Option<nt::StringId>,

    line: Option<usize>,

    defined: bool,

    func_depth: usize,

    break_target: bool,
}

impl<'a> Resolver<'a> {
    /// Create a new name resolver
    pub fn new(ast: &'a ast::Block<'a>) -> Self {
        let mut this = Self {
            ast,
            string_table: vec![],
            string_lookup: HashMap::new(),
            variable_table: vec![],
            locals: vec![],
            scope_depth: 0,
            function_depth: 0,
            label_table: vec![],
            labels: vec![],
        };

        // Insert the `_ENV` local so there is a base case for global variable
        // resolution, so `self.resolve` doesn't infinitely recurse for global
        // variables
        let env_s = this.insert_string(ENVIRONMENT.as_bytes());
        let var = nt::VariableId(
            this.variable_table
                .len()
                .try_into()
                .expect("Too many variables within module"),
        );
        this.variable_table.push(nt::Local {
            line: None,
            name: env_s,
            attr_close: false,
            attr_const: false,
        });
        this.locals.push(Variable::Var {
            depth: 0,
            id: var,
            func_depth: None,
        });

        this
    }

    /// Get the resolved tree for the input ast.
    pub fn run(mut self) -> nt::NameContainer<nt::Block> {
        let env = self.resolve(ENVIRONMENT);
        let ResolveResult::Local(env) = env else {
            panic!("Top level env lookup");
        };

        let tree = self.block(self.ast);

        let label_table = self
            .label_table
            .into_iter()
            .map(|l| {
                if !l.defined {
                    panic!("Use of un-defined label");
                }
                nt::Label {
                    line: l.line,
                    name: l.name,
                }
            })
            .collect();

        nt::NameContainer {
            tree,
            string_table: self.string_table,
            variable_table: self.variable_table,
            label_table,
            env,
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

        let close = self.scope_leave();

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
            ast::Statement::Label(label) => out.push(self.label(label)),
            ast::Statement::Break => out.push(self.goto_break()),
            ast::Statement::Goto(token) => out.push(self.goto(*token)),
            ast::Statement::Block(block) => out.push(nt::Statement::Block(self.block(block))),
            ast::Statement::While { expr, block } => self.while_loop(out, expr, block),
            ast::Statement::Repeat { block, expr } => self.repeat(out, block, expr),
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
            } => self.for_loop(out, *name, initial, limit, step.as_ref(), block),
            ast::Statement::ForEach {
                names,
                exprs,
                block,
            } => self.for_each(out, names, exprs, block),
            ast::Statement::Function { name, body, vis } => {
                out.push(self.function_statement(name, body, *vis))
            }
            ast::Statement::Declare { vis, names, exprs } => self.declare(*vis, names, exprs, out),
            ast::Statement::GlobalCollective { .. } => {
                // do nothing, pretend this doesn't exist
            }
        }
    }

    /// Resolve an assignment
    fn assign(&mut self, vars: &[ast::Var], exps: &[ast::Expression]) -> nt::Statement {
        let exps = exps.iter().map(|e| self.expr(e)).collect();

        let vars = vars
            .iter()
            .map(|v| {
                if let ast::Var::Name(tok) = v {
                    let var = self.resolve(tok.value);
                    let is_const = match var {
                        ResolveResult::Local(var) => {
                            let var = &self.variable_table[var.0 as usize];
                            var.attr_const
                        }
                        ResolveResult::Global { .. } => false,
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
            exps,
            is_global_init: false,
        }
    }

    fn while_loop(
        &mut self,
        out: &mut Vec<nt::Statement>,
        expr: &ast::Expression,
        block: &ast::Block,
    ) {
        let expr = self.expr(expr);
        let break_id = self.push_break_target();
        let block = self.block(block);

        out.push(nt::Statement::While { expr, block });
        out.push(self.pop_break_target(break_id));
    }

    /// Resolve a repeat statement
    fn repeat(&mut self, out: &mut Vec<nt::Statement>, block: &ast::Block, expr: &ast::Expression) {
        // this is not using the normal `self.block` method as we need to extend
        // the scope of the repeat block into the controlling expression after
        // the block finishes.  `repeat local a = 5 until a < 3` should be valid.

        let break_id = self.push_break_target();
        self.scope_enter();

        let mut statements = Vec::with_capacity(block.statements.len());
        for s in &block.statements {
            self.statement(s, &mut statements);
        }
        let ret_stat = block.ret_stat.as_ref().map(|s| self.ret_stat(s));

        let expr = self.expr(expr);

        let leave = self.scope_leave();

        out.push(nt::Statement::Repeat {
            block: nt::Block {
                statements,
                ret_stat,
                close: vec![],
            },
            expr,
            block_end: leave,
        });
        out.push(self.pop_break_target(break_id));
    }

    /// Resolve a for loop
    fn for_loop(
        &mut self,
        out: &mut Vec<nt::Statement>,
        name: Token,
        initial: &ast::Expression,
        limit: &ast::Expression,
        step: Option<&ast::Expression>,
        block: &ast::Block,
    ) {
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
        self.variable_table.push(nt::Local {
            line: Some(name.line),
            name: s,
            attr_close: false,
            attr_const: true,
        });
        self.locals.push(Variable::Var {
            depth: self.scope_depth,
            id: var,
            func_depth: None,
        });

        let break_id = self.push_break_target();
        let block = self.block(block);

        // leave will only close the loop variable which isn't close
        let _ = self.scope_leave();

        out.push(nt::Statement::For {
            name: var,
            initial,
            limit,
            step,
            block,
        });
        out.push(self.pop_break_target(break_id));
    }

    /// Resolve a for each loop
    fn for_each(
        &mut self,
        out: &mut Vec<nt::Statement>,
        names: &[Token],
        exprs: &[ast::Expression],
        block: &ast::Block,
    ) {
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
                self.variable_table.push(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: false,
                    attr_const: idx == 0,
                });
                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                var
            })
            .collect();

        let break_id = self.push_break_target();
        let block = self.block(block);

        // leave will only close the loop variable which isn't close
        let _ = self.scope_leave();

        out.push(nt::Statement::ForEach {
            names,
            exprs,
            block,
        });
        out.push(self.pop_break_target(break_id));
    }

    /// Resolve a function definition statement
    fn function_statement(
        &mut self,
        name: &ast::FunctionName,
        body: &ast::Function,
        vis: Option<ast::Visibility>,
    ) -> nt::Statement {
        let name = match vis {
            Some(ast::Visibility::Local) => {
                if name.method.is_some() {
                    panic!("No method name allowed in local function statement")
                }
                let [name] = name.names.as_slice() else {
                    panic!("Only 1 name component allowed in local function statement")
                };

                let var = nt::VariableId(
                    self.variable_table
                        .len()
                        .try_into()
                        .expect("Too many variables within module"),
                );
                let s = self.insert_string(name.value.as_bytes());
                self.variable_table.push(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: false,
                    attr_const: false,
                });

                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                nt::FunctionName::DefineLocal { var }
            }
            Some(ast::Visibility::Global) => {
                if name.method.is_some() {
                    panic!("No method name allowed in global function statement")
                }
                let [name] = name.names.as_slice() else {
                    panic!("Only 1 name component allowed in global function statement")
                };

                let (env, names) = match self.resolve(name.value) {
                    ResolveResult::Local(id) => (id, vec![]),
                    ResolveResult::Global { env, names } => (env, names),
                };

                nt::FunctionName::DefineGlobal { env, names }
            }
            _ => {
                let [first, tail @ ..] = name.names.as_slice() else {
                    panic!("No name for function?");
                };
                let (start, mut names) = match self.resolve(first.value) {
                    ResolveResult::Local(id) => (id, vec![]),
                    ResolveResult::Global { env, names } => (env, names),
                };
                names.extend(tail.iter().map(|t| self.insert_string(t.value.as_bytes())));

                nt::FunctionName::Path {
                    start,
                    names,
                    method: name.method.map(|m| self.insert_string(m.value.as_bytes())),
                }
            }
        };

        let has_self = matches!(
            name,
            nt::FunctionName::Path {
                method: Some(_),
                ..
            }
        );
        let body = self.function(body, has_self);
        nt::Statement::Function { name, body }
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
                self.variable_table.push(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: is_close,
                    attr_const: is_const,
                });

                self.locals.push(Variable::Var {
                    depth: self.scope_depth,
                    id: var,
                    func_depth: None,
                });
                out.push(nt::Statement::ScopeStart(var));
                nt::Var::LocalName(var)
            })
            .collect();

        if !exprs.is_empty() {
            out.push(nt::Statement::Assign {
                vars: names,
                exps: values,
                is_global_init: false,
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

        if let Some(all) = &names.attrib {
            let name = all.name.value;
            if name == "close" {
                panic!("The `close` attribute can only be applied to local variables");
            } else if name != "const" {
                panic!("Unknown attribute: {name}");
            }
        }

        let names = names
            .names
            .iter()
            .map(|(name, attr)| {
                if let Some(attr) = attr {
                    let name = attr.name.value;
                    if name == "close" {
                        panic!("The `close` attribute can only be applied to local variables");
                    } else if name != "const" {
                        panic!("Unknown attribute: {name}");
                    }
                }

                let res = self.resolve(name.value);
                self.resolved_var(res)
            })
            .collect();

        if !exprs.is_empty() {
            out.push(nt::Statement::Assign {
                vars: names,
                exps: values,
                is_global_init: true,
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
            ast::Expression::DotDotDot => {
                let var = self.resolve("...");
                nt::Expression::Prefix(nt::PrefixExpression::Var(Box::new(self.resolved_var(var))))
            }
            ast::Expression::Function(function) => {
                nt::Expression::Function(self.function(function, false))
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
    fn function(&mut self, function: &ast::Function, has_self: bool) -> nt::Function {
        self.scope_enter();
        self.function_depth += 1;

        let self_var = if has_self {
            let var = nt::VariableId(
                self.variable_table
                    .len()
                    .try_into()
                    .expect("Too many variables within module"),
            );
            let s = self.insert_string(b"self");
            self.variable_table.push(nt::Local {
                line: None,
                name: s,
                attr_close: false,
                attr_const: false,
            });
            self.locals.push(Variable::Var {
                depth: self.scope_depth,
                id: var,
                func_depth: None,
            });
            Some(var)
        } else {
            None
        };

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
                self.variable_table.push(nt::Local {
                    line: Some(name.line),
                    name: s,
                    attr_close: false,
                    attr_const: false,
                });
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
            self.variable_table.push(nt::Local {
                line: None,
                name: s,
                attr_close: false,
                attr_const: false,
            });
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
            self_var,
            names,
            var_name,
        };

        let body = self.block(&function.body);

        // only contains function parameters which cant be <close>
        let _ = self.scope_leave();
        self.function_depth -= 1;

        // remove labels for this function
        while let Some(last) = self.labels.last() {
            let label = &self.label_table[last.0 as usize];
            if label.func_depth > self.function_depth {
                self.labels.pop();
            } else {
                break;
            }
        }

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

    /// Resolve a goto label
    fn label(&mut self, ast: &ast::Label) -> nt::Statement {
        let id = self.get_label(ast.name);
        let label = &mut self.label_table[id.0 as usize];
        if label.defined {
            panic!("Repeated declaration of goto label");
        }
        label.defined = true;
        label.line = Some(ast.name.line);
        nt::Statement::Label(id)
    }

    /// Resolve a break statement into a goto statement
    fn goto_break(&mut self) -> nt::Statement {
        for &id in self.labels.iter().rev() {
            let label = &self.label_table[id.0 as usize];

            if label.func_depth != self.function_depth {
                break;
            }
            if !label.break_target {
                continue;
            }

            return nt::Statement::Goto(id);
        }

        panic!("Break outside of a loop")
    }

    /// Resolve a goto statement
    fn goto(&mut self, tok: Token) -> nt::Statement {
        nt::Statement::Goto(self.get_label(tok))
    }

    /// Get a label id for a given name
    fn get_label(&mut self, name: Token) -> nt::LabelId {
        let s = self.insert_string(name.value.as_bytes());

        for id in self.labels.iter().rev() {
            let label = &self.label_table[id.0 as usize];

            if label.func_depth != self.function_depth {
                break;
            }

            if label.name == Some(s) {
                return *id;
            }
        }

        // no existing label, so make a new one
        let id = nt::LabelId(
            self.label_table
                .len()
                .try_into()
                .expect("Too many labels within module"),
        );
        self.label_table.push(Label {
            name: Some(s),
            line: Some(name.line),
            defined: false,
            func_depth: self.function_depth,
            break_target: false,
        });
        self.labels.push(id);
        id
    }

    /// Add a loop break target to the list of labels
    fn push_break_target(&mut self) -> nt::LabelId {
        let id = nt::LabelId(
            self.label_table
                .len()
                .try_into()
                .expect("Too many labels within module"),
        );
        self.label_table.push(Label {
            name: None,
            line: None,
            defined: true,
            func_depth: self.function_depth,
            break_target: true,
        });
        self.labels.push(id);
        id
    }

    /// Remove a loop break target, following a loop, return the label target statement
    fn pop_break_target(&mut self, break_id: nt::LabelId) -> nt::Statement {
        for (idx, &id) in self.labels.iter().enumerate().rev() {
            let label = &self.label_table[id.0 as usize];

            if label.func_depth != self.function_depth {
                break;
            }
            if !label.break_target || id != break_id {
                continue;
            }

            self.labels.remove(idx);
            return nt::Statement::Label(id);
        }

        panic!("Unmatched loop break")
    }

    /// Resolve a var expression
    fn var(&mut self, var: &ast::Var) -> nt::Var {
        match var {
            ast::Var::Name(token) => {
                let var = self.resolve(token.value);
                self.resolved_var(var)
            }
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

    /// Convert a Resolver result into an nt::Var
    fn resolved_var(&mut self, res: ResolveResult) -> nt::Var {
        match res {
            ResolveResult::Local(variable_id) => nt::Var::LocalName(variable_id),
            ResolveResult::Global { env, names } => nt::Var::GlobalNames { env, names },
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
}

enum ResolveResult {
    Local(nt::VariableId),
    Global {
        /// value of `_ENV` at the point the variable was resolved
        env: nt::VariableId,

        /// string to lookup within `_ENV`
        names: Vec<nt::StringId>,
    },
}

impl<'a> Resolver<'a> {
    /// Get a variable id for a variable name
    fn resolve(&mut self, name: &str) -> ResolveResult {
        let s = self.insert_string(name.as_bytes());

        // iterate through a stack, top first (fifo)
        for l in self.locals.iter().rev() {
            match l {
                Variable::Var { id, func_depth, .. } => {
                    let var = &self.variable_table[id.0 as usize];

                    if var.name == s
                        && (func_depth.is_none() || *func_depth == Some(self.function_depth))
                    {
                        return ResolveResult::Local(*id);
                    }
                }
                Variable::Alias { name, id, .. } => {
                    if *name == s {
                        return ResolveResult::Local(*id);
                    }
                }
            }
        }

        // no locals, so find a global

        // Need an environment for the global to be looked up in, so find a variable
        // that is the current environment.  This recurses back into the resolve
        // function, to stop the infinite recursion, the constructor for the name
        // resolver creates a variable `_ENV` so this will always resolve.
        let env = self.resolve(ENVIRONMENT);
        let (env, mut names) = match env {
            ResolveResult::Local(id) => (id, vec![]),
            ResolveResult::Global { env, names } => (env, names),
        };
        names.push(s);

        ResolveResult::Global { env, names }
    }

    /// Enter a lexical scope
    fn scope_enter(&mut self) {
        self.scope_depth += 1;
    }

    /// Leave a lexical scope, returns the variables that should be closed at
    /// the end of the scope.
    #[must_use = "Ignoring variable `<close>` annotations"]
    fn scope_leave(&mut self) -> Vec<nt::VariableId> {
        let mut out = vec![];
        self.scope_depth -= 1;

        while let Some(last) = self.locals.last() {
            let depth = match last {
                Variable::Var { depth, .. } => *depth,
                Variable::Alias { depth, .. } => *depth,
            };

            if depth > self.scope_depth {
                if let Variable::Var { id: var, .. } = last {
                    out.push(*var);
                }

                self.locals.pop();
            } else {
                break;
            }
        }

        out
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
