//! Virtual Machine instruction set (Opcodes) and Bytecode Chunks.

use crate::value::Value;
use std::fmt::Write;

/// Bytecode instructions for the Aurora Stack-based Virtual Machine.
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    /// Pushes a constant value from the constants pool onto the operand stack.
    Constant(usize),
    /// Pushes `nil` onto the stack.
    Nil,
    /// Pushes `true` onto the stack.
    True,
    /// Pushes `false` onto the stack.
    False,

    /// Pops the top value from the stack.
    Pop,
    /// Duplicates the top value on the stack.
    Dup,
    /// Swaps the top two values on the stack.
    Swap,

    /// Reads a local variable by slot index relative to the current call frame.
    GetLocal(usize),
    /// Writes to a local variable by slot index.
    SetLocal(usize),

    /// Defines a global variable with the name stored at the constant index.
    DefineGlobal(usize),
    /// Reads a global variable by name constant index.
    GetGlobal(usize),
    /// Updates an existing global variable by name constant index.
    SetGlobal(usize),

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,

    // Logical & Comparison
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Control Flow
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Loop(usize),

    // Function Calls
    Call(usize),
    Return,

    // Data Structures
    MakeArray(usize),
    GetIndex,
    SetIndex,
    MakeStruct {
        name_idx: usize,
        field_count: usize,
    },
    GetField(usize),
    SetField(usize),

    // Low-level Memory and Raw Pointer operations
    Alloc,
    Free,
    LoadDeref,
    StoreDeref,
    Load8,
    Store8,

    // Builtins / IO
    Print(usize),
    Println(usize),
}

/// A chunk of bytecode instructions, constant pool, and debug info.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    /// Creates a new, empty bytecode chunk.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an opcode to the chunk, recording its source line.
    pub fn write_opcode(&mut self, op: OpCode, line: usize) {
        self.code.push(op);
        self.lines.push(line);
    }

    /// Adds a value to the constant pool and returns its index.
    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// Disassembles the full chunk into a human-readable bytecode dump.
    #[must_use]
    pub fn disassemble(&self, name: &str) -> String {
        let mut out = format!("== {name} ==\n");
        let mut offset = 0;
        while offset < self.code.len() {
            let (instr_str, next_offset) = self.disassemble_instruction(offset);
            out.push_str(&instr_str);
            out.push('\n');
            offset = next_offset;
        }
        out
    }

    /// Disassembles a single instruction at `offset`.
    #[must_use]
    pub fn disassemble_instruction(&self, offset: usize) -> (String, usize) {
        let mut s = format!("{offset:04} ");

        // Line info
        if offset > 0 && self.lines.get(offset) == self.lines.get(offset - 1) {
            s.push_str("   | ");
        } else {
            let line = self.lines.get(offset).copied().unwrap_or(0);
            let _ = write!(s, "{line:4} ");
        }

        let Some(op) = self.code.get(offset) else {
            return (format!("{s} <invalid offset>"), offset + 1);
        };

        match op {
            OpCode::Constant(idx) => {
                let val_str = self
                    .constants
                    .get(*idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} {idx:4} ({val_str})", "CONSTANT");
            }
            OpCode::Nil => s.push_str("NIL"),
            OpCode::True => s.push_str("TRUE"),
            OpCode::False => s.push_str("FALSE"),
            OpCode::Pop => s.push_str("POP"),
            OpCode::Dup => s.push_str("DUP"),
            OpCode::Swap => s.push_str("SWAP"),
            OpCode::GetLocal(slot) => {
                let _ = write!(s, "{:<16} {slot:4}", "GET_LOCAL");
            }
            OpCode::SetLocal(slot) => {
                let _ = write!(s, "{:<16} {slot:4}", "SET_LOCAL");
            }
            OpCode::DefineGlobal(idx) => {
                let name = self
                    .constants
                    .get(*idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} {idx:4} ({name})", "DEFINE_GLOBAL");
            }
            OpCode::GetGlobal(idx) => {
                let name = self
                    .constants
                    .get(*idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} {idx:4} ({name})", "GET_GLOBAL");
            }
            OpCode::SetGlobal(idx) => {
                let name = self
                    .constants
                    .get(*idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} {idx:4} ({name})", "SET_GLOBAL");
            }
            OpCode::Add => s.push_str("ADD"),
            OpCode::Sub => s.push_str("SUB"),
            OpCode::Mul => s.push_str("MUL"),
            OpCode::Div => s.push_str("DIV"),
            OpCode::Mod => s.push_str("MOD"),
            OpCode::Neg => s.push_str("NEG"),
            OpCode::BitAnd => s.push_str("BIT_AND"),
            OpCode::BitOr => s.push_str("BIT_OR"),
            OpCode::BitXor => s.push_str("BIT_XOR"),
            OpCode::BitNot => s.push_str("BIT_NOT"),
            OpCode::Shl => s.push_str("SHL"),
            OpCode::Shr => s.push_str("SHR"),
            OpCode::Not => s.push_str("NOT"),
            OpCode::Equal => s.push_str("EQUAL"),
            OpCode::NotEqual => s.push_str("NOT_EQUAL"),
            OpCode::Greater => s.push_str("GREATER"),
            OpCode::GreaterEqual => s.push_str("GREATER_EQUAL"),
            OpCode::Less => s.push_str("LESS"),
            OpCode::LessEqual => s.push_str("LESS_EQUAL"),
            OpCode::Jump(target) => {
                let _ = write!(s, "{:<16} -> {target:04}", "JUMP");
            }
            OpCode::JumpIfFalse(target) => {
                let _ = write!(s, "{:<16} -> {target:04}", "JUMP_IF_FALSE");
            }
            OpCode::JumpIfTrue(target) => {
                let _ = write!(s, "{:<16} -> {target:04}", "JUMP_IF_TRUE");
            }
            OpCode::Loop(target) => {
                let _ = write!(s, "{:<16} -> {target:04}", "LOOP");
            }
            OpCode::Call(arity) => {
                let _ = write!(s, "{:<16} arity={arity}", "CALL");
            }
            OpCode::Return => s.push_str("RETURN"),
            OpCode::MakeArray(count) => {
                let _ = write!(s, "{:<16} count={count}", "MAKE_ARRAY");
            }
            OpCode::GetIndex => s.push_str("GET_INDEX"),
            OpCode::SetIndex => s.push_str("SET_INDEX"),
            OpCode::MakeStruct {
                name_idx,
                field_count,
            } => {
                let name = self
                    .constants
                    .get(*name_idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} name={name} fields={field_count}", "MAKE_STRUCT");
            }
            OpCode::GetField(idx) => {
                let name = self
                    .constants
                    .get(*idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} ({name})", "GET_FIELD");
            }
            OpCode::SetField(idx) => {
                let name = self
                    .constants
                    .get(*idx)
                    .map_or_else(|| "<missing>".into(), |v| format!("{v}"));
                let _ = write!(s, "{:<16} ({name})", "SET_FIELD");
            }
            OpCode::Alloc => s.push_str("ALLOC"),
            OpCode::Free => s.push_str("FREE"),
            OpCode::LoadDeref => s.push_str("LOAD_DEREF"),
            OpCode::StoreDeref => s.push_str("STORE_DEREF"),
            OpCode::Load8 => s.push_str("LOAD_8"),
            OpCode::Store8 => s.push_str("STORE_8"),
            OpCode::Print(arity) => {
                let _ = write!(s, "{:<16} arity={arity}", "PRINT");
            }
            OpCode::Println(arity) => {
                let _ = write!(s, "{:<16} arity={arity}", "PRINTLN");
            }
        }

        (s, offset + 1)
    }
}
