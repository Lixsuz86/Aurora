//! Semantic analysis, types, and symbol management for Aurora.

pub mod symbol;
pub mod types;

pub use symbol::{Scope, Symbol, SymbolTable};
pub use types::Type;
