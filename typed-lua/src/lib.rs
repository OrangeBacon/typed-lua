mod name_resolution;
mod parser;
mod utils;

pub use name_resolution::{Resolver, name_tree_print::NtPrint};
pub use parser::{Parser, ast_print::AstPrint, lexer::Lexer};
