//! ABI16 facade over the SAME canonical iteration CFG definition pass.
use super::compilation_report::{
    Measurements, NativeCompilationReport, NativeCompilerConfiguration,
};
use crate::native_artifact::codegen::backend as codegen_backend;
use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
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
use mech_engine::native_artifact::lower_native_program_artifact_with_definition_table_limit;

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
        let inputs = CompilationInputs::legacy();
        let plan = lower_native_program_artifact_with_definition_table_limit(
            artifact,
            catalog,
            limits.max_definition_table_bytes,
        )
        .map_err(backend)?;
        let mut module = codegen_backend::jit_with_inputs(PROGRAM_CODEGEN_FLAGS, &inputs)?;
        preflight(&plan, &module, limits)?;
        let prepared = PreparedNativeProgram::prepare_with_inputs(artifact, catalog, plan, &inputs)
            .map_err(backend)?;
        let (function, stack, code, _codegen) =
            crate::native_artifact::codegen::control::define_cfg(
                CodePlan::Program(&prepared),
                ENTRY,
                VERSION,
                &mut module,
                limits,
                options,
                &inputs,
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
    image_binding: super::image::binding::NativeProgramImageBinding,
    native_metadata: Vec<u8>,
    native_metadata_symbol: String,
    compiled_module_symbol: String,
    native_metadata_binding: super::image::binding::NativeProgramImageBinding,
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
    pub fn image_binding(&self) -> &super::image::binding::NativeProgramImageBinding {
        &self.image_binding
    }
    /// Compiler-resolved typed values, also exported as read-only Object data.
    /// These bytes contain no pointers, owner epochs or invocation caches.
    pub fn native_metadata(&self) -> &[u8] {
        &self.native_metadata
    }
    pub fn compiled_module_symbol(&self) -> &str {
        &self.compiled_module_symbol
    }
    pub fn native_metadata_symbol(&self) -> &str {
        &self.native_metadata_symbol
    }
    pub fn native_metadata_binding(&self) -> &super::image::binding::NativeProgramImageBinding {
        &self.native_metadata_binding
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
            "retained_map_scalar_admission_groups": self.descriptor.metadata().map_admission.groups.iter().filter(|group| group.owner.is_some()).count(),
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
        metadata["compiled_numeric_outlines"] = serde_json::json!(&self.codegen.numeric_outlines);
        if let Some(histogram) = &self.codegen.utf8_histogram {
            metadata["compiled_utf8_histogram"] = serde_json::json!(histogram);
        }
        metadata["compiled_utf8_word_chunk_sites"] =
            serde_json::json!(&self.codegen.utf8_word_chunk_sites);
        metadata["compiled_numeric_functions"] = serde_json::json!(&self.codegen.numeric_functions);
        metadata["compiled_scalar_selects"] = serde_json::json!(self.codegen.scalar_selects);
        metadata["compiled_unsigned_guards"] = serde_json::json!(self.codegen.unsigned_guards);
        metadata["compiled_borrowed_record_maps"] =
            serde_json::json!(&self.descriptor.metadata().borrowed_record_maps.regions);
        metadata["image_binding_symbol"] = serde_json::json!(self.image_binding.symbol());
        metadata["native_metadata_symbol"] = serde_json::json!(self.native_metadata_symbol);
        metadata["compiled_module_symbol"] = serde_json::json!(self.compiled_module_symbol);
        metadata["compiled_module_version"] = serde_json::json!(1);
        metadata["native_metadata_bytes"] = serde_json::json!(self.native_metadata.len());
        metadata["native_metadata_binding_symbol"] =
            serde_json::json!(self.native_metadata_binding.symbol());
        metadata["native_metadata_format"] = serde_json::json!("mech.typed-build-image.v4");
        metadata["native_descriptor_version"] = serde_json::json!(11);
        metadata["ordinary_scalar_admission_groups"] = serde_json::json!(
            self.descriptor
                .metadata()
                .map_admission
                .groups
                .iter()
                .filter(|group| group.owner.is_none())
                .count()
        );
        metadata["ordinary_scalar_admission_sites"] = serde_json::json!(
            self.descriptor
                .metadata()
                .map_admission
                .groups
                .iter()
                .filter(|group| group.owner.is_none())
                .map(|group| group.sites.len())
                .sum::<usize>()
        );
        metadata["compiled_bound_call_shim"] = serde_json::json!(&self.codegen.bound_call_shim);
        metadata["compiled_preserving_call_regions"] =
            serde_json::json!(&self.codegen.preserving_call_regions);
        metadata["compiled_mutation_loops"] = serde_json::json!(&self.codegen.mutation_loops);
        metadata["compiled_count_value_loops"] = serde_json::json!(&self.codegen.count_value_loops);
        metadata["compiled_loop_constants"] = serde_json::json!(&self.codegen.loop_constants);
        metadata["compiled_steady_loops"] = serde_json::json!(&self.codegen.steady_loops);
        metadata["compiled_numeric_map_regions"] =
            serde_json::to_value(&self.descriptor.metadata().numeric_maps.regions).unwrap();
        metadata["compiled_join_map_copies"] = serde_json::json!(&self.codegen.join_map_copies);
        metadata["readonly_numeric_regions"] = serde_json::json!(self.descriptor.metadata().closed_tails.regions.iter().filter_map(|r| r.numeric.as_ref().filter(|p| !p.writes()).map(|p| serde_json::json!({"iteration":r.iteration,"parent":r.tail.parent(),"header":r.tail.header(),"compute_per_traversal":r.compute,"proof":p}))).collect::<Vec<_>>());
        metadata["numeric_write_regions"] = serde_json::json!(self.descriptor.metadata().closed_tails.regions.iter().filter_map(|r| r.numeric.as_ref().filter(|p| p.writes()).map(|p| serde_json::json!({"iteration":r.iteration,"parent":r.tail.parent(),"header":r.tail.header(),"compute_per_traversal":r.compute,"proof":p}))).collect::<Vec<_>>());
        let prepared = self.descriptor.metadata();
        metadata["numeric_generator_candidates"] = serde_json::json!({
            "scope": "Source-closed ordinary generator entry; current finite admission required",
            "selected": !prepared.numeric_generators.regions.is_empty(),
            "inventory": &prepared.numeric_generators,
        });
        metadata["readonly_numeric_candidates"] = serde_json::json!(prepared.plan.iterations().iter().enumerate().filter_map(|(iteration, control)| {
            let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control else { return None };
            if prepared.buffer_frames.fields(tail.carrier()).is_none() { return None }
            let result = crate::native_artifact::program::prepare::readonly_numeric::derive(&prepared.plan, tail, &prepared.buffer_frames, &prepared.scalar_aggregates, &prepared.calls, &prepared.visits);
            Some(serde_json::json!({"iteration":iteration,"header":tail.header(),"parent":tail.parent(),"origin":tail.origin(),"proof_eligible":result.is_ok(),"selected":prepared.closed_tails.regions.iter().any(|region| region.iteration as usize == iteration && region.numeric.is_some()),"rejection":result.err()}))
        }).collect::<Vec<_>>());
        metadata["captured_numeric_candidates"] = serde_json::json!(prepared.plan.iterations().iter().enumerate().filter_map(|(iteration, control)| {
            let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control else { return None };
            let result = crate::native_artifact::program::prepare::captured_numeric::derive(&prepared.plan, tail, &prepared.calls);
            Some(serde_json::json!({"iteration":iteration,"header":tail.header(),"parent":tail.parent(),"scalar_carrier_words":prepared.scalar_aggregates.slot(tail.carrier()),"native_initializer":prepared.scalar_aggregates.action(tail.initialize() as usize).is_some(),"proof_eligible":result.is_ok(),"selected":prepared.closed_tails.regions.iter().any(|region| region.iteration as usize == iteration && region.captures.is_some()),"rejection":result.err()}))
        }).collect::<Vec<_>>());
        let tree_discovery =
            crate::native_artifact::program::prepare::numeric_tree::Discovery::new(&prepared.plan);
        metadata["numeric_tree_candidates"] = serde_json::json!(prepared.plan.iterations().iter().enumerate().filter_map(|(iteration, control)| {
            let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control else { return None };
            let result = tree_discovery.as_ref().map_err(|error| *error).and_then(|discovery| discovery.discover(tail))
                .and_then(|tree| crate::native_artifact::program::prepare::readonly_numeric::derive_tree(&prepared.plan,tail,&prepared.buffer_frames,&prepared.scalar_aggregates,&prepared.calls,&tree).map(|_|tree.nodes.len()));
            Some(serde_json::json!({"iteration":iteration,"header":tail.header(),"parent":tail.parent(),"proof_eligible":result.is_ok(),"nodes":result.as_ref().ok(),"rejection":result.err(),"selected":prepared.closed_tails.regions.iter().any(|region| region.iteration as usize==iteration && region.numeric_tree.is_some())}))
        }).collect::<Vec<_>>());
        metadata["numeric_parent_partitions"] = serde_json::json!(prepared.compositions);
        metadata["compiled_numeric_segments"] = serde_json::json!(&self.codegen.numeric_segments);
        metadata["compiled_readonly_numeric_loops"] =
            serde_json::json!(&self.codegen.readonly_numeric_loops);
        metadata["compiled_numeric_write_loops"] =
            serde_json::json!(&self.codegen.numeric_write_loops);
        metadata["compiled_scalar_loops"] = serde_json::json!(&self.codegen.scalar_loops);
        metadata["compiled_captured_numeric_loops"] =
            serde_json::json!(&self.codegen.captured_numeric_loops);
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
    let inputs = CompilationInputs::legacy();
    let plan = lower_native_program_artifact_with_definition_table_limit(
        artifact,
        catalog,
        limits.max_definition_table_bytes,
    )
    .map_err(backend)?;
    let (mut module, _) =
        codegen_backend::object_with_isa("mech_canonical_program", PROGRAM_CODEGEN_FLAGS, &inputs)?;
    preflight(&plan, &module, limits)?;
    let prepared = PreparedNativeProgram::prepare_with_inputs(artifact, catalog, plan, &inputs)
        .map_err(backend)?;
    let (entry, stack, code, codegen) = crate::native_artifact::codegen::control::define_cfg(
        CodePlan::Program(&prepared),
        ENTRY,
        VERSION,
        &mut module,
        limits,
        options,
        &inputs,
    )?;
    finish_program_object(
        module,
        prepared,
        entry,
        NativeCodeFootprint {
            stack_bytes: stack,
            code_bytes: code,
        },
        codegen,
    )
}

fn finish_program_object(
    mut module: cranelift_object::ObjectModule,
    prepared: PreparedNativeProgram,
    entry: cranelift_module::FuncId,
    footprint: NativeCodeFootprint,
    codegen: crate::native_artifact::codegen::control::CodegenReport,
) -> Result<NativeProgramObject, NativeCompileError> {
    let descriptor = NativeProgramDescriptor::from_prepared(prepared, footprint);
    let identity = NativeProgramObjectIdentity::from_descriptor(&descriptor).map_err(backend)?;
    let identity_id = module
        .declare_data(identity.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.define(identity.bytes().to_vec().into_boxed_slice());
    data.set_align(8);
    module.define_data(identity_id, &data).map_err(backend)?;
    // All plan/layout proofs were established by PreparedNativeProgram before
    // emission. Bind their exact serialized bytes to this same checked Object;
    // no runtime-supplied digest can manufacture this read-only build symbol.
    let image = descriptor.encode_build_owned_image().map_err(backend)?;
    let image_binding = super::image::binding::NativeProgramImageBinding::new(&image, &identity);
    let binding_id = module
        .declare_data(image_binding.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut binding_data = DataDescription::new();
    binding_data.define(image_binding.bytes().to_vec().into_boxed_slice());
    binding_data.set_align(8);
    module
        .define_data(binding_id, &binding_data)
        .map_err(backend)?;
    // Compile textual tokens to typed immutable values once, before linking.
    // Both the data and its binding are genuine read-only exports from this
    // exact checked Object. The inspectable JSON binding above remains intact.
    let native_metadata = descriptor
        .encode_build_owned_table_image()
        .map_err(backend)?;
    let native_metadata_symbol = format!("{}_native_metadata_v1", identity.symbol());
    let native_metadata_binding =
        super::image::binding::NativeProgramImageBinding::new_native_metadata(
            &native_metadata,
            &identity,
        );
    let mut native_ids = Vec::with_capacity(2);
    for (symbol, bytes) in [
        (native_metadata_symbol.as_str(), native_metadata.as_slice()),
        (
            native_metadata_binding.symbol(),
            native_metadata_binding.bytes().as_slice(),
        ),
    ] {
        let id = module
            .declare_data(symbol, Linkage::Export, false, false)
            .map_err(backend)?;
        let mut data = DataDescription::new();
        data.define(bytes.to_vec().into_boxed_slice());
        data.set_align(8);
        module.define_data(id, &data).map_err(backend)?;
        native_ids.push(id);
    }
    let compiled_module_symbol = format!("{}_compiled_module_v1", identity.symbol());
    super::compiled_module::define(
        &mut module,
        &compiled_module_symbol,
        native_ids[0],
        native_metadata.len(),
        identity_id,
        native_ids[1],
        entry,
    )?;
    Ok(NativeProgramObject {
        bytes: module.finish().emit().map_err(backend)?,
        descriptor,
        identity,
        image_binding,
        native_metadata,
        native_metadata_symbol,
        compiled_module_symbol,
        native_metadata_binding,
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

/// Diagnostic compile-time inventory for compact closed-program emission.
pub fn compact_native_layout_for_diagnostic(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<serde_json::Value, NativeCompileError> {
    let compiler_inputs = CompilationInputs::legacy();
    let limits = crate::native_artifact::NativeProgramLimitMode::NoLimits.compilation();
    let plan = lower_native_program_artifact_with_definition_table_limit(
        artifact,
        catalog,
        limits.max_definition_table_bytes,
    )
    .map_err(backend)?;
    let mut inventory = crate::native_artifact::program::compact::inventory(&plan)?;
    if compiler_inputs
        .var_os("MECH_COMPACT_PROOF_INVENTORY")
        .is_some()
    {
        let prepared =
            PreparedNativeProgram::prepare_with_inputs(artifact, catalog, plan, &compiler_inputs)
                .map_err(backend)?;
        let meta = prepared.metadata();
        let complete_headers = compiler_inputs
            .var("MECH_COMPACT_COMPLETE_PROOF_HEADERS")
            .ok()
            .map(|text| {
                text.split(',')
                    .map(|v| v.parse::<u32>().map_err(backend))
                    .collect::<Result<std::collections::BTreeSet<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let rows = meta.plan.iterations().iter().enumerate().filter_map(|(iteration, control)| {
            let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control else { return None };
            let result = crate::native_artifact::program::prepare::readonly_numeric::derive(
                &meta.plan, tail, &meta.buffer_frames, &meta.scalar_aggregates, &meta.calls, &meta.visits);
            let complete_rejection = complete_headers.contains(&tail.header()).then(||
                crate::native_artifact::program::prepare::readonly_numeric::derive_complete_for_diagnostic(
                    &meta.plan, tail, &meta.buffer_frames, &meta.scalar_aggregates, &meta.calls, &meta.visits).err());
            Some(serde_json::json!({"complete_rejection":complete_rejection,"iteration":iteration,"header":tail.header(),"parent":tail.parent(),
                "origin":tail.origin(),"proof_eligible":result.is_ok(),"rejection":result.as_ref().err(),
                "selected_closed_region":meta.closed_tails.regions.iter().any(|v|v.iteration as usize==iteration && v.numeric.is_some()),
                "arrays":result.as_ref().ok().map(|p|p.arrays.len()),
                "native_sites":result.as_ref().ok().map(|p|p.sites.len()),
                "cold_exits":result.as_ref().ok().map(|p|p.exits.len()),
                "cold_exit_records":result.as_ref().ok().map(|p|&p.exits)}))
        }).collect::<Vec<_>>();
        inventory["numeric_candidates"] = serde_json::json!(rows);
    }
    Ok(inventory)
}

/// Diagnostic native closed computation. The ordinary managed entry never
/// selects this function until its full resource/recovery contract is restored.
pub fn emit_compact_closed_object_for_diagnostic(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
    emit_compact_closed_object_with_phase_observer_for_diagnostic(artifact, catalog, false)
}
pub fn emit_compact_closed_object_with_phase_observer_for_diagnostic(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    observe: bool,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
    emit_compact_with_inputs(
        artifact,
        catalog,
        observe,
        &CompilationInputs::legacy(),
        None,
    )
}

/// Compile the compact diagnostic with explicit immutable settings and optional host reporting.
#[doc(hidden)]
pub fn emit_compact_closed_object_with_configuration_for_diagnostic(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    configuration: &NativeCompilerConfiguration,
    observe: bool,
    report: Option<&mut NativeCompilationReport>,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
    emit_compact_with_inputs(artifact, catalog, observe, &configuration.0, report)
}

fn emit_compact_with_inputs(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    observe: bool,
    compiler_inputs: &CompilationInputs,
    report: Option<&mut NativeCompilationReport>,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
    host_supported()?;
    let mut measurements = Measurements(report);
    measurements.identify(
        || format!("{:?}", artifact.revision()),
        compiler_inputs,
        observe,
    );
    eprintln!("compact-stage: canonical lowering");
    let limits = crate::native_artifact::NativeProgramLimitMode::NoLimits.compilation();
    let plan = measurements
        .stage("native_plan", || {
            lower_native_program_artifact_with_definition_table_limit(
                artifact,
                catalog,
                limits.max_definition_table_bytes,
            )
        })
        .map_err(backend)?;
    eprintln!(
        "compact-stage: preparation blocks={} values={}",
        plan.blocks().len(),
        plan.values().len()
    );
    let prepared = measurements
        .stage("prepare", || {
            PreparedNativeProgram::prepare_with_inputs(artifact, catalog, plan, compiler_inputs)
        })
        .map_err(backend)?;
    eprintln!("compact-stage: native module");
    let mut flags = PROGRAM_CODEGEN_FLAGS.to_vec();
    if cfg!(target_arch = "aarch64")
        && !observe
        && let Some(mode) = compiler_inputs.var_os("MECH_COMPACT_PAIR_REGISTER_SPILLS")
    {
        flags.push(("enable_regalloc_spill_pairs", "true"));
        if mode == "checked" {
            flags.push(("regalloc_checker", "true"));
        }
    }
    if cfg!(target_arch = "aarch64")
        && !observe
        && let Some(mode) = compiler_inputs.var_os("MECH_COMPACT_FOLD_INVERTED_CSET")
    {
        flags.push(("enable_inverted_cset_folding", "true"));
        if mode == "branch" || mode == "select" {
            flags.push(("enable_cset_branch_flags", "true"));
        }
        if mode == "select" {
            flags.push(("enable_cset_bit_select", "true"));
        }
    }
    if cfg!(target_arch = "aarch64")
        && !observe
        && compiler_inputs
            .var_os("MECH_COMPACT_FOLD_REDUNDANT_ZERO")
            .is_some()
    {
        flags.push(("enable_redundant_zero_folding", "true"));
    }
    let alignment = compiler_inputs
        .var("MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2")
        .ok();
    if cfg!(target_arch = "aarch64")
        && !observe
        && let Some(alignment) = alignment.as_deref()
    {
        if !matches!(alignment, "4" | "5" | "6" | "7") {
            return Err(backend(
                "compact function alignment must be 16, 32, 64 or 128 bytes",
            ));
        }
        flags.push(("log2_min_function_alignment", alignment));
    }
    let direct_literals = !observe
        && compiler_inputs
            .var_os("MECH_COMPACT_DIRECT_ENTRY_LITERALS")
            .is_some();
    let mut emit = |direct| {
        measurements.start_attempt(direct);
        let result = emit_prepared_compact_object(
            &prepared,
            observe,
            &flags,
            direct,
            compiler_inputs,
            &mut measurements,
        );
        measurements.finish_attempt(&result);
        result
    };
    match emit(direct_literals) {
        Err(error) if direct_literals => {
            eprintln!("compact-stage: direct literal attempt withdrew: {error}");
            emit(false)
        }
        result => result,
    }
}

fn emit_prepared_compact_object(
    prepared: &PreparedNativeProgram,
    observe: bool,
    flags: &[(&'static str, &str)],
    direct_literals: bool,
    compiler_inputs: &CompilationInputs,
    measurements: &mut Measurements<'_>,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
    let (object, isa) = measurements.stage("module_setup", || {
        codegen_backend::object_with_isa("mech_compact_closed_diagnostic", flags, compiler_inputs)
    })?;
    let mut module = super::compact::PreparedObject::new(object, compiler_inputs);
    if !observe && let Ok(mode) = compiler_inputs.var("MECH_COMPACT_DIRECT_RUNTIME_CALLS") {
        module.use_direct_runtime_calls(&mode);
    }
    let direct_runtime_calls = module.direct_runtime_calls();
    let threads = compiler_inputs
        .var("MECH_COMPACT_COMPILE_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
        .min(4);
    let share_phase_bodies = compiler_inputs
        .var_os("MECH_COMPACT_SHARE_PHASE_BODIES")
        .is_some();
    measurements.module(isa.as_ref(), flags, threads);
    let metadata = measurements.stage("lower_and_define", || {
        if threads == 0 && !direct_literals {
            super::compact::define(&mut module, prepared, observe, share_phase_bodies)
        } else {
            super::compact::define_with_isa(
                &mut module,
                prepared,
                observe,
                share_phase_bodies,
                isa,
                threads,
                direct_runtime_calls,
                direct_literals,
            )
        }
    })?;
    if !observe
        && compiler_inputs
            .var_os("MECH_COMPACT_EXPORTS_FIRST")
            .is_some()
    {
        module.place_exports_first();
    }
    if !observe
        && compiler_inputs
            .var_os("MECH_COMPACT_LAYOUT_REGIONS_LAST")
            .is_some()
    {
        module.place_layout_regions_last();
    }
    let product = measurements
        .stage("object_finalization", || module.finish())
        .map_err(backend)?;
    measurements.object(&product);
    let bytes = measurements
        .stage("object_serialization", || product.emit())
        .map_err(backend)?;
    measurements.object_bytes(bytes.len());
    Ok((bytes, metadata))
}

/// Diagnostic compilation only: compact typed numeric functions over the same
/// semantic plan/emitter/backend. No ordinary managed entry selects these
/// functions, and this is NOT a qualified/unmetered Program execution API.
pub fn emit_minimal_numeric_object_for_diagnostic(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
    use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
    use crate::native_artifact::compiler::compiled_stack_bytes;
    use crate::native_artifact::program::codegen::{cfg_cleanup, numeric_compact, numeric_minimal};
    use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    host_supported()?;
    let inputs = CompilationInputs::legacy();
    let limits = crate::native_artifact::NativeProgramLimitMode::NoLimits.compilation();
    let plan = lower_native_program_artifact_with_definition_table_limit(
        artifact,
        catalog,
        limits.max_definition_table_bytes,
    )
    .map_err(backend)?;
    let prepared = PreparedNativeProgram::prepare_with_inputs(artifact, catalog, plan, &inputs)
        .map_err(backend)?;
    let (mut module, _) = codegen_backend::object_with_isa(
        "mech_minimal_numeric_diagnostic",
        PROGRAM_CODEGEN_FLAGS,
        &inputs,
    )?;
    let code = CodePlan::Program(&prepared);
    let closed = &prepared.metadata().closed_tails;
    let controls = CompilationControls::with_inputs(OptimizationOptions::default(), &inputs);
    let mut reports = Vec::new();
    let mut functions = Vec::new();
    for (id, region) in closed.regions.iter().enumerate() {
        let site = region.tail.header() as usize;
        let Some(proof) = region.numeric.as_ref().filter(|p| {
            p.exits.is_empty()
                && p.phase
                    .as_ref()
                    .is_some_and(|p| p.loops.len() > 1 && !p.checked_children)
        }) else {
            continue;
        };
        if closed.sites[site] != Some(id) || region.cold_resume().is_some() {
            continue;
        }
        let symbol = format!("mech_minimal_numeric_{site}");
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); 2];
        signature.returns = vec![AbiParam::new(types::I64); 2];
        let mut context = module.make_context();
        context.func.signature = signature.clone();
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let input = b.block_params(entry)[0];
            let output = b.block_params(entry)[1];
            let zero = b.ins().iconst(types::I64, 0);
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.set_cold_block(error);
            let done = b.create_block();
            // No helper is reachable in this direct private-storage contract.
            // Retained shared source operations must lower or fail explicitly.
            let native =
                numeric_minimal::Boundary::new(&mut b, input, output, region, None, None, None)?;
            let report = numeric_compact::emit(
                &controls, &mut b, code, region, &native, None, None, None, done, error, false,
            )?;
            reports.push(report);
            b.switch_to_block(done);
            b.ins().return_(&[zero, zero]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            let origin = b.ins().load(types::I64, MemFlags::new(), output, 0);
            b.ins().return_(&[status, origin]);
            b.seal_all_blocks();
            b.finalize();
        }
        let function = module
            .declare_function(&symbol, Linkage::Export, &signature)
            .map_err(backend)?;
        let mut bit_counts =
            crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
        let cleanup = cfg_cleanup::optimize(&controls, &mut context, module.isa())?;
        bit_counts +=
            crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
        let calls = context
            .func
            .layout
            .blocks()
            .flat_map(|block| context.func.layout.block_insts(block))
            .filter(|&inst| context.func.dfg.insts[inst].opcode().is_call())
            .count();
        if calls != 0 {
            return Err(backend(
                "minimal diagnostic still contains an unresolved helper call",
            ));
        }
        module
            .define_function(function, &mut context)
            .map_err(backend)?;
        let compiled = context
            .compiled_code()
            .ok_or_else(|| backend("minimal numeric function lacks machine code"))?;
        functions.push(serde_json::json!({"header":site,"symbol":symbol,"input_words":numeric_minimal::input_words(proof,region.tail.carrier().get())?,"output_words":numeric_minimal::output_words(proof,false)?,"arrays":proof.arrays,"scalar_words":proof.scalar_words,"code_bytes":compiled.code_info().total_size,"stack_bytes":compiled_stack_bytes(&compiled.buffer)?,"cleanup":cleanup,"calls":calls,"bit_counts":bit_counts}));
    }
    let metadata = serde_json::json!({"contract":"Diagnostic typed computation, exact ordinary accounting/publication/recovery omitted; not NoLimits or a qualified Program entry.","shared_semantic_plan":true,"shared_scalar_array_emitter_and_backend":true,"source_names_not_used_for_selection":true,"functions":functions,"numeric_reports":reports,"codegen_flags":PROGRAM_CODEGEN_FLAGS});
    Ok((module.finish().emit().map_err(backend)?, metadata))
}
