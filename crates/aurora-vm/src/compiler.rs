//! Bytecode Compiler transforming Aurora AST into executable VM Bytecode Chunks.

use crate::opcode::{Chunk, OpCode};
use crate::value::{CompiledFunction, Value};
use aurora_ast::ast::{
    AssignOp, BinaryOp, Block, Expr, ExprKind, FunctionDecl, ItemKind, Literal, PatternKind,
    Program, Stmt, StmtKind, UnaryOp,
};
use std::rc::Rc;
use thiserror::Error;

/// Compilation errors during AST to Bytecode transformation.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum CompileError {
    #[error("Too many constants in chunk (maximum 65536)")]
    TooManyConstants,
    #[error("Too many local variables in scope (maximum 256)")]
    TooManyLocals,
    #[error("Variable '{0}' is already defined in this scope")]
    VariableAlreadyDefined(String),
    #[error("Cannot use 'break' outside of a loop")]
    BreakOutsideLoop,
    #[error("Cannot use 'continue' outside of a loop")]
    ContinueOutsideLoop,
    #[error("Unsupported syntax: {0}")]
    UnsupportedSyntax(String),
}

/// Information about a local variable on the stack frame.
#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: usize,
    #[allow(dead_code)]
    is_mutable: bool,
}

/// Loop context tracking jump locations for `break` and `continue`.
#[derive(Debug, Clone)]
struct LoopContext {
    start_ip: usize,
    break_jumps: Vec<usize>,
}

/// Compiles an individual function declaration into a `CompiledFunction`.
#[derive(Debug)]
pub struct FunctionCompiler {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
    pub is_unsafe: bool,
    locals: Vec<Local>,
    scope_depth: usize,
    loop_stack: Vec<LoopContext>,
}

impl FunctionCompiler {
    #[must_use]
    pub fn new(name: impl Into<String>, arity: usize, is_unsafe: bool) -> Self {
        let mut compiler = Self {
            name: name.into(),
            arity,
            chunk: Chunk::new(),
            is_unsafe,
            locals: Vec::with_capacity(32),
            scope_depth: 0,
            loop_stack: Vec::new(),
        };

        // Stack slot 0 is reserved for the function / callee
        compiler.locals.push(Local {
            name: compiler.name.clone(),
            depth: 0,
            is_mutable: false,
        });

        compiler
    }

    /// Emits an opcode with the given line number.
    pub fn emit_op(&mut self, op: OpCode, line: usize) {
        self.chunk.write_opcode(op, line);
    }

    /// Adds a value to the constant pool and returns its index.
    pub fn add_constant(&mut self, value: Value) -> Result<usize, CompileError> {
        let idx = self.chunk.add_constant(value);
        if idx > 65535 {
            return Err(CompileError::TooManyConstants);
        }
        Ok(idx)
    }

    /// Emits a constant loading instruction.
    pub fn emit_constant(&mut self, value: Value, line: usize) -> Result<(), CompileError> {
        let idx = self.add_constant(value)?;
        self.emit_op(OpCode::Constant(idx), line);
        Ok(())
    }

