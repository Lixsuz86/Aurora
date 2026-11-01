//! Core Abstract Syntax Tree (AST) node definitions for Aurora.

use crate::span::Span;

/// An identified name with its source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

/// Primitive literal values supported in Aurora syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Pipe, // Pipeline operator: |>
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Deref,     // Raw pointer dereference *ptr
    AddressOf, // Pointer address of &mut / &
}

/// Expression nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Variable(Ident),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Block(Vec<Stmt>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
}

/// An expression tagged with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Statements in Aurora.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    Let {
        name: Ident,
        type_annotation: Option<String>,
        initializer: Option<Expr>,
        is_mutable: bool,
    },
    Return(Option<Expr>),
}

/// A statement tagged with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Top-level declarations in an Aurora program or module.
#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    Function {
        name: Ident,
        params: Vec<(Ident, String)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
        is_unsafe: bool,
    },
    Struct {
        name: Ident,
        fields: Vec<(Ident, String)>,
    },
}

/// A declaration tagged with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub kind: DeclKind,
    pub span: Span,
}

/// An entire compiled compilation unit (Module or File).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub declarations: Vec<Decl>,
    pub statements: Vec<Stmt>,
}
