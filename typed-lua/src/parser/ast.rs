//! Very simple AST, optimised for ease of writing, I am well aware of how inefficient
//! this is, but I'd prefer to have working slow code, than broken fast code.
//! This is a simple translation of the grammar of lua.

use crate::parser::lexer::Token;

/// A sequence of code.  Note that block and chunk are the same in lua.  A file
/// contains 1 block.
/// chunk ::= block
/// block ::= {stat} [retstat]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Block<'a> {
    statements: Vec<Statement<'a>>,
    ret_stat: Option<ReturnStatement<'a>>,
}

/// stat ::=  ‘;’ |
///      varlist ‘=’ explist |
///      functioncall |
///      label |
///      break |
///      goto Name |
///      do block end |
///      while exp do block end |
///      repeat block until exp |
///      if exp then block {elseif exp then block} [else block] end |
///      for Name ‘=’ exp ‘,’ exp [‘,’ exp] do block end |
///      for namelist in explist do block end |
///      function funcname funcbody |
///      local function Name funcbody |
///      global function Name funcbody |
///      local attnamelist [‘=’ explist] |
///      global attnamelist [‘=’ explist] |
///      global [attrib] ‘*’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Statement<'a> {
    Empty,
    Assign {
        vars: Vec<Var<'a>>,
        exps: Vec<Expression<'a>>,
    },
    Call(FunctionCall<'a>),
    Label(Label<'a>),
    Break,
    Goto(Token<'a>),
    Block(Block<'a>),
    While {
        expr: Expression<'a>,
        block: Block<'a>,
    },
    Repeat {
        block: Block<'a>,
        expr: Expression<'a>,
    },
    If {
        expr: Expression<'a>,
        block: Block<'a>,
        elseif: Vec<(Expression<'a>, Block<'a>)>,
        else_block: Option<Block<'a>>,
    },
    For {
        name: Token<'a>,
        initial: Expression<'a>,
        limit: Expression<'a>,
        step: Option<Expression<'a>>,
        block: Block<'a>,
    },
    ForEach {
        names: Vec<Token<'a>>,
        exprs: Vec<Expression<'a>>,
        block: Block<'a>,
    },
    Function {
        name: FunctionName<'a>,
        body: Function<'a>,
        vis: Option<Visibility>,
    },
    Declare {
        vis: Visibility,
        names: AttributeNameList<'a>,
        exprs: Vec<Expression<'a>>,
    },
    GlobalCollective {
        attrib: Option<Attribute<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Visibility {
    Local,
    Global,
}

// attnamelist ::=  [attrib] Name [attrib] {‘,’ Name [attrib]}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AttributeNameList<'a> {
    attr: Attribute<'a>,
    names: Vec<(Token<'a>, Option<Attribute<'a>>)>,
}

/// attrib ::= ‘<’ Name ‘>’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Attribute<'a> {
    name: Token<'a>,
}

/// retstat ::= return [explist] [‘;’]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ReturnStatement<'a> {
    exprs: Vec<Expression<'a>>,
}

/// label ::= ‘::’ Name ‘::’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Label<'a> {
    name: Token<'a>,
}

/// funcname ::= Name {‘.’ Name} [‘:’ Name]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FunctionName<'a> {
    names: Vec<Token<'a>>,
    method: Option<Token<'a>>,
}

/// varlist ::= var {‘,’ var}
/// var ::=  Name | prefixexp ‘[’ exp ‘]’ | prefixexp ‘.’ Name
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Var<'a> {
    Name(Token<'a>),
    Index {
        first: PrefixExpression<'a>,
        index: Expression<'a>,
    },
    Dot {
        first: PrefixExpression<'a>,
        name: Token<'a>,
    },
}

// namelist ::= Name {‘,’ Name}
// type NameList<'a> = Vec<Token<'a>>
// (inlined where needed, no ast node exists for this)

/// explist ::= exp {‘,’ exp}
/// exp ::=  nil | false | true | Numeral | LiteralString | ‘...’ | functiondef |
///      prefixexp | tableconstructor | exp binop exp | unop exp
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Expression<'a> {
    Nil,
    False,
    True,
    Number(Token<'a>),
    String(Token<'a>),
    DotDotDot,
    Function(Function<'a>),
    Prefix(PrefixExpression<'a>),
    Table(FieldList<'a>),
    Binary {
        left: Box<Expression<'a>>,
        op: BinaryOperator,
        right: Box<Expression<'a>>,
    },
    Unary {
        expr: Box<Expression<'a>>,
        op: UnaryOperator,
    },
}

/// prefixexp ::= var | functioncall | ‘(’ exp ‘)’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrefixExpression<'a> {
    Var(Box<Var<'a>>),
    Call(FunctionCall<'a>),
    Expr(Box<Expression<'a>>),
}

/// functioncall ::=  prefixexp args | prefixexp ‘:’ Name args
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FunctionCall<'a> {
    receiver: Box<PrefixExpression<'a>>,
    method_name: Option<Token<'a>>,
    args: FunctionArgs<'a>,
}

/// args ::=  ‘(’ [explist] ‘)’ | tableconstructor | LiteralString
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FunctionArgs<'a> {
    Call { exprs: Vec<Expression<'a>> },
    Table { table: FieldList<'a> },
    String { value: Token<'a> },
}

/// functiondef ::= function funcbody
/// funcbody ::= ‘(’ [parlist] ‘)’ block end
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Function<'a> {
    parameters: ParameterList<'a>,
    body: Block<'a>,
}

/// parlist ::= namelist [‘,’ varargparam] | varargparam
/// varargparam ::= ‘...’ [Name]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ParameterList<'a> {
    names: Vec<Token<'a>>,
    var_name: Option<Token<'a>>,
}

/// tableconstructor ::= ‘{’ [fieldlist] ‘}’
/// fieldlist ::= field {fieldsep field} [fieldsep]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FieldList<'a> {
    fields: Vec<Field<'a>>,
}

// field ::= ‘[’ exp ‘]’ ‘=’ exp | Name ‘=’ exp | exp
// fieldsep ::= ‘,’ | ‘;’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Field<'a> {
    Index {
        index: Expression<'a>,
        expr: Expression<'a>,
    },
    Assign {
        name: Token<'a>,
        expr: Expression<'a>,
    },
    Exp {
        expr: Expression<'a>,
    },
}

/// binop ::=  ‘+’ | ‘-’ | ‘*’ | ‘/’ | ‘//’ | ‘^’ | ‘%’ |
///      ‘&’ | ‘~’ | ‘|’ | ‘>>’ | ‘<<’ | ‘..’ |
///      ‘<’ | ‘<=’ | ‘>’ | ‘>=’ | ‘==’ | ‘~=’ |
///      and | or
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    FloorDivide,
    Exponent,
    Modulo,
    BitAnd,
    BitXor,
    BitOr,
    RightShift,
    LeftShift,
    Concat,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

/// unop ::= ‘-’ | not | ‘#’ | ‘~’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOperator {
    Negate,
    Not,
    Hash,
    Tilde,
}
