mod parser;

pub use parser::lexer::Lexer;

pub fn run() {
    println!("{}", Lexer::new(";^:a and b&--[==[aaa]===]]==]::--hi\n--[[]]\"a\\\"\";"));
}

#[cfg(test)]
#[test]
fn it_works() {
    run();
    todo!();
}
