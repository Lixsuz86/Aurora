//! Virtual Machine instruction set (Opcodes).

use crate::value::Value;

/// Bytecode instructions for the Aurora VM.
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    /// Pushes a constant value onto the operand stack.
    Constant(usize),
    /// Returns nil.
    Nil,
    /// Returns true.
    True,
    /// Returns false.
    False,
    /// Pops value from the stack.
    Pop,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,

    // Logical & Comparison
    Not,
    Equal,
    Greater,
    Less,

    // Low-level memory operations
    LoadDeref,  // Read memory at address on stack
    StoreDeref, // Write value to address on stack

    // Control Flow
    Jump(usize),
    JumpIfFalse(usize),
    Return,
}

/// A chunk of bytecode instructions, constant pool, and debug info.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_opcode(&mut self, op: OpCode, line: usize) {
        self.code.push(op);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
}
