//! Semantic analysis, types, symbols, and safety verification for Aurora.

pub mod analyzer;
pub mod error;
pub mod symbol;
pub mod types;

pub use analyzer::SemanticAnalyzer;
pub use error::SemanticError;
pub use symbol::{Scope, Symbol, SymbolKind, SymbolTable};
pub use types::Type;
