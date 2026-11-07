//! Semantic analysis and type checker implementation for Aurora.

use crate::error::SemanticError;
use crate::symbol::{Symbol, SymbolKind, SymbolTable};
use crate::types::Type;
use aurora_ast::{
    BinaryOp, Block, Expr, ExprKind, FunctionDecl, Item, ItemKind, Literal, MatchArm, Program,
    Stmt, StmtKind, UnaryOp,
};
use std::collections::HashMap;

/// Semantic analyzer that validates types, scoping, mutability, and safety rules.
#[derive(Debug)]
pub struct SemanticAnalyzer {
    symbol_table: SymbolTable,
    struct_defs: HashMap<String, Vec<(String, Type)>>,
    enum_defs: HashMap<String, Vec<(String, Vec<Type>)>>,
    current_return_type: Option<Type>,
    unsafe_depth: usize,
    errors: Vec<SemanticError>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            current_return_type: None,
            unsafe_depth: 0,
            errors: Vec::new(),
        }
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a slice of all accumulated semantic errors.
    #[must_use]
    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }

    /// Consumes the analyzer and returns all collected semantic errors.
    #[must_use]
    pub fn into_errors(self) -> Vec<SemanticError> {
        self.errors
    }

    /// Performs full 2-pass semantic analysis on a Program AST.
    pub fn analyze_program(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        // Pass 1: Collect declarations (structs, enums, functions, types)
        for item in &program.items {
            self.collect_declaration(item);
        }

        // Pass 2: Type check item bodies
        for item in &program.items {
            self.check_item_body(item);
        }

        // Pass 2b: Type check top-level statements
        for stmt in &program.statements {
            self.check_stmt(stmt);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    // =========================================================================
    // Pass 1: Declaration Discovery
    // =========================================================================

    fn collect_declaration(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Struct(st) => {
                let fields = st
                    .fields
                    .iter()
                    .map(|f| (f.name.name.clone(), Type::from_ast(&f.ty)))
                    .collect::<Vec<_>>();
                self.struct_defs
                    .insert(st.name.name.clone(), fields.clone());
                self.symbol_table.insert(Symbol {
                    name: st.name.name.clone(),
                    ty: Type::Struct {
                        name: st.name.name.clone(),
                        fields: fields.clone(),
                    },
                    kind: SymbolKind::Struct { fields },
                    is_mutable: false,
                    span: st.name.span,
                });
            }
            ItemKind::Enum(en) => {
                let variants = en
                    .variants
                    .iter()
                    .map(|v| {
                        let types = match &v.kind {
                            aurora_ast::EnumVariantKind::Unit => Vec::new(),
                            aurora_ast::EnumVariantKind::Tuple(types) => {
                                types.iter().map(Type::from_ast).collect()
                            }
                            aurora_ast::EnumVariantKind::Struct(fields) => {
                                fields.iter().map(|f| Type::from_ast(&f.ty)).collect()
                            }
                        };
                        (v.name.name.clone(), types)
                    })
                    .collect::<Vec<_>>();

                self.enum_defs
                    .insert(en.name.name.clone(), variants.clone());
                self.symbol_table.insert(Symbol {
                    name: en.name.name.clone(),
                    ty: Type::Enum {
                        name: en.name.name.clone(),
                        variants: variants.clone(),
                    },
                    kind: SymbolKind::Enum { variants },
                    is_mutable: false,
                    span: en.name.span,
                });
            }
            ItemKind::Function(func) => {
                let params = func
                    .params
                    .iter()
                    .map(|p| Type::from_ast(&p.type_annotation))
                    .collect::<Vec<_>>();
                let return_type = func.return_type.as_ref().map_or(Type::Unit, Type::from_ast);

                self.symbol_table.insert(Symbol::new_fn(
                    func.name.name.clone(),
                    params,
                    return_type,
                    func.is_unsafe,
                    func.name.span,
                ));
            }
            ItemKind::TypeAlias(ta) => {
                let target = Type::from_ast(&ta.target);
                self.symbol_table.insert(Symbol {
                    name: ta.name.name.clone(),
                    ty: target,
                    kind: SymbolKind::TypeAlias,
                    is_mutable: false,
                    span: ta.name.span,
                });
            }
            ItemKind::Trait(_) | ItemKind::Impl(_) | ItemKind::Use(_) => {
                // Handled in subsequent passes
            }
        }
    }

    // =========================================================================
    // Pass 2: Type and Safety Checking
    // =========================================================================

    fn check_item_body(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function(func) => self.check_function(func),
            ItemKind::Impl(impl_decl) => {
                for method in &impl_decl.methods {
                    self.check_function(method);
                }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, func: &FunctionDecl) {
        let return_type = func.return_type.as_ref().map_or(Type::Unit, Type::from_ast);

        let old_return_type = self.current_return_type.replace(return_type.clone());
        let old_unsafe = self.unsafe_depth;
        if func.is_unsafe {
            self.unsafe_depth += 1;
        }

        self.symbol_table.enter_scope();

        // Register parameters in function scope
        for param in &func.params {
            let pty = Type::from_ast(&param.type_annotation);
            self.symbol_table.insert(Symbol::new_var(
                param.name.name.clone(),
                pty,
                false,
                param.name.span,
            ));
        }

        // Check body
        self.check_block(&func.body);

        self.symbol_table.exit_scope();
        self.unsafe_depth = old_unsafe;
        self.current_return_type = old_return_type;
    }

    fn check_block(&mut self, block: &Block) -> Type {
        self.symbol_table.enter_scope();
        let mut last_type = Type::Unit;

        for stmt in &block.statements {
            last_type = self.check_stmt(stmt);
        }

        self.symbol_table.exit_scope();
        last_type
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Type {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.check_expr(expr),
            StmtKind::Let {
                name,
                type_annotation,
                initializer,
                is_mutable,
            } => {
                let declared_type = type_annotation.as_ref().map(Type::from_ast);
                let init_type = initializer.as_ref().map(|e| self.check_expr(e));

                let final_type = match (declared_type, init_type) {
                    (Some(expected), Some(actual)) => {
                        if !actual.can_coerce_to(&expected) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: expected.clone(),
                                found: actual,
                                span: name.span,
                            });
                        }
                        expected
                    }
                    (Some(expected), None) => expected,
                    (None, Some(actual)) => actual,
                    (None, None) => Type::Unknown,
                };

                self.symbol_table.insert(Symbol::new_var(
                    name.name.clone(),
                    final_type,
                    *is_mutable,
                    name.span,
                ));
                Type::Unit
            }
            StmtKind::Const {
                name,
                type_annotation,
                initializer,
            } => {
                let declared_type = type_annotation.as_ref().map(Type::from_ast);
                let actual = self.check_expr(initializer);

                let final_type = match declared_type {
                    Some(expected) => {
                        if !actual.can_coerce_to(&expected) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: expected.clone(),
                                found: actual,
                                span: name.span,
                            });
                        }
                        expected
                    }
                    None => actual,
                };

                self.symbol_table.insert(Symbol::new_const(
                    name.name.clone(),
                    final_type,
                    name.span,
                ));
                Type::Unit
            }
            StmtKind::Return(expr) => {
                let actual = expr.as_ref().map_or(Type::Unit, |e| self.check_expr(e));

                if let Some(expected) = &self.current_return_type {
                    if !actual.can_coerce_to(expected) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: expected.clone(),
                            found: actual,
                            span: stmt.span,
                        });
                    }
                }
                Type::Never
            }
            StmtKind::Item(item) => {
                self.collect_declaration(item);
                self.check_item_body(item);
                Type::Unit
            }
        }
    }

    /// Evaluates and type-checks an expression, returning its inferred or verified Type.
    pub fn check_expr(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(_) => Type::I64,
                Literal::Float(_) => Type::F64,
                Literal::String(_) | Literal::RawString(_) => Type::String,
                Literal::Char(_) => Type::Char,
                Literal::Bool(_) => Type::Bool,
                Literal::Nil => Type::Optional(Box::new(Type::Unknown)),
            },
            ExprKind::Variable(ident) => {
                if let Some(sym) = self.symbol_table.lookup(&ident.name) {
                    sym.ty.clone()
                } else {
                    self.errors.push(SemanticError::UndefinedVariable {
                        name: ident.name.clone(),
                        span: ident.span,
                    });
                    Type::Unknown
                }
            }
            ExprKind::Path(path) => {
                if let Some(first) = path.first() {
                    if let Some(sym) = self.symbol_table.lookup(&first.name) {
                        sym.ty.clone()
                    } else {
                        self.errors.push(SemanticError::UndefinedVariable {
                            name: first.name.clone(),
                            span: first.span,
                        });
                        Type::Unknown
                    }
                } else {
                    Type::Unknown
                }
            }
            ExprKind::Binary { op, left, right } => {
                // Pipeline operator: left |> right
                if *op == BinaryOp::Pipe {
                    return self.check_pipeline(left, right, expr.span);
                }

                let left_ty = self.check_expr(left);
                let right_ty = self.check_expr(right);

                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::Power => {
                        // String concatenation with '+'
                        if *op == BinaryOp::Add
                            && (left_ty == Type::String || right_ty == Type::String)
                        {
                            return Type::String;
                        }

                        if left_ty.is_numeric() && right_ty.is_numeric() {
                            if left_ty == right_ty {
                                left_ty
                            } else if left_ty.is_float() || right_ty.is_float() {
                                Type::F64
                            } else {
                                Type::I64
                            }
                        } else {
                            self.errors.push(SemanticError::InvalidBinaryOp {
                                op: *op,
                                left: left_ty,
                                right: right_ty,
                                span: expr.span,
                            });
                            Type::Unknown
                        }
                    }
                    BinaryOp::Eq | BinaryOp::NotEq => {
                        if !left_ty.can_coerce_to(&right_ty) && !right_ty.can_coerce_to(&left_ty) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: left_ty,
                                found: right_ty,
                                span: expr.span,
                            });
                        }
                        Type::Bool
                    }
                    BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                        if !left_ty.is_numeric() || !right_ty.is_numeric() {
                            self.errors.push(SemanticError::InvalidBinaryOp {
                                op: *op,
                                left: left_ty,
                                right: right_ty,
                                span: expr.span,
                            });
                        }
                        Type::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if left_ty != Type::Bool || right_ty != Type::Bool {
                            self.errors.push(SemanticError::InvalidBinaryOp {
                                op: *op,
                                left: left_ty,
                                right: right_ty,
                                span: expr.span,
                            });
                        }
                        Type::Bool
                    }
                    BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => {
                        if !left_ty.is_integer() || !right_ty.is_integer() {
                            self.errors.push(SemanticError::InvalidBinaryOp {
                                op: *op,
                                left: left_ty.clone(),
                                right: right_ty,
                                span: expr.span,
                            });
                        }
                        left_ty
                    }
                    BinaryOp::NullCoalesce => {
                        if let Type::Optional(inner) = left_ty {
                            if right_ty.can_coerce_to(&inner) {
                                *inner
                            } else {
                                self.errors.push(SemanticError::TypeMismatch {
                                    expected: *inner,
                                    found: right_ty,
                                    span: expr.span,
                                });
                                Type::Unknown
                            }
                        } else {
                            left_ty
                        }
                    }
                    BinaryOp::Pipe => unreachable!(),
                }
            }
            ExprKind::Unary { op, operand } => {
                let op_ty = self.check_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        if !op_ty.is_numeric() {
                            self.errors.push(SemanticError::InvalidUnaryOp {
                                op: *op,
                                operand: op_ty.clone(),
                                span: expr.span,
                            });
                        }
                        op_ty
                    }
                    UnaryOp::Not => {
                        if op_ty != Type::Bool {
                            self.errors.push(SemanticError::InvalidUnaryOp {
                                op: *op,
                                operand: op_ty,
                                span: expr.span,
                            });
                        }
                        Type::Bool
                    }
                    UnaryOp::BitNot => {
                        if !op_ty.is_integer() {
                            self.errors.push(SemanticError::InvalidUnaryOp {
                                op: *op,
                                operand: op_ty.clone(),
                                span: expr.span,
                            });
                        }
                        op_ty
                    }
                    UnaryOp::Deref => {
                        // Raw pointer dereferencing requires unsafe!
                        if self.unsafe_depth == 0 {
                            self.errors.push(SemanticError::UnsafeRequired {
                                operation: "dereference of raw pointer".into(),
                                span: expr.span,
                            });
                        }

                        match op_ty {
                            Type::Pointer { inner, .. } | Type::Reference { inner, .. } => *inner,
                            _ => {
                                self.errors.push(SemanticError::InvalidUnaryOp {
                                    op: *op,
                                    operand: op_ty,
                                    span: expr.span,
                                });
                                Type::Unknown
                            }
                        }
                    }
                    UnaryOp::AddressOf => Type::Reference {
                        inner: Box::new(op_ty),
                        is_mutable: false,
                    },
                    UnaryOp::AddressOfMut => Type::Reference {
                        inner: Box::new(op_ty),
                        is_mutable: true,
                    },
                }
            }
            ExprKind::Assignment {
                target,
                op: _,
                value,
            } => {
                // Mutability check: if target is a variable, it MUST be declared as mutable
                if let ExprKind::Variable(ident) = &target.kind {
                    if let Some(sym) = self.symbol_table.lookup(&ident.name) {
                        if !sym.is_mutable {
                            self.errors.push(SemanticError::CannotAssignImmutable {
                                name: ident.name.clone(),
                                span: ident.span,
                            });
                        }
                    }
                } else if let ExprKind::Unary {
                    op: UnaryOp::Deref, ..
                } = &target.kind
                {
                    if self.unsafe_depth == 0 {
                        self.errors.push(SemanticError::UnsafeRequired {
                            operation: "assignment through raw pointer dereference".into(),
                            span: target.span,
                        });
                    }
                }

                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);

                if !value_ty.can_coerce_to(&target_ty) {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: target_ty,
                        found: value_ty,
                        span: expr.span,
                    });
                }
                Type::Unit
            }
            ExprKind::Call { callee, args } => {
                let callee_ty = self.check_expr(callee);
                match callee_ty {
                    Type::Function {
                        params,
                        return_type,
                        is_unsafe,
                    } => {
                        if is_unsafe && self.unsafe_depth == 0 {
                            self.errors.push(SemanticError::UnsafeRequired {
                                operation: "call to unsafe function".into(),
                                span: expr.span,
                            });
                        }

                        if args.len() != params.len() {
                            self.errors.push(SemanticError::ArityMismatch {
                                expected: params.len(),
                                found: args.len(),
                                span: expr.span,
                            });
                        }

                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            let arg_ty = self.check_expr(arg);
                            if !arg_ty.can_coerce_to(param_ty) {
                                self.errors.push(SemanticError::TypeMismatch {
                                    expected: param_ty.clone(),
                                    found: arg_ty,
                                    span: arg.span,
                                });
                            }
                        }
                        *return_type
                    }
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.errors.push(SemanticError::NotCallable {
                            ty: other,
                            span: callee.span,
                        });
                        Type::Unknown
                    }
                }
            }
            ExprKind::MemberAccess { object, member } => {
                let obj_ty = self.check_expr(object);
                match &obj_ty {
                    Type::Struct { name, fields } => {
                        let lookup_fields = if fields.is_empty() {
                            self.struct_defs.get(name)
                        } else {
                            Some(fields)
                        };

                        if let Some(f_list) = lookup_fields {
                            if let Some((_, fty)) = f_list.iter().find(|(n, _)| n == &member.name) {
                                return fty.clone();
                            }
                        }

                        self.errors.push(SemanticError::NoSuchField {
                            struct_name: name.clone(),
                            field_name: member.name.clone(),
                            span: member.span,
                        });
                        Type::Unknown
                    }
                    _ => Type::Unknown,
                }
            }
            ExprKind::Index { target, index } => {
                let target_ty = self.check_expr(target);
                let idx_ty = self.check_expr(index);

                if !idx_ty.is_integer() {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: Type::USize,
                        found: idx_ty,
                        span: index.span,
                    });
                }

                match target_ty {
                    Type::Array { element, .. } => *element,
                    Type::String => Type::Char,
                    _ => {
                        self.errors.push(SemanticError::CannotIndex {
                            target_type: target_ty,
                            span: target.span,
                        });
                        Type::Unknown
                    }
                }
            }
            ExprKind::Array(elements) => {
                if let Some(first) = elements.first() {
                    let elem_ty = self.check_expr(first);
                    for elem in &elements[1..] {
                        let ty = self.check_expr(elem);
                        if !ty.can_coerce_to(&elem_ty) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: elem_ty.clone(),
                                found: ty,
                                span: elem.span,
                            });
                        }
                    }
                    Type::Array {
                        element: Box::new(elem_ty),
                        size: Some(elements.len()),
                    }
                } else {
                    Type::Array {
                        element: Box::new(Type::Unknown),
                        size: Some(0),
                    }
                }
            }
            ExprKind::Tuple(elements) => {
                let types = elements.iter().map(|e| self.check_expr(e)).collect();
                Type::Tuple(types)
            }
            ExprKind::Block(block) => self.check_block(block),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.check_expr(condition);
                if cond_ty != Type::Bool {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                        span: condition.span,
                    });
                }

                let then_ty = self.check_block(then_branch);
                if let Some(else_expr) = else_branch {
                    let else_ty = self.check_expr(else_expr);
                    if !else_ty.can_coerce_to(&then_ty) && !then_ty.can_coerce_to(&else_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: then_ty.clone(),
                            found: else_ty,
                            span: else_expr.span,
                        });
                    }
                }
                then_ty
            }
            ExprKind::Match { value, arms } => {
                let val_ty = self.check_expr(value);
                let mut return_type = Type::Unknown;

                for (idx, arm) in arms.iter().enumerate() {
                    self.check_match_arm(arm, &val_ty);
                    let arm_body_ty = self.check_expr(&arm.body);
                    if idx == 0 {
                        return_type = arm_body_ty;
                    } else if !arm_body_ty.can_coerce_to(&return_type) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: return_type.clone(),
                            found: arm_body_ty,
                            span: arm.body.span,
                        });
                    }
                }
                return_type
            }
            ExprKind::While { condition, body } => {
                let cond_ty = self.check_expr(condition);
                if cond_ty != Type::Bool {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: Type::Bool,
                        found: cond_ty,
                        span: condition.span,
                    });
                }
                self.check_block(body);
                Type::Unit
            }
            ExprKind::For { var, iter, body } => {
                let iter_ty = self.check_expr(iter);
                let elem_ty = match iter_ty {
                    Type::Array { element, .. } => *element,
                    _ => Type::Unknown,
                };

                self.symbol_table.enter_scope();
                self.symbol_table.insert(Symbol::new_var(
                    var.name.clone(),
                    elem_ty,
                    false,
                    var.span,
                ));
                self.check_block(body);
                self.symbol_table.exit_scope();

                Type::Unit
            }
            ExprKind::Loop { body } => {
                self.check_block(body);
                Type::Unit
            }
            ExprKind::Break(expr) => {
                if let Some(e) = expr {
                    self.check_expr(e);
                }
                Type::Never
            }
            ExprKind::Continue => Type::Never,
            ExprKind::Return(ret_expr) => {
                let actual = ret_expr.as_ref().map_or(Type::Unit, |e| self.check_expr(e));
                if let Some(expected) = &self.current_return_type {
                    if !actual.can_coerce_to(expected) {
                        let span = ret_expr.as_ref().map_or(expr.span, |e| e.span);
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: expected.clone(),
                            found: actual,
                            span,
                        });
                    }
                }
                Type::Never
            }
            ExprKind::Lambda {
                params,
                return_type,
                body,
            } => {
                let p_types = params
                    .iter()
                    .map(|p| Type::from_ast(&p.type_annotation))
                    .collect::<Vec<_>>();
                let declared_ret = return_type.as_ref().map(Type::from_ast);

                self.symbol_table.enter_scope();
                for (p, pty) in params.iter().zip(p_types.iter()) {
                    self.symbol_table.insert(Symbol::new_var(
                        p.name.name.clone(),
                        pty.clone(),
                        false,
                        p.name.span,
                    ));
                }

                let body_ty = self.check_expr(body);
                self.symbol_table.exit_scope();

                let ret_ty = declared_ret.unwrap_or(body_ty);
                Type::Function {
                    params: p_types,
                    return_type: Box::new(ret_ty),
                    is_unsafe: false,
                }
            }
            ExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.check_expr(s);
                }
                if let Some(e) = end {
                    self.check_expr(e);
                }
                Type::Array {
                    element: Box::new(Type::I64),
                    size: None,
                }
            }
            ExprKind::Cast { expr, target_type } => {
                self.check_expr(expr);
                Type::from_ast(target_type)
            }
            ExprKind::UnsafeBlock(block) => {
                self.unsafe_depth += 1;
                let ty = self.check_block(block);
                self.unsafe_depth -= 1;
                ty
            }
            ExprKind::Sizeof(_) | ExprKind::Alignof(_) => Type::USize,
        }
    }

    fn check_pipeline(&mut self, left: &Expr, right: &Expr, span: aurora_ast::Span) -> Type {
        let left_ty = self.check_expr(left);

        // Desugar pipeline `left |> f(args...)` -> calls f(left, args...)
        if let ExprKind::Call { callee, args } = &right.kind {
            let callee_ty = self.check_expr(callee);
            match callee_ty {
                Type::Function {
                    params,
                    return_type,
                    is_unsafe,
                } => {
                    if is_unsafe && self.unsafe_depth == 0 {
                        self.errors.push(SemanticError::UnsafeRequired {
                            operation: "unsafe function call in pipeline".into(),
                            span,
                        });
                    }

                    if params.is_empty() {
                        self.errors.push(SemanticError::ArityMismatch {
                            expected: 1,
                            found: 0,
                            span,
                        });
                    } else if !left_ty.can_coerce_to(&params[0]) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: params[0].clone(),
                            found: left_ty,
                            span: left.span,
                        });
                    }

                    // Check remaining args
                    for (arg, param_ty) in args.iter().zip(params[1..].iter()) {
                        let arg_ty = self.check_expr(arg);
                        if !arg_ty.can_coerce_to(param_ty) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: param_ty.clone(),
                                found: arg_ty,
                                span: arg.span,
                            });
                        }
                    }

                    *return_type
                }
                _ => Type::Unknown,
            }
        } else {
            // Pipeline to function value `left |> func`
            let right_ty = self.check_expr(right);
            match right_ty {
                Type::Function {
                    params,
                    return_type,
                    ..
                } => {
                    if let Some(first_param) = params.first() {
                        if !left_ty.can_coerce_to(first_param) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: first_param.clone(),
                                found: left_ty,
                                span: left.span,
                            });
                        }
                    }
                    *return_type
                }
                _ => Type::Unknown,
            }
        }
    }

    fn check_match_arm(&mut self, arm: &MatchArm, expected_ty: &Type) {
        match &arm.pattern.kind {
            aurora_ast::PatternKind::Literal(lit) => {
                let lit_ty = match lit {
                    Literal::Int(_) => Type::I64,
                    Literal::Float(_) => Type::F64,
                    Literal::String(_) | Literal::RawString(_) => Type::String,
                    Literal::Char(_) => Type::Char,
                    Literal::Bool(_) => Type::Bool,
                    Literal::Nil => Type::Optional(Box::new(Type::Unknown)),
                };
                if !lit_ty.can_coerce_to(expected_ty) {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: expected_ty.clone(),
                        found: lit_ty,
                        span: arm.pattern.span,
                    });
                }
            }
            aurora_ast::PatternKind::Variable(ident) => {
                self.symbol_table.insert(Symbol::new_var(
                    ident.name.clone(),
                    expected_ty.clone(),
                    false,
                    ident.span,
                ));
            }
            _ => {}
        }

        if let Some(guard) = &arm.guard {
            let guard_ty = self.check_expr(guard);
            if guard_ty != Type::Bool {
                self.errors.push(SemanticError::TypeMismatch {
                    expected: Type::Bool,
                    found: guard_ty,
                    span: guard.span,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_lexer::Lexer;
    use aurora_parser::Parser;

    #[test]
    fn test_typecheck_valid_program() {
        let code = r"
            let x: i64 = 42;
            let y = 10;
            let z = x + y;
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut analyzer = SemanticAnalyzer::new();
        assert!(analyzer.analyze_program(&program).is_ok());
    }

    #[test]
    fn test_typecheck_detects_mismatch() {
        let code = r#"
            let x: i64 = "wrong type";
        "#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(matches!(errs[0], SemanticError::TypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_detects_immutable_reassignment() {
        let code = r"
            let x = 10;
            x = 20;
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(matches!(
            errs[0],
            SemanticError::CannotAssignImmutable { .. }
        ));
    }

    #[test]
    fn test_typecheck_unsafe_context_required() {
        let code = r"
            fn illegal_deref(ptr: *const i32) -> i32 {
                return *ptr;
            }
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_program(&program);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(matches!(errs[0], SemanticError::UnsafeRequired { .. }));
    }

    #[test]
    fn test_typecheck_allowed_in_unsafe_block() {
        let code = r"
            fn legal_deref(ptr: *const i32) -> i32 {
                let val = unsafe {
                    return *ptr;
                };
                return 0;
            }
        ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        let mut analyzer = SemanticAnalyzer::new();
        let res = analyzer.analyze_program(&program);
        if let Err(ref errs) = res {
            panic!("Errors encountered: {errs:?}");
        }
        assert!(res.is_ok());
    }
}
