//! Fast, zero-copy lexical analyzer and diagnostics for Aurora.

pub mod cursor;
pub mod diagnostic;
pub mod lexer;
pub mod token;

pub use cursor::Cursor;
pub use diagnostic::{Diagnostic, DiagnosticLevel};
pub use lexer::Lexer;
pub use token::{FloatSuffix, IntBase, IntSuffix, Token, TokenKind};
