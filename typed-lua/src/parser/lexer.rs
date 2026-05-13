use std::{fmt::Display, str::Chars};

/// Get all tokens from an input file stream
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    start: &'a str,
    current: Chars<'a>,
    line: usize,
}

/// A single section of input source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Token<'a> {
    kind: TokenKind,
    value: &'a str,
    line: usize,
}

/// All possible token kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum TokenKind {
    Eof,

    // Single Character Symbols
    Plus,
    Minus,
    Star,
    Percent,
    Caret,
    Hash,
    Ampersand,
    Bar,
    Comma,
    LeftParen,
    RightParen,
    LeftCurly,
    RightCurly,
    LeftSquare,
    RightSquare,
    SemiColon,

    // Possibly Multi Character Symbols
    Less,
    LessLess,
    LessEqual,

    Greater,
    GreaterGreater,
    GreaterEqual,

    Slash,
    SlashSlash,

    Equal,
    EqualEqual,

    Tilde,
    TildeEqual,

    Colon,
    ColonColon,

    Dot,
    DotDot,
    DotDotDot,

    // Literals
    Name,
    String,
    Number,

    // Keywords
    And,
    Break,
    Do,
    Else,
    Elseif,
    End,
    False,
    For,
    Function,
    Global,
    Goto,
    If,
    In,
    Local,
    Nil,
    Not,
    Or,
    Repeat,
    Return,
    Then,
    True,
    Until,
    While,
}

impl<'a> Lexer<'a> {
    /// Create a lexer for a source file
    pub fn new(src: &'a str) -> Self {
        Self {
            start: src,
            current: src.chars(),
            line: 1,
        }
    }

    /// Get the next token.  Will return an EOF token forever if at the end of
    /// the source file.
    fn next(&mut self) -> Token<'a> {
        self.start = self.current.as_str();

        let Some(ch) = self.current.next() else {
            return self.tok(TokenKind::Eof);
        };

        match ch {
            '+' => self.tok(TokenKind::Plus),
            '-' => self.tok(TokenKind::Minus),
            '*' => self.tok(TokenKind::Star),
            '%' => self.tok(TokenKind::Percent),
            '^' => self.tok(TokenKind::Caret),
            '#' => self.tok(TokenKind::Hash),
            '&' => self.tok(TokenKind::Ampersand),
            '|' => self.tok(TokenKind::Bar),
            ',' => self.tok(TokenKind::Comma),
            '(' => self.tok(TokenKind::LeftParen),
            ')' => self.tok(TokenKind::RightParen),
            '{' => self.tok(TokenKind::LeftCurly),
            '}' => self.tok(TokenKind::RightCurly),
            '[' => self.tok(TokenKind::LeftSquare),
            ']' => self.tok(TokenKind::RightSquare),
            ';' => self.tok(TokenKind::SemiColon),

            '<' => {
                if self.is('<') {
                    self.tok(TokenKind::LessLess)
                } else if self.is('=') {
                    self.tok(TokenKind::LessEqual)
                } else {
                    self.tok(TokenKind::Less)
                }
            }

            '>' => {
                if self.is('>') {
                    self.tok(TokenKind::GreaterGreater)
                } else if self.is('=') {
                    self.tok(TokenKind::GreaterEqual)
                } else {
                    self.tok(TokenKind::Greater)
                }
            }

            '/' => {
                if self.is('/') {
                    self.tok(TokenKind::SlashSlash)
                } else {
                    self.tok(TokenKind::Slash)
                }
            }

            '=' => {
                if self.is('=') {
                    self.tok(TokenKind::EqualEqual)
                } else {
                    self.tok(TokenKind::Equal)
                }
            }

            '~' => {
                if self.is('=') {
                    self.tok(TokenKind::TildeEqual)
                } else {
                    self.tok(TokenKind::Tilde)
                }
            }

            ':' => {
                if self.is(':') {
                    self.tok(TokenKind::ColonColon)
                } else {
                    self.tok(TokenKind::Colon)
                }
            }

            '.' => {
                if self.is('.') {
                    if self.is('.') {
                        self.tok(TokenKind::DotDotDot)
                    } else {
                        self.tok(TokenKind::DotDot)
                    }
                } else {
                    self.tok(TokenKind::Dot)
                }
            }
            _ => panic!("Unexpected character '{}' on line {}", ch, self.line),
        }
    }

    /// If the next character is the input, consume it and return true, otherwise
    /// return false.
    fn is(&mut self, ch: char) -> bool {
        let ret = self.peek() == Some(ch);
        if ret {
            self.current.next();
        }

        ret
    }

    /// Get the next character without consuming it
    fn peek(&self) -> Option<char> {
        self.current.clone().next()
    }

    /// Create a token all characters consumed so far this token
    fn tok(&self, kind: TokenKind) -> Token<'a> {
        let len = self.start.len() - self.current.as_str().len();

        Token {
            kind,
            value: &self.start[..len],
            line: self.line,
        }
    }
}

impl Display for Lexer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut line = usize::MAX;
        let mut this = self.clone();

        loop {
            let token = this.next();
            if token.line != line {
                write!(f, "{:4} ", token.line)?;
                line = token.line;
            } else {
                write!(f, "   | ")?;
            }
            writeln!(f, "{:?} '{}'", token.kind, token.value)?;

            if token.kind == TokenKind::Eof {
                break;
            }
        }

        Ok(())
    }
}
