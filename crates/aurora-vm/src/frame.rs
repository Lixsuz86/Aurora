//! Call Frame tracking for active function invocations on the execution stack.

use crate::value::CompiledFunction;
use std::rc::Rc;

/// A Call Frame representing an active function invocation on the VM stack.
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// The compiled bytecode function being executed.
    pub function: Rc<CompiledFunction>,
    /// Instruction pointer index into the function's bytecode chunk.
    pub ip: usize,
    /// Base offset into the VM value stack for local variables of this frame.
    pub slot_offset: usize,
}

impl CallFrame {
    /// Creates a new call frame starting at instruction 0.
    #[must_use]
    pub fn new(function: Rc<CompiledFunction>, slot_offset: usize) -> Self {
        Self {
            function,
            ip: 0,
            slot_offset,
        }
    }
}
