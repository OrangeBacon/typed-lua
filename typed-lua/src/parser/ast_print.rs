use std::fmt::{self, Display, Formatter};

use crate::parser::{
    ast::{
        Attribute, AttributeNameList, BinaryOperator, Block, Expression, Field, FieldList,
        Function, FunctionArgs, FunctionCall, FunctionName, Label, ParameterList, PrefixExpression,
        ReturnStatement, Statement, UnaryOperator, Var, Visibility,
    },
    ast_size::{Size, SizeOf},
    lexer::Token,
};

/// Pretty printer for all AST nodes
pub struct AstPrint<'a, T> {
    node: &'a T,
    top_level: bool,
    top_size: usize,
    prefix: String,
    child_prefix: String,
    name: String,
}

impl<'a, T: SizeOf> AstPrint<'a, T> {
    /// Create a new node printer
    pub fn new(node: &'a T) -> Self {
        Self {
            node,
            top_level: true,
            top_size: node.size(),
            prefix: String::new(),
            child_prefix: String::new(),
            name: String::new(),
        }
    }
}

impl<'a, T> AstPrint<'a, T> {
    /// Display a non-last-child element of this node
    fn child<U>(&self, child: &'a U) -> AstPrint<'a, U> {
        AstPrint {
            node: child,
            top_level: false,
            top_size: self.top_size,
            prefix: format!("{}|- ", self.child_prefix),
            child_prefix: format!("{}|  ", self.child_prefix),
            name: String::new(),
        }
    }

    /// Display the last child of this node
    fn last<U>(&self, last: &'a U) -> AstPrint<'a, U> {
        AstPrint {
            node: last,
            top_level: false,
            top_size: self.top_size,
            prefix: format!("{}`- ", self.child_prefix),
            child_prefix: format!("{}   ", self.child_prefix),
            name: String::new(),
        }
    }

    /// Set the name of this node
    fn name(self, name: &str) -> Self {
        let name = if self.name.is_empty() {
            name.to_string()
        } else {
            format!("{}: {name}", self.name)
        };

        Self { name, ..self }
    }

    /// Swap the content without changing levels
    fn swap<U>(&self, swap: &'a U) -> AstPrint<'a, U> {
        AstPrint {
            node: swap,
            top_level: self.top_level,
            top_size: self.top_size,
            prefix: self.prefix.clone(),
            child_prefix: self.child_prefix.clone(),
            name: self.name.clone(),
        }
    }
}

impl<T: SizeOf> AstPrint<'_, T> {
    /// Display the name of this node
    fn print(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
        self.print_len(f, name, None)
    }

    /// Display the name of this node, with a node count
    fn print_len(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        len: impl Into<Option<usize>>,
    ) -> fmt::Result {
        if !self.prefix.is_empty() {
            write!(f, "{}", self.prefix)?;
        }

        if !self.name.is_empty() {
            write!(f, "{}: ", self.name)?;
        }

        write!(f, "{name}")?;

        if let Some(len) = len.into() {
            write!(f, "[{len}]")?;
        }

        let size = self.node.size();
        if self.top_level || (size as f64) > (self.top_size as f64) * 0.2 {
            write!(f, " <{}>", Size(size))?;
        }

        writeln!(f)?;

        Ok(())
    }
}

impl Display for AstPrint<'_, &str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, self.node)
    }
}

impl<'a, T> Display for AstPrint<'a, Vec<T>>
where
    AstPrint<'a, T>: Display,
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

impl Display for AstPrint<'_, Token<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("Token {:?}", self.node.value))
    }
}

impl Display for AstPrint<'_, Block<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.statements.len() + self.node.ret_stat.is_some() as usize;

        self.print_len(f, "Block", count)?;

        for (idx, s) in self.node.statements.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        if let Some(ret) = &self.node.ret_stat {
            self.last(ret).fmt(f)?;
        }

        Ok(())
    }
}

impl Display for AstPrint<'_, Statement<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.node {
            Statement::Empty => self.print(f, "Empty Statement"),
            Statement::Assign { vars, exps } => {
                self.print(f, "Assignment")?;
                self.child(vars).name("Variables").fmt(f)?;
                self.last(exps).name("Expressions").fmt(f)
            }
            Statement::Call(call) => self.swap(call).fmt(f),
            Statement::Label(label) => self.swap(label).fmt(f),
            Statement::Break => self.print(f, "Break"),
            Statement::Goto(token) => self.swap(token).name("Goto").fmt(f),
            Statement::Block(block) => self.swap(block).fmt(f),
            Statement::While { expr, block } => {
                self.print(f, "While")?;
                self.child(expr).name("Condition").fmt(f)?;
                self.last(block).fmt(f)
            }
            Statement::Repeat { block, expr } => {
                self.print(f, "Repeat")?;
                self.child(block).fmt(f)?;
                self.last(expr).name("Condition").fmt(f)
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
            Statement::Function { name, body, vis } => {
                self.print(f, "Function")?;
                if let Some(vis) = vis {
                    self.child(vis).fmt(f)?;
                }
                self.child(name).fmt(f)?;
                self.last(body).fmt(f)
            }
            Statement::Declare { vis, names, exprs } => {
                self.print(f, "Declare")?;
                self.child(vis).fmt(f)?;
                self.child(names).name("Names").fmt(f)?;
                self.last(exprs).name("Values").fmt(f)
            }
            Statement::GlobalCollective { attrib } => {
                self.print(f, "Global *")?;
                if let Some(attr) = attrib {
                    self.last(attr).fmt(f)?;
                }
                Ok(())
            }
        }
    }
}

