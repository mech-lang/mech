//! Compile-time lowering of owned, verified native SSA. This module constructs
//! LLVM IR, never a runtime evaluator or an independently trusted descriptor.
mod lower;
mod recording;

pub(super) use recording::RecordingModule;

#[cfg(test)]
mod tests;
