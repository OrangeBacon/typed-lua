use std::fmt::Display;

use crate::parser::{
    ast::{
        Attribute, AttributeNameList, BinaryOperator, Block, Expression, Field, FieldList,
        Function, FunctionArgs, FunctionCall, FunctionName, Label, ParameterList, PrefixExpression,
        ReturnStatement, Statement, UnaryOperator, Var, Visibility,
    },
    lexer::Token,
};

/// Get the size of a structure, including all included allocations
pub trait SizeOf {
    /// get the size of self
    fn size(&self) -> usize;
}

/// Helper for pretty printing byte sizes
pub struct Size(pub usize);

impl<T: SizeOf> SizeOf for Vec<T> {
    fn size(&self) -> usize {
        let alloc = self.capacity() * std::mem::size_of::<T>();
        let inner: usize = self.iter().map(|s| s.size()).sum();
        inner + alloc
    }
}

impl<T: SizeOf> SizeOf for Option<T> {
    fn size(&self) -> usize {
        self.as_ref().map(|t| t.size()).unwrap_or_default()
    }
}

impl<A: SizeOf, B: SizeOf> SizeOf for (A, B) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
}

impl<T: SizeOf> SizeOf for Box<T> {
    fn size(&self) -> usize {
        std::mem::size_of::<T>() + self.as_ref().size()
    }
}

impl<T: SizeOf> SizeOf for &T {
    fn size(&self) -> usize {
        T::size(self)
    }
}

impl SizeOf for &str {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for Token<'_> {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for Block<'_> {
    fn size(&self) -> usize {
        self.statements.size() + self.ret_stat.size()
    }
}

impl SizeOf for Statement<'_> {
    fn size(&self) -> usize {
        match self {
            Statement::Empty => 0,
            Statement::Assign { vars, exps } => vars.size() + exps.size(),
            Statement::Call(call) => call.size(),
            Statement::Label(label) => label.size(),
            Statement::Break => 0,
            Statement::Goto(token) => token.size(),
            Statement::Block(block) => block.size(),
            Statement::While { expr, block } => expr.size() + block.size(),
            Statement::Repeat { block, expr } => block.size() + expr.size(),
            Statement::If {
                expr,
                block,
                elseif,
                else_block,
            } => expr.size() + block.size() + elseif.size() + else_block.size(),
            Statement::For {
                name,
                initial,
                limit,
                step,
                block,
            } => name.size() + initial.size() + limit.size() + step.size() + block.size(),
            Statement::ForEach {
                names,
                exprs,
                block,
            } => names.size() + exprs.size() + block.size(),
            Statement::Function { name, body, vis } => name.size() + body.size() + vis.size(),
            Statement::Declare { vis, names, exprs } => vis.size() + names.size() + exprs.size(),
            Statement::GlobalCollective { attrib } => attrib.size(),
        }
    }
}

impl SizeOf for Visibility {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for AttributeNameList<'_> {
    fn size(&self) -> usize {
        self.attrib.size() + self.names.size()
    }
}

impl SizeOf for Attribute<'_> {
    fn size(&self) -> usize {
        self.name.size()
    }
}

impl SizeOf for ReturnStatement<'_> {
    fn size(&self) -> usize {
        self.exprs.size()
    }
}

impl SizeOf for Label<'_> {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for FunctionName<'_> {
    fn size(&self) -> usize {
        self.names.size() + self.method.size()
    }
}

impl SizeOf for Var<'_> {
    fn size(&self) -> usize {
        match self {
            Var::Name(token) => token.size(),
            Var::Index { first, index } => first.size() + index.size(),
            Var::Dot { first, name } => first.size() + name.size(),
        }
    }
}

impl SizeOf for Expression<'_> {
    fn size(&self) -> usize {
        match self {
            Expression::Nil => 0,
            Expression::False => 0,
            Expression::True => 0,
            Expression::Number(token) => token.size(),
            Expression::String(token) => token.size(),
            Expression::DotDotDot => 0,
            Expression::Function(function) => function.size(),
            Expression::Prefix(pre) => pre.size(),
            Expression::Table(fields) => fields.size(),
            Expression::Binary { left, op, right } => left.size() + op.size() + right.size(),
            Expression::Unary { expr, op } => expr.size() + op.size(),
        }
    }
}

impl SizeOf for PrefixExpression<'_> {
    fn size(&self) -> usize {
        match self {
            PrefixExpression::Var(var) => var.size(),
            PrefixExpression::Call(call) => call.size(),
            PrefixExpression::Expr(expr) => expr.size(),
        }
    }
}

impl SizeOf for FunctionCall<'_> {
    fn size(&self) -> usize {
        self.receiver.size() + self.method_name.size() + self.args.size()
    }
}

impl SizeOf for FunctionArgs<'_> {
    fn size(&self) -> usize {
        match self {
            FunctionArgs::Call { exprs } => exprs.size(),
            FunctionArgs::Table { table } => table.size(),
            FunctionArgs::String { value } => value.size(),
        }
    }
}

impl SizeOf for Function<'_> {
    fn size(&self) -> usize {
        self.parameters.size() + self.body.size()
    }
}

impl SizeOf for ParameterList<'_> {
    fn size(&self) -> usize {
        self.names.size() + self.var_name.size()
    }
}

impl SizeOf for FieldList<'_> {
    fn size(&self) -> usize {
        self.fields.size()
    }
}

impl SizeOf for Field<'_> {
    fn size(&self) -> usize {
        match self {
            Field::Index { index, expr } => index.size() + expr.size(),
            Field::Assign { name, expr } => name.size() + expr.size(),
            Field::Exp { expr } => expr.size(),
        }
    }
}

impl SizeOf for BinaryOperator {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for UnaryOperator {
    fn size(&self) -> usize {
        0
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUFFIX: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];

        let mut num = self.0 as f64;
        for suffix in SUFFIX {
            if num <= 1024.0 {
                return write!(f, "{num:.1} {suffix}");
            }
            num /= 1024.0;
        }

        write!(f, "{num:.1} PiB")
    }
}
