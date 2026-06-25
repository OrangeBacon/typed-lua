use crate::{
    Lexer,
    parser::lexer::{Token, TokenKind},
};

pub mod ast;
pub mod ast_print;
mod ast_size;
pub mod lexer;

/// Create a syntax tree from a token stream
#[derive(Debug, Clone)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token<'a>,
    previous: Token<'a>,
}

/// Order of precedence for all operators, loosest to tightest binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None,
    OrPrec,
    AndPrec,
    Relation,
    BitOr,
    BitXor,
    BitAnd,
    Shift,
    Concat,
    Additive,
    Multiplicative,
    Unary,
    Exponent,
    Call,
    Primary,
}

/// All functions that parse a prefix operator
type PrefixFn = for<'a> fn(&mut Parser<'a>) -> ast::Expression<'a>;

/// All functions that parse an postfix (or infix) operator.  The argument taken
/// is the expression on the left hand side of the operator.  Parse functions may
/// return Err, which signifies that the expression is invalid, based on checking
/// the provided left hand expression.  The returned expression should be the
/// provided left hand side expression.  If the expression is valid, the parser
/// should call `Parser::advance` to consume the operator token that caused the
/// parser to be called.
type PostfixFn = for<'a> fn(&mut Parser<'a>, ast::Expression<'a>) -> ExprResult<'a>;

/// Result type for postfix expression parsers
type ExprResult<'a> = Result<ast::Expression<'a>, ast::Expression<'a>>;

/// Parsers for a given operator token
struct ParseRule {
    prefix: Option<PrefixFn>,
    postfix: Option<PostfixFn>,
    precedence: Precedence,
}

impl<'a> Parser<'a> {
    /// Construct a new parser from the provided token stream
    pub fn new(lexer: Lexer<'a>) -> Self {
        let mut this = Self {
            lexer,
            current: Default::default(),
            previous: Default::default(),
        };
        this.advance();
        this
    }

    /// Get the next token
    fn advance(&mut self) {
        self.previous = self.current;
        self.current = self.lexer.token();
    }

    /// Get the next token and panic if it isn't of the provided type
    fn consume(&mut self, kind: TokenKind, msg: &str) {
        if self.current.kind == kind {
            self.advance();
            return;
        }

        self.error_current(msg)
    }

    /// If the next token is of the provided type, consume it and return true,
    /// otherwise don't and return false.
    fn check(&mut self, kind: TokenKind) -> bool {
        if self.current.kind == kind {
            self.advance();
            return true;
        }
        false
    }

    /// Panic with a syntax error referring to the provided token
    fn error_at(&self, token: Token<'a>, msg: &str) -> ! {
        let pos = match token.kind {
            TokenKind::Eof => "at end".to_string(),
            _ => format!(" at '{}'", token.value),
        };
        panic!("[line {}] Error{}: {}", token.line, pos, msg);
    }

    /// Panic at the current token
    fn error_current(&self, msg: &str) -> ! {
        self.error_at(self.current, msg)
    }

    /// Panic at the most recently consumed token
    fn error(&self, msg: &str) -> ! {
        self.error_at(self.previous, msg)
    }

    /// Run the parser for a full source file and get the output tree.
    pub fn file(mut self) -> ast::Block<'a> {
        let ret = self.block();

