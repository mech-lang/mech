//! Build-owned ABI14 binding. JIT and linked calls use one invocation boundary.
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
    /// Execute only generated closed activation before provider capture.
    /// The opaque result borrows this checked code owner and can resume once,
    /// with the SAME runtime transaction, limits, arena and cumulative ledger.
    #[cfg(feature = "canonical-native-program-host-runtime")]
    pub fn prepare_hosted<'a>(
        &'a self,
        admission: mech_runtime::CanonicalNativeResourceAdmission<'_>,
        limits: NativeProgramExecutionLimits,
        memory: &ManagedMemoryBudget,
    ) -> Result<super::NativeProgramExecutionSession<'a>, NativeProgramExecutionError> {
        super::program_runtime::prepare_hosted(
            &self.descriptor,
            super::program_runtime::CheckedEntry::Linked(self.entry),
            admission,
            limits,
            memory,
        )
    }
    /// # Safety
    /// The entry must be the process-lifetime ABI14 entry emitted by this crate's
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
        let descriptor = prepare_descriptor(artifact, catalog, identity)?;
        Ok(Self { entry, descriptor })
    }
    /// Bind metadata already prepared by the compiler, without repeating its
    /// lowering or proof-selection passes inside the application executable.
    ///
    /// # Safety
    /// All requirements of `from_build_owned_entry` apply. In addition, `image`
    /// must be the unchanged descriptor emitted for that SAME checked Object;
    /// the build must pin the descriptor alongside source, bytecode and Object.
    /// Neither a caller-supplied image nor a matching identity authenticates
    /// arbitrary code or compiler proofs. Only the real maintained factories
    /// from that build may populate `catalog`. Runtime admission is unchanged.
    pub unsafe fn from_build_owned_image(
        image: &[u8],
        catalog: &FunctionCatalog,
        identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
        entry: unsafe extern "C" fn(*mut c_void) -> u64,
    ) -> Result<Self, NativeLinkError> {
        // SAFETY: delegated unchanged compiler/Object/image provenance.
        let descriptor =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(image, catalog) }.map_err(
                |_| NativeLinkError::new("build-owned program descriptor could not be restored"),
            )?;
        check_identity(&descriptor, identity)?;
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

// Metadata preparation alone grants no entry/code authority. The unsafe
// constructor still requires build-owned provenance for the exact Object.
// In particular, nonzero stack/code claims come from that pinned Object;
// runtime-only preparation cannot independently rediscover machine code size.
pub(super) fn prepare_descriptor(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    identity: &[u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
) -> Result<NativeProgramDescriptor, NativeLinkError> {
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
    let plan = lower_native_program_artifact(artifact, catalog)
        .map_err(|_| NativeLinkError::new("program Object has no current canonical selection"))?;
    let descriptor = NativeProgramDescriptor::prepare(artifact, catalog, plan, stack, code)
        .map_err(|_| NativeLinkError::new("program Object selection could not be prepared"))?;
    check_identity(&descriptor, identity)?;
    Ok(descriptor)
}

fn check_identity(
    descriptor: &NativeProgramDescriptor,
    identity: &[u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
) -> Result<(), NativeLinkError> {
    let expected =
        NativeProgramObjectIdentity::from_descriptor(descriptor).map_err(NativeLinkError::new)?;
    if identity != expected.bytes() {
        return Err(NativeLinkError::new(
            "program Object identity/ABI/work/target/typed CFG differs from selected artifact",
        ));
    }
    Ok(())
}
