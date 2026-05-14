mod parser;

pub use parser::lexer::Lexer;

pub fn run() {
    println!("{}", Lexer::new(";^:&--[==[aaa]===]]==]::--hi\n--[[]]\"a\\\"\";"));
}

#[cfg(test)]
#[test]
fn it_works() {
    run();
    todo!();
}
