//! Build-owned canonical Iteration entry, using the same managed evaluator as JIT.
use super::iteration_abi::NativeIterationDescriptor;
use super::iteration_identity::{NATIVE_ITERATION_LINK_IDENTITY_BYTES, descriptor_from_identity};
use super::{
    NativeExecutionLimits, NativeIterationExecutionError, NativeIterationExecutionReport,
    NativeLinkError,
};
use core::ffi::c_void;
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{NativeIterationArtifactPlan, NativeScalarValue};

/// A statically linked checked Iteration entry. Its raw pointer is private.
pub struct LinkedIterationArtifact {
    entry: unsafe extern "C" fn(*mut c_void) -> u64,
    descriptor: NativeIterationDescriptor,
}
impl LinkedIterationArtifact {
    /// Binds metadata to a freshly derived canonical private-witness plan.
    ///
    /// # Safety
    /// `entry` must be the process-lifetime control entry emitted by this crate's
    /// checked shared JIT/Object definition, and `identity` must be its readonly
    /// exported identity symbol from the SAME exact unchanged Object. The build
    /// must verify Object, bytecode and generated source pins before linking.
    /// The emitted entry obeys the private synchronous Iteration ABI/work policy:
    /// no retained pointers, unwinding, ambient effects, foreign callbacks or
    /// access outside admitted transport. Fabricating matching record bytes or
    /// supplying an arbitrary foreign function does not satisfy this contract.
    pub unsafe fn from_build_owned_entry(
        plan: &NativeIterationArtifactPlan,
        identity: &'static [u8; NATIVE_ITERATION_LINK_IDENTITY_BYTES],
        entry: unsafe extern "C" fn(*mut c_void) -> u64,
    ) -> Result<Self, NativeLinkError> {
        let descriptor = descriptor_from_identity(plan, identity).map_err(NativeLinkError::new)?;
        Ok(Self { entry, descriptor })
    }
    pub fn descriptor(&self) -> &NativeIterationDescriptor {
        &self.descriptor
    }
    pub fn evaluate_into(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<NativeIterationExecutionReport, NativeIterationExecutionError> {
        super::iteration_runtime::execute(
            &self.descriptor,
            |frame| {
                // SAFETY: constructor proves the same exact checked entry;
                // execute owns frame/transport for this synchronous invocation.
                unsafe { (self.entry)(frame.cast::<c_void>()) }
            },
            inputs,
            outputs,
            limits,
            budget,
        )
    }
}
