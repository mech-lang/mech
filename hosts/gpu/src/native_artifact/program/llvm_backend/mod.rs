//! Compile-time lowering of owned, verified native SSA. This module constructs
//! LLVM IR, never a runtime evaluator or an independently trusted descriptor.
mod lower;
mod machine;
mod recording;
mod toolchain;

pub(super) use recording::RecordingModule;
pub use toolchain::NativeLlvmToolchain;

#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod image_tests;
#[cfg(test)]
mod tests;
