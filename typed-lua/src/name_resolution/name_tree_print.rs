use std::fmt::{self, Display, Formatter};

use crate::{
    name_resolution::name_tree::*,
    parser::ast,
    utils::{OrderedFloat, SizeOf, TreeCtx},
};

/// Pretty printer for all name tree nodes
#[derive(Debug, Clone)]
pub struct NtPrint<'a, T: ?Sized> {
    node: &'a T,
    tree: TreeCtx,
    strings: &'a [Vec<u8>],
    variables: &'a [Local],
    labels: &'a [Label],
}

impl<'a, T: SizeOf> NtPrint<'a, NameContainer<T>> {
    /// Create a new node printer
    pub fn new(container: &'a NameContainer<T>) -> Self {
        Self {
            node: container,
            tree: TreeCtx::new(container),
            strings: &container.string_table,
            variables: &container.variable_table,
            labels: &container.label_table,
        }
        .name("Name Tree")
    }
}

impl<'a, T> NtPrint<'a, T> {
    /// Display a non-last-child element of this node
    fn child<U>(&self, child: &'a U) -> NtPrint<'a, U> {
        NtPrint {
            node: child,
            tree: self.tree.child(),
            strings: self.strings,
            variables: self.variables,
            labels: self.labels,
        }
    }

    /// Display the last child of this node
    fn last<U>(&self, last: &'a U) -> NtPrint<'a, U> {
        NtPrint {
            node: last,
            tree: self.tree.last(),
            strings: self.strings,
            variables: self.variables,
            labels: self.labels,
        }
    }

    /// Set the name of this node
    fn name(mut self, name: &str) -> Self {
        self.tree.name(name);
        self
    }

    /// Swap the content without changing levels
    fn swap<U: ?Sized>(&self, swap: &'a U) -> NtPrint<'a, U> {
        NtPrint {
            node: swap,
            tree: self.tree.clone(),
            strings: self.strings,
            variables: self.variables,
            labels: self.labels,
        }
    }

    /// Get a printable version of a string from the name tree.  No quotation
    /// marks, un-printable characters are escaped.
    fn display_string(&self, id: StringId) -> String {
        let mut input = self.strings[id.0 as usize].as_slice();
        let mut out = String::new();

        loop {
            match std::str::from_utf8(input) {
                Ok(valid) => {
                    out.extend(valid.escape_default());
                    break;
                }
                Err(error) => {
                    let (valid, after_valid) = input.split_at(error.valid_up_to());
                    out.extend(std::str::from_utf8(valid).unwrap().escape_default());

                    let invalid_len = error.error_len().unwrap_or(after_valid.len());
                    for &byte in &after_valid[0..=invalid_len] {
                        out.push_str(&format!(r"\x{:X}", byte));
                    }

                    input = &after_valid[invalid_len..];
                }
            }
        }

        out
    }
}

impl<T: SizeOf> NtPrint<'_, T> {
    /// Display the name of this node
    fn print(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
        self.tree.print(f, name, None, self.node)
    }

    /// Display the name of this node, with a node count
    fn print_len(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        len: impl Into<Option<usize>>,
    ) -> fmt::Result {
        self.tree.print(f, name, len, self.node)
    }
}

impl Display for NtPrint<'_, str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.tree.print(f, self.node, None, &self.node)
    }
}

impl Display for NtPrint<'_, &str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, self.node)
    }
}

impl Display for NtPrint<'_, usize> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{}", *self.node))
    }
}

impl Display for NtPrint<'_, bool> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if *self.node {
            self.print(f, "True")
        } else {
            self.print(f, "False")
        }
    }
}

impl<'a, T> Display for NtPrint<'a, Vec<T>>
where
    NtPrint<'a, T>: Display,
    T: SizeOf,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.len();

        self.print_len(f, "List", count)?;
        for (idx, s) in self.node.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        Ok(())
    }
}

impl<'a, T> Display for NtPrint<'a, NameContainer<T>>
where
    NtPrint<'a, T>: Display,
    T: SizeOf,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Container")?;
        self.child(&self.node.env).name("Env").fmt(f)?;
        self.child(&self.node.string_table.len())
            .name("Strings")
            .fmt(f)?;
        self.child(&self.node.variable_table.len())
            .name("Variables")
            .fmt(f)?;
        self.child(&self.node.label_table.len())
            .name("Labels")
            .fmt(f)?;
        self.last(&self.node.tree).fmt(f)
    }
}

impl Display for NtPrint<'_, StringId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("\"{}\"", self.display_string(*self.node)))
    }
}

impl Display for NtPrint<'_, VariableId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let var = &self.variables[self.node.0 as usize];
        let name = self.display_string(var.name);

        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.print(f, &format!("%{name}.{}", var.name.0))
        } else {
            self.print(f, &format!("%\"{name}\".{}", var.name.0))
        }
    }
}

