//! Type representations for Aurora.

use std::fmt;

/// Primitive and composite types in Aurora.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,
    Bool,
    Int64,
    Float64,
    String,
    Pointer(Box<Type>),
    MutablePointer(Box<Type>),
    Optional(Box<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Custom(String),
    Unknown,
}

impl Type {
    /// Checks if the type is a raw pointer (low-level).
    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer(_) | Self::MutablePointer(_))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "()"),
            Self::Bool => write!(f, "bool"),
            Self::Int64 => write!(f, "i64"),
            Self::Float64 => write!(f, "f64"),
            Self::String => write!(f, "string"),
            Self::Pointer(inner) => write!(f, "*const {inner}"),
            Self::MutablePointer(inner) => write!(f, "*mut {inner}"),
            Self::Optional(inner) => write!(f, "{inner}?"),
            Self::Function {
                params,
                return_type,
            } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Self::Custom(name) => write!(f, "{name}"),
            Self::Unknown => write!(f, "<unknown>"),
        }
    }
}
