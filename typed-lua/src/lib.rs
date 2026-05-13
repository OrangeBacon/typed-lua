mod parser;

pub use parser::lexer::Lexer;

pub fn run() {
    println!("{}", Lexer::new(";^:&::"));
}

#[cfg(test)]
#[test]
fn it_works() {
    run();
    todo!();
}
