use crate::{
    Lexer,
    parser::lexer::{Token, TokenKind},
};

mod ast;
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
type PrefixFn = for<'a, 'b> fn(&'b mut Parser<'a>) -> ast::Expression<'a>;

/// All functions that parse an postfix (or infix) operator.  The argument taken
/// is the expression on the left hand side of the operator.
type PostfixFn = for<'a, 'b> fn(&'b mut Parser<'a>, ast::Expression<'a>) -> ast::Expression<'a>;

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
    pub fn file(&mut self) -> ast::Block<'a> {
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
        if self.check(TokenKind::SemiColon) {
            Some(ast::Statement::Empty)
        } else if self.check(TokenKind::ColonColon) {
            Some(ast::Statement::Label(self.label()))
        } else if self.check(TokenKind::Break) {
            Some(ast::Statement::Break)
        } else if self.check(TokenKind::Goto) {
            self.consume(TokenKind::Name, "Expected Name after goto");
            Some(ast::Statement::Goto(self.previous))
        } else {
            None
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

    /// Parse an expression
    fn expression(&mut self) -> Option<ast::Expression<'a>> {
        self.parse_precedence(Precedence::OrPrec)
    }

    /// Parse an expression with the given precedence
    fn parse_precedence(&mut self, prec: Precedence) -> Option<ast::Expression<'a>> {
        self.advance();
        let prefix_rule = ParseRule::get(self.previous.kind).prefix?;

        let mut expr = prefix_rule(self);

        while prec <= ParseRule::get(self.current.kind).precedence {
            self.advance();
            let Some(infix_rule) = ParseRule::get(self.previous.kind).postfix else {
                break;
            };
            expr = infix_rule(self, expr);
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
fn binary<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ast::Expression<'a> {
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

    ast::Expression::Binary {
        left: Box::new(lhs),
        op,
        right: Box::new(expr),
    }
}

/// Parse a right associative binary operator
fn right<'a>(this: &mut Parser<'a>, lhs: ast::Expression<'a>) -> ast::Expression<'a> {
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

    ast::Expression::Binary {
        left: Box::new(lhs),
        op,
        right: Box::new(expr),
    }
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
            Precedence::Additive => Self::Multiplicative,
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
            LeftParen => None.prefix(grouping),
            RightParen => None.into(),
            LeftCurly => None.into(),
            RightCurly => None.into(),
            LeftSquare => None.into(),
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
            Colon => None.into(),
            ColonColon => None.into(),
            Dot => None.into(),
            DotDot => Concat.postfix(right),
            DotDotDot => None.prefix(dot_dot_dot),
            Name => None.into(),
            String => None.prefix(string),
            Number => None.prefix(number),
            And => AndPrec.postfix(binary),
            Break => None.into(),
            Do => None.into(),
            Else => None.into(),
            Elseif => None.into(),
            End => None.into(),
            False => None.prefix(expr_false),
            For => None.into(),
            Function => None.into(),
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
