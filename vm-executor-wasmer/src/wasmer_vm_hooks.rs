use std::rc::Rc;

use crate::executor_interface::{MemLength, MemPtr, VMHooksLegacy};
use wasmer::WasmerEnv;

#[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    pub vm_hooks: Rc<dyn VMHooksLegacy>,
}

// Wasmer 2.2 requires its environment to be Send + Sync even though the
// MultiversX VM hook path is executed as a single-threaded contract context.
// Keep this upstream assertion local to the Wasmer environment wrapper; the
// wrapped VMHooksLegacy implementations themselves remain free to use RefCell
// and fail fast on same-thread reentry.
unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

impl WasmerEnv for VMHooksWrapper {}

impl VMHooksWrapper {
    pub(crate) fn convert_mem_ptr(&self, raw: i32) -> MemPtr {
        raw as MemPtr
    }

    pub(crate) fn convert_mem_length(&self, raw: i32) -> MemLength {
        raw as MemLength
    }
}