        if !self.check(TokenKind::Eof) {
            self.error_current(
                "Unexpected content after file content (return statements terminate a source file)",
            );
        }
        ret
    }

    /// Parse a block (= function body, file, etc)
    fn block(&mut self) -> ast::Block<'a> {
        let mut content = vec![];

        while !self.check(TokenKind::Eof) {
            if let Some(s) = self.statement() {
                content.push(s);
            } else {
                break;
            }
        }

        let mut ret = None;
        if self.check(TokenKind::Return) {
            ret = Some(self.return_statement());
        }

        ast::Block {
            statements: content,
            ret_stat: ret,
        }
    }

    /// Try to get a statement, if possible, otherwise return None
    fn statement(&mut self) -> Option<ast::Statement<'a>> {
        let kind = self.current.kind;
        match kind {
            TokenKind::SemiColon => {
                self.advance();
                Some(ast::Statement::Empty)
            }
            TokenKind::ColonColon => {
                self.advance();
                Some(ast::Statement::Label(self.label()))
            }
            TokenKind::Break => {
                self.advance();
                Some(ast::Statement::Break)
            }
            TokenKind::Goto => {
                self.advance();
                self.consume(TokenKind::Name, "Expected Name after goto");
                Some(ast::Statement::Goto(self.previous))
            }
            TokenKind::Do => {
                self.advance();
                let block = self.block();
                self.consume(TokenKind::End, "Expected `end` after block");
                Some(ast::Statement::Block(block))
            }
            TokenKind::While => {
                self.advance();
                let Some(expr) = self.expression() else {
                    self.error("Expected while loop condition expression");
                };
                self.consume(TokenKind::Do, "Expected `do` after while loop condition");
                let block = self.block();
                self.consume(TokenKind::End, "Expected `end` after block");
                Some(ast::Statement::While { expr, block })
            }
            TokenKind::Repeat => {
                self.advance();
                let block = self.block();
                self.consume(TokenKind::Until, "Expected `until` after repeat loop body");
                let Some(expr) = self.expression() else {
                    self.error("Expected repeat loop condition expression");
                };
                Some(ast::Statement::Repeat { block, expr })
            }
            TokenKind::If => {
                self.advance();
                Some(self.if_statement())
            }
            TokenKind::For => {
                self.advance();
                Some(self.for_statement())
            }
            TokenKind::Function => {
                self.advance();
                let name = self.function_name();
                let body = self.function_body();
                Some(ast::Statement::Function {
                    name,
                    body,
                    vis: None,
                })
            }
            TokenKind::Local => {
                self.advance();
                Some(self.local())
            }
            TokenKind::Global => {
                self.advance();
                Some(self.global())
            }
            _ if let Some(expr) = self.parse_precedence(Precedence::Call) => match expr {
                ast::Expression::Prefix(pre) => Some(self.prefix_statement(pre)),
                _ => self.error("Unexpected expression"),
            },
            _ => None,
        }
    }

    /// Parse a return statement
    fn return_statement(&mut self) -> ast::ReturnStatement<'a> {
        let content = self.comma(Self::expression).unwrap_or_default();

        self.check(TokenKind::SemiColon);

        ast::ReturnStatement { exprs: content }
    }

    /// Parse a goto label declaration
    fn label(&mut self) -> ast::Label<'a> {
        self.consume(TokenKind::Name, "Expected label name after `::`");
        let ret = ast::Label {
            name: self.previous,
        };
        self.consume(TokenKind::ColonColon, "Expected `::` after label name");
        ret
    }

    /// Parse an assignment or function call statement
    fn prefix_statement(&mut self, pre: ast::PrefixExpression<'a>) -> ast::Statement<'a> {
        match pre {
            ast::PrefixExpression::Var(var) => self.assignment(*var),
            ast::PrefixExpression::Call(call) => ast::Statement::Call(call),
            ast::PrefixExpression::Expr(_) => self.error("Unexpected parenthesised expression"),
        }
    }

    /// Parse an assignment statement
    fn assignment(&mut self, lhs: ast::Var<'a>) -> ast::Statement<'a> {
        let mut lhs = vec![lhs];

        if self.check(TokenKind::Comma) {
            let vars = self
                .comma(|s| s.parse_precedence(Precedence::Call))
                .unwrap_or_default();
            lhs.extend(vars.into_iter().map(|v| match v {
                ast::Expression::Prefix(ast::PrefixExpression::Var(v)) => *v,
                _ => self.error("Expected variable name expression in assignment"),
            }));
        }

        self.consume(TokenKind::Equal, "Expected `=` in assignment");

        let rhs = self.comma(Self::expression).unwrap_or_default();

        ast::Statement::Assign {
            vars: lhs,
            exps: rhs,
        }
    }

    /// Parse an if statement and all elseif/else blocks
    fn if_statement(&mut self) -> ast::Statement<'a> {
        let Some(expr) = self.expression() else {
            self.error("Expected if condition")
        };
        self.consume(TokenKind::Then, "Expected `then` after if condition");
        let block = self.block();

        let mut elseif = vec![];
        while self.check(TokenKind::Elseif) {
            let Some(expr) = self.expression() else {
                self.error("Expected elseif condition")
            };
            self.consume(TokenKind::Then, "Expected `then` after elseif condition");
            let block = self.block();
            elseif.push((expr, block));
        }

        let else_block = if self.check(TokenKind::Else) {
            Some(self.block())
        } else {
            None
        };

        self.consume(TokenKind::End, "Expected `end` after if statement");

        ast::Statement::If {
            expr,
            block,
            elseif,
            else_block,
        }
    }

    /// Parse a for or for-each statement
    fn for_statement(&mut self) -> ast::Statement<'a> {
        let Some(names) = self.comma(|this| {
            if this.check(TokenKind::Name) {
                Some(this.previous)
            } else {
                None
            }
        }) else {
            self.error("Expected list of names for for loop variables");
        };

        match names.as_slice() {
            [name] if self.check(TokenKind::Equal) => self.for_numeric(*name),
            _ if self.check(TokenKind::In) => self.for_each(names),
            [_] => self.error("Expected `in` or `=` after for loop variable"),
            _ => self.error("Expected `in` after for loop variables"),
        }
    }

    /// Parse a for-each statement
    fn for_each(&mut self, names: Vec<Token<'a>>) -> ast::Statement<'a> {
        let Some(exprs) = self.comma(Self::expression) else {
            self.error("Expected expressions to loop over for for-each loop")
        };

        self.consume(
            TokenKind::Do,
            "Expected `do` after for-each loop expressions",
        );
        let block = self.block();
        self.consume(TokenKind::End, "Expected `end` after for-each loop");

        ast::Statement::ForEach {
            names,
            exprs,
            block,
        }
    }

    /// Parse a numeric for loop
    fn for_numeric(&mut self, name: Token<'a>) -> ast::Statement<'a> {
        let Some(initial) = self.expression() else {
            self.error("Expected for loop initial bound")
        };
        self.consume(TokenKind::Comma, "Expected comma after initial expression");

        let Some(limit) = self.expression() else {
            self.error("Expected for loop limit")
        };

        let step = if self.check(TokenKind::Comma) {
            let Some(step) = self.expression() else {
                self.error("Expected for loop step")
            };
            Some(step)
        } else {
            None
        };

        self.consume(TokenKind::Do, "Expected `do` after for loop expressions");
        let block = self.block();
        self.consume(TokenKind::End, "Expected `end` after for loop");

        ast::Statement::For {
            name,
            initial,
            limit,
            step,
            block,
        }
    }

    /// Parse a function name
    fn function_name(&mut self) -> ast::FunctionName<'a> {
        self.consume(TokenKind::Name, "Expected function name");
        let mut names = vec![self.previous];

        while self.check(TokenKind::Dot) {
            self.consume(TokenKind::Name, "Expected name after `.`");
            names.push(self.previous);
        }

        let method = if self.check(TokenKind::Colon) {
            self.consume(TokenKind::Name, "Expected method name after `:`");
            Some(self.previous)
        } else {
            None
        };

        ast::FunctionName { names, method }
    }

    /// Parse the body of a function
    fn function_body(&mut self) -> ast::Function<'a> {
        self.consume(
            TokenKind::LeftParen,
            "Expected `(` at start of function parameters",
        );
        let parameters = self.parameters();
        self.consume(
            TokenKind::RightParen,
            "Expected `)` after function parameters",
        );

        let body = self.block();
        self.consume(TokenKind::End, "Expected `end` after function body");

        ast::Function { parameters, body }
    }

    /// Parse function parameters
    fn parameters(&mut self) -> ast::ParameterList<'a> {
        let mut names = vec![];
        let mut var_name = None;

        if self.current.kind == TokenKind::RightParen {
            return ast::ParameterList { names, var_name };
        }

        loop {
            if self.check(TokenKind::DotDotDot) {
                var_name = Some(if self.check(TokenKind::Name) {
                    Some(self.previous)
                } else {
                    None
                });
                break;
            }

            self.consume(TokenKind::Name, "Expected parameter name");
            names.push(self.previous);

            if !self.check(TokenKind::Comma) {
                break;
            }
        }

        ast::ParameterList { names, var_name }
    }

    /// Parse a statement beginning with `local`
    fn local(&mut self) -> ast::Statement<'a> {
        if self.check(TokenKind::Function) {
            let name = self.function_name();
            let body = self.function_body();
            return ast::Statement::Function {
                name,
                body,
                vis: Some(ast::Visibility::Local),
            };
        }

        let attrib = if self.check(TokenKind::Less) {
            self.consume(TokenKind::Name, "Expected attribute name");
            let name = ast::Attribute {
                name: self.previous,
            };
            self.consume(TokenKind::Greater, "Expected `>` after attribute name");
            Some(name)
        } else {
            None
        };

        let names = self.attrib_names();
        let exprs = if self.check(TokenKind::Equal) {
            let Some(exprs) = self.comma(Self::expression) else {
                self.error("Expected expression after `=`")
            };
            exprs
        } else {
            vec![]
        };

        ast::Statement::Declare {
            vis: ast::Visibility::Local,
            names: ast::AttributeNameList { attrib, names },
            exprs,
        }
    }

    /// Parse a statement beginning with `global`
    fn global(&mut self) -> ast::Statement<'a> {
        if self.check(TokenKind::Function) {
            let name = self.function_name();
            let body = self.function_body();
            return ast::Statement::Function {
                name,
                body,
                vis: Some(ast::Visibility::Global),
            };
        }

        let attrib = if self.check(TokenKind::Less) {
            self.consume(TokenKind::Name, "Expected attribute name");
            let name = ast::Attribute {
                name: self.previous,
            };
            self.consume(TokenKind::Greater, "Expected `>` after attribute name");
            Some(name)
        } else {
            None
        };

        if self.check(TokenKind::Star) {
            return ast::Statement::GlobalCollective { attrib };
        }

        let names = self.attrib_names();

        let exprs = if self.check(TokenKind::Equal) {
            let Some(exprs) = self.comma(Self::expression) else {
                self.error("Expected expression after `=`")
            };
            exprs
        } else {
            vec![]
        };

        ast::Statement::Declare {
            vis: ast::Visibility::Global,
            names: ast::AttributeNameList { attrib, names },
            exprs,
        }
    }

    /// Parse the names and attributes within a `local` or `global`
    fn attrib_names(&mut self) -> Vec<(Token<'a>, Option<ast::Attribute<'a>>)> {
        let Some(vals) = self.comma(|this| {
            this.consume(TokenKind::Name, "Expected variable name");
            let name = this.previous;

            let attrib = if this.check(TokenKind::Less) {
                this.consume(TokenKind::Name, "Expected attribute name");
                let name = ast::Attribute {
                    name: this.previous,
                };
                this.consume(TokenKind::Greater, "Expected `>` after attribute name");
                Some(name)
            } else {
                None
            };

            Some((name, attrib))
        }) else {
            self.error("Expected name after `local` or `global`")
        };
        vals
    }

    /// Parse an expression
    fn expression(&mut self) -> Option<ast::Expression<'a>> {
        self.parse_precedence(Precedence::OrPrec)
    }

    /// Parse an expression with the given precedence
    fn parse_precedence(&mut self, prec: Precedence) -> Option<ast::Expression<'a>> {
        let prefix_rule = ParseRule::get(self.current.kind).prefix?;
        self.advance();

        let mut expr = prefix_rule(self);

        while let rule = ParseRule::get(self.current.kind)
            && prec <= rule.precedence
            && !self.check(TokenKind::Eof)
        {
            let Some(infix_rule) = rule.postfix else {
                break;
            };
            match infix_rule(self, expr) {
                Ok(postfix) => expr = postfix,
                Err(postfix) => {
                    expr = postfix;
                    break;
                }
            };
        }

        Some(expr)
    }

    /// Parse a comma separated list, using the provided parser.  Errors if a
    /// trailing comma is parsed.  If the provided parser returns None:
    /// - if no items parsed, the whole method returns None
    /// - otherwise, ends the comma separated list.
    fn comma<T>(&mut self, mut f: impl FnMut(&mut Self) -> Option<T>) -> Option<Vec<T>> {
        let mut res = vec![f(self)?];

        while self.check(TokenKind::Comma) {
            let Some(t) = f(self) else {
                self.error("Unexpected trailing comma");
            };
            res.push(t);
        }

        Some(res)
    }
}

