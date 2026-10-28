//! ABI16 facade over the SAME canonical iteration CFG definition pass.
use crate::native_artifact::codegen::backend as codegen_backend;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::{backend, host_supported};
use crate::native_artifact::program::image::NativeCodeFootprint;
use crate::native_artifact::program::prepare::PreparedNativeProgram;
use crate::native_artifact::program::protocol::abi::{ENTRY, Entry, VERSION};
use crate::native_artifact::{
    NativeCompileError, NativeCompileLimits, NativeProgramDescriptor, NativeProgramExecutionError,
    NativeProgramExecutionLimits, NativeProgramExecutionReport, NativeProgramObjectIdentity,
};
use cranelift_module::{DataDescription, Linkage, Module};
use mech_core::{FunctionCatalog, ManagedMemoryBudget, Value};
use mech_engine::ProgramArtifact;
use mech_engine::native_artifact::lower_native_program_artifact_with_analysis_limit;

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

pub struct JitProgramArtifact {
    _module: cranelift_jit::JITModule,
    entry: Entry,
    descriptor: NativeProgramDescriptor,
    #[cfg(test)]
    codegen: crate::native_artifact::codegen::control::CodegenReport,
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
    ) -> Result<
        crate::native_artifact::NativeProgramExecutionSession<'a>,
        NativeProgramExecutionError,
    > {
        crate::native_artifact::program::runtime::prepare_hosted(
            &self.descriptor,
            crate::native_artifact::program::runtime::CheckedEntry::Jit(self.entry),
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
        Self::compile_with_options(
            artifact,
            catalog,
            limits,
            &crate::native_artifact::codegen::options::OptimizationOptions::default(),
        )
    }
    fn compile_with_options(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        limits: NativeCompileLimits,
        options: &crate::native_artifact::codegen::options::OptimizationOptions,
    ) -> Result<Self, NativeCompileError> {
        host_supported()?;
        let plan = lower_native_program_artifact_with_analysis_limit(
            artifact,
            catalog,
            limits.max_analysis_bytes,
        )
        .map_err(backend)?;
        let mut module = codegen_backend::jit(PROGRAM_CODEGEN_FLAGS)?;
        preflight(&plan, &module, limits)?;
        let prepared = PreparedNativeProgram::prepare(artifact, catalog, plan).map_err(backend)?;
        let (function, stack, code, _codegen) =
            crate::native_artifact::codegen::control::define_cfg(
                CodePlan::Program(&prepared),
                ENTRY,
                VERSION,
                &mut module,
                limits,
                options,
            )?;
        let descriptor = NativeProgramDescriptor::from_prepared(
            prepared,
            NativeCodeFootprint {
                stack_bytes: stack,
                code_bytes: code,
            },
        );
        module.finalize_definitions().map_err(backend)?;
        // SAFETY: define_cfg emits this module's current private entry signature, and
        // this owner retains the module for every synchronous invocation.
        let entry = unsafe {
            std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(function))
        };
        Ok(Self {
            _module: module,
            entry,
            descriptor,
            #[cfg(test)]
            codegen: _codegen,
        })
    }
    #[cfg(test)]
    pub(in crate::native_artifact) fn compile_for_test(
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        pass: crate::native_artifact::codegen::options::OptimizationPass,
    ) -> Result<Self, NativeCompileError> {
        Self::compile_with_options(
            artifact,
            catalog,
            NativeCompileLimits::default(),
            &crate::native_artifact::codegen::options::OptimizationOptions::without(pass),
        )
    }
    #[cfg(test)]
    pub(in crate::native_artifact) fn test_emissions(
        &self,
    ) -> crate::native_artifact::codegen::options::EmissionCounters {
        self.codegen.emission_counts()
    }
    pub fn descriptor(&self) -> &NativeProgramDescriptor {
        &self.descriptor
    }
    /// Tests may transparently observe the actual generated helper protocol;
    /// this is not another evaluator and never exists in production builds.
    #[cfg(test)]
    pub(in crate::native_artifact) fn test_checked_entry(&self) -> Entry {
        self.entry
    }
    #[cfg(test)]
    pub(in crate::native_artifact) fn test_generated_entry(
        &self,
    ) -> impl FnMut(*mut crate::native_artifact::program::protocol::abi::Frame) -> u64 + '_ {
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
        crate::native_artifact::program::runtime::execute(
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
    codegen: crate::native_artifact::codegen::control::CodegenReport,
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
            "work_policy_version": crate::native_artifact::program::image::identity::NATIVE_PROGRAM_WORK_POLICY_VERSION,
            "typed_ir_version": crate::native_artifact::program::image::identity::NATIVE_PROGRAM_TYPED_IR_VERSION,
            "requirements": self.descriptor.plan().resource_requirements().len(),
            "observations": self.descriptor.plan().observations().len(),
            "effects": self.descriptor.plan().effects().len(),
            "transport_inputs": self.descriptor.plan().transport_input_count(),
            "transport_outputs": self.descriptor.plan().transport_output_count(),
            "typed_ir_sha256": crate::native_artifact::identity::hex(self.descriptor.typed_ir_sha256()),
            "identity_symbol": self.identity.symbol(),
            "target_tag": crate::native_artifact::identity::host_target_tag(),
            "program_revision": crate::native_artifact::identity::hex(self.descriptor.plan().program_revision().as_bytes()),
            "stack_bytes": self.descriptor.stack_bytes(),
            "code_bytes": self.descriptor.code_bytes(),
            "object_bytes": self.bytes.len(),
            "object_optimization": "speed",
            "compiled_window_accumulator": (!self.descriptor.metadata().window_regions.is_empty()).then_some("hashbrown-0.15.5-entry-updates"),
            "compiled_window_query_regions": self.descriptor.metadata().window_regions.iter().filter(|region| region.query().is_some()).count(),
            "compiled_window_histogram_regions": self.descriptor.metadata().window_regions.iter().filter(|region| region.query().is_none()).count(),
            "compiled_string_map_regions": self.descriptor.metadata().string_map_regions.len(),
            "unboxed_scalar_aggregate_values": self.descriptor.metadata().scalar_aggregates.value_count(),
            "borrowed_string_regions": self.descriptor.metadata().borrowed_strings.regions.len(),
            "closed_tail_regions": self.descriptor.metadata().closed_tails.regions.len(),
            "private_frozen_map_candidates": self.descriptor.metadata().frozen_maps.regions.len(),
            "retained_map_scalar_admission_groups": self.descriptor.metadata().map_admission.groups.len(),
            "selected_matrix_join_regions": self.descriptor.metadata().selected_joins.regions.len(),
            "private_line_matrix_regions": self.descriptor.metadata().line_matrices.regions.len(),
            "compiled_header_continuation_regions": usize::from(self.descriptor.metadata().prefix.is_some()),
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
    emit_program_object_with_options(
        artifact,
        catalog,
        limits,
        &crate::native_artifact::codegen::options::OptimizationOptions::default(),
    )
}
pub(in crate::native_artifact) fn emit_program_object_with_options(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    limits: NativeCompileLimits,
    options: &crate::native_artifact::codegen::options::OptimizationOptions,
) -> Result<NativeProgramObject, NativeCompileError> {
    host_supported()?;
    let plan = lower_native_program_artifact_with_analysis_limit(
        artifact,
        catalog,
        limits.max_analysis_bytes,
    )
    .map_err(backend)?;
    let mut module = codegen_backend::object("mech_canonical_program", PROGRAM_CODEGEN_FLAGS)?;
    preflight(&plan, &module, limits)?;
    let prepared = PreparedNativeProgram::prepare(artifact, catalog, plan).map_err(backend)?;
    let (_, stack, code, codegen) = crate::native_artifact::codegen::control::define_cfg(
        CodePlan::Program(&prepared),
        ENTRY,
        VERSION,
        &mut module,
        limits,
        options,
    )?;
    let descriptor = NativeProgramDescriptor::from_prepared(
        prepared,
        NativeCodeFootprint {
            stack_bytes: stack,
            code_bytes: code,
        },
    );
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

fn preflight<M: Module>(
    plan: &mech_engine::native_artifact::NativeProgramArtifactPlan,
    module: &M,
    limits: NativeCompileLimits,
) -> Result<(), NativeCompileError> {
    if module.target_config().pointer_type() != cranelift_codegen::ir::types::I64 {
        return Err(NativeCompileError::UnsupportedHost);
    }
    crate::native_artifact::codegen::control::preflight_counts(
        plan.blocks().len(),
        plan.transport_input_count(),
        plan.transport_output_count(),
        plan.values().len(),
        limits,
    )
}

#[cfg(test)]
pub(in crate::native_artifact) fn emit_program_object_for_test(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    pass: crate::native_artifact::codegen::options::OptimizationPass,
) -> Result<NativeProgramObject, NativeCompileError> {
    emit_program_object_with_options(
        artifact,
        catalog,
        NativeCompileLimits::default(),
        &crate::native_artifact::codegen::options::OptimizationOptions::without(pass),
    )
}
#[cfg(test)]
impl NativeProgramObject {
    pub(in crate::native_artifact) fn test_emissions(
        &self,
    ) -> crate::native_artifact::codegen::options::EmissionCounters {
        self.codegen.emission_counts()
    }
}
