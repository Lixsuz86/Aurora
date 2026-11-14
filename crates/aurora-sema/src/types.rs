//! Comprehensive Type representation and compatibility rules for Aurora.

use aurora_ast::{TypeExpr, TypeExprKind};
use std::fmt;

/// Concrete primitive and compound types in Aurora.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // Unit & Never
    Unit,
    Never, // Bottom type (!)

    // Primitives
    Bool,
    Char,
    String,

    // Signed Integers
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,

    // Unsigned Integers
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,

    // Floating point
    F32,
    F64,

    // Low-Level / Systems Types
    Pointer {
        inner: Box<Type>,
        is_mutable: bool,
    },
    Reference {
        inner: Box<Type>,
        is_mutable: bool,
    },

    // High-Level / Composite Types
    Optional(Box<Type>),
    Array {
        element: Box<Type>,
        size: Option<usize>,
    },
    Tuple(Vec<Type>),
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
    Enum {
        name: String,
        variants: Vec<(String, Vec<Type>)>,
    },
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        is_unsafe: bool,
    },

    // Special placeholder for unresolved or error types
    Unknown,
}

impl Type {
    /// Returns true if this type is an integer (signed or unsigned).
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::I128
                | Self::ISize
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::U128
                | Self::USize
        )
    }

    /// Returns true if this type is a floating-point number.
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// Returns true if this type is numeric (integer or float).
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Returns true if this type is a raw pointer (low-level).
    #[must_use]
    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer { .. })
    }

    /// Returns true if this is an optional type `T?`.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional(_))
    }

    /// Checks if a value of `self` can be coerced or assigned to `target`.
    #[must_use]
    pub fn can_coerce_to(&self, target: &Self) -> bool {
        if self == target
            || *self == Self::Never
            || *self == Self::Unknown
            || *target == Self::Unknown
        {
            return true;
        }

        // T can be implicitly wrapped into T? (Optional promotion)
        if let Self::Optional(inner_target) = target {
            if self.can_coerce_to(inner_target) {
                return true;
            }
        }

        // Integers can coerce to target integer types
        if self.is_integer() && target.is_integer() {
            return true;
        }

        // Floats can coerce to target float types
        if self.is_float() && target.is_float() {
            return true;
        }

        // Array coercion: [T; N] -> [T] (slice) or [T; N] -> [T; N]
        if let (
            Self::Array {
                element: src_elem,
                size: src_size,
            },
            Self::Array {
                element: tgt_elem,
                size: tgt_size,
            },
        ) = (self, target)
        {
            if src_elem.can_coerce_to(tgt_elem) && (tgt_size.is_none() || *tgt_size == *src_size) {
                return true;
            }
        }

        // Pointer coercion:
        // - *mut T -> *const T
        // - *mut u8 / *const u8 -> *mut T / *const T (void* style allocation)
        // - *mut T / *const T -> *mut u8 / *const u8
        if let (
            Self::Pointer {
                inner: src_inner,
                is_mutable: src_mut,
            },
            Self::Pointer {
                inner: tgt_inner,
                is_mutable: tgt_mut,
            },
        ) = (self, target)
        {
            if *tgt_mut && !*src_mut {
                return false;
            }
            if src_inner == tgt_inner || **src_inner == Self::U8 || **tgt_inner == Self::U8 {
                return true;
            }
        }

        false
    }

    /// Resolves an AST `TypeExpr` into a semantic Type using simple name matching.
    pub fn from_ast(ast_type: &TypeExpr) -> Self {
        match &ast_type.kind {
            TypeExprKind::Named(ident) => match ident.name.as_str() {
                "i8" => Self::I8,
                "i16" => Self::I16,
                "i32" => Self::I32,
                "i64" | "int" => Self::I64,
                "i128" => Self::I128,
                "isize" => Self::ISize,
                "u8" | "byte" => Self::U8,
                "u16" => Self::U16,
                "u32" => Self::U32,
                "u64" | "uint" => Self::U64,
                "u128" => Self::U128,
                "usize" => Self::USize,
                "f32" => Self::F32,
                "f64" | "float" => Self::F64,
                "bool" => Self::Bool,
                "char" => Self::Char,
                "string" | "str" => Self::String,
                "void" | "unit" => Self::Unit,
                "auto" => Self::Unknown,
                custom => Self::Struct {
                    name: custom.to_string(),
                    fields: Vec::new(),
                },
            },
            TypeExprKind::Generic { name, args } => {
                if name.name == "Option" && args.len() == 1 {
                    Self::Optional(Box::new(Self::from_ast(&args[0])))
                } else if name.name == "Array" && args.len() == 1 {
                    Self::Array {
                        element: Box::new(Self::from_ast(&args[0])),
                        size: None,
                    }
                } else {
                    Self::Unknown
                }
            }
            TypeExprKind::Pointer { inner, is_mutable } => Self::Pointer {
                inner: Box::new(Self::from_ast(inner)),
                is_mutable: *is_mutable,
            },
            TypeExprKind::Reference { inner, is_mutable } => Self::Reference {
                inner: Box::new(Self::from_ast(inner)),
                is_mutable: *is_mutable,
            },
            TypeExprKind::Optional(inner) => Self::Optional(Box::new(Self::from_ast(inner))),
            TypeExprKind::Array { element, size } => Self::Array {
                element: Box::new(Self::from_ast(element)),
                size: *size,
            },
            TypeExprKind::Tuple(types) => Self::Tuple(types.iter().map(Self::from_ast).collect()),
            TypeExprKind::Function {
                params,
                return_type,
            } => Self::Function {
                params: params.iter().map(Self::from_ast).collect(),
                return_type: Box::new(Self::from_ast(return_type)),
                is_unsafe: false,
            },
            TypeExprKind::Unit => Self::Unit,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "()"),
            Self::Never => write!(f, "!"),
            Self::Bool => write!(f, "bool"),
            Self::Char => write!(f, "char"),
            Self::String => write!(f, "string"),
            Self::I8 => write!(f, "i8"),
            Self::I16 => write!(f, "i16"),
            Self::I32 => write!(f, "i32"),
            Self::I64 => write!(f, "i64"),
            Self::I128 => write!(f, "i128"),
            Self::ISize => write!(f, "isize"),
            Self::U8 => write!(f, "u8"),
            Self::U16 => write!(f, "u16"),
            Self::U32 => write!(f, "u32"),
            Self::U64 => write!(f, "u64"),
            Self::U128 => write!(f, "u128"),
            Self::USize => write!(f, "usize"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
            Self::Pointer { inner, is_mutable } => {
                let mut_str = if *is_mutable { "mut" } else { "const" };
                write!(f, "*{mut_str} {inner}")
            }
            Self::Reference { inner, is_mutable } => {
                let mut_str = if *is_mutable { "mut " } else { "" };
                write!(f, "&{mut_str}{inner}")
            }
            Self::Optional(inner) => write!(f, "{inner}?"),
            Self::Array { element, size } => {
                if let Some(s) = size {
                    write!(f, "[{element}; {s}]")
                } else {
                    write!(f, "[{element}]")
                }
            }
            Self::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{t}")?;
                }
                write!(f, ")")
            }
            Self::Struct { name, .. } | Self::Enum { name, .. } => write!(f, "{name}"),
            Self::Function {
                params,
                return_type,
                is_unsafe,
            } => {
                let unsafe_prefix = if *is_unsafe { "unsafe " } else { "" };
                write!(f, "{unsafe_prefix}fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Self::Unknown => write!(f, "<unknown>"),
        }
    }
}