/// Parse a number token
fn number<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::Number(this.previous)
}

/// Parse a string token
fn string<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::String(this.previous)
}

/// Parse a boolean true token
fn expr_true<'a>(_: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::True
}

/// Parse a boolean false token
fn expr_false<'a>(_: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::False
}

/// Parse a `...` token
fn dot_dot_dot<'a>(_: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::DotDotDot
}

/// Parse a nil token
fn nil<'a>(_: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::Nil
}

/// Parse an identifier name
fn name<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::Prefix(ast::PrefixExpression::Var(Box::new(ast::Var::Name(
        this.previous,
    ))))
}

/// Parse a function (lambda) expression
fn function<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::Function(this.function_body())
}

/// Parse a table constructor
fn table<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    ast::Expression::Table(table_inner(this))
}

/// Parse a table constructor
fn table_inner<'a>(this: &mut Parser<'a>) -> ast::FieldList<'a> {
    let mut fields = vec![];

    if this.check(TokenKind::RightCurly) {
        // empty constructor
        return ast::FieldList { fields };
    }

    fields.push(field(this));
    while this.check(TokenKind::Comma) || this.check(TokenKind::SemiColon) {
        if this.current.kind == TokenKind::RightCurly {
            break;
        }

        fields.push(field(this));
    }

    // field list
    this.consume(
        TokenKind::RightCurly,
        "Expected `}` after table constructor",
    );

    ast::FieldList { fields }
}

