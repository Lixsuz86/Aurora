//! High-performance Bytecode Virtual Machine, Linear Memory Manager, and Compiler for Aurora.

pub mod compiler;
pub mod frame;
pub mod memory;
pub mod opcode;
pub mod value;
pub mod vm;

pub use compiler::{BytecodeCompiler, CompileError, FunctionCompiler};
pub use frame::CallFrame;
pub use memory::{LinearMemory, MemoryError};
pub use opcode::{Chunk, OpCode};
pub use value::{CompiledFunction, NativeFn, NativeFunction, StructInstance, Value};
pub use vm::{Vm, VmError};
