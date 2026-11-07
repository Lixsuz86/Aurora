//! Syntax and parsing error types.

use aurora_ast::Span;
use thiserror::Error;

/// Parse errors with span information.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("Unexpected token: found '{found}' at {span}, expected {expected}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Unexpected end of file, expected {expected} at {span}")]
    UnexpectedEof { expected: String, span: Span },

    #[error("Invalid assignment target at {span}")]
    InvalidAssignmentTarget { span: Span },

    #[error("Invalid syntax: {message} at {span}")]
    Custom { message: String, span: Span },
}

impl ParseError {
    /// Returns the source span of the error.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::UnexpectedToken { span, .. }
            | Self::UnexpectedEof { span, .. }
            | Self::InvalidAssignmentTarget { span }
            | Self::Custom { span, .. } => *span,
        }
    }
}