/// Parse an individual field within a table constructor
fn field<'a>(this: &mut Parser<'a>) -> ast::Field<'a> {
    if this.check(TokenKind::LeftSquare) {
        let Some(lhs) = this.expression() else {
            this.error("Expected expression inside index field");
        };
        this.consume(TokenKind::RightSquare, "Expected `]` after index");
        this.consume(TokenKind::Equal, "Expected `=` after index");
        let Some(rhs) = this.expression() else {
            this.error("Expected expression inside index field");
        };
        ast::Field::Index {
            index: lhs,
            expr: rhs,
        }
    } else if let Some(expr) = this.expression() {
        if let ast::Expression::Prefix(ast::PrefixExpression::Var(var)) = expr {
            // attempt to match a `Name = value` assignment
            if let ast::Var::Name(name) = var.as_ref()
                && this.check(TokenKind::Equal)
            {
                let Some(rhs) = this.expression() else {
                    this.error("Expected expression after `=`");
                };
                return ast::Field::Assign {
                    name: *name,
                    expr: rhs,
                };
            }
            return ast::Field::Exp {
                expr: ast::Expression::Prefix(ast::PrefixExpression::Var(var)),
            };
        }

        ast::Field::Exp { expr }
    } else {
        this.error("Expected expression or index assignment in table constructor");
    }
}

