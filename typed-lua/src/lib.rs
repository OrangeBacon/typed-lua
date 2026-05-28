mod name_resolution;
mod parser;

pub use name_resolution::Resolver;
pub use parser::{Parser, ast_print::AstPrint, lexer::Lexer};
