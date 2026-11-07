//! Core Abstract Syntax Tree (AST) node definitions for Aurora.

use crate::span::Span;
use std::fmt;

/// An identified name tagged with its source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    #[inline]
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Primitive literal values supported in Aurora.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    RawString(String),
    Char(char),
    Bool(bool),
    Nil,
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(fl) => write!(f, "{fl}"),
            Self::String(s) => write!(f, "\"{s}\""),
            Self::RawString(s) => write!(f, "r\"{s}\""),
            Self::Char(c) => write!(f, "'{c}'"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Nil => write!(f, "nil"),
        }
    }
}

/// Type expressions in Aurora source code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExprKind {
    /// Primitive named types or identifiers (e.g. `i32`, `f64`, `bool`, `string`, `MyType`).
    Named(Ident),
    /// Generic type application (e.g. `Option<T>`, `Map<K, V>`).
    Generic { name: Ident, args: Vec<TypeExpr> },
    /// Raw pointer (e.g. `*const T`, `*mut T`).
    Pointer {
        inner: Box<TypeExpr>,
        is_mutable: bool,
    },
    /// Reference type (e.g. `&T`, `&mut T`).
    Reference {
        inner: Box<TypeExpr>,
        is_mutable: bool,
    },
    /// Optional type syntax sugar (e.g. `T?`).
    Optional(Box<TypeExpr>),
    /// Slice or array type (e.g. `[T]` or `[T; N]`).
    Array {
        element: Box<TypeExpr>,
        size: Option<usize>,
    },
    /// Tuple type (e.g. `(i32, string)`).
    Tuple(Vec<TypeExpr>),
    /// Function signature (e.g. `fn(i32, i32) -> bool`).
    Function {
        params: Vec<TypeExpr>,
        return_type: Box<TypeExpr>,
    },
    /// Unit type `()`.
    Unit,
}

/// A type expression tagged with its source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

impl TypeExpr {
    #[inline]
    pub fn new(kind: TypeExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,          // +
    Sub,          // -
    Mul,          // *
    Div,          // /
    Mod,          // %
    Power,        // **
    Eq,           // ==
    NotEq,        // !=
    Lt,           // <
    LtEq,         // <=
    Gt,           // >
    GtEq,         // >=
    And,          // &&
    Or,           // ||
    BitAnd,       // &
    BitOr,        // |
    BitXor,       // ^
    Shl,          // <<
    Shr,          // >>
    Pipe,         // |>
    NullCoalesce, // ??
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
            Self::Power => write!(f, "**"),
            Self::Eq => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::BitAnd => write!(f, "&"),
            Self::BitOr => write!(f, "|"),
            Self::BitXor => write!(f, "^"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
            Self::Pipe => write!(f, "|>"),
            Self::NullCoalesce => write!(f, "??"),
        }
    }
}

/// Compound and simple assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    Assign,       // =
    AddAssign,    // +=
    SubAssign,    // -=
    MulAssign,    // *=
    DivAssign,    // /=
    ModAssign,    // %=
    BitAndAssign, // &=
    BitOrAssign,  // |=
    BitXorAssign, // ^=
    ShlAssign,    // <<=
    ShrAssign,    // >>=
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,          // -
    Not,          // !
    BitNot,       // ~
    Deref,        // *ptr
    AddressOf,    // &
    AddressOfMut, // &mut
}

/// A block of statements evaluated in sequence.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    #[inline]
    pub fn new(statements: Vec<Stmt>, span: Span) -> Self {
        Self { statements, span }
    }
}

/// Function parameter definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub type_annotation: TypeExpr,
    pub span: Span,
}

/// An arm in a match expression: `pattern => expr` or `pattern => { block }`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// Pattern representation for match expressions and destructuring.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    Wildcard, // _
    Literal(Literal),
    Variable(Ident),
    Tuple(Vec<Pattern>),
    Path(Vec<Ident>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    #[inline]
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Expression kinds in Aurora.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Variable(Ident),
    Path(Vec<Ident>), // e.g. std::io::println
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Assignment {
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    MemberAccess {
        object: Box<Expr>,
        member: Ident,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    Block(Block),
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Box<Expr>>, // Can be another `If` expr or `Block`
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    While {
        condition: Box<Expr>,
        body: Block,
    },
    For {
        var: Ident,
        iter: Box<Expr>,
        body: Block,
    },
    Loop {
        body: Block,
    },
    Break(Option<Box<Expr>>),
    Continue,
    Return(Option<Box<Expr>>),
    Lambda {
        params: Vec<Param>,
        return_type: Option<TypeExpr>,
        body: Box<Expr>,
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    Cast {
        expr: Box<Expr>,
        target_type: TypeExpr,
    },
    UnsafeBlock(Block),
    Sizeof(TypeExpr),
    Alignof(TypeExpr),
}

/// An expression tagged with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    #[inline]
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Statement kinds in Aurora.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    Let {
        name: Ident,
        type_annotation: Option<TypeExpr>,
        initializer: Option<Expr>,
        is_mutable: bool,
    },
    Const {
        name: Ident,
        type_annotation: Option<TypeExpr>,
        initializer: Expr,
    },
    Return(Option<Expr>),
    Item(Item),
}

/// A statement tagged with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    #[inline]
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Function definition declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub is_unsafe: bool,
    pub is_pub: bool,
    pub is_async: bool,
}

/// Struct field definition.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: Ident,
    pub ty: TypeExpr,
    pub is_pub: bool,
    pub span: Span,
}

/// Struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<StructField>,
    pub is_pub: bool,
}

/// Enum variant definition.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantKind {
    Unit,
    Tuple(Vec<TypeExpr>),
    Struct(Vec<StructField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: Ident,
    pub kind: EnumVariantKind,
    pub span: Span,
}

/// Enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
}

/// Trait / Interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    pub name: Ident,
    pub methods: Vec<FunctionDecl>,
    pub is_pub: bool,
}

/// Implementation block (`impl Trait for Type` or `impl Type`).
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    pub trait_name: Option<Ident>,
    pub target_type: TypeExpr,
    pub methods: Vec<FunctionDecl>,
}

/// Type alias declaration (`type Word = u64;`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDecl {
    pub name: Ident,
    pub target: TypeExpr,
    pub is_pub: bool,
}

/// Import declaration (`use std::io;`).
#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub path: Vec<Ident>,
    pub is_glob: bool,
}

/// Items that can appear at module or file scope.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Function(FunctionDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    TypeAlias(TypeAliasDecl),
    Use(UseDecl),
}

/// An item tagged with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

impl Item {
    #[inline]
    pub fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// A complete Aurora source file / module compilation unit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub items: Vec<Item>,
    pub statements: Vec<Stmt>,
}