/// Parse a parenthesised group
fn grouping<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    let Some(expr) = this.expression() else {
        this.error("Expected expression within `()` group");
    };

    let expr = Box::new(expr);
    this.consume(TokenKind::RightParen, "Expected ')' after expression.");
    ast::Expression::Prefix(ast::PrefixExpression::Expr(expr))
}

/// Parse a unary operator
fn unary<'a>(this: &mut Parser<'a>) -> ast::Expression<'a> {
    let operator_type = this.previous.kind;

    let Some(expr) = this.parse_precedence(Precedence::Unary) else {
        this.error("Expected expression after operator");
    };
    let expr = Box::new(expr);

    let op = match operator_type {
        TokenKind::Minus => ast::UnaryOperator::Negate,
        TokenKind::Tilde => ast::UnaryOperator::Tilde,
        TokenKind::Hash => ast::UnaryOperator::Hash,
        TokenKind::Not => ast::UnaryOperator::Not,
        _ => unreachable!(),
    };

    ast::Expression::Unary { expr, op }
}

/// Parse a binary operator
fn binary<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    this.advance();

    let operator_type = this.previous.kind;
    let rule = ParseRule::get(operator_type);

    let Some(expr) = this.parse_precedence(rule.precedence.next()) else {
        this.error("Expected expression after operator");
    };

    let op = match operator_type {
        TokenKind::Plus => ast::BinaryOperator::Plus,
        TokenKind::Minus => ast::BinaryOperator::Minus,
        TokenKind::Star => ast::BinaryOperator::Multiply,
        TokenKind::Slash => ast::BinaryOperator::Divide,
        TokenKind::SlashSlash => ast::BinaryOperator::FloorDivide,
        TokenKind::Percent => ast::BinaryOperator::Modulo,
        TokenKind::LessLess => ast::BinaryOperator::LeftShift,
        TokenKind::GreaterGreater => ast::BinaryOperator::RightShift,
        TokenKind::Ampersand => ast::BinaryOperator::BitAnd,
        TokenKind::Tilde => ast::BinaryOperator::BitXor,
        TokenKind::Bar => ast::BinaryOperator::BitOr,
        TokenKind::Or => ast::BinaryOperator::Or,
        TokenKind::And => ast::BinaryOperator::And,
        TokenKind::Less => ast::BinaryOperator::Less,
        TokenKind::Greater => ast::BinaryOperator::Greater,
        TokenKind::LessEqual => ast::BinaryOperator::LessEqual,
        TokenKind::GreaterEqual => ast::BinaryOperator::GreaterEqual,
        TokenKind::TildeEqual => ast::BinaryOperator::NotEqual,
        TokenKind::EqualEqual => ast::BinaryOperator::Equal,
        _ => unreachable!(),
    };

    Ok(ast::Expression::Binary {
        left: Box::new(lhs),
        op,
        right: Box::new(expr),
    })
}

