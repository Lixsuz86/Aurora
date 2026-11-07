//! Zero-copy Lexical Analyzer for the Aurora programming language.

use crate::cursor::Cursor;
use crate::diagnostic::Diagnostic;
use crate::token::{FloatSuffix, IntBase, IntSuffix, Token, TokenKind};
use aurora_ast::Span;

/// Lexical analyzer that transforms Aurora source text into a stream of tokens.
#[derive(Debug)]
pub struct Lexer<'a> {
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    /// Creates a new Lexer for the given source code slice.
    pub fn new(source: &'a str) -> Self {
        Self {
            cursor: Cursor::new(source),
            diagnostics: Vec::new(),
        }
    }

    /// Returns all collected diagnostics (errors, warnings).
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Consumes the lexer and returns any diagnostics collected during tokenization.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Scans the entire input and collects all tokens until EOF into a vector.
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
            // Arithmetic & Compound
            '+' => {
                if self.cursor.consume_if_eq('=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.cursor.consume_if_eq('>') {
                    TokenKind::Arrow
                } else if self.cursor.consume_if_eq('=') {
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.cursor.consume_if_eq('*') {
                    TokenKind::StarStar
                } else if self.cursor.consume_if_eq('=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.cursor.consume_if_eq('=') {
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                if self.cursor.consume_if_eq('=') {
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }

            // Bitwise & Compound
            '&' => {
                if self.cursor.consume_if_eq('&') {
                    TokenKind::AmpAmp
                } else if self.cursor.consume_if_eq('=') {
                    TokenKind::AmpEq
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.cursor.consume_if_eq('|') {
                    TokenKind::PipePipe
                } else if self.cursor.consume_if_eq('>') {
                    TokenKind::PipeGt
                } else if self.cursor.consume_if_eq('=') {
                    TokenKind::PipeEq
                } else {
                    TokenKind::Pipe
                }
            }
            '^' => {
                if self.cursor.consume_if_eq('=') {
                    TokenKind::CaretEq
                } else {
                    TokenKind::Caret
                }
            }
            '~' => TokenKind::Tilde,

            // Relational & Equality
            '=' => {
                if self.cursor.consume_if_eq('=') {
                    TokenKind::EqEq
                } else if self.cursor.consume_if_eq('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Equal
                }
            }
            '!' => {
                if self.cursor.consume_if_eq('=') {
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                if self.cursor.consume_if_eq('<') {
                    if self.cursor.consume_if_eq('=') {
                        TokenKind::ShlEq
                    } else {
                        TokenKind::Shl
                    }
                } else if self.cursor.consume_if_eq('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.cursor.consume_if_eq('>') {
                    if self.cursor.consume_if_eq('=') {
                        TokenKind::ShrEq
                    } else {
                        TokenKind::Shr
                    }
                } else if self.cursor.consume_if_eq('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }

            // Punctuation & Delimiters
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '@' => TokenKind::At,
            ':' => {
                if self.cursor.consume_if_eq(':') {
                    TokenKind::DoubleColon
                } else {
                    TokenKind::Colon
                }
            }
            '?' => {
                if self.cursor.consume_if_eq('?') {
                    TokenKind::QuestionQuestion
                } else {
                    TokenKind::Question
                }
            }
            '.' => {
                if self.cursor.consume_if_eq('.') {
                    if self.cursor.consume_if_eq('=') {
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }

            // String & Character Literals
            '"' => self.lex_string(start),
            '\'' => self.lex_char(start),

            // Raw strings: r"..." or identifiers
            'r' if self.cursor.peek() == Some('"') => {
                self.cursor.next_char(); // consume "
                self.lex_raw_string(start)
            }

            // Numbers
            '0' if self
                .cursor
                .peek()
                .is_some_and(|c| matches!(c, 'x' | 'X' | 'b' | 'B' | 'o' | 'O')) =>
            {
                self.lex_radix_number(start)
            }
            c if c.is_ascii_digit() => self.lex_decimal_or_float(start),

            // Identifiers & Keywords
            c if is_ident_start(c) => self.lex_identifier(start),

            other => {
                let err_msg = format!("Unexpected character '{other}' in input stream");
                let span = Span::new(start, self.cursor.offset());
                self.diagnostics.push(
                    Diagnostic::error(&err_msg, span)
                        .with_label("unknown or unsupported character")
                        .with_hint("ensure your source file is encoded in UTF-8 without control characters"),
                );
                TokenKind::Error(err_msg)
            }
        };

        let end = self.cursor.offset();
        Token::new(kind, Span::new(start, end))
    }

    /// Skips whitespace, single-line comments, and nested multi-line block comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.cursor.peek() {
                Some(c) if c.is_whitespace() => {
                    self.cursor.next_char();
                }
                Some('/') if self.cursor.peek_next() == Some('/') => {
                    // Line comment: // until \n or EOF
                    self.cursor.next_char();
                    self.cursor.next_char();
                    while let Some(c) = self.cursor.peek() {
                        if c == '\n' {
                            self.cursor.next_char();
                            break;
                        }
                        self.cursor.next_char();
                    }
                }
                Some('/') if self.cursor.peek_next() == Some('*') => {
                    // Nested block comments: /* /* inner */ */
                    let start_comment = self.cursor.offset();
                    self.cursor.next_char(); // '/'
                    self.cursor.next_char(); // '*'
                    let mut depth = 1usize;

                    while depth > 0 {
                        match self.cursor.next_char() {
                            Some('/') if self.cursor.peek() == Some('*') => {
                                self.cursor.next_char();
                                depth += 1;
                            }
                            Some('*') if self.cursor.peek() == Some('/') => {
                                self.cursor.next_char();
                                depth -= 1;
                            }
                            Some(_) => {}
                            None => {
                                let span = Span::new(start_comment, self.cursor.offset());
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        "Unterminated multi-line block comment",
                                        span,
                                    )
                                    .with_label("comment started here")
                                    .with_hint("add '*/' to close the block comment"),
                                );
                                break;
                            }
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// Scans standard double-quoted string with full escape sequence support.
    fn lex_string(&mut self, start: usize) -> TokenKind {
        let mut buf = String::new();

        while let Some(c) = self.cursor.next_char() {
            match c {
                '"' => return TokenKind::Str(buf),
                '\\' => {
                    let esc_start = self.cursor.offset() - 1;
                    if let Some(escaped) = self.cursor.next_char() {
                        match escaped {
                            'n' => buf.push('\n'),
                            't' => buf.push('\t'),
                            'r' => buf.push('\r'),
                            '0' => buf.push('\0'),
                            '\\' => buf.push('\\'),
                            '"' => buf.push('"'),
                            '\'' => buf.push('\''),
                            'x' => {
                                // \xHH (two hex digits)
                                let h1 = self.cursor.next_char().unwrap_or('\0');
                                let h2 = self.cursor.next_char().unwrap_or('\0');
                                let hex_str = format!("{h1}{h2}");
                                if let Ok(byte_val) = u8::from_str_radix(&hex_str, 16) {
                                    buf.push(byte_val as char);
                                } else {
                                    let span = Span::new(esc_start, self.cursor.offset());
                                    self.diagnostics.push(
                                        Diagnostic::error(
                                            format!("Invalid hex escape sequence '\\x{hex_str}'"),
                                            span,
                                        )
                                        .with_label("expected two hexadecimal digits (0-9, a-f)")
                                        .with_hint("example: '\\x7F'"),
                                    );
                                }
                            }
                            'u' if self.cursor.peek() == Some('{') => {
                                // \u{HHHH}
                                self.cursor.next_char(); // consume '{'
                                let hex_start = self.cursor.offset();
                                self.cursor.eat_while(|ch| ch.is_ascii_hexdigit());
                                let hex_str = self.cursor.slice(hex_start, self.cursor.offset());
                                if self.cursor.consume_if_eq('}') {
                                    if let Ok(code_point) = u32::from_str_radix(hex_str, 16) {
                                        if let Some(valid_char) = char::from_u32(code_point) {
                                            buf.push(valid_char);
                                        } else {
                                            let span = Span::new(esc_start, self.cursor.offset());
                                            self.diagnostics.push(
                                                Diagnostic::error(
                                                    format!("Invalid unicode scalar value '\\u{{{hex_str}}}'"),
                                                    span,
                                                )
                                                .with_hint("unicode code points must be within 0x0 to 0x10FFFF"),
                                            );
                                        }
                                    }
                                } else {
                                    let span = Span::new(esc_start, self.cursor.offset());
                                    self.diagnostics.push(
                                        Diagnostic::error(
                                            "Unterminated unicode escape sequence",
                                            span,
                                        )
                                        .with_hint("close with '}'"),
                                    );
                                }
                            }
                            other => {
                                let span = Span::new(esc_start, self.cursor.offset());
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        format!("Unknown escape sequence '\\{other}'"),
                                        span,
                                    )
                                    .with_hint("valid escape sequences are \\n, \\t, \\r, \\0, \\\\, \\\", \\', \\xHH, \\u{...}"),
                                );
                                buf.push(other);
                            }
                        }
                    }
                }
                other => buf.push(other),
            }
        }

        let span = Span::new(start, self.cursor.offset());
        let err_msg = "Unterminated string literal".to_string();
        self.diagnostics.push(
            Diagnostic::error(&err_msg, span)
                .with_label("string started here")
                .with_hint("add a closing '\"' before the end of the file"),
        );
        TokenKind::Error(err_msg)
    }

    /// Scans a raw string literal `r"..."`.
    fn lex_raw_string(&mut self, start: usize) -> TokenKind {
        let content_start = self.cursor.offset();
        while let Some(c) = self.cursor.next_char() {
            if c == '"' {
                let content = self.cursor.slice(content_start, self.cursor.offset() - 1);
                return TokenKind::RawStr(content.to_string());
            }
        }

        let span = Span::new(start, self.cursor.offset());
        let err_msg = "Unterminated raw string literal".to_string();
        self.diagnostics.push(
            Diagnostic::error(&err_msg, span)
                .with_label("raw string opened here")
                .with_hint("add a closing '\"'"),
        );
        TokenKind::Error(err_msg)
    }

    /// Scans a character literal `'c'`.
    fn lex_char(&mut self, start: usize) -> TokenKind {
        let Some(c) = self.cursor.next_char() else {
            let span = Span::new(start, self.cursor.offset());
            let msg = "Unterminated character literal".to_string();
            self.diagnostics.push(Diagnostic::error(&msg, span));
            return TokenKind::Error(msg);
        };

        let ch_val = if c == '\\' {
            match self.cursor.next_char() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('0') => '\0',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some('"') => '"',
                Some(other) => other,
                None => {
                    let span = Span::new(start, self.cursor.offset());
                    let msg = "Unterminated escape sequence in character literal".to_string();
                    self.diagnostics.push(Diagnostic::error(&msg, span));
                    return TokenKind::Error(msg);
                }
            }
        } else {
            c
        };

        if self.cursor.consume_if_eq('\'') {
            TokenKind::Char(ch_val)
        } else {
            let span = Span::new(start, self.cursor.offset());
            let err_msg =
                "Character literal must contain exactly one character and close with '\''"
                    .to_string();
            self.diagnostics.push(Diagnostic::error(&err_msg, span));
            TokenKind::Error(err_msg)
        }
    }

    /// Scans numbers with alternate radix (0x hex, 0b binary, 0o octal).
    fn lex_radix_number(&mut self, start: usize) -> TokenKind {
        let prefix = self.cursor.next_char().unwrap(); // 'x', 'b', or 'o'
        let (base, radix) = match prefix {
            'x' | 'X' => (IntBase::Hexadecimal, 16),
            'b' | 'B' => (IntBase::Binary, 2),
            'o' | 'O' => (IntBase::Octal, 8),
            _ => unreachable!(),
        };

        let digits_start = self.cursor.offset();
        match base {
            IntBase::Hexadecimal => self.cursor.eat_while(|c| c.is_ascii_hexdigit() || c == '_'),
            IntBase::Binary => self.cursor.eat_while(|c| matches!(c, '0' | '1' | '_')),
            IntBase::Octal => self.cursor.eat_while(|c| matches!(c, '0'..='7' | '_')),
            IntBase::Decimal => self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_'),
        }
        let raw_digits = self.cursor.slice(digits_start, self.cursor.offset());
        let clean_digits: String = raw_digits.chars().filter(|&c| c != '_').collect();

        if clean_digits.is_empty() {
            let span = Span::new(start, self.cursor.offset());
            let err_msg = format!("Missing digits after '{prefix}' base prefix");
            self.diagnostics.push(Diagnostic::error(&err_msg, span));
            return TokenKind::Error(err_msg);
        }

        let suffix = self.lex_int_suffix();

        match u64::from_str_radix(&clean_digits, radix) {
            Ok(value) => TokenKind::Int {
                value: value as i64,
                base,
                suffix,
            },
            Err(err) => {
                let span = Span::new(start, self.cursor.offset());
                let err_msg = format!("Invalid {base:?} integer literal '{raw_digits}': {err}");
                self.diagnostics.push(Diagnostic::error(&err_msg, span));
                TokenKind::Error(err_msg)
            }
        }
    }

    /// Scans decimal numbers or floating-point numbers.
    fn lex_decimal_or_float(&mut self, start: usize) -> TokenKind {
        let mut is_float = false;

        // Scan integer digits & separators
        self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');

        // Check for decimal point (make sure it's not range `..`)
        if self.cursor.peek() == Some('.')
            && self.cursor.peek_next().is_some_and(|c| c.is_ascii_digit())
        {
            is_float = true;
            self.cursor.next_char(); // consume '.'
            self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
        }

        // Check for scientific exponent: e.g. 1e10, 2.5e-3
        if self.cursor.peek().is_some_and(|c| matches!(c, 'e' | 'E')) {
            is_float = true;
            self.cursor.next_char(); // consume 'e' or 'E'
            if self.cursor.peek().is_some_and(|c| matches!(c, '+' | '-')) {
                self.cursor.next_char();
            }
            self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
        }

        let raw_slice = self.cursor.slice(start, self.cursor.offset());
        let clean: String = raw_slice.chars().filter(|&c| c != '_').collect();

        if is_float {
            let suffix = self.lex_float_suffix();
            match clean.parse::<f64>() {
                Ok(value) => TokenKind::Float { value, suffix },
                Err(err) => {
                    let span = Span::new(start, self.cursor.offset());
                    let err_msg = format!("Invalid floating-point literal '{raw_slice}': {err}");
                    self.diagnostics.push(Diagnostic::error(&err_msg, span));
                    TokenKind::Error(err_msg)
                }
            }
        } else {
            let suffix = self.lex_int_suffix();
            match clean.parse::<i64>() {
                Ok(value) => TokenKind::Int {
                    value,
                    base: IntBase::Decimal,
                    suffix,
                },
                Err(err) => {
                    let span = Span::new(start, self.cursor.offset());
                    let err_msg = format!("Invalid integer literal '{raw_slice}': {err}");
                    self.diagnostics.push(Diagnostic::error(&err_msg, span));
                    TokenKind::Error(err_msg)
                }
            }
        }
    }

    /// Looks for integer suffixes: u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    fn lex_int_suffix(&mut self) -> Option<IntSuffix> {
        let suffix_start = self.cursor.offset();
        self.cursor.eat_while(|c| c.is_ascii_alphanumeric());
        let suffix_str = self.cursor.slice(suffix_start, self.cursor.offset());

        match suffix_str {
            "i8" => Some(IntSuffix::I8),
            "i16" => Some(IntSuffix::I16),
            "i32" => Some(IntSuffix::I32),
            "i64" => Some(IntSuffix::I64),
            "i128" => Some(IntSuffix::I128),
            "isize" => Some(IntSuffix::Isize),
            "u8" => Some(IntSuffix::U8),
            "u16" => Some(IntSuffix::U16),
            "u32" => Some(IntSuffix::U32),
            "u64" => Some(IntSuffix::U64),
            "u128" => Some(IntSuffix::U128),
            "usize" => Some(IntSuffix::Usize),
            "" => None,
            other => {
                let span = Span::new(suffix_start, self.cursor.offset());
                self.diagnostics.push(
                    Diagnostic::error(format!("Invalid integer type suffix '{other}'"), span)
                        .with_hint("valid suffixes are i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize"),
                );
                None
            }
        }
    }

    /// Looks for float suffixes: f32, f64
    fn lex_float_suffix(&mut self) -> Option<FloatSuffix> {
        let suffix_start = self.cursor.offset();
        self.cursor.eat_while(|c| c.is_ascii_alphanumeric());
        let suffix_str = self.cursor.slice(suffix_start, self.cursor.offset());

        match suffix_str {
            "f32" => Some(FloatSuffix::F32),
            "f64" => Some(FloatSuffix::F64),
            "" => None,
            other => {
                let span = Span::new(suffix_start, self.cursor.offset());
                self.diagnostics.push(
                    Diagnostic::error(format!("Invalid float type suffix '{other}'"), span)
                        .with_hint("valid float suffixes are f32 or f64"),
                );
                None
            }
        }
    }

    /// Scans identifiers and keyword matches.
    fn lex_identifier(&mut self, start: usize) -> TokenKind {
        self.cursor.eat_while(is_ident_continue);
        let text = self.cursor.slice(start, self.cursor.offset());

        match text {
            // High-Level Keywords
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "yield" => TokenKind::Yield,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "type" => TokenKind::Type,
            "use" => TokenKind::Use,
            "pub" => TokenKind::Pub,
            "mod" => TokenKind::Mod,
            "as" => TokenKind::As,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "self" => TokenKind::SelfValue,
            "Self" => TokenKind::SelfType,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            "nil" => TokenKind::Nil,

            // Low-Level / Systems Keywords
            "unsafe" => TokenKind::Unsafe,
            "static" => TokenKind::Static,
            "sizeof" => TokenKind::Sizeof,
            "alignof" => TokenKind::Alignof,
            "volatile" => TokenKind::Volatile,
            "union" => TokenKind::Union,
            "extern" => TokenKind::Extern,

            _ => TokenKind::Ident(text.to_string()),
        }
    }
}

#[inline]
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

#[inline]
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_radix_numbers() {
        let code = "0xDEAD_BEEFu64 0b1010_1100u8 0o755i32 1_000_000usize";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(
            tokens[0].kind,
            TokenKind::Int {
                value: 0xDEAD_BEEF,
                base: IntBase::Hexadecimal,
                suffix: Some(IntSuffix::U64)
            }
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Int {
                value: 0b1010_1100,
                base: IntBase::Binary,
                suffix: Some(IntSuffix::U8)
            }
        );
        assert_eq!(
            tokens[2].kind,
            TokenKind::Int {
                value: 0o755,
                base: IntBase::Octal,
                suffix: Some(IntSuffix::I32)
            }
        );
        assert_eq!(
            tokens[3].kind,
            TokenKind::Int {
                value: 1_000_000,
                base: IntBase::Decimal,
                suffix: Some(IntSuffix::Usize)
            }
        );
    }

    #[test]
    fn test_lex_floats_and_scientific() {
        let code = "3.14159f64 1.5e-3f32 2E+4";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(
            tokens[0].kind,
            TokenKind::Float {
                value: 3.14159,
                suffix: Some(FloatSuffix::F64)
            }
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Float {
                value: 1.5e-3,
                suffix: Some(FloatSuffix::F32)
            }
        );
        assert_eq!(
            tokens[2].kind,
            TokenKind::Float {
                value: 20000.0,
                suffix: None
            }
        );
    }

    #[test]
    fn test_lex_nested_comments() {
        let code = "/* outer /* inner */ still outer */ let x = 42";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Ident("x".into()));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert_eq!(
            tokens[3].kind,
            TokenKind::Int {
                value: 42,
                base: IntBase::Decimal,
                suffix: None
            }
        );
    }

    #[test]
    fn test_lex_escaped_and_raw_strings() {
        let code = r#""hello\nworld\t\"escaped\"" r"C:\Windows\System32" 'A' '\n'"#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(
            tokens[0].kind,
            TokenKind::Str("hello\nworld\t\"escaped\"".into())
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::RawStr(r"C:\Windows\System32".into())
        );
        assert_eq!(tokens[2].kind, TokenKind::Char('A'));
        assert_eq!(tokens[3].kind, TokenKind::Char('\n'));
    }

    #[test]
    fn test_lex_systems_and_modern_operators() {
        let code = "unsafe sizeof alignof |> ..= -> => ?? ** += <<=";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Unsafe);
        assert_eq!(tokens[1].kind, TokenKind::Sizeof);
        assert_eq!(tokens[2].kind, TokenKind::Alignof);
        assert_eq!(tokens[3].kind, TokenKind::PipeGt);
        assert_eq!(tokens[4].kind, TokenKind::DotDotEq);
        assert_eq!(tokens[5].kind, TokenKind::Arrow);
        assert_eq!(tokens[6].kind, TokenKind::FatArrow);
        assert_eq!(tokens[7].kind, TokenKind::QuestionQuestion);
        assert_eq!(tokens[8].kind, TokenKind::StarStar);
        assert_eq!(tokens[9].kind, TokenKind::PlusEq);
        assert_eq!(tokens[10].kind, TokenKind::ShlEq);
    }
}