impl Display for AstPrint<'_, (&Expression<'_>, &Block<'_>)> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Branch")?;
        self.child(self.node.0).name("Condition").fmt(f)?;
        self.last(self.node.1).fmt(f)
    }
}

impl Display for AstPrint<'_, Visibility> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Visibility::Local => self.print(f, "Local"),
            Visibility::Global => self.print(f, "Global"),
        }
    }
}

impl Display for AstPrint<'_, AttributeNameList<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.names.len();
        self.print_len(f, "Attribute Names", count)?;

        if let Some(name) = &self.node.attrib {
            if count == 0 {
                self.last(name)
            } else {
                self.child(name)
            }
            .name("Attribute")
            .fmt(f)?
        };

        for (idx, a) in self.node.names.iter().enumerate() {
            if idx + 1 == count {
                self.last(a).fmt(f)?;
            } else {
                self.child(a).fmt(f)?;
            }
        }

        Ok(())
    }
}

impl Display for AstPrint<'_, (Token<'_>, Option<Attribute<'_>>)> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(attr) = &self.node.1 {
            self.print(f, "Name/Attr")?;
            self.child(&self.node.0).name("Name").fmt(f)?;
            self.last(attr).fmt(f)
        } else {
            self.swap(&self.node.0).name("Name").fmt(f)
        }
    }
}

impl Display for AstPrint<'_, Attribute<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.swap(&self.node.name).name("Attribute").fmt(f)
    }
}

impl Display for AstPrint<'_, ReturnStatement<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.swap(&self.node.exprs).name("Return").fmt(f)
    }
}

impl Display for AstPrint<'_, Label<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.swap(&self.node.name).name("Label").fmt(f)
    }
}

impl Display for AstPrint<'_, FunctionName<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.names.len() + self.node.method.is_some() as usize;

        self.print_len(f, "Function Name", count)?;

        for (idx, s) in self.node.names.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        if let Some(name) = self.node.method {
            self.last(&name).name("Method").fmt(f)?
        };

        Ok(())
    }
}

impl Display for AstPrint<'_, Var<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Var::Name(token) => self.swap(token).name("Name").fmt(f),
            Var::Index { first, index } => {
                self.print(f, "Var Index")?;
                self.child(first).fmt(f)?;
                self.last(index).name("Index").fmt(f)
            }
            Var::Dot { first, name } => {
                self.print(f, "Var Dot")?;
                self.child(first).fmt(f)?;
                self.last(name).name("Name").fmt(f)
            }
        }
    }
}

impl Display for AstPrint<'_, Expression<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            Expression::Nil => self.swap(&"Nil").fmt(f),
            Expression::False => self.swap(&"False").fmt(f),
            Expression::True => self.swap(&"True").fmt(f),
            Expression::Number(token) => self.swap(token).name("Number").fmt(f),
            Expression::String(token) => self.swap(token).name("String").fmt(f),
            Expression::DotDotDot => self.swap(&"\"...\"").fmt(f),
            Expression::Function(function) => self.swap(function).fmt(f),
            Expression::Prefix(pre) => self.swap(pre).fmt(f),
            Expression::Table(fields) => self.swap(fields).name("Table").fmt(f),
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

impl Display for AstPrint<'_, PrefixExpression<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            PrefixExpression::Var(var) => self.swap(var.as_ref()).fmt(f),
            PrefixExpression::Call(call) => self.swap(call).fmt(f),
            PrefixExpression::Expr(expression) => {
                self.print(f, "Parenthesised")?;
                self.last(expression.as_ref()).fmt(f)
            }
        }
    }
}

impl Display for AstPrint<'_, FunctionCall<'_>> {
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

impl Display for AstPrint<'_, FunctionArgs<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.node {
            FunctionArgs::Call { exprs } => self.swap(exprs).name("Arguments").fmt(f),
            FunctionArgs::Table { table } => self.swap(table).name("Table Arg").fmt(f),
            FunctionArgs::String { value } => self.swap(value).name("String Arg").fmt(f),
        }
    }
}

impl Display for AstPrint<'_, Function<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Function")?;

        self.child(&self.node.parameters).fmt(f)?;
        self.last(&self.node.body).fmt(f)
    }
}

impl Display for AstPrint<'_, ParameterList<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.names.len() + self.node.var_name.is_some() as usize;

        self.print_len(f, "Parameters", count)?;

        for (idx, s) in self.node.names.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        match self.node.var_name {
            Some(Some(name)) => self.last(&name).name("Var args").fmt(f)?,
            Some(None) => self.last(&"<un-named>").name("Var args").fmt(f)?,
            None => (),
        };

        Ok(())
    }
}

impl Display for AstPrint<'_, FieldList<'_>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.fields.len();

        self.print_len(f, "Field List", count)?;
        for (idx, s) in self.node.fields.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        Ok(())
    }
}

impl Display for AstPrint<'_, Field<'_>> {
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

impl Display for AstPrint<'_, BinaryOperator> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{:?}", self.node))
    }
}

impl Display for AstPrint<'_, UnaryOperator> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{:?}", self.node))
    }
}