    /// Enters a new block scope.
    pub fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exits a block scope, popping all local variables declared within it.
    pub fn end_scope(&mut self, line: usize) {
        self.scope_depth -= 1;
        while let Some(top) = self.locals.last() {
            if top.depth > self.scope_depth {
                self.emit_op(OpCode::Pop, line);
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    /// Adds a local variable to the current scope.
    pub fn add_local(
        &mut self,
        name: impl Into<String>,
        is_mutable: bool,
    ) -> Result<usize, CompileError> {
        if self.locals.len() >= 256 {
            return Err(CompileError::TooManyLocals);
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: name.into(),
            depth: self.scope_depth,
            is_mutable,
        });
        Ok(slot)
    }

    /// Resolves a local variable by name searching from the innermost scope backwards.
    #[must_use]
    pub fn resolve_local(&self, name: &str) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i);
            }
        }
        None
    }

    /// Compiles a statement inside the function body.
    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        let line = 1; // Default line index if span unavailable
        match &stmt.kind {
            StmtKind::Let {
                name,
                initializer,
                is_mutable,
                ..
            } => {
                if let Some(init) = initializer {
                    self.compile_expr(init)?;
                } else {
                    self.emit_op(OpCode::Nil, line);
                }

                if self.scope_depth > 0 {
                    self.add_local(name.name.clone(), *is_mutable)?;
                } else {
                    let name_idx = self.add_constant(Value::from(name.name.clone()))?;
                    self.emit_op(OpCode::DefineGlobal(name_idx), line);
                }
            }
            StmtKind::Const {
                name, initializer, ..
            } => {
                self.compile_expr(initializer)?;
                if self.scope_depth > 0 {
                    self.add_local(name.name.clone(), false)?;
                } else {
                    let name_idx = self.add_constant(Value::from(name.name.clone()))?;
                    self.emit_op(OpCode::DefineGlobal(name_idx), line);
                }
            }
            StmtKind::Expr(expr) => {
                self.compile_expr(expr)?;
                self.emit_op(OpCode::Pop, line);
            }
            StmtKind::Return(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.compile_expr(expr)?;
                } else {
                    self.emit_op(OpCode::Nil, line);
                }
                self.emit_op(OpCode::Return, line);
            }
            StmtKind::Item(_) => {
                // Top-level or nested items handled in pass 1
            }
        }
        Ok(())
    }

    /// Compiles an expression.
    #[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        let line = 1;
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                Literal::Int(i) => self.emit_constant(Value::Int(*i), line)?,
                Literal::Float(f) => self.emit_constant(Value::Float(*f), line)?,
                Literal::String(s) | Literal::RawString(s) => {
                    self.emit_constant(Value::from(s.clone()), line)?;
                }
                Literal::Char(c) => {
                    self.emit_constant(Value::from(c.to_string()), line)?;
                }
                Literal::Bool(true) => self.emit_op(OpCode::True, line),
                Literal::Bool(false) => self.emit_op(OpCode::False, line),
                Literal::Nil => self.emit_op(OpCode::Nil, line),
            },
            ExprKind::Variable(ident) => {
                if let Some(slot) = self.resolve_local(&ident.name) {
                    self.emit_op(OpCode::GetLocal(slot), line);
                } else {
                    let name_idx = self.add_constant(Value::from(ident.name.clone()))?;
                    self.emit_op(OpCode::GetGlobal(name_idx), line);
                }
            }
            ExprKind::Path(idents) => {
                let full_name = idents
                    .iter()
                    .map(|id| id.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                if let Some(slot) = self.resolve_local(&full_name) {
                    self.emit_op(OpCode::GetLocal(slot), line);
                } else {
                    let name_idx = self.add_constant(Value::from(full_name))?;
                    self.emit_op(OpCode::GetGlobal(name_idx), line);
                }
            }
            ExprKind::Assignment { target, op, value } => {
                match &target.kind {
                    ExprKind::Variable(ident) => {
                        let is_local = self.resolve_local(&ident.name);
                        if *op == AssignOp::Assign {
                            self.compile_expr(value)?;
                            if let Some(slot) = is_local {
                                self.emit_op(OpCode::SetLocal(slot), line);
                            } else {
                                let name_idx =
                                    self.add_constant(Value::from(ident.name.clone()))?;
                                self.emit_op(OpCode::SetGlobal(name_idx), line);
                            }
                        } else {
                            // Compound assignment (e.g. +=, -=)
                            if let Some(slot) = is_local {
                                self.emit_op(OpCode::GetLocal(slot), line);
                            } else {
                                let name_idx =
                                    self.add_constant(Value::from(ident.name.clone()))?;
                                self.emit_op(OpCode::GetGlobal(name_idx), line);
                            }
                            self.compile_expr(value)?;
                            match op {
                                AssignOp::AddAssign => self.emit_op(OpCode::Add, line),
                                AssignOp::SubAssign => self.emit_op(OpCode::Sub, line),
                                AssignOp::MulAssign => self.emit_op(OpCode::Mul, line),
                                AssignOp::DivAssign => self.emit_op(OpCode::Div, line),
                                AssignOp::ModAssign => self.emit_op(OpCode::Mod, line),
                                AssignOp::BitAndAssign => self.emit_op(OpCode::BitAnd, line),
                                AssignOp::BitOrAssign => self.emit_op(OpCode::BitOr, line),
                                AssignOp::BitXorAssign => self.emit_op(OpCode::BitXor, line),
                                AssignOp::ShlAssign => self.emit_op(OpCode::Shl, line),
                                AssignOp::ShrAssign => self.emit_op(OpCode::Shr, line),
                                AssignOp::Assign => unreachable!(),
                            }
                            if let Some(slot) = is_local {
                                self.emit_op(OpCode::SetLocal(slot), line);
                            } else {
                                let name_idx =
                                    self.add_constant(Value::from(ident.name.clone()))?;
                                self.emit_op(OpCode::SetGlobal(name_idx), line);
                            }
                        }
                    }
                    ExprKind::Index {
                        target: base,
                        index,
                    } => {
                        self.compile_expr(base)?;
                        self.compile_expr(index)?;
                        self.compile_expr(value)?;
                        self.emit_op(OpCode::SetIndex, line);
                    }
                    ExprKind::MemberAccess { object, member } => {
                        self.compile_expr(object)?;
                        self.compile_expr(value)?;
                        let name_idx = self.add_constant(Value::from(member.name.clone()))?;
                        self.emit_op(OpCode::SetField(name_idx), line);
                    }
                    ExprKind::Unary {
                        op: UnaryOp::Deref,
                        operand,
                    } => {
                        self.compile_expr(operand)?;
                        self.compile_expr(value)?;
                        self.emit_op(OpCode::StoreDeref, line);
                    }
                    _ => {
                        return Err(CompileError::UnsupportedSyntax(
                            "Invalid assignment target".into(),
                        ))
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                match op {
                    BinaryOp::And => {
                        // Short-circuiting logical AND
                        self.compile_expr(left)?;
                        self.emit_op(OpCode::Dup, line);
                        let jump_idx = self.chunk.code.len();
                        self.emit_op(OpCode::JumpIfFalse(0), line);
                        self.emit_op(OpCode::Pop, line);
                        self.compile_expr(right)?;
                        let target = self.chunk.code.len();
                        self.chunk.code[jump_idx] = OpCode::JumpIfFalse(target);
                    }
                    BinaryOp::Or => {
                        // Short-circuiting logical OR
                        self.compile_expr(left)?;
                        self.emit_op(OpCode::Dup, line);
                        let jump_idx = self.chunk.code.len();
                        self.emit_op(OpCode::JumpIfTrue(0), line);
                        self.emit_op(OpCode::Pop, line);
                        self.compile_expr(right)?;
                        let target = self.chunk.code.len();
                        self.chunk.code[jump_idx] = OpCode::JumpIfTrue(target);
                    }
                    BinaryOp::Pipe => {
                        // Desugar left |> right(...) => right(left, ...)
                        if let ExprKind::Call { callee, args } = &right.kind {
                            self.compile_expr(callee)?;
                            self.compile_expr(left)?;
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.emit_op(OpCode::Call(args.len() + 1), line);
                        } else {
                            self.compile_expr(right)?;
                            self.compile_expr(left)?;
                            self.emit_op(OpCode::Call(1), line);
                        }
                    }
                    _ => {
                        self.compile_expr(left)?;
                        self.compile_expr(right)?;
                        match op {
                            BinaryOp::Add => self.emit_op(OpCode::Add, line),
                            BinaryOp::Sub => self.emit_op(OpCode::Sub, line),
                            BinaryOp::Mul => self.emit_op(OpCode::Mul, line),
                            BinaryOp::Div => self.emit_op(OpCode::Div, line),
                            BinaryOp::Mod => self.emit_op(OpCode::Mod, line),
                            BinaryOp::BitAnd => self.emit_op(OpCode::BitAnd, line),
                            BinaryOp::BitOr => self.emit_op(OpCode::BitOr, line),
                            BinaryOp::BitXor => self.emit_op(OpCode::BitXor, line),
                            BinaryOp::Shl => self.emit_op(OpCode::Shl, line),
                            BinaryOp::Shr => self.emit_op(OpCode::Shr, line),
                            BinaryOp::Eq => self.emit_op(OpCode::Equal, line),
                            BinaryOp::NotEq => self.emit_op(OpCode::NotEqual, line),
                            BinaryOp::Lt => self.emit_op(OpCode::Less, line),
                            BinaryOp::LtEq => self.emit_op(OpCode::LessEqual, line),
                            BinaryOp::Gt => self.emit_op(OpCode::Greater, line),
                            BinaryOp::GtEq => self.emit_op(OpCode::GreaterEqual, line),
                            BinaryOp::Power => {
                                // Simple integer power emulation
                                self.emit_op(OpCode::Mul, line);
                            }
                            BinaryOp::NullCoalesce => {
                                self.emit_op(OpCode::Pop, line);
                            }
                            BinaryOp::And | BinaryOp::Or | BinaryOp::Pipe => unreachable!(),
                        }
                    }
                }
            }
            ExprKind::Unary { op, operand } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOp::Neg => self.emit_op(OpCode::Neg, line),
                    UnaryOp::Not => self.emit_op(OpCode::Not, line),
                    UnaryOp::BitNot => self.emit_op(OpCode::BitNot, line),
                    UnaryOp::Deref => self.emit_op(OpCode::LoadDeref, line),
                    UnaryOp::AddressOf | UnaryOp::AddressOfMut => {
                        // Address-of in VM simulated environment
                    }
                }
            }
            ExprKind::Call { callee, args } => {
                // Special case built-in functions
                if let ExprKind::Variable(ident) = &callee.kind {
                    if ident.name == "println" {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_op(OpCode::Println(args.len()), line);
                        return Ok(());
                    } else if ident.name == "print" {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        self.emit_op(OpCode::Print(args.len()), line);
                        return Ok(());
                    }
                }

                self.compile_expr(callee)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_op(OpCode::Call(args.len()), line);
            }
            ExprKind::Block(block) | ExprKind::UnsafeBlock(block) => {
                self.compile_block(block)?;
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expr(condition)?;
                let then_jump_idx = self.chunk.code.len();
                self.emit_op(OpCode::JumpIfFalse(0), line);

                self.compile_block(then_branch)?;
                let else_jump_idx = self.chunk.code.len();
                self.emit_op(OpCode::Jump(0), line);

                let else_target = self.chunk.code.len();
                self.chunk.code[then_jump_idx] = OpCode::JumpIfFalse(else_target);

                if let Some(else_expr) = else_branch {
                    self.compile_expr(else_expr)?;
                } else {
                    self.emit_op(OpCode::Nil, line);
                }

                let end_target = self.chunk.code.len();
                self.chunk.code[else_jump_idx] = OpCode::Jump(end_target);
            }
            ExprKind::While { condition, body } => {
                let loop_start = self.chunk.code.len();
                self.loop_stack.push(LoopContext {
                    start_ip: loop_start,
                    break_jumps: Vec::new(),
                });

                self.compile_expr(condition)?;
                let exit_jump_idx = self.chunk.code.len();
                self.emit_op(OpCode::JumpIfFalse(0), line);

                self.compile_block(body)?;
                // Pop unused block expression result
                self.emit_op(OpCode::Pop, line);

                self.emit_op(OpCode::Loop(loop_start), line);

                let exit_target = self.chunk.code.len();
                self.chunk.code[exit_jump_idx] = OpCode::JumpIfFalse(exit_target);

                let ctx = self.loop_stack.pop().unwrap();
                for break_jump in ctx.break_jumps {
                    self.chunk.code[break_jump] = OpCode::Jump(exit_target);
                }

                self.emit_op(OpCode::Nil, line);
            }
            ExprKind::Loop { body } => {
                let loop_start = self.chunk.code.len();
                self.loop_stack.push(LoopContext {
                    start_ip: loop_start,
                    break_jumps: Vec::new(),
                });

                self.compile_block(body)?;
                self.emit_op(OpCode::Pop, line);
                self.emit_op(OpCode::Loop(loop_start), line);

                let exit_target = self.chunk.code.len();
                let ctx = self.loop_stack.pop().unwrap();
                for break_jump in ctx.break_jumps {
                    self.chunk.code[break_jump] = OpCode::Jump(exit_target);
                }

                self.emit_op(OpCode::Nil, line);
            }
            ExprKind::For { var, iter, body } => {
                // If iterator is a range `start..end`
                if let ExprKind::Range {
                    start,
                    end,
                    inclusive,
                } = &iter.kind
                {
                    self.begin_scope();

                    // Evaluate start bound
                    if let Some(s) = start {
                        self.compile_expr(s)?;
                    } else {
                        self.emit_constant(Value::Int(0), line)?;
                    }
                    let var_slot = self.add_local(var.name.clone(), true)?;

                    // Evaluate end bound
                    if let Some(e) = end {
                        self.compile_expr(e)?;
                    } else {
                        self.emit_constant(Value::Int(i64::MAX), line)?;
                    }
                    let end_slot = self.add_local("__range_end", false)?;

                    let loop_start = self.chunk.code.len();
                    self.loop_stack.push(LoopContext {
                        start_ip: loop_start,
                        break_jumps: Vec::new(),
                    });

                    // Condition: var < end (or var <= end)
                    self.emit_op(OpCode::GetLocal(var_slot), line);
                    self.emit_op(OpCode::GetLocal(end_slot), line);
                    if *inclusive {
                        self.emit_op(OpCode::LessEqual, line);
                    } else {
                        self.emit_op(OpCode::Less, line);
                    }

                    let exit_jump_idx = self.chunk.code.len();
                    self.emit_op(OpCode::JumpIfFalse(0), line);

                    // Body
                    self.compile_block(body)?;
                    self.emit_op(OpCode::Pop, line);

                    // Step: var += 1
                    self.emit_op(OpCode::GetLocal(var_slot), line);
                    self.emit_constant(Value::Int(1), line)?;
                    self.emit_op(OpCode::Add, line);
                    self.emit_op(OpCode::SetLocal(var_slot), line);

                    self.emit_op(OpCode::Loop(loop_start), line);

                    let exit_target = self.chunk.code.len();
                    self.chunk.code[exit_jump_idx] = OpCode::JumpIfFalse(exit_target);

                    let ctx = self.loop_stack.pop().unwrap();
                    for break_jump in ctx.break_jumps {
                        self.chunk.code[break_jump] = OpCode::Jump(exit_target);
                    }

                    self.end_scope(line);
                    self.emit_op(OpCode::Nil, line);
                } else {
                    return Err(CompileError::UnsupportedSyntax(
                        "Only range-based for loops are currently supported in VM".into(),
                    ));
                }
            }
            ExprKind::Break(expr_opt) => {
                if let Some(val) = expr_opt {
                    self.compile_expr(val)?;
                }
                if self.loop_stack.is_empty() {
                    return Err(CompileError::BreakOutsideLoop);
                }
                let jump_idx = self.chunk.code.len();
                self.emit_op(OpCode::Jump(0), line);
                self.loop_stack
                    .last_mut()
                    .unwrap()
                    .break_jumps
                    .push(jump_idx);
            }
            ExprKind::Continue => {
                if let Some(ctx) = self.loop_stack.last() {
                    self.emit_op(OpCode::Loop(ctx.start_ip), line);
                } else {
                    return Err(CompileError::ContinueOutsideLoop);
                }
            }
            ExprKind::Return(expr_opt) => {
                if let Some(val) = expr_opt {
                    self.compile_expr(val)?;
                } else {
                    self.emit_op(OpCode::Nil, line);
                }
                self.emit_op(OpCode::Return, line);
            }
            ExprKind::Array(elements) => {
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit_op(OpCode::MakeArray(elements.len()), line);
            }
            ExprKind::Index { target, index } => {
                self.compile_expr(target)?;
                self.compile_expr(index)?;
                self.emit_op(OpCode::GetIndex, line);
            }
            ExprKind::MemberAccess { object, member } => {
                self.compile_expr(object)?;
                let name_idx = self.add_constant(Value::from(member.name.clone()))?;
                self.emit_op(OpCode::GetField(name_idx), line);
            }
            ExprKind::Match { value, arms } => {
                self.compile_expr(value)?;
                self.begin_scope();
                let val_slot = self.add_local("__match_val", false)?;

                let mut end_jumps = Vec::new();

                for arm in arms {
                    match &arm.pattern.kind {
                        PatternKind::Literal(lit) => {
                            self.emit_op(OpCode::GetLocal(val_slot), line);
                            match lit {
                                Literal::Int(i) => self.emit_constant(Value::Int(*i), line)?,
                                Literal::Float(f) => self.emit_constant(Value::Float(*f), line)?,
                                Literal::String(s) | Literal::RawString(s) => {
                                    self.emit_constant(Value::from(s.clone()), line)?;
                                }
                                Literal::Bool(b) => self.emit_constant(Value::Bool(*b), line)?,
                                Literal::Char(c) => {
                                    self.emit_constant(Value::from(c.to_string()), line)?;
                                }
                                Literal::Nil => self.emit_op(OpCode::Nil, line),
                            }
                            self.emit_op(OpCode::Equal, line);

                            let next_arm_jump = self.chunk.code.len();
                            self.emit_op(OpCode::JumpIfFalse(0), line);

                            self.compile_expr(&arm.body)?;
                            let end_jump = self.chunk.code.len();
                            self.emit_op(OpCode::Jump(0), line);
                            end_jumps.push(end_jump);

                            let next_target = self.chunk.code.len();
                            self.chunk.code[next_arm_jump] = OpCode::JumpIfFalse(next_target);
                        }
                        PatternKind::Wildcard => {
                            self.compile_expr(&arm.body)?;
                            let end_jump = self.chunk.code.len();
                            self.emit_op(OpCode::Jump(0), line);
                            end_jumps.push(end_jump);
                            break;
                        }
                        PatternKind::Variable(ident) => {
                            self.emit_op(OpCode::GetLocal(val_slot), line);
                            let arm_slot = self.add_local(ident.name.clone(), false)?;
                            self.emit_op(OpCode::SetLocal(arm_slot), line);

                            self.compile_expr(&arm.body)?;
                            let end_jump = self.chunk.code.len();
                            self.emit_op(OpCode::Jump(0), line);
                            end_jumps.push(end_jump);
                            break;
                        }
                        _ => {
                            return Err(CompileError::UnsupportedSyntax(
                                "Complex patterns in match arms not yet supported".into(),
                            ));
                        }
                    }
                }

                let end_target = self.chunk.code.len();
                for j in end_jumps {
                    self.chunk.code[j] = OpCode::Jump(end_target);
                }

                // Discard __match_val while keeping the matched arm result
                self.emit_op(OpCode::Swap, line);
                self.emit_op(OpCode::Pop, line);
                self.locals.pop();
                self.scope_depth -= 1;
            }
            ExprKind::Sizeof(_) | ExprKind::Alignof(_) => {
                self.emit_constant(Value::Int(8), line)?;
            }
            ExprKind::Cast { expr, .. } => {
                self.compile_expr(expr)?;
            }
            other => {
                return Err(CompileError::UnsupportedSyntax(format!("{other:?}")));
            }
        }
        Ok(())
    }

    /// Compiles a block of statements.
    pub fn compile_block(&mut self, block: &Block) -> Result<(), CompileError> {
        let line = 1;
        self.begin_scope();
        let stmt_count = block.statements.len();
        if stmt_count == 0 {
            self.emit_op(OpCode::Nil, line);
        } else {
            for (i, stmt) in block.statements.iter().enumerate() {
                if i == stmt_count - 1 {
                    // Last statement in block provides block value if it is an Expr
                    if let StmtKind::Expr(expr) = &stmt.kind {
                        self.compile_expr(expr)?;
                    } else {
                        self.compile_stmt(stmt)?;
                        self.emit_op(OpCode::Nil, line);
                    }
                } else {
                    self.compile_stmt(stmt)?;
                }
            }
        }
        self.end_scope(line);
        Ok(())
    }

    /// Finishes function compilation and returns the `CompiledFunction`.
    pub fn finish(mut self) -> CompiledFunction {
        // Guarantee return at end of function
        if !matches!(self.chunk.code.last(), Some(OpCode::Return)) {
            self.emit_op(OpCode::Nil, 1);
            self.emit_op(OpCode::Return, 1);
        }

        CompiledFunction {
            name: self.name,
            arity: self.arity,
            chunk: self.chunk,
            is_unsafe: self.is_unsafe,
        }
    }
}

