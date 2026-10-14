//! General closed-loop Cranelift AOT execution and bounded range scheduling.
#[cfg(feature = "sequence-cli")]
mod bigint;
#[cfg(feature = "sequence-cli")]
mod runtime;
#[cfg(feature = "sequence-cli")]
pub use runtime::run_cli;
mod compiler;
pub use compiler::{JitProgram, emit};
mod resident;
pub use resident::ResidentProgram;

// Private same-object ABI. Storage remains owned by a core planned projection.
#[repr(C)]
struct RefArenaContext {
    base: *mut u64,
    capacity: u64,
    cursor: u64,
    arity: u64,
    layout: u64,
}

impl RefArenaContext {
    const BASE: i32 = core::mem::offset_of!(Self, base) as i32;
    const CAPACITY: i32 = core::mem::offset_of!(Self, capacity) as i32;
    const CURSOR: i32 = core::mem::offset_of!(Self, cursor) as i32;
    const LAYOUT: i32 = core::mem::offset_of!(Self, layout) as i32;
}
