//! Explicit build-owned FFI binding, followed by the same managed evaluator as JIT.
use super::abi::NativeAbiDescriptor;
use super::identity::{NATIVE_LINK_IDENTITY_BYTES, descriptor_from_identity};
use super::runtime::{NativeExecutionError, NativeExecutionLimits, NativeExecutionReport};
use core::ffi::c_void;
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{NativeArtifactPlan, NativeScalarValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLinkError {
    reason: &'static str,
}
impl std::fmt::Display for NativeLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "canonical native object binding: {}",
            self.reason
        )
    }
}
impl std::error::Error for NativeLinkError {}

/// A statically linked build-owned entry with a checked opaque-plan identity.
/// Its raw pointer is private. There is no safe raw-entry constructor, plan
/// deserializer, resident executor fallback, or source-name lookup.
pub struct LinkedArtifact {
    entry: unsafe extern "C" fn(*mut c_void) -> u64,
    descriptor: NativeAbiDescriptor,
}
impl LinkedArtifact {
    /// Checks the object identity against a freshly selected canonical plan.
    /// The metadata traversal happens at binding, outside evaluation.
    ///
    /// # Safety
    /// `entry` must be the process-lifetime entry emitted by this crate's
    /// shared checked compiler, and `identity` must be its read-only exported
    /// identity symbol from the SAME exact object, with no post-emission code
    /// or metadata changes. The build must verify its object/bytecode/generated
    /// source pins before linking. The emitted entry must obey the private
    /// synchronous ABI for this ABI/work-policy version: no retained pointers,
    /// unwind, foreign callbacks, ambient effects, or access outside the frame's
    /// admitted transport. Merely constructing matching identity bytes does
    /// not satisfy this contract. An arbitrary foreign function is unsound.
    pub unsafe fn from_build_owned_entry(
        plan: &NativeArtifactPlan,
        identity: &'static [u8; NATIVE_LINK_IDENTITY_BYTES],
        entry: unsafe extern "C" fn(*mut c_void) -> u64,
    ) -> Result<Self, NativeLinkError> {
        let descriptor = descriptor_from_identity(plan, identity)
            .map_err(|reason| NativeLinkError { reason })?;
        Ok(Self { entry, descriptor })
    }
    pub fn descriptor(&self) -> &NativeAbiDescriptor {
        &self.descriptor
    }
    /// Admits and owns all transport/callback state, then atomically publishes
    /// validated typed outputs. This is the exact evaluator used by JitArtifact.
    pub fn evaluate_into(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<NativeExecutionReport, NativeExecutionError> {
        super::runtime::execute(
            &self.descriptor,
            |frame| {
                // SAFETY: the unsafe constructor established this exact
                // build-owned entry; execute retains the private frame and
                // admitted transport for the complete synchronous invocation.
                unsafe { (self.entry)(frame.cast::<c_void>()) }
            },
            inputs,
            outputs,
            limits,
            budget,
        )
    }
}