/// Parse a right associative binary operator
fn right<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    this.advance();

    let operator_type = this.previous.kind;
    let rule = ParseRule::get(operator_type);
    let Some(expr) = this.parse_precedence(rule.precedence) else {
        this.error("Expected expression after operator");
    };

    let op = match operator_type {
        TokenKind::DotDot => ast::BinaryOperator::Concat,
        TokenKind::Caret => ast::BinaryOperator::Exponent,
        _ => unreachable!(),
    };

    Ok(ast::Expression::Binary {
        left: Box::new(lhs),
        op,
        right: Box::new(expr),
    })
}

/// Parse an indexing expression `a[5]`
fn index<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    let ast::Expression::Prefix(pre) = lhs else {
        return Err(lhs);
    };

    this.advance();
    let Some(expr) = this.expression() else {
        this.error("Expected index expression");
    };
    this.consume(TokenKind::RightSquare, "Expected `]` after index");

    Ok(ast::Expression::Prefix(ast::PrefixExpression::Var(
        Box::new(ast::Var::Index {
            first: pre,
            index: expr,
        }),
    )))
}

/// Parse a member access expression `a.b`
fn dot<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    let ast::Expression::Prefix(pre) = lhs else {
        return Err(lhs);
    };

    this.advance();
    this.consume(TokenKind::Name, "Expected identifier after `.`");
    let name = this.previous;

    Ok(ast::Expression::Prefix(ast::PrefixExpression::Var(
        Box::new(ast::Var::Dot { first: pre, name }),
    )))
}

/// Parse a call with parenthesised arguments `a(1)`
fn call_args<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    let ast::Expression::Prefix(pre) = lhs else {
        return Err(lhs);
    };

    this.advance();
    let args = this.comma(Parser::expression).unwrap_or_default();
    this.consume(
        TokenKind::RightParen,
        "Expected `)` after function arguments",
    );

    Ok(ast::Expression::Prefix(ast::PrefixExpression::Call(
        ast::FunctionCall {
            receiver: Box::new(pre),
            method_name: None,
            args: ast::FunctionArgs::Call { exprs: args },
        },
    )))
}

/// Parse a call with parenthesised arguments `a:b(1)`
fn call_method<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    let ast::Expression::Prefix(pre) = lhs else {
        return Err(lhs);
    };

    this.advance();
    this.consume(TokenKind::Name, "Expected method name after `:`");
    let name = this.previous;

    // args to the method "" | {} | ()
    let args = if this.check(TokenKind::String) {
        ast::FunctionArgs::String {
            value: this.previous,
        }
    } else if this.check(TokenKind::LeftCurly) {
        let table = table_inner(this);
        ast::FunctionArgs::Table { table }
    } else if this.check(TokenKind::LeftParen) {
        let args = this.comma(Parser::expression).unwrap_or_default();
        this.consume(
            TokenKind::RightParen,
            "Expected `)` after function arguments",
        );
        ast::FunctionArgs::Call { exprs: args }
    } else {
        this.error("Expected function arguments after method name")
    };

    Ok(ast::Expression::Prefix(ast::PrefixExpression::Call(
        ast::FunctionCall {
            receiver: Box::new(pre),
            method_name: Some(name),
            args,
        },
    )))
}

/// Parse a call with a string argument `a "1"`
fn call_string<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    let ast::Expression::Prefix(pre) = lhs else {
        return Err(lhs);
    };

    this.advance();
    let value = this.previous;

    Ok(ast::Expression::Prefix(ast::PrefixExpression::Call(
        ast::FunctionCall {
            receiver: Box::new(pre),
            method_name: None,
            args: ast::FunctionArgs::String { value },
        },
    )))
}

