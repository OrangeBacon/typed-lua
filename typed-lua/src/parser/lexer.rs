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
        self.skip_whitespace();

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

            '[' => self.square(),
            '"' | '\'' => self.string(ch),
            '0'..='9' => self.number(),
            'a'..='z' | 'A'..='Z' | '_' => self.ident(),

            _ => panic!("Unexpected character '{}' on line {}", ch, self.line),
        }
    }

    /// Parse tokens starting with a '['
    fn square(&mut self) -> Token<'a> {
        if let Some(len) = self.open_long_bracket(true) {
            while !self.close_long_bracket(len) {
                if self.current.next().is_none() {
                    panic!("Unexpected EOF while parsing string on line {}", self.line)
                }
            }

            self.tok(TokenKind::String)
        } else {
            self.tok(TokenKind::LeftSquare)
        }
    }

    /// Parse a string literal, does not validate escape sequences
    fn string(&mut self, start: char) -> Token<'a> {
        while let Some(ch) = self.current.next() {
            if ch == '\\' {
                self.current.next();
            } else if ch == start {
                break;
            }
        }

        let tok = self.tok(TokenKind::String);
        if !tok.value.ends_with(start) {
            panic!("Unterminated string at line {}", self.line)
        }
        tok
    }

    /// Parse number literals
    fn number(&mut self) -> Token<'a> {
        todo!()
    }

    /// Parse an identifier
    fn ident(&mut self) -> Token<'a> {
        todo!()
    }

    /// Skip all whitespace and comments
    fn skip_whitespace(&mut self) {
        loop {
            let Some(ch) = self.peek() else {
                return;
            };

            match ch {
                ' ' | '\r' | '\t' | '\x0B' | '\x0C' => {
                    self.current.next();
                }
                '\n' => {
                    self.line += 1;
                    self.current.next();
                }
                '-' => {
                    if !self.comment() {
                        return;
                    }
                }

                _ => return,
            }
        }
    }

    /// Attempt to skip a comment, assuming that a '-' has already been peeked.
    /// Returns true if it skipped anything and false if no comment was detected.
    fn comment(&mut self) -> bool {
        // check is a '--'
        if self.peek_next() != Some('-') {
            return false;
        }

        // consume the '--'
        self.current.nth(1);

        if let Some(len) = self.open_long_bracket(false) {
            while !self.close_long_bracket(len) {
                if self.current.next().is_none() {
                    panic!(
                        "Unexpected EOF while parsing long comment on line {}",
                        self.line
                    )
                }
            }
        } else {
            while let Some(ch) = self.peek()
                && ch != '\n'
            {
                self.current.next();
            }
        }

        true
    }

    /// Try to parse an opening long bracket, if succeeded return its length,
    /// otherwise don't consume anything.  If the leading '[' has already been
    /// consumed from the stream, pass true as the argument.
    fn open_long_bracket(&mut self, opened: bool) -> Option<usize> {
        let mut peek = self.current.clone().peekable();

        if !opened && peek.next() != Some('[') {
            return None;
        }

        let mut length = 0;
        while peek.peek() == Some(&'=') {
            length += 1;
            peek.next();
        }

        if peek.next() != Some('[') {
            return None;
        }

        // succeeded in parsing, so consume the long bracket, +2 for each `[`,
        // - 1 for 0 indexing
        self.current.nth(length + 1);

        Some(length)
    }

    /// Try to parse a closing long bracket, if succeeded return true, otherwise
    /// don't consume anything and return false
    fn close_long_bracket(&mut self, len: usize) -> bool {
        let mut peek = self.current.clone();

        if peek.next() != Some(']') {
            return false;
        }

        for _ in 0..len {
            if peek.next() != Some('=') {
                return false;
            }
        }

        if peek.next() != Some(']') {
            return false;
        }

        // succeeded in parsing, so consume the long bracket, + 2 for each `[`,
        // - 1 for 0 indexing
        self.current.nth(len + 1);

        true
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

    /// Get the character after next without consuming it
    fn peek_next(&self) -> Option<char> {
        self.current.clone().nth(1)
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