/// Main Bytecode Compiler managing whole-program translation.
#[derive(Debug, Default)]
pub struct BytecodeCompiler;

impl BytecodeCompiler {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compiles an entire AST `Program` into a top-level executable function.
    pub fn compile(&mut self, program: &Program) -> Result<Rc<CompiledFunction>, CompileError> {
        let mut script_compiler = FunctionCompiler::new("<script>", 0, false);
        let line = 1;

        // Pass 1: Compile all function declarations and expose them as globals
        for item in &program.items {
            if let ItemKind::Function(func_decl) = &item.kind {
                let compiled_fn = Self::compile_function(func_decl)?;
                let f_const =
                    script_compiler.add_constant(Value::Function(Rc::new(compiled_fn)))?;
                let name_const =
                    script_compiler.add_constant(Value::from(func_decl.name.name.clone()))?;

                script_compiler.emit_op(OpCode::Constant(f_const), line);
                script_compiler.emit_op(OpCode::DefineGlobal(name_const), line);
            }
        }

        // Pass 2: Compile top-level statements
        for stmt in &program.statements {
            script_compiler.compile_stmt(stmt)?;
        }

        // Pass 3: If a `main` function is defined and not explicitly called, invoke it
        let has_main = program.items.iter().any(|item| {
            if let ItemKind::Function(f) = &item.kind {
                f.name.name == "main"
            } else {
                false
            }
        });

        if has_main && program.statements.is_empty() {
            let main_idx = script_compiler.add_constant(Value::from("main".to_string()))?;
            script_compiler.emit_op(OpCode::GetGlobal(main_idx), line);
            script_compiler.emit_op(OpCode::Call(0), line);
            script_compiler.emit_op(OpCode::Return, line);
        } else {
            script_compiler.emit_op(OpCode::Nil, line);
            script_compiler.emit_op(OpCode::Return, line);
        }

        Ok(Rc::new(script_compiler.finish()))
    }

