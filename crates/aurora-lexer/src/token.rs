//! Comprehensive token types for the Aurora programming language.

use aurora_ast::Span;
use std::fmt;

/// Radix base for integer literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntBase {
    Decimal,
    Hexadecimal, // 0x...
    Binary,      // 0b...
    Octal,       // 0o...
}

/// Suffix on integer literals (e.g. 42u64, 100i32, 10usize).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntSuffix {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

/// Suffix on float literals (e.g. 3.14f32, 2.71f64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatSuffix {
    F32,
    F64,
}

/// Distinct kinds of tokens recognized by the Aurora lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // End of Input
    Eof,

    // Identifiers
    Ident(String),

    // Literals
    Int {
        value: i64,
        base: IntBase,
        suffix: Option<IntSuffix>,
    },
    Float {
        value: f64,
        suffix: Option<FloatSuffix>,
    },
    Str(String),
    RawStr(String),
    Char(char),
    Bool(bool),
    Nil,

    // High-Level Keywords
    Fn,
    Let,
    Mut,
    Const,
    If,
    Else,
    Match,
    While,
    For,
    In,
    Loop,
    Break,
    Continue,
    Return,
    Yield,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Use,
    Pub,
    Mod,
    As,
    Async,
    Await,
    SelfValue, // 'self'
    SelfType,  // 'Self'

    // Low-Level / Systems Keywords
    Unsafe,
    Static,
    Sizeof,
    Alignof,
    Volatile,
    Union,
    Extern,

    // Arithmetic Operators
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    StarStar, // ** (power)

    // Compound Assignment Operators
    PlusEq,    // +=
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    PercentEq, // %=
    AmpEq,     // &=
    PipeEq,    // |=
    CaretEq,   // ^=
    ShlEq,     // <<=
    ShrEq,     // >>=

    // Comparison Operators
    Equal,  // =
    EqEq,   // ==
    Bang,   // !
    BangEq, // !=
    Lt,     // <
    LtEq,   // <=
    Gt,     // >
    GtEq,   // >=

    // Logical Operators
    AmpAmp,   // &&
    PipePipe, // ||

    // Bitwise Operators
    Ampersand, // &
    Pipe,      // |
    Caret,     // ^
    Tilde,     // ~
    Shl,       // <<
    Shr,       // >>

    // Range Operators
    DotDot,   // ..
    DotDotEq, // ..=

    // Modern Flow & Punctuation
    PipeGt,           // |> (pipeline)
    Arrow,            // -> (return type)
    FatArrow,         // => (match arm / lambda)
    Colon,            // :
    DoubleColon,      // :: (namespace separator)
    Semicolon,        // ;
    Comma,            // ,
    Dot,              // .
    Question,         // ? (optional / try)
    QuestionQuestion, // ?? (null coalescing)
    At,               // @ (pattern binding / attribute)

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
    #[inline]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "<EOF>"),
            Self::Ident(s) => write!(f, "identifier '{s}'"),
            Self::Int {
                value,
                base,
                suffix,
            } => {
                let suffix_str = suffix.map_or("", |s| match s {
                    IntSuffix::I8 => "i8",
                    IntSuffix::I16 => "i16",
                    IntSuffix::I32 => "i32",
                    IntSuffix::I64 => "i64",
                    IntSuffix::I128 => "i128",
                    IntSuffix::Isize => "isize",
                    IntSuffix::U8 => "u8",
                    IntSuffix::U16 => "u16",
                    IntSuffix::U32 => "u32",
                    IntSuffix::U64 => "u64",
                    IntSuffix::U128 => "u128",
                    IntSuffix::Usize => "usize",
                });
                match base {
                    IntBase::Decimal => write!(f, "{value}{suffix_str}"),
                    IntBase::Hexadecimal => write!(f, "0x{value:x}{suffix_str}"),
                    IntBase::Binary => write!(f, "0b{value:b}{suffix_str}"),
                    IntBase::Octal => write!(f, "0o{value:o}{suffix_str}"),
                }
            }
            Self::Float { value, suffix } => {
                let suffix_str = suffix.map_or("", |s| match s {
                    FloatSuffix::F32 => "f32",
                    FloatSuffix::F64 => "f64",
                });
                write!(f, "{value}{suffix_str}")
            }
            Self::Str(s) => write!(f, "\"{s}\""),
            Self::RawStr(s) => write!(f, "r\"{s}\""),
            Self::Char(c) => write!(f, "'{c}'"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Nil => write!(f, "nil"),
            Self::Fn => write!(f, "fn"),
            Self::Let => write!(f, "let"),
            Self::Mut => write!(f, "mut"),
            Self::Const => write!(f, "const"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::Match => write!(f, "match"),
            Self::While => write!(f, "while"),
            Self::For => write!(f, "for"),
            Self::In => write!(f, "in"),
            Self::Loop => write!(f, "loop"),
            Self::Break => write!(f, "break"),
            Self::Continue => write!(f, "continue"),
            Self::Return => write!(f, "return"),
            Self::Yield => write!(f, "yield"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Trait => write!(f, "trait"),
            Self::Impl => write!(f, "impl"),
            Self::Type => write!(f, "type"),
            Self::Use => write!(f, "use"),
            Self::Pub => write!(f, "pub"),
            Self::Mod => write!(f, "mod"),
            Self::As => write!(f, "as"),
            Self::Async => write!(f, "async"),
            Self::Await => write!(f, "await"),
            Self::SelfValue => write!(f, "self"),
            Self::SelfType => write!(f, "Self"),
            Self::Unsafe => write!(f, "unsafe"),
            Self::Static => write!(f, "static"),
            Self::Sizeof => write!(f, "sizeof"),
            Self::Alignof => write!(f, "alignof"),
            Self::Volatile => write!(f, "volatile"),
            Self::Union => write!(f, "union"),
            Self::Extern => write!(f, "extern"),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Percent => write!(f, "%"),
            Self::StarStar => write!(f, "**"),
            Self::PlusEq => write!(f, "+="),
            Self::MinusEq => write!(f, "-="),
            Self::StarEq => write!(f, "*="),
            Self::SlashEq => write!(f, "/="),
            Self::PercentEq => write!(f, "%="),
            Self::AmpEq => write!(f, "&="),
            Self::PipeEq => write!(f, "|="),
            Self::CaretEq => write!(f, "^="),
            Self::ShlEq => write!(f, "<<="),
            Self::ShrEq => write!(f, ">>="),
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
            Self::Ampersand => write!(f, "&"),
            Self::Pipe => write!(f, "|"),
            Self::Caret => write!(f, "^"),
            Self::Tilde => write!(f, "~"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
            Self::DotDot => write!(f, ".."),
            Self::DotDotEq => write!(f, "..="),
            Self::PipeGt => write!(f, "|>"),
            Self::Arrow => write!(f, "->"),
            Self::FatArrow => write!(f, "=>"),
            Self::Colon => write!(f, ":"),
            Self::DoubleColon => write!(f, "::"),
            Self::Semicolon => write!(f, ";"),
            Self::Comma => write!(f, ","),
            Self::Dot => write!(f, "."),
            Self::Question => write!(f, "?"),
            Self::QuestionQuestion => write!(f, "??"),
            Self::At => write!(f, "@"),
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
