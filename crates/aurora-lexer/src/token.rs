//! Token types for Aurora.

use aurora_ast::Span;
use std::fmt;

/// Distinct kinds of tokens recognized by the Aurora lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // End of Input
    Eof,

    // Identifiers & Literals
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),

    // Keywords
    Fn,
    Let,
    Mut,
    Const,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Struct,
    Enum,
    Trait,
    Impl,
    Unsafe,
    Match,
    Nil,

    // Operators & Punctuation
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Equal,       // =
    EqEq,        // ==
    Bang,        // !
    BangEq,      // !=
    Lt,          // <
    LtEq,        // <=
    Gt,          // >
    GtEq,        // >=
    AmpAmp,      // &&
    PipePipe,    // ||
    PipeGt,      // |>
    Arrow,       // ->
    FatArrow,    // =>
    Colon,       // :
    DoubleColon, // ::
    Semicolon,   // ;
    Comma,       // ,
    Dot,         // .
    Question,    // ?
    Ampersand,   // &
    Pipe,        // |
    Caret,       // ^

    // Delimiters
    OpenParen,    // (
    CloseParen,   // )
    OpenBrace,    // {
    CloseBrace,   // }
    OpenBracket,  // [
    CloseBracket, // ]

    // Lexer Error Token
    Error(String),
}

/// A token tagged with its position span in source code.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "<EOF>"),
            Self::Ident(s) => write!(f, "identifier '{s}'"),
            Self::Int(i) => write!(f, "int '{i}'"),
            Self::Float(fl) => write!(f, "float '{fl}'"),
            Self::Str(s) => write!(f, "string \"{s}\""),
            Self::Bool(b) => write!(f, "bool '{b}'"),
            Self::Fn => write!(f, "fn"),
            Self::Let => write!(f, "let"),
            Self::Mut => write!(f, "mut"),
            Self::Const => write!(f, "const"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::While => write!(f, "while"),
            Self::For => write!(f, "for"),
            Self::In => write!(f, "in"),
            Self::Return => write!(f, "return"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Trait => write!(f, "trait"),
            Self::Impl => write!(f, "impl"),
            Self::Unsafe => write!(f, "unsafe"),
            Self::Match => write!(f, "match"),
            Self::Nil => write!(f, "nil"),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Percent => write!(f, "%"),
            Self::Equal => write!(f, "="),
            Self::EqEq => write!(f, "=="),
            Self::Bang => write!(f, "!"),
            Self::BangEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
            Self::AmpAmp => write!(f, "&&"),
            Self::PipePipe => write!(f, "||"),
            Self::PipeGt => write!(f, "|>"),
            Self::Arrow => write!(f, "->"),
            Self::FatArrow => write!(f, "=>"),
            Self::Colon => write!(f, ":"),
            Self::DoubleColon => write!(f, "::"),
            Self::Semicolon => write!(f, ";"),
            Self::Comma => write!(f, ","),
            Self::Dot => write!(f, "."),
            Self::Question => write!(f, "?"),
            Self::Ampersand => write!(f, "&"),
            Self::Pipe => write!(f, "|"),
            Self::Caret => write!(f, "^"),
            Self::OpenParen => write!(f, "("),
            Self::CloseParen => write!(f, ")"),
            Self::OpenBrace => write!(f, "{{"),
            Self::CloseBrace => write!(f, "}}"),
            Self::OpenBracket => write!(f, "["),
            Self::CloseBracket => write!(f, "]"),
            Self::Error(msg) => write!(f, "error: {msg}"),
        }
    }
}
