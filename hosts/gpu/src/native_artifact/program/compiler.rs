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
        let plan = lower_native_program_artifact_with_definition_table_limit(
            artifact,
            catalog,
            limits.max_definition_table_bytes,
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
    image_binding: super::image::binding::NativeProgramImageBinding,
    native_metadata: Vec<u8>,
    native_metadata_symbol: String,
    compiled_module_symbol: String,
    native_metadata_binding: super::image::binding::NativeProgramImageBinding,
    codegen: crate::native_artifact::codegen::control::CodegenReport,
    machine_facts: Option<serde_json::Value>,
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
        if let Some(histogram) = &self.codegen.utf8_histogram {
            metadata["compiled_utf8_histogram"] = serde_json::json!(histogram);
        }
        metadata["compiled_numeric_functions"] = serde_json::json!(&self.codegen.numeric_functions);
        metadata["compiled_scalar_selects"] = serde_json::json!(self.codegen.scalar_selects);
        metadata["compiled_wide_retirement_pairs"] = serde_json::json!(
            self.codegen.numeric_write_loops.iter().chain(&self.codegen.readonly_numeric_loops)
                .map(|r| r.wide_retirement_pairs).sum::<usize>()
        );
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
        metadata["native_metadata_format"] = serde_json::json!("mech.typed-build-image.v3");
        metadata["native_descriptor_version"] =
            serde_json::json!(if self.descriptor.machine_backend.is_some() {
                10
            } else {
                9
            });
        if let Some(backend) = &self.descriptor.machine_backend {
            metadata["machine_backend"] = serde_json::json!(backend);
            metadata["object_optimization"] = serde_json::json!("llvm-O3-apple-m3");
            metadata["compiled_machine_facts"] = serde_json::json!(&self.machine_facts);
        }
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
    let plan = lower_native_program_artifact_with_definition_table_limit(
        artifact,
        catalog,
        limits.max_definition_table_bytes,
    )
    .map_err(backend)?;
    let mut module = codegen_backend::object("mech_canonical_program", PROGRAM_CODEGEN_FLAGS)?;
    preflight(&plan, &module, limits)?;
    let prepared = PreparedNativeProgram::prepare(artifact, catalog, plan).map_err(backend)?;
    let (entry, stack, code, codegen) = crate::native_artifact::codegen::control::define_cfg(
        CodePlan::Program(&prepared),
        ENTRY,
        VERSION,
        &mut module,
        limits,
        options,
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

/// Compile the ordinary source-proved native definition graph to inspectable
/// LLVM IR. This is compiler output only, not a NativeProgramObject: LLVM
/// machine admission and backend-bound image construction are separate steps.
pub fn emit_program_llvm_ir(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
) -> Result<String, NativeCompileError> {
    emit_program_llvm_ir_with_limits(artifact, catalog, NativeCompileLimits::default())
}

pub fn emit_program_llvm_ir_with_limits(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    limits: NativeCompileLimits,
) -> Result<String, NativeCompileError> {
    host_supported()?;
    let plan = lower_native_program_artifact_with_definition_table_limit(
        artifact,
        catalog,
        limits.max_definition_table_bytes,
    )
    .map_err(backend)?;
    let mut module = super::llvm_backend::RecordingModule::new(codegen_backend::object(
        "mech_canonical_program_llvm_ssa",
        PROGRAM_CODEGEN_FLAGS,
    )?);
    preflight(&plan, &module, limits)?;
    let prepared = PreparedNativeProgram::prepare(artifact, catalog, plan).map_err(backend)?;
    crate::native_artifact::codegen::control::define_cfg(
        CodePlan::Program(&prepared),
        ENTRY,
        VERSION,
        &mut module,
        limits,
        &crate::native_artifact::codegen::options::OptimizationOptions::default(),
    )?;
    module.llvm_ir().map_err(backend)
}

// Code and stack admission must be derived by the selected machine emitter
// before this same-Object identity, image and compiled-module binding is made.
// Keeping finalization separate prevents a second backend from substituting
// machine text under a descriptor which describes different code.
fn finish_program_object(
    module: cranelift_object::ObjectModule,
    prepared: PreparedNativeProgram,
    entry: cranelift_module::FuncId,
    footprint: NativeCodeFootprint,
    codegen: crate::native_artifact::codegen::control::CodegenReport,
) -> Result<NativeProgramObject, NativeCompileError> {
    finish_program_object_using(
        module,
        prepared,
        entry,
        footprint,
        codegen,
        None,
        None,
        |module| module.finish().emit().map_err(backend),
    )
}

fn finish_program_object_using(
    mut module: cranelift_object::ObjectModule,
    prepared: PreparedNativeProgram,
    entry: cranelift_module::FuncId,
    footprint: NativeCodeFootprint,
    codegen: crate::native_artifact::codegen::control::CodegenReport,
    machine_backend: Option<super::image::machine_backend::MachineBackend>,
    machine_facts: Option<serde_json::Value>,
    finish: impl FnOnce(cranelift_object::ObjectModule) -> Result<Vec<u8>, NativeCompileError>,
) -> Result<NativeProgramObject, NativeCompileError> {
    let separate_machine_metadata = machine_backend.is_some();
    let descriptor = NativeProgramDescriptor::from_prepared(prepared, footprint);
    let descriptor = if let Some(backend_stamp) = machine_backend {
        descriptor
            .bind_machine_backend(backend_stamp)
            .map_err(backend)?
    } else {
        descriptor
    };
    let identity = NativeProgramObjectIdentity::from_descriptor(&descriptor).map_err(backend)?;
    let identity_id = module
        .declare_data(identity.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    if separate_machine_metadata {
        data.set_segment_section("__TEXT", "__mech_meta");
    }
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
    if separate_machine_metadata {
        binding_data.set_segment_section("__TEXT", "__mech_meta");
    }
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
        if separate_machine_metadata {
            data.set_segment_section("__TEXT", "__mech_meta");
        }
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
        bytes: finish(module)?,
        descriptor,
        identity,
        image_binding,
        native_metadata,
        native_metadata_symbol,
        compiled_module_symbol,
        native_metadata_binding,
        codegen,
        machine_facts,
    })
}

/// Compile a genuine LLVM native Object. All source proofs are prepared through
/// the ordinary compiler; machine facts and backend association precede image
/// finalization. LLVM and the linker run only during this AOT compilation.
pub fn emit_program_llvm_object_with_limits(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    limits: NativeCompileLimits,
    toolchain: &super::llvm_backend::NativeLlvmToolchain,
) -> Result<NativeProgramObject, NativeCompileError> {
    use crate::native_artifact::compiler::limit;
    use sha2::{Digest, Sha256};
    if !cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        return Err(NativeCompileError::UnsupportedHost);
    }
    let plan = lower_native_program_artifact_with_definition_table_limit(
        artifact,
        catalog,
        limits.max_definition_table_bytes,
    )
    .map_err(backend)?;
    let mut source = super::llvm_backend::RecordingModule::new(codegen_backend::object(
        "mech_llvm_source_ssa",
        PROGRAM_CODEGEN_FLAGS,
    )?);
    preflight(&plan, &source, limits)?;
    let prepared = PreparedNativeProgram::prepare(artifact, catalog, plan).map_err(backend)?;
    // Source/definition limits remain intact. Discarded Cranelift machine sizes
    // must neither admit nor reject a different machine emitter's output.
    let source_limits = NativeCompileLimits {
        max_code_bytes: u64::MAX,
        max_stack_bytes: u64::MAX,
        ..limits
    };
    let (_, _, _, mut codegen) = crate::native_artifact::codegen::control::define_cfg(
        CodePlan::Program(&prepared),
        ENTRY,
        VERSION,
        &mut source,
        source_limits,
        &crate::native_artifact::codegen::options::OptimizationOptions::default(),
    )?;
    let names = source.definition_names().map_err(backend)?;
    let ir = source.llvm_ir().map_err(backend)?;
    let llvm = toolchain.compile(&ir, &names).map_err(backend)?;
    let root = llvm
        .facts
        .functions
        .get(ENTRY)
        .ok_or_else(|| backend("LLVM root lacks machine facts"))?;
    let helper = llvm
        .facts
        .functions
        .iter()
        .filter(|(name, _)| name.as_str() != ENTRY)
        .map(|(_, f)| f.stack_bytes)
        .max()
        .unwrap_or(0);
    let stack = root
        .stack_bytes
        .checked_add(helper)
        .ok_or_else(|| backend("LLVM native stack envelope overflows"))?;
    limit("Iteration stack", stack, limits.max_stack_bytes)?;
    limit(
        "Iteration code",
        llvm.facts.code_bytes,
        limits.max_code_bytes,
    )?;
    for function in &mut codegen.numeric_functions {
        let actual = llvm
            .facts
            .functions
            .get(&function.symbol)
            .ok_or_else(|| backend("LLVM outlined function lacks machine facts"))?;
        function.stack_bytes = actual.stack_bytes;
        function.code_bytes = actual.code_bytes;
    }
    for function in &mut codegen.outlined_loops {
        let actual = llvm
            .facts
            .functions
            .get(&function.symbol)
            .ok_or_else(|| backend("LLVM packed function lacks machine facts"))?;
        function.stack_bytes = actual.stack_bytes;
        function.code_bytes = actual.code_bytes;
    }
    for function in &mut codegen.join_map_copies {
        let actual = llvm
            .facts
            .functions
            .get(&function.symbol)
            .ok_or_else(|| backend("LLVM join-map copy lacks machine facts"))?;
        function.stack_bytes = actual.stack_bytes;
        function.code_bytes = actual.code_bytes;
    }
    for function in &mut codegen.steady_loops {
        let mut current = Some(function);
        while let Some(function) = current {
            let actual = llvm
                .facts
                .functions
                .get(&function.symbol)
                .ok_or_else(|| backend("LLVM steady loop lacks machine facts"))?;
            function.stack_bytes = actual.stack_bytes;
            function.code_bytes = actual.code_bytes;
            current = function.lookup_cache_function.as_deref_mut();
        }
    }
    if let Some(function) = &mut codegen.utf8_histogram {
        let actual = llvm
            .facts
            .functions
            .get(&function.symbol)
            .ok_or_else(|| backend("LLVM histogram lacks machine facts"))?;
        function.stack_bytes = actual.stack_bytes;
        function.code_bytes = actual.code_bytes;
    }
    if let Some(function) = &mut codegen.bound_call_shim {
        let actual = llvm
            .facts
            .functions
            .get(&function.symbol)
            .ok_or_else(|| backend("LLVM preserving bridge lacks machine facts"))?;
        if actual.code_bytes % 4 != 0 || actual.code_bytes / 4 > function.template_bound as u64 {
            return Err(backend(
                "LLVM preserving bridge exceeds source template envelope",
            ));
        }
        function.stack_bytes = actual.stack_bytes;
        function.code_bytes = actual.code_bytes;
    }
    let stamp = super::image::machine_backend::MachineBackend::llvm(
        NativeProgramDescriptor::source_identity(&prepared),
        llvm.toolchain,
        Sha256::digest(&llvm.object).into(),
    )
    .map_err(backend)?;
    let mut metadata =
        codegen_backend::object("mech_llvm_program_metadata", PROGRAM_CODEGEN_FLAGS)?;
    let mut signature = metadata.make_signature();
    signature.params.push(cranelift_codegen::ir::AbiParam::new(
        cranelift_codegen::ir::types::I64,
    ));
    signature.returns.push(cranelift_codegen::ir::AbiParam::new(
        cranelift_codegen::ir::types::I64,
    ));
    let entry = metadata
        .declare_function(ENTRY, Linkage::Import, &signature)
        .map_err(backend)?;
    let facts = serde_json::to_value(&llvm.facts).map_err(backend)?;
    finish_program_object_using(
        metadata,
        prepared,
        entry,
        NativeCodeFootprint {
            stack_bytes: stack,
            code_bytes: llvm.facts.code_bytes,
        },
        codegen,
        Some(stamp),
        Some(facts),
        |module| {
            let bytes = module.finish().emit().map_err(backend)?;
            llvm.assemble(&bytes).map_err(backend)
        },
    )
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
