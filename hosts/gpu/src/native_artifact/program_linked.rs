//! Build-owned ABI5 binding. JIT and linked calls use one invocation boundary.
use super::{
    NATIVE_PROGRAM_LINK_IDENTITY_BYTES, NativeLinkError, NativeProgramDescriptor,
    NativeProgramExecutionError, NativeProgramExecutionLimits, NativeProgramExecutionReport,
    NativeProgramObjectIdentity,
};
use mech_core::{FunctionCatalog, ManagedMemoryBudget, Value};
use mech_engine::{ProgramArtifact, native_artifact::lower_native_program_artifact};
use std::ffi::c_void;
pub struct LinkedProgramArtifact {
    entry: unsafe extern "C" fn(*mut c_void) -> u64,
    descriptor: NativeProgramDescriptor,
}
impl LinkedProgramArtifact {
    /// # Safety
    /// The entry must be the process-lifetime ABI5 entry emitted by this crate's
    /// checked shared CFG definition, and identity its readonly symbol in the
    /// SAME unchanged Object. The build must verify source/Object/bytecode pins
    /// before linking. No retained pointers, unwinding, ambient effects, foreign
    /// callbacks or access outside admitted transport are allowed. Matching
    /// metadata alone cannot authenticate arbitrary code or satisfy this duty.
    pub unsafe fn from_build_owned_entry(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        entry: unsafe extern "C" fn(*mut c_void) -> u64,
    ) -> Result<Self, NativeLinkError> {
        let get = |at: usize| {
            u64::from_le_bytes(
                identity[at..at + 8]
                    .try_into()
                    .expect("fixed identity field"),
            )
        };
        let stack = get(112);
        let code = get(120);
        if stack == 0 || code == 0 {
            return Err(NativeLinkError::new(
                "program Object lacks checked stack/code metadata",
            ));
        }
        let plan = lower_native_program_artifact(artifact, catalog).map_err(|_| {
            NativeLinkError::new("program Object has no current canonical selection")
        })?;
        let descriptor = NativeProgramDescriptor::prepare(artifact, catalog, plan, stack, code)
            .map_err(|_| NativeLinkError::new("program Object selection could not be prepared"))?;
        let expected = NativeProgramObjectIdentity::from_descriptor(&descriptor)
            .map_err(NativeLinkError::new)?;
        if identity != expected.bytes() {
            return Err(NativeLinkError::new(
                "program Object identity/ABI/work/target/typed CFG differs from selected artifact",
            ));
        }
        Ok(Self { entry, descriptor })
    }
    pub fn descriptor(&self) -> &NativeProgramDescriptor {
        &self.descriptor
    }
    pub fn evaluate_into(
        &self,
        inputs: &[Value],
        outputs: &mut [Option<Value>],
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
    ) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
        super::program_runtime::execute(
            &self.descriptor,
            |frame| {
                // SAFETY: the constructor's build provenance binds this retained
                // process entry; execute owns all disjoint backing until return.
                unsafe { (self.entry)(frame.cast::<c_void>()) }
            },
            inputs,
            outputs,
            limits,
            memory,
        )
    }
}
