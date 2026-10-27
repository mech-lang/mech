//! ABI16 facade over the SAME canonical iteration CFG definition pass.
use super::compiler::{backend, host_supported};
use super::iteration_code_plan::CodePlan;
use super::program_abi::{ENTRY, Entry, VERSION};
use super::{
    NativeCompileError, NativeCompileLimits, NativeProgramDescriptor, NativeProgramExecutionError,
    NativeProgramExecutionLimits, NativeProgramExecutionReport, NativeProgramObjectIdentity,
};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_module::{DataDescription, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::{FunctionCatalog, ManagedMemoryBudget, Value};
use mech_engine::{ProgramArtifact, native_artifact::lower_native_program_artifact};

// Keep generated function placement independent of unrelated host link-size
// changes on AArch64. 128-byte alignment covers both 64- and 128-byte cache
// lines; other architectures retain Cranelift's normal alignment policy.
const PROGRAM_CODEGEN_FLAGS: &[(&str, &str)] = &[
    ("opt_level", "speed"),
    ("is_pic", "true"),
    ("preserve_frame_pointers", "true"),
    ("unwind_info", "true"),
    (
        "log2_min_function_alignment",
        if cfg!(target_arch = "aarch64") {
            "7"
        } else {
            "0"
        },
    ),
];

#[path = "program_ascii_map_codegen.rs"]
pub(super) mod ascii_map_codegen;
#[path = "program_borrowed_codegen.rs"]
pub(super) mod borrowed_codegen;
#[path = "program_cfg_cleanup.rs"]
pub(super) mod cfg_cleanup;
#[path = "program_join_map_codegen.rs"]
pub(super) mod join_map_codegen;
#[path = "program_map_owner_codegen.rs"]
pub(super) mod map_owner_codegen;
#[path = "program_packed_loop_codegen.rs"]
pub(super) mod packed_loop_codegen;
#[path = "program_packed_map_codegen.rs"]
pub(super) mod packed_maps;
#[path = "program_parallel_codegen.rs"]
pub(super) mod parallel_codegen;
#[path = "program_prefix_codegen.rs"]
pub(super) mod prefix_codegen;
#[path = "program_scalar_aggregates.rs"]
pub(super) mod scalar_aggregates;
#[path = "program_scalar_loop_codegen.rs"]
pub(super) mod scalar_loop_codegen;
#[path = "program_ssa_words.rs"]
pub(super) mod ssa_words;
#[path = "program_string_map_codegen.rs"]
pub(super) mod string_map_codegen;
#[path = "program_tail_codegen.rs"]
pub(super) mod tail_codegen;
#[path = "program_window_codegen.rs"]
pub(super) mod window_codegen;

pub struct JitProgramArtifact {
    _module: cranelift_jit::JITModule,
    entry: Entry,
    descriptor: NativeProgramDescriptor,
}
impl JitProgramArtifact {
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
            super::program_runtime::CheckedEntry::Jit(self.entry),
            admission,
            limits,
            memory,
        )
    }
    pub fn compile(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
    ) -> Result<Self, NativeCompileError> {
        Self::compile_with_limits(artifact, catalog, NativeCompileLimits::default())
    }
    pub fn compile_with_limits(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        limits: NativeCompileLimits,
    ) -> Result<Self, NativeCompileError> {
        host_supported()?;
        let plan = lower_native_program_artifact(artifact, catalog).map_err(backend)?;
        let builder =
            cranelift_jit::JITBuilder::with_flags(PROGRAM_CODEGEN_FLAGS, default_libcall_names())
                .map_err(backend)?;
        let mut module = cranelift_jit::JITModule::new(builder);
        let (function, stack, code, _) = super::iteration_compiler::define_cfg(
            CodePlan::Program(&plan),
            ENTRY,
            VERSION,
            &mut module,
            limits,
        )?;
        let descriptor = NativeProgramDescriptor::prepare(artifact, catalog, plan, stack, code)
            .map_err(backend)?;
        module.finalize_definitions().map_err(backend)?;
        // SAFETY: define_cfg emits exactly the private ABI16 signature, and
        // this owner retains the module for every synchronous invocation.
        let entry = unsafe {
            std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(function))
        };
        Ok(Self {
            _module: module,
            entry,
            descriptor,
        })
    }
    pub fn descriptor(&self) -> &NativeProgramDescriptor {
        &self.descriptor
    }
    /// Tests may transparently observe the actual generated helper protocol;
    /// this is not another evaluator and never exists in production builds.
    #[cfg(test)]
    pub(super) fn test_checked_entry(&self) -> Entry {
        self.entry
    }
    #[cfg(test)]
    pub(super) fn test_generated_entry(
        &self,
    ) -> impl FnMut(*mut super::program_abi::Frame) -> u64 + '_ {
        // SAFETY: the test's Prepared session owns the exact admitted frame,
        // and this borrow retains the checked JIT owner across every entry.
        |frame| unsafe { (self.entry)(frame) }
    }
    /// Evaluate one fresh, stateless invocation: closed pure activation DAG
    /// followed by one turn. No activation cache, state or suspended instance
    /// survives a call. Both phases share the same limits and atomic outputs.
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
                // SAFETY: this owner retains the exact checked ABI16 entry.
                unsafe { (self.entry)(frame) }
            },
            inputs,
            outputs,
            limits,
            memory,
        )
    }
}

