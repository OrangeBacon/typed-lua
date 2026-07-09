//! Result of running name a name resolution pass over the ast.  Modified version
//! of the ast, but with variable names resolved, and a couple of other syntax changes:
//! - No lifetime parameters, unlike the ast.
//! - String literals have escape sequences processed
//! - Number literals are converted into u64/f64
//! - Var args are always named with a resolved variable id
//! - The right side of `.` and `:` operators are string typed until type resolution
//!   (or potentially after), so uses the string table, not variable table
//! - Global variables are resolved, so the statements `global *`, `global<const> a`
//!   etc. are removed as they only have an effect during name resolution.
//! - Local variable definitions (`local a`, `local a = 5`) are either removed or
//!   resolved as required.
//! - Function definition statements with local/global have local/global removed,
//!   and the variable defined in the variable table. It is checked that the
//!   local and global ones only have one name (no `.` or `:`) as required in the
//!   grammar.
//! - Attributes are resolved into properties on their definition in the variable
//!   table, rather than being within the tree.  This includes preventing them
//!   from being string typed (rejecting invalid attributes).  It is checked that
//!   `<const>` values are not on the left of an assignment.  Statements are inserted
//!   into the tree to specify when (and therefore the order in which) variables
//!   are closed, as scope information from declaration is removed when the variables
//!   are defined.
//! - Within methods, `self` is resolved to the method that caused it to be defined.
//! - Goto statements are linked to the labels they refer to.
//! - Break statements are converted into goto and a label (and checked whether
//!   the break is in a loop that can be broken out of).

use crate::parser::ast;

/// Container to associate variables and strings with a provided name tree
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct NameContainer<T> {
    /// Root of this tree
    pub tree: T,

    /// Strings used in the program (which may not be utf-8 due to escape sequence processing)
    pub string_table: Vec<Vec<u8>>,

    /// All variables defined or used in this tree.
    pub variable_table: Vec<Local>,

    /// All labels used in this tree
    pub label_table: Vec<Label>,

    /// The initial environment variable that is set prior to entering this piece
    /// of code, value of resolving `_ENV` prior to anything happening.
    pub env: VariableId,
}

/// ID of a string within the string table
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringId(pub u32);

/// ID of a variable name
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableId(pub u32);

/// ID of a label
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelId(pub u32);

/// Number in lua, converted from the string representation
#[derive(Debug, Clone)]
pub enum Number {
    Integer(u64),
    Float(f64),
}

/// Local variables
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Local {
    pub line: Option<usize>,
    pub name: StringId,
    pub attr_close: bool,
    pub attr_const: bool,
}

/// Goto Labels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label {
    pub line: Option<usize>,
    pub name: Option<StringId>,
}

/// A sequence of code.  Note that block and chunk are the same in lua.  A file
/// contains 1 block.
/// chunk ::= block
/// block ::= {stat} [retstat]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub ret_stat: Option<ReturnStatement>,
    pub close: Vec<Statement>,
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
pub enum Statement {
    Empty,
    Assign {
        vars: Vec<Var>,
        exps: Vec<Expression>,
        is_global_init: bool,
    },
    Call(FunctionCall),
    Label(LabelId),
    Goto(LabelId),
    Block(Block),
    While {
        expr: Expression,
        block: Block,
    },
    Repeat {
        block: Block,
        expr: Expression,
        block_end: Vec<Statement>,
    },
    If {
        expr: Expression,
        block: Block,
        elseif: Vec<(Expression, Block)>,
        else_block: Option<Block>,
    },
    For {
        name: VariableId,
        initial: Expression,
        limit: Expression,
        step: Option<Expression>,
        block: Block,
    },
    ForEach {
        names: Vec<VariableId>,
        exprs: Vec<Expression>,
        block: Block,
    },
    Function {
        name: FunctionName,
        body: Function,
    },
    ScopeStart(VariableId),
    ScopeEnd(VariableId),
}

/// retstat ::= return [explist] [‘;’]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReturnStatement {
    pub exprs: Vec<Expression>,
}

/// funcname ::= Name {‘.’ Name} [‘:’ Name]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionName {
    /// Used for local or global function declarations (depending on the kind of
    /// variable referenced)
    DefineLocal { var: VariableId },
    DefineGlobal {
        env: VariableId,
        names: Vec<StringId>,
    },
    /// A function which isn't local or global, so can be assigned outside of the
    /// root of an object.  If the start of the path is a global variable, the start
    /// will be re-written to be the current value of `_ENV`, with the name of
    /// the global as the first element of the `names` vec.
    Path {
        start: VariableId,
        names: Vec<StringId>,
        method: Option<StringId>,
    },
}

/// varlist ::= var {‘,’ var}
/// var ::=  Name | prefixexp ‘[’ exp ‘]’ | prefixexp ‘.’ Name
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Var {
    LocalName(VariableId),
    GlobalNames {
        env: VariableId,
        names: Vec<StringId>,
    },
    Index {
        first: PrefixExpression,
        index: Expression,
    },
    Dot {
        first: PrefixExpression,
        name: StringId,
    },
}

// namelist ::= Name {‘,’ Name}

/// explist ::= exp {‘,’ exp}
/// exp ::=  nil | false | true | Numeral | LiteralString | ‘...’ | functiondef |
///      prefixexp | tableconstructor | exp binop exp | unop exp
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Expression {
    Nil,
    Bool(bool),
    Number(Number),
    String(StringId),
    Function(Function),
    Prefix(PrefixExpression),
    Table(FieldList),
    Binary {
        left: Box<Expression>,
        op: ast::BinaryOperator,
        right: Box<Expression>,
    },
    Unary {
        expr: Box<Expression>,
        op: ast::UnaryOperator,
    },
}

/// prefixexp ::= var | functioncall | ‘(’ exp ‘)’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrefixExpression {
    Var(Box<Var>),
    Call(FunctionCall),
    Expr(Box<Expression>),
}

/// functioncall ::=  prefixexp args | prefixexp ‘:’ Name args
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionCall {
    pub receiver: Box<PrefixExpression>,
    pub method_name: Option<StringId>,
    pub args: FunctionArgs,
}

/// args ::=  ‘(’ [explist] ‘)’ | tableconstructor | LiteralString
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionArgs {
    Call { exprs: Vec<Expression> },
    Table { table: FieldList },
    String { value: StringId },
}

/// functiondef ::= function funcbody
/// funcbody ::= ‘(’ [parlist] ‘)’ block end
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Function {
    pub parameters: ParameterList,
    pub body: Block,
}

/// parlist ::= namelist [‘,’ varargparam] | varargparam
/// varargparam ::= ‘...’ [Name]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterList {
    pub self_var: Option<VariableId>,
    pub names: Vec<VariableId>,
    pub var_name: Option<VariableId>,
}

/// tableconstructor ::= ‘{’ [fieldlist] ‘}’
/// fieldlist ::= field {fieldsep field} [fieldsep]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldList {
    pub fields: Vec<Field>,
}

// field ::= ‘[’ exp ‘]’ ‘=’ exp | Name ‘=’ exp | exp
// fieldsep ::= ‘,’ | ‘;’
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Index { index: Expression, expr: Expression },
    Assign { name: StringId, expr: Expression },
    Exp { expr: Expression },
}
