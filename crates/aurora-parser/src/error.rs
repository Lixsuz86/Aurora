//! Syntax and parsing error types.

use aurora_ast::Span;
use thiserror::Error;

/// Parse errors with span information.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token: found '{found}' at {span}, expected {expected}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Unexpected end of file, expected {expected} at {span}")]
    UnexpectedEof { expected: String, span: Span },

    #[error("Invalid syntax: {message} at {span}")]
    Custom { message: String, span: Span },
}
