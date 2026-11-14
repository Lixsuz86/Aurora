//! Runtime value representation and heap objects.

use crate::opcode::Chunk;
use crate::vm::{Vm, VmError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// Signature for native functions callable by Aurora bytecode.
pub type NativeFn = fn(&mut Vm, &[Value]) -> Result<Value, VmError>;

/// A native Rust function exposed to the Aurora runtime.
#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub arity: Option<usize>,
    pub func: NativeFn,
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// A compiled Aurora bytecode function.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFunction {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
    pub is_unsafe: bool,
}

impl CompiledFunction {
    #[must_use]
    pub fn new(name: impl Into<String>, arity: usize, is_unsafe: bool) -> Self {
        Self {
            name: name.into(),
            arity,
            chunk: Chunk::new(),
            is_unsafe,
        }
    }
}

/// An instantiated user-defined struct at runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct StructInstance {
    pub name: String,
    pub fields: HashMap<String, Value>,
}

/// Runtime value representation for Aurora bytecode execution.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Rc<String>),
    Array(Rc<RefCell<Vec<Value>>>),
    Pointer(usize), // Direct memory address for low-level operations
    Function(Rc<CompiledFunction>),
    Native(NativeFunction),
    Struct(Rc<RefCell<StructInstance>>),
}

impl Value {
    /// Determines whether the value is considered truthy in conditional checks.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Nil => false,
            Self::Bool(b) => *b,
            Self::Int(i) => *i != 0,
            Self::Float(f) => *f != 0.0 && !f.is_nan(),
            Self::String(s) => !s.is_empty(),
            Self::Array(arr) => !arr.borrow().is_empty(),
            Self::Pointer(p) => *p != 0,
            Self::Function(_) | Self::Native(_) | Self::Struct(_) => true,
        }
    }

    /// Returns a human-readable name of the runtime type.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Pointer(_) => "pointer",
            Self::Function(_) => "function",
            Self::Native(_) => "native function",
            Self::Struct(_) => "struct",
        }
    }

    /// Extracts the integer value or returns a `TypeMismatch` error.
    pub fn as_int(&self) -> Result<i64, VmError> {
        match self {
            Self::Int(i) => Ok(*i),
            other => Err(VmError::TypeMismatch {
                expected: "int",
                found: other.type_name(),
            }),
        }
    }

    /// Extracts the float value (coercing integer if needed) or returns a `TypeMismatch` error.
    pub fn as_float(&self) -> Result<f64, VmError> {
        match self {
            Self::Float(f) => Ok(*f),
            #[allow(clippy::cast_precision_loss)]
            Self::Int(i) => Ok(*i as f64),
            other => Err(VmError::TypeMismatch {
                expected: "float",
                found: other.type_name(),
            }),
        }
    }

    /// Extracts the boolean value or returns a `TypeMismatch` error.
    pub fn as_bool(&self) -> Result<bool, VmError> {
        match self {
            Self::Bool(b) => Ok(*b),
            other => Err(VmError::TypeMismatch {
                expected: "bool",
                found: other.type_name(),
            }),
        }
    }

    /// Extracts the pointer address (or integer address) or returns a `TypeMismatch` error.
    pub fn as_ptr(&self) -> Result<usize, VmError> {
        match self {
            Self::Pointer(p) => Ok(*p),
            Self::Int(i) if *i >= 0 => Ok(*i as usize),
            other => Err(VmError::TypeMismatch {
                expected: "pointer",
                found: other.type_name(),
            }),
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Self::Int(i64::from(i))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(Rc::new(s))
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(Rc::new(s.to_string()))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(fl) => {
                if fl.fract() == 0.0 {
                    write!(f, "{fl:.1}")
                } else {
                    write!(f, "{fl}")
                }
            }
            Self::String(s) => write!(f, "{s}"),
            Self::Pointer(p) => write!(f, "0x{p:08x}"),
            Self::Array(arr) => {
                let items = arr.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Self::Function(func) => write!(f, "<fn {}>", func.name),
            Self::Native(native) => write!(f, "<native fn {}>", native.name),
            Self::Struct(s) => {
                let inst = s.borrow();
                write!(f, "{} {{ ", inst.name)?;
                for (i, (key, val)) in inst.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key}: {val}")?;
                }
                write!(f, " }}")
            }
        }
    }
}