impl Display for NtPrint<'_, LabelId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let label = &self.labels[self.node.0 as usize];

        if let Some(id) = label.name {
            let name = self.display_string(id);

            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                self.print(f, &format!("@{name}.{}", self.node.0))
            } else {
                self.print(f, &format!("@\"{name}\".{}", self.node.0))
            }
        } else {
            self.print(f, &format!("@.{}", self.node.0))
        }
    }
}

impl Display for NtPrint<'_, Number> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Number::Integer(num) => self.print(f, &format!("Integer {num}")),
            Number::Float(OrderedFloat(num)) => self.print(f, &format!("Float {num}")),
        }
    }
}

impl Display for NtPrint<'_, Block> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.statements.len() + self.node.ret_stat.is_some() as usize;

        self.print_len(f, "Block", count)?;
        let full_count = count + (!self.node.close.is_empty()) as usize;

        for (idx, s) in self.node.statements.iter().enumerate() {
            if idx + 1 == full_count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        if let Some(ret) = &self.node.ret_stat {
            if self.node.close.is_empty() {
                self.last(ret).fmt(f)?;
            } else {
                self.child(ret).fmt(f)?;
            }
        }

        if !self.node.close.is_empty() {
            self.last(&self.node.close)
                .name("Closed Variables")
                .fmt(f)?;
        }

        Ok(())
    }
}

impl Display for NtPrint<'_, Statement> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Statement::Empty => self.print(f, "Empty Statement"),
            Statement::Assign {
                vars,
                exps,
                is_global_init,
            } => {
                self.print(f, "Assign")?;
                self.child(is_global_init).name("Init Globals").fmt(f)?;
                self.child(vars).name("Variables").fmt(f)?;
                self.last(exps).name("Expressions").fmt(f)
            }
            Statement::Call(call) => self.swap(call).fmt(f),
            Statement::Label(label) => self.swap(label).name("Label").fmt(f),
            Statement::Goto(label) => self.swap(label).name("Goto").fmt(f),
            Statement::Block(block) => self.swap(block).fmt(f),
            Statement::While { expr, block } => {
                self.print(f, "While")?;
                self.child(expr).name("Condition").fmt(f)?;
                self.last(block).fmt(f)
            }
            Statement::Repeat {
                block,
                expr,
                block_end,
            } => {
                self.print(f, "Repeat")?;
                self.child(block).fmt(f)?;

                if block_end.is_empty() {
                    self.last(expr).name("Condition").fmt(f)
                } else {
                    self.child(expr).name("Condition").fmt(f)?;
                    self.last(block_end).name("Closed Variables").fmt(f)
                }
            }
            Statement::If {
                expr,
                block,
                elseif,
                else_block,
            } => {
                let count = 1 + elseif.len() + else_block.is_some() as usize;
                self.print_len(f, "If", count)?;

                let a = std::iter::once((expr, block));
                let b = elseif.iter().map(|(a, b)| (a, b));
                for (idx, e) in a.chain(b).enumerate() {
                    if idx + 1 == count {
                        self.last(&e).fmt(f)?;
                    } else {
                        self.child(&e).fmt(f)?;
                    }
                }

                if let Some(else_block) = else_block {
                    self.last(else_block).name("Else").fmt(f)?;
                }

                Ok(())
            }
            Statement::For {
                name,
                initial,
                limit,
                step,
                block,
            } => {
                self.print(f, "For")?;
                self.child(name).name("Name").fmt(f)?;
                self.child(initial).name("Initial").fmt(f)?;
                self.child(limit).name("Limit").fmt(f)?;
                if let Some(step) = step {
                    self.child(step).name("Step").fmt(f)?;
                }
                self.last(block).fmt(f)
            }
            Statement::ForEach {
                names,
                exprs,
                block,
            } => {
                self.print(f, "For Each")?;
                self.child(names).name("Names").fmt(f)?;
                self.child(exprs).name("Values").fmt(f)?;
                self.last(block).fmt(f)
            }
            Statement::Function { name, body } => {
                self.print(f, "Function")?;
                self.child(name).name("Name").fmt(f)?;
                self.last(body).fmt(f)
            }
            Statement::ScopeStart(id) => self.swap(id).name("Scope Start").fmt(f),
            Statement::ScopeEnd(id) => self.swap(id).name("Scope End").fmt(f),
        }
    }
}

impl Display for NtPrint<'_, (&Expression, &Block)> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Branch")?;
        self.child(self.node.0).name("Condition").fmt(f)?;
        self.last(self.node.1).fmt(f)
    }
}

impl Display for NtPrint<'_, ReturnStatement> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.swap(&self.node.exprs).name("Return").fmt(f)
    }
}

