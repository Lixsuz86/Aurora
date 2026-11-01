//! High-performance stack-based virtual machine execution loop.

use crate::opcode::{Chunk, OpCode};
use crate::value::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("Invalid constant index: {0}")]
    InvalidConstant(usize),
}

/// The Aurora Bytecode Virtual Machine.
#[derive(Debug, Default)]
pub struct Vm {
    stack: Vec<Value>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
        }
    }

    pub fn push(&mut self, val: Value) {
        self.stack.push(val);
    }

    pub fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, VmError> {
        let mut ip = 0;
        while ip < chunk.code.len() {
            let op = &chunk.code[ip];
            ip += 1;

            match op {
                OpCode::Constant(idx) => {
                    let val = chunk
                        .constants
                        .get(*idx)
                        .cloned()
                        .ok_or(VmError::InvalidConstant(*idx))?;
                    self.push(val);
                }
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::False => self.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.pop()?;
                }
                OpCode::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x + y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x + y)),
                        (Value::String(x), Value::String(y)) => {
                            self.push(Value::String(format!("{x}{y}")));
                        }
                        (x, y) => {
                            return Err(VmError::TypeMismatch(format!("Cannot add {x} and {y}")))
                        }
                    }
                }
                OpCode::Sub => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x - y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x - y)),
                        (x, y) => {
                            return Err(VmError::TypeMismatch(format!(
                                "Cannot subtract {y} from {x}"
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
                        (x, y) => {
                            return Err(VmError::TypeMismatch(format!(
                                "Cannot multiply {x} and {y}"
                            )))
                        }
                    }
                }
                OpCode::Div => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x / y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Float(x / y)),
                        (x, y) => {
                            return Err(VmError::TypeMismatch(format!("Cannot divide {x} by {y}")))
                        }
                    }
                }
                OpCode::Mod => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x % y)),
                        (x, y) => {
                            return Err(VmError::TypeMismatch(format!("Cannot modulo {x} by {y}")))
                        }
                    }
                }
                OpCode::Neg => {
                    let a = self.pop()?;
                    match a {
                        Value::Int(x) => self.push(Value::Int(-x)),
                        Value::Float(x) => self.push(Value::Float(-x)),
                        x => return Err(VmError::TypeMismatch(format!("Cannot negate {x}"))),
                    }
                }
                OpCode::Not => {
                    let a = self.pop()?;
                    self.push(Value::Bool(!a.is_truthy()));
                }
                OpCode::Equal => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a == b));
                }
                OpCode::Greater => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(x > y)),
                        (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(x > y)),
                        (x, y) => {
                            return Err(VmError::TypeMismatch(format!("Cannot compare {x} > {y}")))
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
                            return Err(VmError::TypeMismatch(format!("Cannot compare {x} < {y}")))
                        }
                    }
                }
                OpCode::LoadDeref | OpCode::StoreDeref => {
                    // Low-level pointer operations placeholder
                }
                OpCode::Jump(target) => {
                    ip = *target;
                }
                OpCode::JumpIfFalse(target) => {
                    if let Some(val) = self.stack.last() {
                        if !val.is_truthy() {
                            ip = *target;
                        }
                    }
                }
                OpCode::Return => {
                    return self.pop().or(Ok(Value::Nil));
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
}
