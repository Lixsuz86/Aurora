//! High-performance Stack-based Virtual Machine and Bytecode Interpreter.

use crate::frame::CallFrame;
use crate::memory::{LinearMemory, MemoryError};
use crate::opcode::{Chunk, OpCode};
use crate::value::{CompiledFunction, NativeFn, NativeFunction, StructInstance, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use thiserror::Error;

/// Runtime errors produced during bytecode execution.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum VmError {
    #[error("Operand stack underflow")]
    StackUnderflow,
    #[error("Call stack overflow (recursion limit exceeded)")]
    StackOverflow,
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    #[error("Invalid constant pool index: {0}")]
    InvalidConstant(usize),
    #[error("Undefined variable '{0}'")]
    UndefinedVariable(String),
    #[error("Undefined struct field '{0}'")]
    UndefinedField(String),
    #[error("Index out of bounds: index {index} on collection of length {length}")]
    IndexOutOfBounds { index: i64, length: usize },
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Null pointer dereference: attempted to access address 0x0")]
    NullPointerDereference,
    #[error("Memory out of bounds: address 0x{address:08x} for size {size}")]
    MemoryOutOfBounds { address: usize, size: usize },
    #[error("Function arity mismatch: expected {expected} arguments, found {found}")]
    InvalidArity { expected: usize, found: usize },
    #[error("Type '{0}' is not callable")]
    NotCallable(String),
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
    #[error("Memory error: {0}")]
    MemoryError(#[from] MemoryError),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

/// The Aurora Bytecode Virtual Machine.
#[derive(Debug)]
pub struct Vm {
    pub frames: Vec<CallFrame>,
    pub stack: Vec<Value>,
    pub globals: HashMap<String, Value>,
    pub memory: LinearMemory,
    pub output_capture: Option<String>,
    pub max_frames: usize,
    pub start_time: Instant,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    /// Creates a new virtual machine instance pre-populated with standard built-in functions.
    pub fn new() -> Self {
        let mut vm = Self {
            frames: Vec::with_capacity(64),
            stack: Vec::with_capacity(256),
            globals: HashMap::new(),
            memory: LinearMemory::default(),
            output_capture: None,
            max_frames: 1024,
            start_time: Instant::now(),
        };

        vm.register_builtins();
        vm
    }

    /// Configures the VM to capture stdout prints into an internal string for test assertions.
    #[must_use]
    pub fn with_output_capture(mut self) -> Self {
        self.output_capture = Some(String::new());
        self
    }

    /// Returns captured output as a string slice if output capturing is enabled.
    #[must_use]
    pub fn captured_output(&self) -> Option<&str> {
        self.output_capture.as_deref()
    }

    /// Returns captured output split into lines.
    #[must_use]
    pub fn captured_lines(&self) -> Option<Vec<String>> {
        self.output_capture
            .as_ref()
            .map(|s| s.lines().map(ToString::to_string).collect())
    }

    /// Pushes a value onto the operand stack.
    #[inline]
    pub fn push(&mut self, val: Value) {
        self.stack.push(val);
    }

    /// Pops a value from the operand stack.
    #[inline]
    pub fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    /// Peeks at a value on the stack `distance` items from the top (0 = top item).
    #[inline]
    pub fn peek(&self, distance: usize) -> Result<&Value, VmError> {
        if distance >= self.stack.len() {
            return Err(VmError::StackUnderflow);
        }
        Ok(&self.stack[self.stack.len() - 1 - distance])
    }

    /// Defines a global variable by name.
    pub fn define_global(&mut self, name: impl Into<String>, val: Value) {
        self.globals.insert(name.into(), val);
    }

    /// Defines a native function available globally.
    pub fn define_native(&mut self, name: &str, arity: Option<usize>, func: NativeFn) {
        let native = NativeFunction {
            name: name.to_string(),
            arity,
            func,
        };
        self.globals.insert(name.to_string(), Value::Native(native));
    }

    /// Emits output text without a trailing newline.
    pub fn print_output(&mut self, text: &str) {
        if let Some(ref mut cap) = self.output_capture {
            cap.push_str(text);
        } else {
            print!("{text}");
        }
    }

    /// Emits output text with a trailing newline.
    pub fn println_output(&mut self, text: &str) {
        if let Some(ref mut cap) = self.output_capture {
            cap.push_str(text);
            cap.push('\n');
        } else {
            println!("{text}");
        }
    }

    /// Registers the standard built-in functions.
    fn register_builtins(&mut self) {
        // clock(): returns elapsed seconds as float
        self.define_native("clock", Some(0), |vm, _args| {
            #[allow(clippy::cast_precision_loss)]
            let elapsed = vm.start_time.elapsed().as_secs_f64();
            Ok(Value::Float(elapsed))
        });

        // alloc(size): allocates linear memory
        self.define_native("alloc", Some(1), |vm, args| {
            let size = args[0].as_int()? as usize;
            let ptr = vm.memory.allocate(size, 8);
            Ok(Value::Pointer(ptr))
        });

        // free(ptr): releases linear memory
        self.define_native("free", Some(1), |vm, args| {
            let ptr = args[0].as_ptr()?;
            vm.memory.free(ptr);
            Ok(Value::Nil)
        });

        // len(collection): returns string or array length
        self.define_native("len", Some(1), |_vm, args| match &args[0] {
            Value::String(s) => Ok(Value::Int(s.len() as i64)),
            Value::Array(arr) => Ok(Value::Int(arr.borrow().len() as i64)),
            other => Err(VmError::TypeMismatch {
                expected: "string or array",
                found: other.type_name(),
            }),
        });

        // push(array, value): appends item to array
        self.define_native("push", Some(2), |_vm, args| match &args[0] {
            Value::Array(arr) => {
                arr.borrow_mut().push(args[1].clone());
                Ok(Value::Nil)
            }
            other => Err(VmError::TypeMismatch {
                expected: "array",
                found: other.type_name(),
            }),
        });

        // pop(array): removes and returns last item
        self.define_native("pop", Some(1), |_vm, args| match &args[0] {
            Value::Array(arr) => Ok(arr.borrow_mut().pop().unwrap_or(Value::Nil)),
            other => Err(VmError::TypeMismatch {
                expected: "array",
                found: other.type_name(),
            }),
        });

        // assert(cond, [msg])
        self.define_native("assert", None, |_vm, args| {
            if args.is_empty() {
                return Err(VmError::InvalidArity {
                    expected: 1,
                    found: 0,
                });
            }
            if !args[0].is_truthy() {
                let msg = args
                    .get(1)
                    .map_or_else(|| "assertion failed".into(), ToString::to_string);
                return Err(VmError::AssertionFailed(msg));
            }
            Ok(Value::Bool(true))
        });

        // assert_eq(actual, expected)
        self.define_native("assert_eq", Some(2), |_vm, args| {
            if args[0] != args[1] {
                let msg = format!("assert_eq failed: {} != {}", args[0], args[1]);
                return Err(VmError::AssertionFailed(msg));
            }
            Ok(Value::Bool(true))
        });

        // println(...args): prints arguments separated by space with a newline
        self.define_native("println", None, |vm, args| {
            let line = args
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            vm.println_output(&line);
            Ok(Value::Nil)
        });

        // print(...args): prints arguments separated by space without newline
        self.define_native("print", None, |vm, args| {
            let text = args
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            vm.print_output(&text);
            Ok(Value::Nil)
        });
    }

    /// Retrieves a string constant from the current function chunk.
    fn get_constant_string(chunk: &Chunk, idx: usize) -> Result<String, VmError> {
        match chunk.constants.get(idx) {
            Some(Value::String(s)) => Ok((**s).clone()),
            Some(other) => Ok(other.to_string()),
            None => Err(VmError::InvalidConstant(idx)),
        }
    }

    /// Executes a single standalone `Chunk` (e.g. for testing and backward compatibility).
    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, VmError> {
        let func = Rc::new(CompiledFunction {
            name: "<script>".to_string(),
            arity: 0,
            chunk: chunk.clone(),
            is_unsafe: false,
        });
        self.run_main(func)
    }

    /// Executes a compiled top-level function on the VM.
    pub fn run_main(&mut self, main_func: Rc<CompiledFunction>) -> Result<Value, VmError> {
        self.frames.clear();
        self.stack.clear();

        // Slot 0 holds the main function
        self.stack.push(Value::Function(main_func.clone()));
        self.frames.push(CallFrame::new(main_func, 0));

        self.execute_loop()
    }

    /// Inner instruction dispatch and execution loop.
    #[allow(clippy::too_many_lines)]
    fn execute_loop(&mut self) -> Result<Value, VmError> {
        while let Some(frame) = self.frames.last_mut() {
            if frame.ip >= frame.function.chunk.code.len() {
                // Fell off end of function without explicit Return
                let finished_offset = frame.slot_offset;
                self.frames.pop();
                self.stack.truncate(finished_offset);
                if self.frames.is_empty() {
                    return Ok(Value::Nil);
                }
                self.stack.push(Value::Nil);
                continue;
            }

            let op = frame.function.chunk.code[frame.ip].clone();
            frame.ip += 1;

            match op {
                OpCode::Constant(idx) => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let val = chunk
                        .constants
                        .get(idx)
                        .cloned()
                        .ok_or(VmError::InvalidConstant(idx))?;
                    self.push(val);
                }
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::False => self.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.pop()?;
                }
                OpCode::Dup => {
                    let val = self.peek(0)?.clone();
                    self.push(val);
                }
                OpCode::Swap => {
                    let len = self.stack.len();
                    if len < 2 {
                        return Err(VmError::StackUnderflow);
                    }
                    self.stack.swap(len - 1, len - 2);
                }
                OpCode::GetLocal(slot) => {
                    let offset = self.frames.last().unwrap().slot_offset + slot;
                    let val = self.stack.get(offset).cloned().ok_or_else(|| {
                        VmError::RuntimeError(format!("Invalid local slot {slot}"))
                    })?;
                    self.push(val);
                }
                OpCode::SetLocal(slot) => {
                    let val = self.peek(0)?.clone();
                    let offset = self.frames.last().unwrap().slot_offset + slot;
                    if offset >= self.stack.len() {
                        return Err(VmError::RuntimeError(format!("Invalid local slot {slot}")));
                    }
                    self.stack[offset] = val;
                }
                OpCode::DefineGlobal(name_idx) => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let name = Self::get_constant_string(chunk, name_idx)?;
                    let val = self.pop()?;
                    self.globals.insert(name, val);
                }
                OpCode::GetGlobal(name_idx) => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let name = Self::get_constant_string(chunk, name_idx)?;
                    if let Some(val) = self.globals.get(&name) {
                        self.push(val.clone());
                    } else {
                        return Err(VmError::UndefinedVariable(name));
                    }
                }
                OpCode::SetGlobal(name_idx) => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let name = Self::get_constant_string(chunk, name_idx)?;
                    let val = self.peek(0)?.clone();
                    match self.globals.entry(name) {
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            entry.insert(val);
                        }
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            return Err(VmError::UndefinedVariable(entry.into_key()));
                        }
                    }
                }

                // Arithmetic
                OpCode::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x + y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Int(x), Value::Float(y)) => self.push(Value::Float(x as f64 + y)),
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Float(x), Value::Int(y)) => self.push(Value::Float(x + y as f64)),
                        (Value::String(x), Value::String(y)) => {
                            self.push(Value::from(format!("{x}{y}")));
                        }
                        (Value::String(x), other) => {
                            self.push(Value::from(format!("{x}{other}")));
                        }
                        (other, Value::String(y)) => {
                            self.push(Value::from(format!("{other}{y}")));
                        }
                        (Value::Pointer(p), Value::Int(offset)) => {
                            let new_ptr = if offset >= 0 {
                                p + offset as usize
                            } else {
                                p - (-offset) as usize
                            };
                            self.push(Value::Pointer(new_ptr));
                        }
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot add {} and {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::Sub => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x - y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Int(x), Value::Float(y)) => self.push(Value::Float(x as f64 - y)),
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Float(x), Value::Int(y)) => self.push(Value::Float(x - y as f64)),
                        (Value::Pointer(p), Value::Int(offset)) => {
                            let new_ptr = if offset >= 0 {
                                p - offset as usize
                            } else {
                                p + (-offset) as usize
                            };
                            self.push(Value::Pointer(new_ptr));
                        }
                        (Value::Pointer(p1), Value::Pointer(p2)) => {
                            self.push(Value::Int(p1 as i64 - p2 as i64));
                        }
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot subtract {} from {}",
                                y.type_name(),
                                x.type_name()
                            )))
                        }
                    }
                }
                OpCode::Mul => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x * y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x * y)),
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Int(x), Value::Float(y)) => self.push(Value::Float(x as f64 * y)),
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Float(x), Value::Int(y)) => self.push(Value::Float(x * y as f64)),
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot multiply {} and {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::Div => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(_), Value::Int(0)) => return Err(VmError::DivisionByZero),
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x / y)),
                        (Value::Float(x), Value::Float(y)) => {
                            if y == 0.0 {
                                return Err(VmError::DivisionByZero);
                            }
                            self.push(Value::Float(x / y));
                        }
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Int(x), Value::Float(y)) => {
                            if y == 0.0 {
                                return Err(VmError::DivisionByZero);
                            }
                            self.push(Value::Float(x as f64 / y));
                        }
                        #[allow(clippy::cast_precision_loss)]
                        (Value::Float(x), Value::Int(y)) => {
                            if y == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            self.push(Value::Float(x / y as f64));
                        }
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot divide {} by {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::Mod => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(_), Value::Int(0)) => return Err(VmError::DivisionByZero),
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x % y)),
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot modulo {} by {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::Neg => {
                    let a = self.pop()?;
                    match a {
                        Value::Int(x) => self.push(Value::Int(-x)),
                        Value::Float(x) => self.push(Value::Float(-x)),
                        other => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot negate {}",
                                other.type_name()
                            )))
                        }
                    }
                }

                // Bitwise
                OpCode::BitAnd => {
                    let b = self.pop()?.as_int()?;
                    let a = self.pop()?.as_int()?;
                    self.push(Value::Int(a & b));
                }
                OpCode::BitOr => {
                    let b = self.pop()?.as_int()?;
                    let a = self.pop()?.as_int()?;
                    self.push(Value::Int(a | b));
                }
                OpCode::BitXor => {
                    let b = self.pop()?.as_int()?;
                    let a = self.pop()?.as_int()?;
                    self.push(Value::Int(a ^ b));
                }
                OpCode::BitNot => {
                    let a = self.pop()?.as_int()?;
                    self.push(Value::Int(!a));
                }
                OpCode::Shl => {
                    let b = self.pop()?.as_int()?;
                    let a = self.pop()?.as_int()?;
                    self.push(Value::Int(a << b));
                }
                OpCode::Shr => {
                    let b = self.pop()?.as_int()?;
                    let a = self.pop()?.as_int()?;
                    self.push(Value::Int(a >> b));
                }

                // Logical & Comparison
                OpCode::Not => {
                    let a = self.pop()?;
                    self.push(Value::Bool(!a.is_truthy()));
                }
                OpCode::Equal => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a == b));
                }
                OpCode::NotEqual => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a != b));
                }
                OpCode::Greater => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x > y)),
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot compare {} > {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::GreaterEqual => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x >= y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x >= y)),
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot compare {} >= {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::Less => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x < y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x < y)),
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot compare {} < {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }
                OpCode::LessEqual => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x <= y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x <= y)),
                        (x, y) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot compare {} <= {}",
                                x.type_name(),
                                y.type_name()
                            )))
                        }
                    }
                }

                // Control Flow
                OpCode::Jump(target) | OpCode::Loop(target) => {
                    self.frames.last_mut().unwrap().ip = target;
                }
                OpCode::JumpIfFalse(target) => {
                    let cond = self.pop()?;
                    if !cond.is_truthy() {
                        self.frames.last_mut().unwrap().ip = target;
                    }
                }
                OpCode::JumpIfTrue(target) => {
                    let cond = self.pop()?;
                    if cond.is_truthy() {
                        self.frames.last_mut().unwrap().ip = target;
                    }
                }

                // Calls
                OpCode::Call(arity) => {
                    let callee_idx = self
                        .stack
                        .len()
                        .checked_sub(arity + 1)
                        .ok_or(VmError::StackUnderflow)?;
                    let callee = self.stack[callee_idx].clone();

                    match callee {
                        Value::Function(func) => {
                            if func.arity != arity {
                                return Err(VmError::InvalidArity {
                                    expected: func.arity,
                                    found: arity,
                                });
                            }
                            if self.frames.len() >= self.max_frames {
                                return Err(VmError::StackOverflow);
                            }
                            let frame = CallFrame::new(func, callee_idx);
                            self.frames.push(frame);
                        }
                        Value::Native(native) => {
                            if let Some(expected) = native.arity {
                                if expected != arity {
                                    return Err(VmError::InvalidArity {
                                        expected,
                                        found: arity,
                                    });
                                }
                            }
                            let args = self.stack[self.stack.len() - arity..].to_vec();
                            let result = (native.func)(self, &args)?;
                            self.stack.truncate(callee_idx);
                            self.push(result);
                        }
                        other => return Err(VmError::NotCallable(other.type_name().to_string())),
                    }
                }
                OpCode::Return => {
                    let result = self.pop().unwrap_or(Value::Nil);
                    let finished = self.frames.pop().unwrap();
                    self.stack.truncate(finished.slot_offset);
                    if self.frames.is_empty() {
                        return Ok(result);
                    }
                    self.push(result);
                }

                // Data Structures
                OpCode::MakeArray(count) => {
                    let mut items = Vec::with_capacity(count);
                    for _ in 0..count {
                        items.push(self.pop()?);
                    }
                    items.reverse();
                    self.push(Value::Array(Rc::new(RefCell::new(items))));
                }
                OpCode::GetIndex => {
                    let index = self.pop()?;
                    let target = self.pop()?;
                    match (target, index) {
                        (Value::Array(arr), Value::Int(idx)) => {
                            let items = arr.borrow();
                            let len = items.len();
                            let effective_idx = if idx < 0 {
                                (len as i64 + idx) as usize
                            } else {
                                idx as usize
                            };
                            if effective_idx >= len {
                                return Err(VmError::IndexOutOfBounds {
                                    index: idx,
                                    length: len,
                                });
                            }
                            self.push(items[effective_idx].clone());
                        }
                        (Value::String(s), Value::Int(idx)) => {
                            let len = s.chars().count();
                            let effective_idx = if idx < 0 {
                                (len as i64 + idx) as usize
                            } else {
                                idx as usize
                            };
                            if effective_idx >= len {
                                return Err(VmError::IndexOutOfBounds {
                                    index: idx,
                                    length: len,
                                });
                            }
                            let ch = s.chars().nth(effective_idx).unwrap();
                            self.push(Value::from(ch.to_string()));
                        }
                        (t, i) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot index {} with {}",
                                t.type_name(),
                                i.type_name()
                            )))
                        }
                    }
                }
                OpCode::SetIndex => {
                    let val = self.pop()?;
                    let index = self.pop()?;
                    let target = self.pop()?;
                    match (target, index) {
                        (Value::Array(arr), Value::Int(idx)) => {
                            let mut items = arr.borrow_mut();
                            let len = items.len();
                            let effective_idx = if idx < 0 {
                                (len as i64 + idx) as usize
                            } else {
                                idx as usize
                            };
                            if effective_idx >= len {
                                return Err(VmError::IndexOutOfBounds {
                                    index: idx,
                                    length: len,
                                });
                            }
                            items[effective_idx] = val.clone();
                            self.push(val);
                        }
                        (t, _) => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot assign index to {}",
                                t.type_name()
                            )))
                        }
                    }
                }
                OpCode::MakeStruct {
                    name_idx,
                    field_count,
                } => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let name = Self::get_constant_string(chunk, name_idx)?;
                    let mut fields = HashMap::with_capacity(field_count);
                    for _ in 0..field_count {
                        let val = self.pop()?;
                        let field_name_val = self.pop()?;
                        let field_name = match field_name_val {
                            Value::String(s) => (*s).clone(),
                            _ => {
                                return Err(VmError::TypeMismatch {
                                    expected: "string",
                                    found: field_name_val.type_name(),
                                })
                            }
                        };
                        fields.insert(field_name, val);
                    }
                    self.push(Value::Struct(Rc::new(RefCell::new(StructInstance {
                        name,
                        fields,
                    }))));
                }
                OpCode::GetField(idx) => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let field_name = Self::get_constant_string(chunk, idx)?;
                    let target = self.pop()?;
                    match target {
                        Value::Struct(s) => {
                            let borrow = s.borrow();
                            if let Some(val) = borrow.fields.get(&field_name) {
                                self.push(val.clone());
                            } else {
                                return Err(VmError::UndefinedField(format!(
                                    "{}.{field_name}",
                                    borrow.name
                                )));
                            }
                        }
                        other => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot read field '{field_name}' on {}",
                                other.type_name()
                            )))
                        }
                    }
                }
                OpCode::SetField(idx) => {
                    let chunk = &self.frames.last().unwrap().function.chunk;
                    let field_name = Self::get_constant_string(chunk, idx)?;
                    let val = self.pop()?;
                    let target = self.pop()?;
                    match target {
                        Value::Struct(s) => {
                            s.borrow_mut().fields.insert(field_name, val.clone());
                            self.push(val);
                        }
                        other => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot write field '{field_name}' on {}",
                                other.type_name()
                            )))
                        }
                    }
                }

                // Low-level Memory and Raw Pointer operations
                OpCode::Alloc => {
                    let size = self.pop()?.as_int()? as usize;
                    let ptr = self.memory.allocate(size, 8);
                    self.push(Value::Pointer(ptr));
                }
                OpCode::Free => {
                    let ptr = self.pop()?.as_ptr()?;
                    self.memory.free(ptr);
                }
                OpCode::LoadDeref => {
                    let ptr = self.pop()?.as_ptr()?;
                    let val = self.memory.read_i64(ptr)?;
                    self.push(Value::Int(val));
                }
                OpCode::StoreDeref => {
                    let val = self.pop()?;
                    let ptr = self.pop()?.as_ptr()?;
                    match &val {
                        Value::Int(i) => self.memory.write_i64(ptr, *i)?,
                        Value::Float(f) => self.memory.write_f64(ptr, *f)?,
                        Value::Bool(b) => self.memory.write_u8(ptr, u8::from(*b))?,
                        Value::Pointer(p) => self.memory.write_u64(ptr, *p as u64)?,
                        other => {
                            return Err(VmError::RuntimeError(format!(
                                "Cannot store {} to raw pointer",
                                other.type_name()
                            )))
                        }
                    }
                    self.push(val);
                }
                OpCode::Load8 => {
                    let ptr = self.pop()?.as_ptr()?;
                    let b = self.memory.read_u8(ptr)?;
                    self.push(Value::Int(i64::from(b)));
                }
                OpCode::Store8 => {
                    let val = self.pop()?;
                    let ptr = self.pop()?.as_ptr()?;
                    let byte = val.as_int()? as u8;
                    self.memory.write_u8(ptr, byte)?;
                    self.push(val);
                }

                // Builtins / IO
                OpCode::Print(arity) => {
                    let mut args = Vec::with_capacity(arity);
                    for _ in 0..arity {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let text = args
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ");
                    self.print_output(&text);
                    self.push(Value::Nil);
                }
                OpCode::Println(arity) => {
                    let mut args = Vec::with_capacity(arity);
                    for _ in 0..arity {
                        args.push(self.pop()?);
                    }
                    args.reverse();
                    let text = args
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ");
                    self.println_output(&text);
                    self.push(Value::Nil);
                }
            }
        }

        self.pop().or(Ok(Value::Nil))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_addition() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();

        let c1 = chunk.add_constant(Value::Int(40));
        let c2 = chunk.add_constant(Value::Int(2));

        chunk.write_opcode(OpCode::Constant(c1), 1);
        chunk.write_opcode(OpCode::Constant(c2), 1);
        chunk.write_opcode(OpCode::Add, 1);
        chunk.write_opcode(OpCode::Return, 1);

        let result = vm.run(&chunk).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_vm_function_call_and_locals() {
        let mut vm = Vm::new();

        // Sub-function: add(a, b) -> a + b
        let mut add_chunk = Chunk::new();
        add_chunk.write_opcode(OpCode::GetLocal(1), 1); // a
        add_chunk.write_opcode(OpCode::GetLocal(2), 1); // b
        add_chunk.write_opcode(OpCode::Add, 1);
        add_chunk.write_opcode(OpCode::Return, 1);

        let add_func = Rc::new(CompiledFunction {
            name: "add".to_string(),
            arity: 2,
            chunk: add_chunk,
            is_unsafe: false,
        });

        // Main function: calls add(15, 27)
        let mut main_chunk = Chunk::new();
        let f_const = main_chunk.add_constant(Value::Function(add_func));
        let c1 = main_chunk.add_constant(Value::Int(15));
        let c2 = main_chunk.add_constant(Value::Int(27));

        main_chunk.write_opcode(OpCode::Constant(f_const), 1);
        main_chunk.write_opcode(OpCode::Constant(c1), 1);
        main_chunk.write_opcode(OpCode::Constant(c2), 1);
        main_chunk.write_opcode(OpCode::Call(2), 1);
        main_chunk.write_opcode(OpCode::Return, 1);

        let main_func = Rc::new(CompiledFunction {
            name: "main".to_string(),
            arity: 0,
            chunk: main_chunk,
            is_unsafe: false,
        });

        let result = vm.run_main(main_func).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_vm_pointer_deref_and_store() {
        let mut vm = Vm::new();
        let mut chunk = Chunk::new();

        let size = chunk.add_constant(Value::Int(8));
        let val = chunk.add_constant(Value::Int(999));

        // alloc 8 bytes
        chunk.write_opcode(OpCode::Constant(size), 1);
        chunk.write_opcode(OpCode::Alloc, 1);
        chunk.write_opcode(OpCode::SetLocal(1), 1); // save ptr to slot 1

        // store 999 into *ptr
        chunk.write_opcode(OpCode::GetLocal(1), 1);
        chunk.write_opcode(OpCode::Constant(val), 1);
        chunk.write_opcode(OpCode::StoreDeref, 1);

        // read back *ptr
        chunk.write_opcode(OpCode::GetLocal(1), 1);
        chunk.write_opcode(OpCode::LoadDeref, 1);
        chunk.write_opcode(OpCode::Return, 1);

        let main_func = Rc::new(CompiledFunction {
            name: "main".to_string(),
            arity: 0,
            chunk,
            is_unsafe: true,
        });

        // Initialize stack slot 1
        vm.stack.push(Value::Nil);
        let result = vm.run_main(main_func).unwrap();
        assert_eq!(result, Value::Int(999));
    }
}
