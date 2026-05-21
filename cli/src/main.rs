use std::error::Error;

/// Incredibly basic commandline code runner
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    args.next();

    let path = match args.next() {
        Some(f) => f,
        None => "main.tlua".into(),
    };

    let src = std::fs::read_to_string(path)?;

    let lexer = typed_lua::Lexer::new(&src);
    let mut parser = typed_lua::Parser::new(lexer);
    let expr = parser.expression();

    println!("{expr:?}");

    Ok(())
}