pub struct NativeProgramObject {
    bytes: Vec<u8>,
    descriptor: NativeProgramDescriptor,
    identity: NativeProgramObjectIdentity,
    codegen: super::iteration_compiler::CodegenReport,
}
impl NativeProgramObject {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn descriptor(&self) -> &NativeProgramDescriptor {
        &self.descriptor
    }
    pub fn identity(&self) -> &NativeProgramObjectIdentity {
        &self.identity
    }
    pub fn metadata(&self) -> serde_json::Value {
        let mut metadata = serde_json::json!({
            "schema": "mech.canonical-managed-program-object.v1",
            "abi_version": VERSION,
            "entry_symbol": ENTRY,
            "work_policy_version": super::program_identity::NATIVE_PROGRAM_WORK_POLICY_VERSION,
            "typed_ir_version": super::program_identity::NATIVE_PROGRAM_TYPED_IR_VERSION,
            "requirements": self.descriptor.plan().resource_requirements().len(),
            "observations": self.descriptor.plan().observations().len(),
            "effects": self.descriptor.plan().effects().len(),
            "transport_inputs": self.descriptor.plan().transport_input_count(),
            "transport_outputs": self.descriptor.plan().transport_output_count(),
            "typed_ir_sha256": super::identity::hex(self.descriptor.typed_ir_sha256()),
            "identity_symbol": self.identity.symbol(),
            "target_tag": super::identity::host_target_tag(),
            "program_revision": super::identity::hex(self.descriptor.plan().program_revision().as_bytes()),
            "stack_bytes": self.descriptor.stack_bytes(),
            "code_bytes": self.descriptor.code_bytes(),
            "object_bytes": self.bytes.len(),
            "object_optimization": "speed",
            "compiled_window_accumulator": (!self.descriptor.window_regions.is_empty()).then_some("hashbrown-0.15.5-entry-updates"),
            "compiled_window_query_regions": self.descriptor.window_regions.iter().filter(|region| region.query().is_some()).count(),
            "compiled_window_histogram_regions": self.descriptor.window_regions.iter().filter(|region| region.query().is_none()).count(),
            "compiled_string_map_regions": self.descriptor.string_map_regions.len(),
            "unboxed_scalar_aggregate_values": self.descriptor.scalar_aggregates.value_count(),
            "borrowed_string_regions": self.descriptor.borrowed_strings.regions.len(),
            "closed_tail_regions": self.descriptor.closed_tails.regions.len(),
            "private_frozen_map_candidates": self.descriptor.frozen_maps.regions.len(),
            "retained_map_scalar_admission_groups": self.descriptor.map_admission.groups.len(),
            "selected_matrix_join_regions": self.descriptor.selected_joins.regions.len(),
            "private_line_matrix_regions": self.descriptor.line_matrices.regions.len(),
            "compiled_header_continuation_regions": usize::from(self.descriptor.prefix.is_some()),
            "compiled_source_edge_facts": &self.codegen.source_edge_facts,
            "compiled_tag_threading": &self.codegen.tag_threading,
            "compiled_frame_effects": &self.codegen.frame_effects,
            "compiled_guarded_loops": &self.codegen.guarded_loops,
            "compiled_lookup_facts": &self.codegen.lookup_facts,
            "compiled_cfg_cleanup": &self.codegen.cfg_cleanup,
            "compiled_outlined_loops": &self.codegen.outlined_loops,
            "compiled_loop_invariants": &self.codegen.loop_invariants,
            "linked": false, "executed": false, "timing_samples": 0,
        });
        metadata["compiled_bound_call_shim"] = serde_json::json!(&self.codegen.bound_call_shim);
        metadata["compiled_preserving_call_regions"] =
            serde_json::json!(&self.codegen.preserving_call_regions);
        metadata["compiled_mutation_loops"] = serde_json::json!(&self.codegen.mutation_loops);
        metadata["compiled_count_value_loops"] = serde_json::json!(&self.codegen.count_value_loops);
        metadata["compiled_loop_constants"] = serde_json::json!(&self.codegen.loop_constants);
        metadata["compiled_steady_loops"] = serde_json::json!(&self.codegen.steady_loops);
        metadata["compiled_join_map_copies"] = serde_json::json!(&self.codegen.join_map_copies);
        metadata["compiled_scalar_loops"] = serde_json::json!(&self.codegen.scalar_loops);
        metadata
    }
}
pub fn emit_program_object(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<NativeProgramObject, NativeCompileError> {
    emit_program_object_with_limits(artifact, catalog, NativeCompileLimits::default())
}
pub fn emit_program_object_with_limits(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    limits: NativeCompileLimits,
) -> Result<NativeProgramObject, NativeCompileError> {
    host_supported()?;
    let plan = lower_native_program_artifact(artifact, catalog).map_err(backend)?;
    let mut flags = settings::builder();
    for &(key, value) in PROGRAM_CODEGEN_FLAGS {
        flags.set(key, value).map_err(backend)?;
    }
    let isa = cranelift_native::builder()
        .map_err(backend)?
        .finish(settings::Flags::new(flags))
        .map_err(backend)?;
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "mech_canonical_program", default_libcall_names())
            .map_err(backend)?,
    );
    let (_, stack, code, codegen) = super::iteration_compiler::define_cfg(
        CodePlan::Program(&plan),
        ENTRY,
        VERSION,
        &mut module,
        limits,
    )?;
    let descriptor =
        NativeProgramDescriptor::prepare(artifact, catalog, plan, stack, code).map_err(backend)?;
    let identity = NativeProgramObjectIdentity::from_descriptor(&descriptor).map_err(backend)?;
    let id = module
        .declare_data(identity.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.define(identity.bytes().to_vec().into_boxed_slice());
    data.set_align(8);
    module.define_data(id, &data).map_err(backend)?;
    Ok(NativeProgramObject {
        bytes: module.finish().emit().map_err(backend)?,
        descriptor,
        identity,
        codegen,
    })
}