/// Parse a call with a string argument `a "1"`
fn call_table<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ExprResult<'a> {
    let ast::Expression::Prefix(pre) = lhs else {
        return Err(lhs);
    };

    this.advance();

    let table = table_inner(this);

    Ok(ast::Expression::Prefix(ast::PrefixExpression::Call(
        ast::FunctionCall {
            receiver: Box::new(pre),
            method_name: None,
            args: ast::FunctionArgs::Table { table },
        },
    )))
}

impl Precedence {
    /// Convert a precedence into an empty parse rule
    fn into(self) -> ParseRule {
        ParseRule {
            prefix: None,
            postfix: None,
            precedence: self,
        }
    }

    /// Create a parse rule with the given precedence and prefix parser
    fn prefix(self, f: PrefixFn) -> ParseRule {
        ParseRule {
            prefix: Some(f),
            postfix: None,
            precedence: self,
        }
    }

    /// Create a parse rule with the given precedence and postfix parser
    fn postfix(self, f: PostfixFn) -> ParseRule {
        ParseRule {
            prefix: None,
            postfix: Some(f),
            precedence: self,
        }
    }

    /// Get the next highest precedence after this one
    fn next(self) -> Precedence {
        match self {
            Precedence::None => Precedence::OrPrec,
            Precedence::OrPrec => Precedence::AndPrec,
            Precedence::AndPrec => Precedence::Relation,
            Precedence::Relation => Precedence::BitOr,
            Precedence::BitOr => Precedence::BitXor,
            Precedence::BitXor => Precedence::BitAnd,
            Precedence::BitAnd => Precedence::Shift,
            Precedence::Shift => Precedence::Concat,
            Precedence::Concat => Precedence::Additive,
            Precedence::Additive => Precedence::Multiplicative,
            Precedence::Multiplicative => Precedence::Unary,
            Precedence::Unary => Precedence::Exponent,
            Precedence::Exponent => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

impl ParseRule {
    /// Add a postfix parser to this parse rule
    fn postfix(self, f: PostfixFn) -> Self {
        Self {
            postfix: Some(f),
            ..self
        }
    }

    /// Get a parse rule for the provided token kind
    fn get(tok: TokenKind) -> Self {
        use Precedence::*;
        use TokenKind::*;

        match tok {
            Eof => None.into(),
            Plus => Additive.postfix(binary),
            Minus => Additive.prefix(unary).postfix(binary),
            Star => Multiplicative.postfix(binary),
            Percent => Multiplicative.postfix(binary),
            Caret => Exponent.postfix(right),
            Hash => None.prefix(unary),
            Ampersand => BitAnd.postfix(binary),
            Bar => BitOr.postfix(binary),
            Comma => None.into(),
            LeftParen => Call.prefix(grouping).postfix(call_args),
            RightParen => None.into(),
            LeftCurly => Call.prefix(table).postfix(call_table),
            RightCurly => None.into(),
            LeftSquare => Call.postfix(index),
            RightSquare => None.into(),
            SemiColon => None.into(),
            Less => Relation.postfix(binary),
            LessLess => Shift.postfix(binary),
            LessEqual => Relation.postfix(binary),
            Greater => Relation.postfix(binary),
            GreaterGreater => Shift.postfix(binary),
            GreaterEqual => Relation.postfix(binary),
            Slash => Multiplicative.postfix(binary),
            SlashSlash => Multiplicative.postfix(binary),
            Equal => None.into(),
            EqualEqual => Relation.postfix(binary),
            Tilde => None.prefix(unary).postfix(binary),
            TildeEqual => Relation.postfix(binary),
            Colon => Call.postfix(call_method),
            ColonColon => None.into(),
            Dot => Call.postfix(dot),
            DotDot => Concat.postfix(right),
            DotDotDot => None.prefix(dot_dot_dot),
            Name => None.prefix(name),
            String => Call.prefix(string).postfix(call_string),
            Number => None.prefix(number),
            And => AndPrec.postfix(binary),
            Break => None.into(),
            Do => None.into(),
            Else => None.into(),
            Elseif => None.into(),
            End => None.into(),
            False => None.prefix(expr_false),
            For => None.into(),
            Function => None.prefix(function),
            Global => None.into(),
            Goto => None.into(),
            If => None.into(),
            In => None.into(),
            Local => None.into(),
            Nil => None.prefix(nil),
            Not => None.prefix(unary),
            Or => OrPrec.postfix(binary),
            Repeat => None.into(),
            Return => None.into(),
            Then => None.into(),
            True => None.prefix(expr_true),
            Until => None.into(),
            While => None.into(),
        }
    }
}
