use crate::{name_resolution::name_tree::*, utils::SizeOf};

impl<T: SizeOf> SizeOf for NameContainer<T> {
    fn size(&self) -> usize {
        let NameContainer {
            tree,
            string_table,
            variable_table,
            label_table,
            env,
        } = self;
        tree.size() + string_table.size() + variable_table.size() + label_table.size() + env.size()
    }
}

impl SizeOf for StringId {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for VariableId {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for LabelId {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for Number {
    fn size(&self) -> usize {
        0
    }
}

impl SizeOf for Local {
    fn size(&self) -> usize {
        let Local {
            line,
            name,
            attr_close,
            attr_const,
        } = self;
        line.size() + name.size() + attr_close.size() + attr_const.size()
    }
}

impl SizeOf for Label {
    fn size(&self) -> usize {
        let Label { line, name } = self;
        line.size() + name.size()
    }
}

impl SizeOf for Block {
    fn size(&self) -> usize {
        let Block {
            statements,
            ret_stat,
            close,
        } = self;
        statements.size() + ret_stat.size() + close.size()
    }
}

impl SizeOf for Statement {
    fn size(&self) -> usize {
        match self {
            Statement::Empty => 0,
            Statement::Assign {
                vars,
                exps,
                is_global_init,
            } => vars.size() + exps.size() + is_global_init.size(),
            Statement::Call(function_call) => function_call.size(),
            Statement::Label(label_id) => label_id.size(),
            Statement::Goto(label_id) => label_id.size(),
            Statement::Block(block) => block.size(),
            Statement::While { expr, block } => expr.size() + block.size(),
            Statement::Repeat {
                block,
                expr,
                block_end,
            } => block.size() + expr.size() + block_end.size(),
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
            Statement::Function { name, body } => name.size() + body.size(),
            Statement::ScopeStart(variable_id) => variable_id.size(),
            Statement::ScopeEnd(variable_id) => variable_id.size(),
        }
    }
}

impl SizeOf for ReturnStatement {
    fn size(&self) -> usize {
        let ReturnStatement { exprs } = self;
        exprs.size()
    }
}

impl SizeOf for FunctionName {
    fn size(&self) -> usize {
        match self {
            FunctionName::DefineLocal { var } => var.size(),
            FunctionName::DefineGlobal { env, names } => env.size() + names.size(),
            FunctionName::Path {
                start,
                names,
                method,
            } => start.size() + names.size() + method.size(),
        }
    }
}

impl SizeOf for Var {
    fn size(&self) -> usize {
        match self {
            Var::LocalName(variable_id) => variable_id.size(),
            Var::GlobalNames { env, names } => env.size() + names.size(),
            Var::Index { first, index } => first.size() + index.size(),
            Var::Dot { first, name } => first.size() + name.size(),
        }
    }
}

impl SizeOf for Expression {
    fn size(&self) -> usize {
        match self {
            Expression::Nil => 0,
            Expression::Bool(b) => b.size(),
            Expression::Number(number) => number.size(),
            Expression::String(string_id) => string_id.size(),
            Expression::Function(function) => function.size(),
            Expression::Prefix(prefix_expression) => prefix_expression.size(),
            Expression::Table(field_list) => field_list.size(),
            Expression::Binary { left, op, right } => left.size() + op.size() + right.size(),
            Expression::Unary { expr, op } => expr.size() + op.size(),
        }
    }
}

impl SizeOf for PrefixExpression {
    fn size(&self) -> usize {
        match self {
            PrefixExpression::Var(var) => var.size(),
            PrefixExpression::Call(function_call) => function_call.size(),
            PrefixExpression::Expr(expression) => expression.size(),
        }
    }
}

impl SizeOf for FunctionCall {
    fn size(&self) -> usize {
        let FunctionCall {
            receiver,
            method_name,
            args,
        } = self;
        receiver.size() + method_name.size() + args.size()
    }
}

impl SizeOf for FunctionArgs {
    fn size(&self) -> usize {
        match self {
            FunctionArgs::Call { exprs } => exprs.size(),
            FunctionArgs::Table { table } => table.size(),
            FunctionArgs::String { value } => value.size(),
        }
    }
}

impl SizeOf for Function {
    fn size(&self) -> usize {
        let Function { parameters, body } = self;
        parameters.size() + body.size()
    }
}

impl SizeOf for ParameterList {
    fn size(&self) -> usize {
        let ParameterList {
            self_var,
            names,
            var_name,
        } = self;
        self_var.size() + names.size() + var_name.size()
    }
}

impl SizeOf for FieldList {
    fn size(&self) -> usize {
        let FieldList { fields } = self;
        fields.size()
    }
}

impl SizeOf for Field {
    fn size(&self) -> usize {
        match self {
            Field::Index { index, expr } => index.size() + expr.size(),
            Field::Assign { name, expr } => name.size() + expr.size(),
            Field::Exp { expr } => expr.size(),
        }
    }
}
