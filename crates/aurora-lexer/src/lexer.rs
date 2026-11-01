//! Lexical analyzer implementation for Aurora.

use crate::cursor::Cursor;
use crate::token::{Token, TokenKind};
use aurora_ast::Span;

/// Lexer that scans Aurora source code into a stream of tokens.
#[derive(Debug)]
pub struct Lexer<'a> {
    cursor: Cursor<'a>,
}

impl<'a> Lexer<'a> {
    /// Creates a new Lexer for the given source code slice.
    pub fn new(source: &'a str) -> Self {
        Self {
            cursor: Cursor::new(source),
        }
    }

    /// Collects all tokens until EOF into a vector.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Produces the next token in the stream.
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start = self.cursor.offset();
        let Some(ch) = self.cursor.next_char() else {
            return Token::new(TokenKind::Eof, Span::at(start));
        };

        let kind = match ch {
            '+' => TokenKind::Plus,
            '-' => {
                if self.cursor.next_if(|c| c == '>').is_some() {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '?' => TokenKind::Question,
            '^' => TokenKind::Caret,
            ':' => {
                if self.cursor.next_if(|c| c == ':').is_some() {
                    TokenKind::DoubleColon
                } else {
                    TokenKind::Colon
                }
            }
            '=' => {
                if self.cursor.next_if(|c| c == '=').is_some() {
                    TokenKind::EqEq
                } else if self.cursor.next_if(|c| c == '>').is_some() {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Equal
                }
            }
            '!' => {
                if self.cursor.next_if(|c| c == '=').is_some() {
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                if self.cursor.next_if(|c| c == '=').is_some() {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.cursor.next_if(|c| c == '=').is_some() {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.cursor.next_if(|c| c == '&').is_some() {
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.cursor.next_if(|c| c == '|').is_some() {
                    TokenKind::PipePipe
                } else if self.cursor.next_if(|c| c == '>').is_some() {
                    TokenKind::PipeGt
                } else {
                    TokenKind::Pipe
                }
            }
            '"' => self.string(start),
            c if c.is_ascii_digit() => self.number(c, start),
            c if is_ident_start(c) => self.identifier(start),
            other => TokenKind::Error(format!("Unexpected character: '{other}'")),
        };

        let end = self.cursor.offset();
        Token::new(kind, Span::new(start, end))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.cursor.peek() {
                Some(c) if c.is_ascii_whitespace() => {
                    self.cursor.next_char();
                }
                Some('/') if self.cursor.peek_next() == Some('/') => {
                    // Line comment
                    self.cursor.next_char();
                    self.cursor.next_char();
                    while let Some(c) = self.cursor.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.cursor.next_char();
                    }
                }
                _ => break,
            }
        }
    }

    fn string(&mut self, start: usize) -> TokenKind {
        let mut string_content = String::new();
        while let Some(c) = self.cursor.next_char() {
            match c {
                '"' => return TokenKind::Str(string_content),
                '\\' => {
                    if let Some(escaped) = self.cursor.next_char() {
                        match escaped {
                            'n' => string_content.push('\n'),
                            't' => string_content.push('\t'),
                            'r' => string_content.push('\r'),
                            '\\' => string_content.push('\\'),
                            '"' => string_content.push('"'),
                            other => string_content.push(other),
                        }
                    }
                }
                other => string_content.push(other),
            }
        }
        TokenKind::Error(format!("Unterminated string literal at offset {start}"))
    }

    fn number(&mut self, _first: char, start: usize) -> TokenKind {
        let mut is_float = false;
        while let Some(c) = self.cursor.peek() {
            if c.is_ascii_digit() {
                self.cursor.next_char();
            } else if c == '.'
                && !is_float
                && self
                    .cursor
                    .peek_next()
                    .is_some_and(|next| next.is_ascii_digit())
            {
                is_float = true;
                self.cursor.next_char();
            } else {
                break;
            }
        }

        let slice = self.cursor.slice(start, self.cursor.offset());
        if is_float {
            slice.parse::<f64>().map_or_else(
                |_| TokenKind::Error(format!("Invalid float literal: '{slice}'")),
                TokenKind::Float,
            )
        } else {
            slice.parse::<i64>().map_or_else(
                |_| TokenKind::Error(format!("Invalid integer literal: '{slice}'")),
                TokenKind::Int,
            )
        }
    }

    fn identifier(&mut self, start: usize) -> TokenKind {
        while let Some(c) = self.cursor.peek() {
            if is_ident_continue(c) {
                self.cursor.next_char();
            } else {
                break;
            }
        }

        let text = self.cursor.slice(start, self.cursor.offset());
        match text {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "unsafe" => TokenKind::Unsafe,
            "match" => TokenKind::Match,
            "nil" => TokenKind::Nil,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            _ => TokenKind::Ident(text.to_string()),
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_basic_tokens() {
        let code = "fn add(a: i32, b: i32) -> i32 { return a + b }";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Ident("add".into()));
        assert_eq!(tokens[2].kind, TokenKind::OpenParen);
    }

    #[test]
    fn test_pipeline_and_arrow() {
        let code = "x |> map() -> int";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Ident("x".into()));
        assert_eq!(tokens[1].kind, TokenKind::PipeGt);
        assert_eq!(tokens[2].kind, TokenKind::Ident("map".into()));
        assert_eq!(tokens[3].kind, TokenKind::OpenParen);
        assert_eq!(tokens[4].kind, TokenKind::CloseParen);
        assert_eq!(tokens[5].kind, TokenKind::Arrow);
    }
}
