//! Symbol table and lexical scoping for Aurora semantic analysis.

use crate::types::Type;
use aurora_ast::Span;
use std::collections::HashMap;

/// Differentiates kinds of symbols in the symbol table.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable {
        is_mutable: bool,
    },
    Constant,
    Function {
        is_unsafe: bool,
        params: Vec<Type>,
        return_type: Type,
    },
    Struct {
        fields: Vec<(String, Type)>,
    },
    Enum {
        variants: Vec<(String, Vec<Type>)>,
    },
    TypeAlias,
}

/// Symbol information stored in a lexical scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub kind: SymbolKind,
    pub is_mutable: bool,
    pub span: Span,
}

impl Symbol {
    pub fn new_var(name: impl Into<String>, ty: Type, is_mutable: bool, span: Span) -> Self {
        Self {
            name: name.into(),
            ty,
            kind: SymbolKind::Variable { is_mutable },
            is_mutable,
            span,
        }
    }

    pub fn new_const(name: impl Into<String>, ty: Type, span: Span) -> Self {
        Self {
            name: name.into(),
            ty,
            kind: SymbolKind::Constant,
            is_mutable: false,
            span,
        }
    }

    pub fn new_fn(
        name: impl Into<String>,
        params: Vec<Type>,
        return_type: Type,
        is_unsafe: bool,
        span: Span,
    ) -> Self {
        let fn_ty = Type::Function {
            params: params.clone(),
            return_type: Box::new(return_type.clone()),
            is_unsafe,
        };
        Self {
            name: name.into(),
            ty: fn_ty,
            kind: SymbolKind::Function {
                is_unsafe,
                params,
                return_type,
            },
            is_mutable: false,
            span,
        }
    }
}

/// A single lexical scope holding symbol declarations.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }
}

/// Hierarchical Symbol Table supporting nested lexical scopes and type lookup.
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        let mut root = Scope::new();
        // Register core built-in symbols
        root.insert(Symbol::new_fn(
            "println",
            vec![Type::String],
            Type::Unit,
            false,
            Span::default(),
        ));
        root.insert(Symbol::new_fn(
            "print",
            vec![Type::String],
            Type::Unit,
            false,
            Span::default(),
        ));
        root.insert(Symbol::new_fn(
            "alloc",
            vec![Type::USize],
            Type::Pointer {
                inner: Box::new(Type::U8),
                is_mutable: true,
            },
            true,
            Span::default(),
        ));
        root.insert(Symbol::new_fn(
            "free",
            vec![Type::Pointer {
                inner: Box::new(Type::U8),
                is_mutable: true,
            }],
            Type::Unit,
            true,
            Span::default(),
        ));
        root.insert(Symbol::new_fn(
            "clock",
            vec![],
            Type::F64,
            false,
            Span::default(),
        ));
        root.insert(Symbol::new_fn(
            "assert",
            vec![Type::Bool],
            Type::Unit,
            false,
            Span::default(),
        ));
        root.insert(Symbol::new_fn(
            "assert_eq",
            vec![Type::Unknown, Type::Unknown],
            Type::Unit,
            false,
            Span::default(),
        ));
        Self { scopes: vec![root] }
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new lexical scope.
    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Pops the current lexical scope.
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Inserts a symbol into the active (innermost) scope.
    pub fn insert(&mut self, symbol: Symbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(symbol);
        }
    }

    /// Looks up a symbol searching from the innermost scope outwards.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.lookup(name) {
                return Some(sym);
            }
        }
        None
    }

    /// Looks up a symbol strictly in the active (current) scope without looking at outer scopes.
    #[must_use]
    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        self.scopes.last().and_then(|scope| scope.lookup(name))
    }

    /// Returns the current nesting depth (1 for global/file scope).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}