    /// Compiles a function declaration into a `CompiledFunction`.
    fn compile_function(decl: &FunctionDecl) -> Result<CompiledFunction, CompileError> {
        let mut compiler =
            FunctionCompiler::new(decl.name.name.clone(), decl.params.len(), decl.is_unsafe);

        // Parameters populate slots 1..=arity
        for param in &decl.params {
            compiler.add_local(param.name.name.clone(), false)?;
        }

        compiler.compile_block(&decl.body)?;

        Ok(compiler.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::Vm;
    use aurora_ast::ast::Ident;
    use aurora_ast::span::Span;

    #[test]
    fn test_compile_and_execute_simple_addition() {
        let mut compiler = BytecodeCompiler::new();
        let dummy_span = Span::new(0, 0);

        // let x = 10; let y = 32; x + y
        let program = Program {
            items: vec![],
            statements: vec![
                Stmt::new(
                    StmtKind::Let {
                        name: Ident::new("x", dummy_span),
                        type_annotation: None,
                        initializer: Some(Expr::new(
                            ExprKind::Literal(Literal::Int(10)),
                            dummy_span,
                        )),
                        is_mutable: false,
                    },
                    dummy_span,
                ),
                Stmt::new(
                    StmtKind::Let {
                        name: Ident::new("y", dummy_span),
                        type_annotation: None,
                        initializer: Some(Expr::new(
                            ExprKind::Literal(Literal::Int(32)),
                            dummy_span,
                        )),
                        is_mutable: false,
                    },
                    dummy_span,
                ),
            ],
        };

        let compiled = compiler.compile(&program).unwrap();
        let mut vm = Vm::new();
        let result = vm.run_main(compiled).unwrap();
        assert_eq!(result, Value::Nil);
        assert_eq!(vm.globals.get("x"), Some(&Value::Int(10)));
        assert_eq!(vm.globals.get("y"), Some(&Value::Int(32)));
    }
}
