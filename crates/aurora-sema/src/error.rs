//! Semantic, type-checking, and scoping error definitions.

use crate::types::Type;
use aurora_ast::{BinaryOp, Span, UnaryOp};
use aurora_lexer::Diagnostic;
use thiserror::Error;

/// Semantic and type analysis errors with source spans.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SemanticError {
    #[error("Type mismatch: expected '{expected}', found '{found}' at {span}")]
    TypeMismatch {
        expected: Type,
        found: Type,
        span: Span,
    },

    #[error("Undefined variable '{name}' at {span}")]
    UndefinedVariable { name: String, span: Span },

    #[error("Cannot assign to immutable variable '{name}' at {span}")]
    CannotAssignImmutable { name: String, span: Span },

    #[error(
        "Unsafe operation '{operation}' requires an unsafe function or unsafe block at {span}"
    )]
    UnsafeRequired { operation: String, span: Span },

    #[error("Binary operator '{op}' cannot be applied to types '{left}' and '{right}' at {span}")]
    InvalidBinaryOp {
        op: BinaryOp,
        left: Type,
        right: Type,
        span: Span,
    },

    #[error("Unary operator '{op:?}' cannot be applied to type '{operand}' at {span}")]
    InvalidUnaryOp {
        op: UnaryOp,
        operand: Type,
        span: Span,
    },

    #[error("Function expected {expected} arguments, but received {found} at {span}")]
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("Expression of type '{ty}' is not callable at {span}")]
    NotCallable { ty: Type, span: Span },

    #[error("Type '{target_type}' cannot be indexed at {span}")]
    CannotIndex { target_type: Type, span: Span },

    #[error("Struct '{struct_name}' has no field named '{field_name}' at {span}")]
    NoSuchField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    #[error("Symbol '{name}' is already declared in this scope at {current_span}")]
    RedeclaredSymbol {
        name: String,
        previous_span: Span,
        current_span: Span,
    },

    #[error("Invalid return: {message} at {span}")]
    InvalidReturn { message: String, span: Span },

    #[error("Semantic error: {message} at {span}")]
    Custom { message: String, span: Span },
}

impl SemanticError {
    /// Returns the primary span of this error.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::TypeMismatch { span, .. }
            | Self::UndefinedVariable { span, .. }
            | Self::CannotAssignImmutable { span, .. }
            | Self::UnsafeRequired { span, .. }
            | Self::InvalidBinaryOp { span, .. }
            | Self::InvalidUnaryOp { span, .. }
            | Self::ArityMismatch { span, .. }
            | Self::NotCallable { span, .. }
            | Self::CannotIndex { span, .. }
            | Self::NoSuchField { span, .. }
            | Self::InvalidReturn { span, .. }
            | Self::Custom { span, .. } => *span,
            Self::RedeclaredSymbol { current_span, .. } => *current_span,
        }
    }

    /// Converts the semantic error into a rich compiler Diagnostic with hints and labels.
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::TypeMismatch { expected, found, span } => {
                Diagnostic::error(
                    format!("mismatched types: expected `{expected}`, found `{found}`"),
                    *span,
                )
                .with_label(format!("expected `{expected}`"))
                .with_hint(format!("ensure the expression produces `{expected}` instead of `{found}`"))
            }
            Self::UndefinedVariable { name, span } => {
                Diagnostic::error(format!("cannot find value `{name}` in this scope"), *span)
                    .with_label("not found in this scope")
                    .with_hint(format!("declare `{name}` with `let {name} = ...` before using it"))
            }
            Self::CannotAssignImmutable { name, span } => {
                Diagnostic::error(format!("cannot assign to immutable variable `{name}`"), *span)
                    .with_label("cannot assign twice to immutable variable")
                    .with_hint(format!("consider declaring this variable as mutable: `let mut {name} = ...`"))
            }
            Self::UnsafeRequired { operation, span } => {
                Diagnostic::error(
                    format!("operation `{operation}` requires an unsafe context"),
                    *span,
                )
                .with_label("unsafe operation requires unsafe function or block")
                .with_hint("wrap this operation in an `unsafe { ... }` block or mark enclosing function as `unsafe fn`")
            }
            Self::InvalidBinaryOp { op, left, right, span } => {
                Diagnostic::error(
                    format!("cannot apply operator `{op}` to `{left}` and `{right}`"),
                    *span,
                )
                .with_label("invalid operand types")
            }
            Self::ArityMismatch { expected, found, span } => {
                Diagnostic::error(
                    format!("this function takes {expected} arguments but {found} were supplied"),
                    *span,
                )
                .with_label(format!("expected {expected} arguments"))
            }
            Self::RedeclaredSymbol { name, previous_span: _, current_span } => {
                Diagnostic::error(format!("the name `{name}` is defined multiple times"), *current_span)
                    .with_label("re-declaration in the same scope")
            }
            other => Diagnostic::error(other.to_string(), other.span()),
        }
    }
}
