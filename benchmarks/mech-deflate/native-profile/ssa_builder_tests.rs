//! Exercise the exact vendored builder without the upstream x86 dev features.
extern crate alloc;
pub use cranelift_frontend::Variable;
#[allow(dead_code)]
#[path = "../../../vendor/cranelift-frontend/src/ssa.rs"]
mod ssa;