impl Display for NtPrint<'_, FunctionName> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            FunctionName::DefineLocal { var } => self.swap(var).name("Define Local").fmt(f),
            FunctionName::DefineGlobal { env, names } => {
                self.print(f, "Define Global")?;
                self.child(env).name("Environment").fmt(f)?;
                self.last(names).name("Path").fmt(f)
            }
            FunctionName::Path {
                start,
                names,
                method,
            } => {
                self.print(f, "Assignment")?;
                self.child(start).name("Root").fmt(f)?;

                if let Some(meth) = method {
                    self.child(names).name("Path").fmt(f)?;
                    self.last(meth).name("Method").fmt(f)
                } else {
                    self.last(names).name("Path").fmt(f)
                }
            }
        }
    }
}

impl Display for NtPrint<'_, Var> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Var::LocalName(id) => self.swap(id).name("Local").fmt(f),
            Var::GlobalNames { env, names } => {
                self.print(f, "Global")?;
                self.child(env).name("Environment").fmt(f)?;
                self.last(names).fmt(f)
            }
            Var::Index { first, index } => {
                self.print(f, "Index")?;
                self.child(first).name("Base").fmt(f)?;
                self.last(index).name("Index").fmt(f)
            }
            Var::Dot { first, name } => {
                self.print(f, "Dot Access")?;
                self.child(first).name("Base").fmt(f)?;
                self.last(name).name("Value").fmt(f)
            }
        }
    }
}

impl Display for NtPrint<'_, Expression> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Expression::Nil => self.swap("Nil").fmt(f),
            Expression::Bool(b) => {
                if *b {
                    self.swap("True").fmt(f)
                } else {
                    self.swap("False").fmt(f)
                }
            }
            Expression::Number(number) => self.swap(number).fmt(f),
            Expression::String(id) => self.swap(id).name("String").fmt(f),
            Expression::Function(function) => self.swap(function).fmt(f),
            Expression::Prefix(prefix_expression) => self.swap(prefix_expression).fmt(f),
            Expression::Table(field_list) => self.swap(field_list).name("Table").fmt(f),
            Expression::Binary { left, op, right } => {
                self.print(f, "Binary")?;
                self.child(left.as_ref()).name("Left").fmt(f)?;
                self.child(op).name("Op").fmt(f)?;
                self.last(right.as_ref()).name("Right").fmt(f)
            }
            Expression::Unary { expr, op } => {
                self.print(f, "Unary")?;
                self.child(op).name("Op").fmt(f)?;
                self.last(expr.as_ref()).fmt(f)
            }
        }
    }
}

impl Display for NtPrint<'_, PrefixExpression> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            PrefixExpression::Var(var) => self.swap(var.as_ref()).fmt(f),
            PrefixExpression::Call(call) => self.swap(call).fmt(f),
            PrefixExpression::Expr(expr) => {
                self.print(f, "Parenthesised")?;
                self.last(expr.as_ref()).fmt(f)
            }
        }
    }
}

impl Display for NtPrint<'_, FunctionCall> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Call")?;
        self.child(self.node.receiver.as_ref())
            .name("Receiver")
            .fmt(f)?;

        if let Some(method) = self.node.method_name {
            self.child(&method).name("Method").fmt(f)?;
        }

        self.last(&self.node.args).fmt(f)
    }
}

impl Display for NtPrint<'_, FunctionArgs> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            FunctionArgs::Call { exprs } => self.swap(exprs).name("Arguments").fmt(f),
            FunctionArgs::Table { table } => self.swap(table).name("Table Arg").fmt(f),
            FunctionArgs::String { value } => self.swap(value).name("String Arg").fmt(f),
        }
    }
}

impl Display for NtPrint<'_, Function> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Function")?;
        self.child(&self.node.parameters).fmt(f)?;
        self.last(&self.node.body).fmt(f)
    }
}

impl Display for NtPrint<'_, ParameterList> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.self_var.is_some() as usize
            + self.node.names.len()
            + self.node.var_name.is_some() as usize;

        self.print_len(f, "Parameters", count)?;

        if let Some(name) = self.node.self_var {
            self.last(&name).name("Self var").fmt(f)?
        };

        for (idx, s) in self.node.names.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        if let Some(name) = self.node.var_name {
            self.last(&name).name("Var args").fmt(f)?
        };

        Ok(())
    }
}

impl Display for NtPrint<'_, FieldList> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.swap(&self.node.fields).name("Fields").fmt(f)
    }
}

impl Display for NtPrint<'_, Field> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Field::Index { index, expr } => {
                self.print(f, "Index Field")?;
                self.child(index).name("Index").fmt(f)?;
                self.last(expr).name("Value").fmt(f)
            }
            Field::Assign { name, expr } => {
                self.print(f, "Assign Field")?;
                self.child(name).name("Name").fmt(f)?;
                self.last(expr).name("Value").fmt(f)
            }
            Field::Exp { expr } => self.swap(expr).name("Expr Field").fmt(f),
        }
    }
}

impl Display for NtPrint<'_, ast::BinaryOperator> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{:?}", self.node))
    }
}

impl Display for NtPrint<'_, ast::UnaryOperator> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{:?}", self.node))
    }
}
