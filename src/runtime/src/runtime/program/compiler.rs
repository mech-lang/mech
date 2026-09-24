//! Isolated compiler workspace for resident source products.
//!
//! Canonical source planning is local to one compilation and is dropped after
//! finalization. Only immutable compilation products and detached typed
//! initialization values escape this module.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

#[cfg(feature = "compute")]
use mech_compute::{
    ComputeInitializerSet, ComputeRegionInterface, ComputeValue, TensorLayout,
    build_compute_region_interface,
};
#[cfg(feature = "compute")]
use mech_core::OperationContractDeclaration;
use mech_core::{
    ApplicationRequirement, ExecutionResourceRequest, MResult, MechError, MechErrorKind,
    ModuleManifestCatalog, ReactiveInstanceId, ResourceIntent, Value, ValueCell,
};
use mech_engine::__resident::activate_external;
#[cfg(feature = "compute")]
use mech_engine::ComputeRegionDeclaration;
use mech_engine::expressions::ReactiveComprehensionStructureUnsupported;
use mech_engine::resident::ActivationFacts;
use mech_engine::resident::ResidentIntegrityMode;
use mech_engine::{
    CanonicalSourceFrontend, CanonicalSourceProgram, CompiledResourceSendOperation,
    CompilerPlanningConfig, ProgramArtifact, ProgramArtifactCompilationProduct,
    ProgramCompilationProduct,
};

#[cfg(feature = "compute")]
use crate::SourceContextCapabilityScope;
use crate::{
    CapabilityRequest, HostInterfaceCatalog, ModuleBuildOptions, ModuleBuilder,
    ResidentExternalContractResolver, ResolvedSource, RuntimeCapabilityOperation,
    RuntimeHostInputValue, RuntimeInvalidOperationError, RuntimeModuleDependencyMissingError,
    RuntimeResourceKey, RuntimeResourceProviderNotFound, RuntimeResourceReadRequest,
    RuntimeResourceRegistry, RuntimeResourceWriteCommand, RuntimeResourceWriteIntent,
    SourceContextBase, SourceDocument, SourceImportAlias, SourceImportDeclaration, SourceIndex,
    SourceRequest, SourceResolver, import_may_resolve_source_dependency,
    import_requires_source_dependency, module_namespace_for_import, source_request_for_import,
};

use super::{ResidentRouteFailure, ResidentRouteFailureClass, route_failure};

fn canonical_frontend(document: &SourceDocument) -> CanonicalSourceFrontend {
    document
        .nominal_origin()
        .cloned()
        .map_or(CanonicalSourceFrontend, |origin| {
            CanonicalSourceFrontend.with_nominal_origin(origin)
        })
}

pub(super) fn canonical_dependency_identity_hash(
    source: &str,
    origin: Option<&mech_core::CanonicalNominalPath>,
    package_id: Option<&str>,
) -> u64 {
    if origin.is_none() && package_id.is_none() {
        return mech_core::hash_str(source);
    }
    let mut bytes = b"mech-source-dependency-provenance-v1\0".to_vec();
    bytes.extend_from_slice(&(source.len() as u64).to_le_bytes());
    bytes.extend_from_slice(source.as_bytes());
    match origin {
        Some(origin) => {
            let path = origin.canonical_bytes();
            bytes.push(1);
            bytes.extend_from_slice(&(path.len() as u64).to_le_bytes());
            bytes.extend_from_slice(&path);
        }
        None => bytes.push(0),
    }
    match package_id {
        Some(package_id) => {
            bytes.push(1);
            bytes.extend_from_slice(&(package_id.len() as u64).to_le_bytes());
            bytes.extend_from_slice(package_id.as_bytes());
        }
        None => bytes.push(0),
    }
    mech_core::hash_bytes(&bytes)
}

fn canonical_document_dependency_hash(document: &SourceDocument) -> MResult<u64> {
    let source = document.source().to_contiguous_string();
    let nominal = !canonical_frontend(document)
        .declared_enum_names(&document.document())
        .map_err(|error| canonical_compilation_error(error.to_string()))?
        .is_empty();
    Ok(if nominal {
        canonical_dependency_identity_hash(
            &source,
            document.nominal_origin(),
            document.nominal_package_id(),
        )
    } else {
        mech_core::hash_str(&source)
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalProgramCompilationError {
    pub reason: String,
}

impl MechErrorKind for CanonicalProgramCompilationError {
    fn name(&self) -> &str {
        "CanonicalProgramCompilationError"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

fn canonical_compilation_error(reason: impl Into<String>) -> MechError {
    MechError::new(
        CanonicalProgramCompilationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn retained_compiler_document(source: &str) -> MResult<SourceDocument> {
    SourceDocument::parse_resolved(
        "runtime:program-compiler",
        mech_syntax::document::Revision(0),
        Arc::<str>::from(source),
        mech_syntax::document::ParseConfig::default(),
    )
    .map_err(|error| canonical_compilation_error(format!("invalid retained source: {error:?}")))
}

fn admit_resolved_canonical_source(mut resolved: ResolvedSource) -> MResult<ResolvedSource> {
    if matches!(resolved.kind, crate::SourceKind::Mech)
        && resolved.source_document().is_none()
        && matches!(&resolved.source, mech_core::MechSourceCode::String(_))
    {
        resolved = resolved.retain_source_document(
            mech_syntax::document::Revision(0),
            mech_syntax::document::ParseConfig::default(),
        )?;
    }
    resolved.admit_canonical_document()
}

/// The sole owner of source-to-resident-artifact compilation.
///
/// Deliberately absent: input drivers, a live runtime store, runtime
/// transactions, retained reactive state, event history, and program
/// ownership.
pub struct ProgramCompiler {
    function_catalog: Arc<mech_core::FunctionCatalog>,
    source_resolver: Box<dyn SourceResolver>,
    resources: RuntimeResourceRegistry,
    module_builder: ModuleBuilder,
    host_interfaces: HostInterfaceCatalog,
    module_manifests: ModuleManifestCatalog,
    program_config: CompilerPlanningConfig,
}

#[cfg(feature = "compute")]
#[derive(Debug)]
pub struct MixedProgramCompilation {
    pub coordinator: ProgramArtifactCompilationProduct,
    pub compute: ComputeRegionCompilation,
    /// Declaration-time values sent across the compute boundary by the
    /// coordinator. Fixed-shape backends use these ordinary Mech arrays to
    /// infer the outer broadcast extent; the inner compute interface remains
    /// the schema for one independent instance.
    pub activation_inputs: BTreeMap<String, ComputeValue>,
    /// Concrete sampled outputs admitted by the coordinator's declared read
    /// capability. These form the demand-retention contract across compatible
    /// compute-host generations; undeclared outputs remain GPU-resident.
    pub retained_outputs: BTreeSet<String>,
    /// Exact transitive retained revisions shared by both compiled products.
    pub source_dependencies: BTreeMap<String, u64>,
}

#[cfg(feature = "compute")]
#[derive(Debug)]
pub struct ComputeRegionCompilation {
    pub declaration: ComputeRegionDeclaration,
    pub artifact: ProgramArtifact,
    pub interface: ComputeRegionInterface,
    pub initializers: ComputeInitializerSet,
}

impl std::fmt::Debug for ProgramCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgramCompiler")
            .field("function_catalog", &"<FunctionCatalog>")
            .field("source_resolver", &"<dyn SourceResolver>")
            .field("resources", &self.resources)
            .field("module_builder", &self.module_builder)
            .field("host_interfaces", &self.host_interfaces)
            .field("module_manifests", &self.module_manifests)
            .field("program_config", &self.program_config)
            .finish()
    }
}

impl ProgramCompiler {
    pub(crate) fn new(
        function_catalog: Arc<mech_core::FunctionCatalog>,
        source_resolver: Box<dyn SourceResolver>,
        resources: RuntimeResourceRegistry,
        module_builder: ModuleBuilder,
        host_interfaces: HostInterfaceCatalog,
        module_manifests: ModuleManifestCatalog,
        program_config: CompilerPlanningConfig,
    ) -> Self {
        Self {
            function_catalog,
            source_resolver,
            resources,
            module_builder,
            host_interfaces,
            module_manifests,
            program_config,
        }
    }

    pub fn compile_source(&mut self, source: &str) -> MResult<ProgramCompilationProduct> {
        self.view().compile_source(source)
    }

    pub fn compile_document(
        &mut self,
        document: &SourceDocument,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_document(document)
    }

    /// Resolve retained source dependencies and seal their detached exports into a canonical root.
    pub fn compile_canonical_root(
        &mut self,
        request: SourceRequest,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_canonical_root(request, false, None)
    }

    /// Compile the supplied retained revision without resolving the root again.
    pub fn compile_canonical_resolved_root(
        &mut self,
        resolved: ResolvedSource,
    ) -> MResult<ProgramCompilationProduct> {
        self.view()
            .compile_canonical_resolved_root(resolved, false, None)
    }

    /// Compile explicit retained roots in one graph with caller-ordered results.
    pub fn compile_canonical_roots(
        &mut self,
        requests: &[SourceRequest],
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_canonical_roots(requests, options)
    }

    /// Resolve a canonical graph and publish its live interactive root symbols.
    pub fn compile_canonical_interactive_root(
        &mut self,
        request: SourceRequest,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_canonical_root(request, true, None)
    }

    /// Preserve an already resolved interactive revision and its import referrer.
    pub fn compile_canonical_interactive_resolved_root(
        &mut self,
        resolved: ResolvedSource,
    ) -> MResult<ProgramCompilationProduct> {
        self.view()
            .compile_canonical_resolved_root(resolved, true, None)
    }

    /// Preserve the module builder's target, edition, feature and capability identity.
    pub fn compile_canonical_root_with_options(
        &mut self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view()
            .compile_canonical_root(request, false, Some(options))
    }

    pub fn compile_canonical_interactive_root_with_options(
        &mut self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view()
            .compile_canonical_root(request, true, Some(options))
    }

    pub fn compile_canonical_resolved_root_with_options(
        &mut self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view()
            .compile_canonical_resolved_root(resolved, false, Some(options))
    }

    pub fn compile_canonical_interactive_resolved_root_with_options(
        &mut self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view()
            .compile_canonical_resolved_root(resolved, true, Some(options))
    }

    pub fn compile_interactive_document(
        &mut self,
        document: &SourceDocument,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_interactive_document(document)
    }

    /// Prepared canonical source entry point for the coordinated C cutover.
    /// B keeps the shipping source route unchanged while proving this product
    /// boundary against retained documents and real activation.
    pub fn compile_canonical_source(&mut self, source: &str) -> MResult<ProgramCompilationProduct> {
        let document = retained_compiler_document(source)?;
        self.compile_document(&document)
    }

    /// Compiles source for immediate artifact activation without returning or
    /// retaining a duplicate durable bytecode container.
    pub fn compile_source_artifact(
        &mut self,
        source: &str,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        let document = retained_compiler_document(source)?;
        self.compile_document_artifact(&document)
    }

    pub fn compile_document_artifact(
        &mut self,
        document: &SourceDocument,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        self.view().compile_document_artifact(document)
    }

    /// Compile with detached planning values; only explicitly selected names
    /// remain live inputs in the resulting artifact.
    pub fn compile_document_artifact_with_inputs(
        &mut self,
        document: &SourceDocument,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        self.view()
            .compile_document_artifact_with_input_initializers(
                document,
                inputs,
                external_input_names,
            )
            .map(|(product, _)| product)
    }

    /// Return detached declaration-time values from one temporary canonical
    /// activation. The same retained document supplies the live projection;
    /// no compiler cells or runtime host effects escape planning.
    pub fn compile_document_artifact_with_input_initializers(
        &mut self,
        document: &SourceDocument,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<(
        ProgramArtifactCompilationProduct,
        BTreeMap<String, RuntimeHostInputValue>,
    )> {
        self.view()
            .compile_document_artifact_with_input_initializers(
                document,
                inputs,
                external_input_names,
            )
    }

    pub fn evaluate_static_document_symbols(
        &mut self,
        document: &SourceDocument,
        names: &[&str],
    ) -> MResult<BTreeMap<String, RuntimeHostInputValue>> {
        self.evaluate_static_document_symbols_with_inputs(document, &BTreeMap::new(), names)
    }

    pub fn evaluate_static_document_symbols_with_inputs(
        &mut self,
        document: &SourceDocument,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        names: &[&str],
    ) -> MResult<BTreeMap<String, RuntimeHostInputValue>> {
        let view = self.view();
        let context = view.canonical_planning_context(document, inputs)?;
        let names = names.iter().map(|name| (*name).to_owned()).collect();
        let program =
            view.canonical_planning_projection(document, &context, &BTreeSet::new(), &names, true)?;
        let artifact = program.compile_artifact_with_external_contracts(
            &ResidentExternalContractResolver::new(view.resources),
        )?;
        execute_named_canonical_outputs(&artifact, &view.function_catalog, &names)
    }

    pub fn compile_canonical_source_artifact(
        &mut self,
        source: &str,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        let document = retained_compiler_document(source)?;
        self.compile_document_artifact(&document)
    }

    /// Compiles an inline one-document application into its ordinary resident
    /// coordinator and one backend-neutral compute region.
    #[cfg(feature = "compute")]
    pub fn compile_mixed_source(&mut self, source: &str) -> MResult<MixedProgramCompilation> {
        let document = retained_compiler_document(source)?;
        self.compile_mixed_document(&document)
    }

    #[cfg(feature = "compute")]
    pub fn compile_mixed_document(
        &mut self,
        document: &SourceDocument,
    ) -> MResult<MixedProgramCompilation> {
        self.view().compile_mixed_document(document)
    }

    /// Compile both mixed products from one retained resolved graph.
    #[cfg(feature = "compute")]
    pub fn compile_canonical_mixed_root(
        &mut self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<MixedProgramCompilation> {
        request.validate()?;
        let resolved = self.source_resolver.resolve(&request)?.ok_or_else(|| {
            canonical_compilation_error(format!("missing canonical root {}", request.specifier))
        })?;
        self.compile_canonical_mixed_resolved_root(resolved, options)
    }

    #[cfg(feature = "compute")]
    pub fn compile_canonical_mixed_resolved_root(
        &mut self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<MixedProgramCompilation> {
        self.view()
            .compile_canonical_mixed_resolved_root(resolved, options)
    }

    fn view(&self) -> ProgramCompilerView<'_> {
        ProgramCompilerView::new(
            Arc::clone(&self.function_catalog),
            self.source_resolver.as_ref(),
            &self.resources,
            &self.module_builder,
            &self.host_interfaces,
            &self.module_manifests,
            self.program_config.limits.max_planning_steps,
        )
    }
}

/// A short-lived borrowed view used when an already constructed runtime loads
/// source through the same compiler implementation.
pub(crate) struct ProgramCompilerView<'a> {
    function_catalog: Arc<mech_core::FunctionCatalog>,
    source_resolver: &'a dyn SourceResolver,
    resources: &'a RuntimeResourceRegistry,
    module_builder: &'a ModuleBuilder,
    host_interfaces: &'a HostInterfaceCatalog,
    module_manifests: &'a ModuleManifestCatalog,
    max_planning_steps: usize,
}

/// One resolution/planning session. Module versions use the same ModuleBuilder
/// authority as maintained rooted compilation and never enter a runtime store.
struct CanonicalGraphCompilation<'a> {
    options: Option<ModuleBuildOptions<'a>>,
    active: Vec<String>,
    exports: HashMap<String, BTreeMap<String, crate::RuntimeValueSnapshot>>,
    source_dependencies: BTreeMap<String, u64>,
    module_versions: HashMap<String, crate::ModuleVersionId>,
    nominal_owners: BTreeMap<Vec<String>, (Option<String>, mech_syntax::document::DocumentId)>,
}

impl<'a> CanonicalGraphCompilation<'a> {
    fn new(options: Option<ModuleBuildOptions<'a>>) -> Self {
        Self {
            options,
            active: Vec::new(),
            exports: HashMap::new(),
            source_dependencies: BTreeMap::new(),
            module_versions: HashMap::new(),
            nominal_owners: BTreeMap::new(),
        }
    }

    fn enum_qualifiers(&self) -> MResult<BTreeMap<mech_core::NominalKey, String>> {
        self.nominal_owners
            .keys()
            .map(|path| {
                let origin =
                    mech_core::CanonicalNominalPath::new(path.clone()).map_err(|error| {
                        canonical_compilation_error(format!(
                            "invalid enum declaration path: {error:?}"
                        ))
                    })?;
                Ok((
                    mech_core::NominalKey::from_path(mech_core::NominalKind::Enum, &origin),
                    path.last()
                        .expect("registered enum paths have a name")
                        .clone(),
                ))
            })
            .collect()
    }
}

struct CanonicalDocumentPlanning {
    schemas: BTreeMap<String, mech_core::SchemaBody>,
    reads: BTreeMap<String, ExecutionResourceRequest>,
    writes: BTreeMap<String, ExecutionResourceRequest>,
    values: BTreeMap<String, Value>,
}

impl<'a> ProgramCompilerView<'a> {
    pub(crate) fn new(
        function_catalog: Arc<mech_core::FunctionCatalog>,
        source_resolver: &'a dyn SourceResolver,
        resources: &'a RuntimeResourceRegistry,
        module_builder: &'a ModuleBuilder,
        host_interfaces: &'a HostInterfaceCatalog,
        module_manifests: &'a ModuleManifestCatalog,
        max_planning_steps: usize,
    ) -> Self {
        Self {
            function_catalog,
            source_resolver,
            resources,
            module_builder,
            host_interfaces,
            module_manifests,
            max_planning_steps,
        }
    }

    pub(crate) fn compile_source(&self, source: &str) -> MResult<ProgramCompilationProduct> {
        let document = retained_compiler_document(source)?;
        self.compile_document(&document)
    }

    pub(crate) fn compile_interactive_source(
        &self,
        source: &str,
    ) -> MResult<ProgramCompilationProduct> {
        let document = retained_compiler_document(source)?;
        self.compile_interactive_document(&document)
    }

    pub(crate) fn compile_document(
        &self,
        document: &SourceDocument,
    ) -> MResult<ProgramCompilationProduct> {
        let artifact = self.canonical_document_artifact(document)?;
        ProgramCompilationProduct::from_canonical_artifact(artifact)
    }

    pub(crate) fn compile_interactive_document(
        &self,
        document: &SourceDocument,
    ) -> MResult<ProgramCompilationProduct> {
        ProgramCompilationProduct::from_canonical_artifact(
            self.canonical_document_artifact_with_projection(document, true)?,
        )
    }

    pub(crate) fn compile_document_artifact(
        &self,
        document: &SourceDocument,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        self.canonical_document_artifact(document)
            .map(ProgramArtifactCompilationProduct::from_artifact)
    }

    fn canonical_planning_context(
        &self,
        document: &SourceDocument,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
    ) -> MResult<CanonicalDocumentPlanning> {
        let index = document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        let (mut schemas, reads, writes, mut values) =
            self.canonical_document_resources(&index.root, &document.document())?;
        for (name, input) in inputs {
            if reads.contains_key(name) {
                return Err(canonical_compilation_error(format!(
                    "planning input {name} conflicts with a configured resource read"
                )));
            }
            let value = input.clone().into_value()?;
            schemas.insert(
                name.clone(),
                ValueCell::from_snapshot(value.clone())?.closed_schema_body()?,
            );
            values.insert(name.clone(), value);
        }
        Ok(CanonicalDocumentPlanning {
            schemas,
            reads,
            writes,
            values,
        })
    }

    fn canonical_planning_projection(
        &self,
        document: &SourceDocument,
        context: &CanonicalDocumentPlanning,
        external: &BTreeSet<String>,
        published: &BTreeSet<String>,
        initialization: bool,
    ) -> MResult<CanonicalSourceProgram> {
        let mut program = canonical_frontend(document)
            .compile_document_with_planning_contract(
                &document.document(),
                Arc::clone(&self.function_catalog),
                context.schemas.clone(),
                context.writes.clone(),
                external,
                published,
                &BTreeSet::new(),
            )
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        if initialization {
            program = program
                .retain_static_outputs(published)
                .map_err(|error| canonical_compilation_error(error.to_string()))?;
        }
        let referenced = program.referenced_input_names();
        let mut constants = Vec::new();
        for (ordinal, input) in program.program().inputs.iter().enumerate() {
            if !initialization
                && (external.contains(&input.name)
                    || (context.reads.contains_key(&input.name)
                        && referenced.contains(&input.name)))
            {
                continue;
            }
            let value = context.values.get(&input.name).ok_or_else(|| {
                canonical_compilation_error(format!(
                    "canonical input {} has no planning value or live declaration",
                    input.name
                ))
            })?;
            constants.push((ordinal as u32, value.clone()));
        }
        program = program
            .bind_input_constants(&constants)
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        self.validate_canonical_planning_step_limit(&program)?;
        if !initialization {
            self.validate_canonical_planning_candidate(&program, &context.values)?;
            for name in external {
                if !program
                    .program()
                    .inputs
                    .iter()
                    .any(|input| input.name == *name)
                {
                    return Err(canonical_compilation_error(format!(
                        "requested live input {name} is not defined or used by the document"
                    )));
                }
            }
            for (name, request) in &context.reads {
                if !program
                    .program()
                    .inputs
                    .iter()
                    .any(|input| input.name == *name)
                {
                    continue;
                }
                program = program
                    .bind_resource_input(name, request.clone())
                    .map_err(|error| canonical_compilation_error(error.to_string()))?;
            }
        }
        Ok(program)
    }

    fn compile_document_artifact_with_input_initializers(
        &self,
        document: &SourceDocument,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<(
        ProgramArtifactCompilationProduct,
        BTreeMap<String, RuntimeHostInputValue>,
    )> {
        let context = self.canonical_planning_context(document, inputs)?;
        let program = self.canonical_planning_projection(
            document,
            &context,
            external_input_names,
            &BTreeSet::new(),
            false,
        )?;
        let initializers = if external_input_names.is_empty() {
            BTreeMap::new()
        } else {
            let initializer_program = self.canonical_planning_projection(
                document,
                &context,
                &BTreeSet::new(),
                external_input_names,
                true,
            )?;
            let artifact = initializer_program.compile_artifact_with_external_contracts(
                &ResidentExternalContractResolver::new(self.resources),
            )?;
            execute_named_canonical_outputs(
                &artifact,
                &self.function_catalog,
                external_input_names,
            )?
        };
        let artifact = program.compile_artifact_with_external_contracts(
            &ResidentExternalContractResolver::new(self.resources),
        )?;
        Ok((
            ProgramArtifactCompilationProduct::from_artifact(artifact),
            initializers,
        ))
    }

    fn validate_canonical_planning_candidate(
        &self,
        program: &CanonicalSourceProgram,
        values: &BTreeMap<String, Value>,
    ) -> MResult<()> {
        self.validate_canonical_planning_step_limit(program)?;
        let has_write = program.program().nodes.iter().any(|node| {
            let mech_engine::SourceNodeBody::Operation {
                requirement: Some(id),
                ..
            } = &node.body
            else {
                return false;
            };
            matches!(program.program().requirements.get(*id),
                Some(ApplicationRequirement::Resource(request))
                if matches!(request.intent, ResourceIntent::Assign | ResourceIntent::Send))
        });
        if !has_write && values.is_empty() {
            return Ok(());
        }
        let bindings = program
            .program()
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(ordinal, input)| {
                values
                    .get(&input.name)
                    .map(|value| (ordinal as u32, value.clone()))
            })
            .collect::<Vec<_>>();
        let planning = program
            .clone()
            .bind_input_constants(&bindings)
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        // Explicit live ports can have their defaults evaluated by the
        // separate initializer projection. Do not fabricate a planning value
        // for a still-open input merely to execute an unrelated pure graph.
        if !has_write && !planning.program().inputs.is_empty() {
            return Ok(());
        }
        let artifact = planning.compile_artifact_with_external_contracts(
            &ResidentExternalContractResolver::new(self.resources),
        )?;
        let mut instance = activate_external(
            ReactiveInstanceId::new(0x4350_4c4e, 0),
            &artifact,
            &self.function_catalog,
            &ActivationFacts::default(),
            ResidentIntegrityMode::Checked,
        )
        .map_err(|error| {
            canonical_compilation_error(format!(
                "canonical resource planning activation failed: {error:?}"
            ))
        })?;
        let prepared = instance.prepare_initial_turn(&[]).map_err(|error| {
            canonical_compilation_error(format!("canonical resource planning failed: {error:?}"))
        })?;
        preflight_canonical_effect_payloads(&prepared, &artifact, self.resources)?;
        // Planning owns no publication authority. All candidate state and
        // captured effects are discarded after the provider's effect-free hook.
        prepared.abort();
        Ok(())
    }

    fn validate_canonical_planning_step_limit(
        &self,
        program: &CanonicalSourceProgram,
    ) -> MResult<()> {
        if program.program().nodes.len() > self.max_planning_steps {
            return Err(canonical_compilation_error(format!(
                "canonical planning exceeds the configured {} step limit",
                self.max_planning_steps
            )));
        }
        Ok(())
    }

    fn canonical_document_artifact(
        &self,
        document: &SourceDocument,
    ) -> MResult<mech_engine::ProgramArtifact> {
        self.canonical_document_artifact_with_projection(document, false)
    }

    fn canonical_document_artifact_with_projection(
        &self,
        document: &SourceDocument,
        interactive: bool,
    ) -> MResult<mech_engine::ProgramArtifact> {
        let index = document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        let (input_schemas, resource_reads, resource_writes, planned_reads) =
            self.canonical_document_resources(&index.root, &document.document())?;
        let compile = if interactive {
            CanonicalSourceFrontend::compile_interactive_document_with_catalog_and_resources
        } else {
            CanonicalSourceFrontend::compile_document_with_catalog_and_resources
        };
        let frontend = canonical_frontend(document);
        let mut program = compile(
            &frontend,
            &document.document(),
            Arc::clone(&self.function_catalog),
            input_schemas,
            BTreeMap::new(),
            resource_writes,
        )
        .map_err(|error| canonical_compilation_error(error.to_string()))?;
        self.validate_canonical_planning_candidate(&program, &planned_reads)?;
        for (name, request) in resource_reads {
            if program
                .program()
                .inputs
                .iter()
                .any(|input| input.name == name)
            {
                program = program
                    .bind_resource_input(&name, request)
                    .map_err(|error| canonical_compilation_error(error.to_string()))?;
            }
        }
        program
            .compile_artifact_with_external_contracts(&ResidentExternalContractResolver::new(
                self.resources,
            ))
            .map_err(|error| {
                canonical_compilation_error(format!(
                    "unable to compile canonical ProgramArtifact: {error:?}"
                ))
            })
    }

    pub(crate) fn compile_canonical_roots(
        &self,
        requests: &[SourceRequest],
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        use crate::resolver::{CanonicalResolvedImport, SourceScope, canonical_import_values};
        use mech_engine::{CanonicalOrderedDocument, CanonicalOrderedImport};
        if requests.is_empty() {
            return Err(canonical_compilation_error(
                "ordered compilation requires at least one root",
            ));
        }
        let mut resolved = requests
            .iter()
            .map(|request| {
                request.validate()?;
                admit_resolved_canonical_source(self.source_resolver.resolve(request)?.ok_or_else(
                    || {
                        canonical_compilation_error(format!(
                            "missing canonical root {}",
                            request.specifier
                        ))
                    },
                )?)
            })
            .collect::<MResult<Vec<_>>>()?;
        let mut identities = BTreeMap::new();
        for (ordinal, root) in resolved.iter().enumerate() {
            if identities
                .insert(root.canonical_uri.clone(), ordinal)
                .is_some()
            {
                return Err(canonical_compilation_error(format!(
                    "duplicate ordered root {}",
                    root.canonical_uri
                )));
            }
        }
        let requested_roots = resolved.len();
        let mut indexes = Vec::new();
        let mut root_imports = Vec::new();
        let mut detached_indexes = Vec::new();
        let mut ordinal = 0;
        while ordinal < resolved.len() {
            let uri = resolved[ordinal].canonical_uri.clone();
            let index = resolved[ordinal]
                .source_document()
                .ok_or_else(|| {
                    canonical_compilation_error("ordered root has no retained document")
                })?
                .index()
                .map_err(|error| MechError::new(error, None))?;
            let mut detached = index.root.clone();
            let mut imports = Vec::new();
            for declaration in index.root.program_imports() {
                if !import_may_resolve_source_dependency(&declaration) {
                    continue;
                }
                let request = source_request_for_import(&declaration, Some(&uri));
                let Some(dependency) = self.source_resolver.resolve(&request)? else {
                    continue;
                };
                let dependency = admit_resolved_canonical_source(dependency)?;
                let dependency_id = if let Some(identity) =
                    identities.get(&dependency.canonical_uri).copied()
                {
                    let retained = resolved[identity].source_document().ok_or_else(|| {
                        canonical_compilation_error("ordered root has no retained document")
                    })?;
                    let current = dependency.source_document().ok_or_else(|| {
                        canonical_compilation_error("canonical dependency has no retained document")
                    })?;
                    if mech_core::hash_str(&retained.source().to_contiguous_string())
                        != mech_core::hash_str(&current.source().to_contiguous_string())
                    {
                        return Err(canonical_compilation_error(format!(
                            "canonical dependency {} changed during compilation",
                            dependency.canonical_uri
                        )));
                    }
                    identity
                } else {
                    let identity = resolved.len();
                    identities.insert(dependency.canonical_uri.clone(), identity);
                    resolved.push(dependency);
                    identity
                };
                detached
                    .imports
                    .retain(|item| item.declaration != declaration);
                imports.push((declaration, dependency_id));
            }
            indexes.push(index);
            detached_indexes.push(detached);
            root_imports.push(imports);
            ordinal += 1;
        }
        fn visit(
            root: usize,
            imports: &[Vec<(SourceImportDeclaration, usize)>],
            active: &mut BTreeSet<usize>,
            done: &mut BTreeSet<usize>,
            order: &mut Vec<usize>,
        ) -> MResult<()> {
            if done.contains(&root) {
                return Ok(());
            }
            if !active.insert(root) {
                return Err(canonical_compilation_error("ordered root dependency cycle"));
            }
            for (_, dependency) in &imports[root] {
                visit(*dependency, imports, active, done, order)?;
            }
            active.remove(&root);
            done.insert(root);
            order.push(root);
            Ok(())
        }
        let mut order = Vec::new();
        let mut done = BTreeSet::new();
        for ordinal in 0..resolved.len() {
            visit(
                ordinal,
                &root_imports,
                &mut BTreeSet::new(),
                &mut done,
                &mut order,
            )?;
        }
        let mut context = CanonicalGraphCompilation::new(Some(options));
        // Ordered roots and recursively compiled detached dependencies share
        // one nominal namespace, even though their artifacts are built by
        // different compiler entry points.
        for root in &resolved {
            self.register_nominal_declarations(
                root.source_document().expect("validated retained root"),
                &mut context,
            )?;
        }
        let mut documents = Vec::new();
        let mut import_uses = Vec::new();
        let mut reads = BTreeMap::new();
        let mut values = BTreeMap::new();
        for ordinal in order {
            let root = &resolved[ordinal];
            let document = root.source_document().expect("validated retained root");
            let index = &indexes[ordinal].root;
            let mut detached = self.canonical_graph_imports(
                &detached_indexes[ordinal],
                &root.canonical_uri,
                &mut context,
            )?;
            let mut imports = detached
                .iter()
                .map(|import| CanonicalResolvedImport {
                    declaration: import.declaration.clone(),
                    canonical_uri: import.canonical_uri.clone(),
                    exports: import
                        .exports
                        .iter()
                        .map(|(name, value)| {
                            (
                                name.clone(),
                                CanonicalOrderedImport::Value(value.to_value()),
                            )
                        })
                        .collect(),
                })
                .collect::<Vec<_>>();
            for (declaration, dependency) in &root_imports[ordinal] {
                let target = &resolved[*dependency];
                imports.push(CanonicalResolvedImport {
                    declaration: declaration.clone(),
                    canonical_uri: target.canonical_uri.clone(),
                    exports: indexes[*dependency]
                        .root
                        .program_exports()
                        .into_iter()
                        .map(|export| {
                            (
                                export.name.clone(),
                                CanonicalOrderedImport::RootExport {
                                    root: *dependency,
                                    name: export.name,
                                },
                            )
                        })
                        .collect(),
                });
                // Module identity depends on the resolved edge and target version,
                // independently of whether its export is a snapshot or live graph value.
                detached.push(CanonicalResolvedImport {
                    declaration: declaration.clone(),
                    canonical_uri: target.canonical_uri.clone(),
                    exports: BTreeMap::new(),
                });
                context.source_dependencies.insert(
                    target.canonical_uri.clone(),
                    canonical_document_dependency_hash(target.source_document().unwrap())?,
                );
            }
            let modules = imports
                .iter()
                .filter_map(|import| {
                    import
                        .declaration
                        .module
                        .clone()
                        .or_else(|| module_namespace_for_import(&import.declaration))
                })
                .collect();
            import_uses.push((ordinal, imports.clone()));
            let imports = canonical_import_values(index, &SourceScope::Program, &imports)
                .map_err(|error| canonical_compilation_error(error.to_string()))?;
            let (schemas, resource_reads, writes, planned) =
                self.canonical_document_resources(index, &document.document())?;
            for (name, request) in resource_reads {
                reads.insert(format!("root:{ordinal}/{name}"), request);
            }
            for (name, value) in planned {
                values.insert(format!("root:{ordinal}/{name}"), value);
            }
            documents.push(CanonicalOrderedDocument {
                document: document.document(),
                nominal_origin: document.nominal_origin().cloned(),
                nominal_package_id: document.nominal_package_id().map(str::to_owned),
                identity: ordinal,
                publish_result: ordinal < requested_roots,
                input_schemas: schemas,
                resource_writes: writes,
                imports,
                resolved_modules: modules,
            });
            self.canonical_graph_module_identity(root, &detached, &mut context)?;
        }
        let mut program = CanonicalSourceFrontend
            .with_imported_enum_qualifiers(context.enum_qualifiers()?)
            .compile_ordered_documents_with_catalog(&documents, Arc::clone(&self.function_catalog))
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        for (ordinal, imports) in import_uses {
            use mech_syntax::document::AstNode;
            let document = resolved[ordinal].source_document().unwrap().document();
            let owner = document.syntax().source();
            let inputs = program
                .program()
                .inputs
                .iter()
                .zip(program.source_map().inputs.iter())
                .filter(|(_, anchor)| {
                    anchor.document == owner.document() && anchor.revision == owner.revision()
                })
                .map(|(input, _)| input.name.as_str());
            crate::resolver::validate_canonical_import_uses(
                &indexes[ordinal].root,
                &SourceScope::Program,
                &imports,
                inputs,
            )
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        }
        self.validate_canonical_planning_candidate(&program, &values)?;
        for (name, request) in reads {
            if program
                .program()
                .inputs
                .iter()
                .any(|input| input.name == name)
            {
                program = program
                    .bind_resource_input(&name, request)
                    .map_err(|error| canonical_compilation_error(error.to_string()))?;
            }
        }
        let artifact = program.compile_artifact_with_external_contracts(
            &ResidentExternalContractResolver::new(self.resources),
        )?;
        ProgramCompilationProduct::from_canonical_artifact(artifact)
            .map(|product| product.with_source_dependencies(context.source_dependencies))
    }

    fn compile_canonical_root(
        &self,
        request: SourceRequest,
        interactive: bool,
        options: Option<ModuleBuildOptions<'_>>,
    ) -> MResult<ProgramCompilationProduct> {
        request.validate()?;
        let resolved = self.source_resolver.resolve(&request)?.ok_or_else(|| {
            canonical_compilation_error(format!("missing canonical root {}", request.specifier))
        })?;
        self.compile_canonical_resolved_root(resolved, interactive, options)
    }

    pub(crate) fn compile_canonical_resolved_root(
        &self,
        resolved: ResolvedSource,
        interactive: bool,
        options: Option<ModuleBuildOptions<'_>>,
    ) -> MResult<ProgramCompilationProduct> {
        let resolved = admit_resolved_canonical_source(resolved)?;
        let mut context = CanonicalGraphCompilation::new(options);
        let program =
            self.compile_canonical_graph_document(&resolved, &mut context, false, interactive)?;
        let artifact = program.compile_artifact_with_external_contracts(
            &ResidentExternalContractResolver::new(self.resources),
        )?;
        ProgramCompilationProduct::from_canonical_artifact(artifact)
            .map(|product| product.with_source_dependencies(context.source_dependencies))
    }

    fn compile_canonical_graph_document(
        &self,
        resolved: &ResolvedSource,
        context: &mut CanonicalGraphCompilation<'_>,
        planning_dependency: bool,
        interactive: bool,
    ) -> MResult<mech_engine::CanonicalSourceProgram> {
        use crate::resolver::{CanonicalDocumentCompilation, SourceScope, canonical_import_values};
        let uri = &resolved.canonical_uri;
        let document = resolved.source_document().ok_or_else(|| {
            canonical_compilation_error("canonical root has no retained document")
        })?;
        let frontend = canonical_frontend(document);
        self.register_nominal_declarations(document, context)?;
        if context.active.iter().any(|entry| entry == uri) {
            return Err(canonical_compilation_error(format!(
                "canonical source dependency cycle at {uri}"
            )));
        }
        context.active.push(uri.to_owned());
        let index = document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        let imports = self.canonical_graph_imports(&index.root, uri, context)?;
        let frontend = frontend.with_imported_enum_qualifiers(context.enum_qualifiers()?);
        let imported = canonical_import_values(&index.root, &SourceScope::Program, &imports)
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        let (mut schemas, reads, writes, planned_reads) =
            self.canonical_document_resources(&index.root, &document.document())?;
        for (name, value) in imported {
            schemas.insert(
                name,
                ValueCell::from_snapshot(value.to_value())?.closed_schema_body()?,
            );
        }
        let compile = if interactive {
            CanonicalSourceFrontend::compile_interactive_document_with_planning_contract
        } else {
            CanonicalSourceFrontend::compile_document_with_planning_contract
        };
        let program = compile(
            &frontend,
            &document.document(),
            Arc::clone(&self.function_catalog),
            schemas,
            writes,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &imports
                .iter()
                .filter_map(|import| {
                    import
                        .declaration
                        .module
                        .clone()
                        .or_else(|| module_namespace_for_import(&import.declaration))
                })
                .collect(),
        )
        .map_err(|error| canonical_compilation_error(error.to_string()))?;
        let compilation = CanonicalDocumentCompilation {
            index: index.root.clone(),
            scope: SourceScope::Program,
            program,
        };
        let bindings = compilation
            .bind_resolved_imports(&imports)
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        let mut program = compilation
            .program
            .bind_input_constants(
                &bindings
                    .into_iter()
                    .map(|binding| (binding.input, binding.value.to_value()))
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        self.validate_canonical_planning_candidate(&program, &planned_reads)?;
        if planning_dependency {
            let bindings = program
                .program()
                .inputs
                .iter()
                .enumerate()
                .filter_map(|(ordinal, input)| {
                    planned_reads
                        .get(&input.name)
                        .map(|value| (ordinal as u32, value.clone()))
                })
                .collect::<Vec<_>>();
            program = program
                .bind_input_constants(&bindings)
                .map_err(|error| canonical_compilation_error(error.to_string()))?;
        } else {
            for (name, request) in reads {
                if program
                    .program()
                    .inputs
                    .iter()
                    .any(|input| input.name == name)
                {
                    program = program
                        .bind_resource_input(&name, request)
                        .map_err(|error| canonical_compilation_error(error.to_string()))?;
                }
            }
        }
        self.canonical_graph_module_identity(resolved, &imports, context)?;
        context.active.pop();
        Ok(program)
    }

    fn register_nominal_declarations(
        &self,
        document: &SourceDocument,
        context: &mut CanonicalGraphCompilation<'_>,
    ) -> MResult<()> {
        let Some(origin) = document.nominal_origin() else {
            return Ok(());
        };
        for name in canonical_frontend(document)
            .declared_enum_names(&document.document())
            .map_err(|error| canonical_compilation_error(error.to_string()))?
        {
            let path = origin
                .segments()
                .iter()
                .cloned()
                .chain(std::iter::once(name))
                .collect::<Vec<_>>();
            let owner = document.nominal_package_id().map(str::to_owned);
            let defining_document = document.source().document();
            if let Some(previous) = context.nominal_owners.get(&path) {
                if previous.0 != owner || previous.1 != defining_document {
                    return Err(canonical_compilation_error(format!(
                        "source-semantics/ambiguous-nominal-declaration-v1: {} has distinct defining sources",
                        path.join("/")
                    )));
                }
            } else {
                context
                    .nominal_owners
                    .insert(path, (owner, defining_document));
            }
        }
        Ok(())
    }

    fn canonical_graph_module_identity(
        &self,
        resolved: &ResolvedSource,
        imports: &[crate::resolver::CanonicalResolvedImport],
        context: &mut CanonicalGraphCompilation<'_>,
    ) -> MResult<()> {
        let uri = &resolved.canonical_uri;
        if let Some(options) = context.options {
            let mut resolved = resolved.clone();
            resolved.capability_requirements.extend(
                options
                    .capability_requirements
                    .iter()
                    .map(|resource| CapabilityRequest::from_keys("compiler", "use", *resource)),
            );
            let dependency_versions = imports
                .iter()
                .map(|import| {
                    context
                        .module_versions
                        .get(&import.canonical_uri)
                        .copied()
                        .ok_or_else(|| {
                            canonical_compilation_error(format!(
                                "missing canonical module identity for {}",
                                import.canonical_uri
                            ))
                        })
                })
                .collect::<MResult<Vec<_>>>()?;
            let feature_flags = options
                .feature_flags
                .iter()
                .map(|flag| (*flag).to_owned())
                .collect::<Vec<_>>();
            let requirements = resolved.capability_requirements.clone();
            let record = self.module_builder.clone().build_resolved_source(
                resolved,
                options.compiler_version,
                options.language_edition,
                options.target,
                &feature_flags,
                &dependency_versions,
                &requirements,
            )?;
            context
                .module_versions
                .insert(uri.clone(), record.module_version);
        }
        Ok(())
    }

    fn canonical_graph_imports(
        &self,
        index: &SourceIndex,
        uri: &str,
        context: &mut CanonicalGraphCompilation<'_>,
    ) -> MResult<Vec<crate::resolver::CanonicalResolvedImport>> {
        use crate::resolver::CanonicalResolvedImport;
        let mut imports = Vec::new();
        for declaration in index.program_imports() {
            if !import_may_resolve_source_dependency(&declaration) {
                continue;
            }
            let request = source_request_for_import(&declaration, Some(uri));
            let Some(dependency) = self.source_resolver.resolve(&request)? else {
                if import_requires_source_dependency(&declaration) {
                    return Err(MechError::new(
                        RuntimeModuleDependencyMissingError {
                            module: uri.to_owned(),
                            specifier: request.specifier,
                            referrer: request.referrer,
                        },
                        None,
                    ));
                }
                continue;
            };
            let dependency = admit_resolved_canonical_source(dependency)?;
            let dependency_document = dependency.source_document().ok_or_else(|| {
                canonical_compilation_error("canonical dependency has no retained document")
            })?;
            let source_hash = canonical_document_dependency_hash(dependency_document)?;
            if context
                .source_dependencies
                .insert(dependency.canonical_uri.clone(), source_hash)
                .is_some_and(|previous| previous != source_hash)
            {
                return Err(canonical_compilation_error(format!(
                    "canonical dependency {} changed during compilation",
                    dependency.canonical_uri
                )));
            }
            if !context.exports.contains_key(&dependency.canonical_uri) {
                let program =
                    self.compile_canonical_graph_document(&dependency, context, true, false)?;
                let artifact = program.compile_artifact_with_external_contracts(
                    &ResidentExternalContractResolver::new(self.resources),
                )?;
                let mut instance = activate_external(
                    ReactiveInstanceId::new(0x4344_4550, 0),
                    &artifact,
                    &self.function_catalog,
                    &ActivationFacts::default(),
                    ResidentIntegrityMode::Checked,
                )
                .map_err(|error| {
                    canonical_compilation_error(format!(
                        "canonical dependency activation failed: {error:?}"
                    ))
                })?;
                let prepared = instance.prepare_initial_turn(&[]).map_err(|error| {
                    canonical_compilation_error(format!(
                        "canonical dependency execution failed: {error:?}"
                    ))
                })?;
                let values = program
                    .document_exports()
                    .iter()
                    .map(|export| {
                        let value = crate::RuntimeValueSnapshot::from_value(
                            prepared
                                .copied_output(export.output as usize)
                                .map_err(|error| {
                                    canonical_compilation_error(format!(
                                        "canonical dependency export failed: {error:?}"
                                    ))
                                })?,
                        )?;
                        Ok((export.name.clone(), value))
                    })
                    .collect::<MResult<BTreeMap<_, _>>>()?;
                prepared.abort();
                context
                    .exports
                    .insert(dependency.canonical_uri.clone(), values);
            }
            imports.push(CanonicalResolvedImport {
                declaration,
                exports: context.exports[&dependency.canonical_uri].clone(),
                canonical_uri: dependency.canonical_uri,
            });
        }
        Ok(imports)
    }

    fn canonical_document_resources(
        &self,
        index: &SourceIndex,
        document: &mech_syntax::document::DocumentSyntax,
    ) -> MResult<(
        BTreeMap<String, mech_core::SchemaBody>,
        BTreeMap<String, ExecutionResourceRequest>,
        BTreeMap<String, ExecutionResourceRequest>,
        BTreeMap<String, Value>,
    )> {
        self.canonical_document_resources_with_read_planner(index, document, |request| {
            self.resources.plan_read(request).map(Some)
        })
    }

    fn canonical_document_resources_with_read_planner(
        &self,
        index: &SourceIndex,
        document: &mech_syntax::document::DocumentSyntax,
        mut plan_read: impl FnMut(RuntimeResourceReadRequest) -> MResult<Option<Value>>,
    ) -> MResult<(
        BTreeMap<String, mech_core::SchemaBody>,
        BTreeMap<String, ExecutionResourceRequest>,
        BTreeMap<String, ExecutionResourceRequest>,
        BTreeMap<String, Value>,
    )> {
        use mech_syntax::document::{AstNode, ContextSendSyntax, VariableStemSyntax};

        let imports = index.program_imports();
        let mut contexts = index.program_contexts();
        self.materialize_inline_context_imports(&imports, &mut contexts)?;
        let bindings = resolve_canonical_context_bindings(&contexts)?;
        let operations = canonical_resource_send_operations(&contexts, &bindings);

        let mut input_schemas = BTreeMap::new();
        let mut reads = BTreeMap::new();
        let mut planned_reads = BTreeMap::new();
        for reference in index.program_address_references() {
            let (context_name, base_uri) = bindings.get(&reference.target).ok_or_else(|| {
                canonical_compilation_error(format!(
                    "canonical resource read references unknown context @{}",
                    reference.target,
                ))
            })?;
            let key = RuntimeResourceKey::new(base_uri, &reference.name)?;
            let request = ExecutionResourceRequest {
                base_uri: key.base_uri.clone(),
                path: key.path.clone(),
                context_name: context_name.clone(),
                operation: "read".to_owned(),
                intent: ResourceIntent::Read,
                delivery: mech_core::ResourceDelivery::Live,
            };
            let name = format!("@{}/{}", reference.target, reference.name);
            reads.insert(name.clone(), request);
            if let Some(value) = plan_read(RuntimeResourceReadRequest {
                base_uri: key.base_uri,
                path: key.path,
                context_name: context_name.clone(),
            })
            .map_err(classify_source_planning)?
            {
                input_schemas.insert(name.clone(), canonical_planned_read_schema(&value)?);
                planned_reads.insert(name, value);
            }
        }

        let mut writes = BTreeMap::new();
        let mut pending = vec![document.syntax().clone()];
        while let Some(node) = pending.pop() {
            if matches!(
                node.kind(),
                mech_syntax::document::SyntaxKind::MikaSection
                    | mech_syntax::document::SyntaxKind::InlineMechCode
            ) {
                continue;
            }
            if let Some(fence) = mech_syntax::document::CodeBlockSyntax::cast(node.clone()) {
                if !matches!(
                    fence.info().map(|info| info.scope),
                    Some(mech_syntax::document::CodeFenceScope::Root)
                ) {
                    continue;
                }
            }
            if let Some(assignment) =
                mech_syntax::document::VariableAssignSyntax::cast(node.clone())
            {
                if let Some(target) = assignment.target() {
                    if let Some(mech_syntax::document::SliceStemSyntax::Context(path)) =
                        target.stem()
                    {
                        let context = path
                            .context()
                            .and_then(|value| value.syntax().text().ok())
                            .ok_or_else(|| {
                                canonical_compilation_error("resource assignment has no context")
                            })?;
                        let address = path
                            .address()
                            .and_then(|value| value.syntax().text().ok())
                            .ok_or_else(|| {
                                canonical_compilation_error("resource assignment has no path")
                            })?;
                        let (context_name, base_uri) = bindings.get(&context).ok_or_else(|| {
                            canonical_compilation_error(format!(
                                "resource assignment references unknown context @{context}"
                            ))
                        })?;
                        let key = RuntimeResourceKey::new(base_uri, &address)?;
                        writes.insert(
                            format!("=@{context}/{address}"),
                            ExecutionResourceRequest {
                                base_uri: key.base_uri,
                                path: key.path,
                                context_name: context_name.clone(),
                                operation: "write".to_owned(),
                                intent: ResourceIntent::Assign,
                                delivery: mech_core::ResourceDelivery::Snapshot,
                            },
                        );
                        // Its value may itself contain resource reads, already covered by the index.
                        continue;
                    }
                }
            }
            if let Some(send) = ContextSendSyntax::cast(node.clone()) {
                let target = send.target().ok_or_else(|| {
                    canonical_compilation_error("canonical resource send is missing its target")
                })?;
                let Some(VariableStemSyntax::Context(path)) = target.stem() else {
                    return Err(canonical_compilation_error(
                        "canonical resource send target is not context-addressed",
                    ));
                };
                let context = path
                    .context()
                    .and_then(|value| value.syntax().text().ok())
                    .ok_or_else(|| canonical_compilation_error("resource send has no context"))?;
                let address = path
                    .address()
                    .and_then(|value| value.syntax().text().ok())
                    .ok_or_else(|| canonical_compilation_error("resource send has no path"))?;
                let (context_name, base_uri) = bindings.get(&context).ok_or_else(|| {
                    canonical_compilation_error(format!(
                        "canonical resource send references unknown context @{context}",
                    ))
                })?;
                let key = RuntimeResourceKey::new(base_uri, &address)?;
                let mut request = ExecutionResourceRequest {
                    base_uri: key.base_uri,
                    path: key.path,
                    context_name: context_name.clone(),
                    operation: "write".to_owned(),
                    intent: ResourceIntent::Send,
                    delivery: mech_core::ResourceDelivery::Snapshot,
                };
                if let Some(operation) = declared_resource_send_operation(&request, &operations)? {
                    request.operation = operation.to_owned();
                }
                writes.insert(format!("@{context}/{address}"), request);
                continue;
            }
            pending.extend(node.children());
        }
        Ok((input_schemas, reads, writes, planned_reads))
    }

    #[cfg(feature = "compute")]
    fn compile_canonical_mixed_resolved_root(
        &self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<MixedProgramCompilation> {
        let resolved = admit_resolved_canonical_source(resolved)?;
        let document = resolved
            .source_document()
            .ok_or_else(|| canonical_compilation_error("mixed root has no retained document"))?;
        let index = document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        let mut context = CanonicalGraphCompilation::new(Some(options));
        self.register_nominal_declarations(document, &mut context)?;
        context.active.push(resolved.canonical_uri.clone());
        let imports =
            self.canonical_graph_imports(&index.root, &resolved.canonical_uri, &mut context)?;
        self.canonical_graph_module_identity(&resolved, &imports, &mut context)?;
        let mut mixed = self.compile_mixed_document_with_imports(
            document,
            &imports,
            context.enum_qualifiers()?,
        )?;
        mixed.source_dependencies = context.source_dependencies;
        Ok(mixed)
    }

    #[cfg(feature = "compute")]
    fn compile_mixed_document(
        &self,
        document: &SourceDocument,
    ) -> MResult<MixedProgramCompilation> {
        self.compile_mixed_document_with_imports(document, &[], BTreeMap::new())
    }

    #[cfg(feature = "compute")]
    fn compile_mixed_document_with_imports(
        &self,
        document: &SourceDocument,
        imports: &[crate::resolver::CanonicalResolvedImport],
        imported_enum_qualifiers: BTreeMap<mech_core::NominalKey, String>,
    ) -> MResult<MixedProgramCompilation> {
        let index = document
            .index()
            .map_err(|error| MechError::new(error, None))?;
        let external_input_names = canonical_declared_compute_inputs(&index.root)?;
        let retained_outputs = canonical_declared_compute_outputs(&index.root)?;
        let declared_compute_reads = canonical_declared_compute_reads(&index.root)?;
        // Retained host paths name flattened interface leaves (e.g. result.1.0).
        // Source publication owns their lexical producers; the interface below
        // remains the authority for validating exact leaf names.
        let published_compute_bindings = retained_outputs
            .iter()
            .map(|name| name.split('.').next().unwrap_or(name).to_owned())
            .collect();
        let (mut input_schemas, resource_reads, resource_writes, mut planned_reads) = self
            .canonical_document_resources_with_read_planner(
                &index.root,
                &document.document(),
                |request| {
                    if is_compute_kernel_base(&request.base_uri) {
                        Ok(None)
                    } else {
                        self.resources.plan_read(request).map(Some)
                    }
                },
            )?;
        let imported = crate::resolver::canonical_import_values(
            &index.root,
            &crate::resolver::SourceScope::Program,
            imports,
        )
        .map_err(|error| canonical_compilation_error(error.to_string()))?;
        for (name, value) in &imported {
            input_schemas.insert(
                name.clone(),
                ValueCell::from_snapshot(value.to_value())?.closed_schema_body()?,
            );
        }
        let modules = imports
            .iter()
            .filter_map(|import| {
                import
                    .declaration
                    .module
                    .clone()
                    .or_else(|| module_namespace_for_import(&import.declaration))
            })
            .collect();
        let mut programs = canonical_frontend(document)
            .with_imported_enum_qualifiers(imported_enum_qualifiers)
            .prepare_mixed_document_with_planning_contract(
                &document.document(),
                Arc::clone(&self.function_catalog),
                input_schemas,
                resource_writes,
                &external_input_names,
                &published_compute_bindings,
                &modules,
            )
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        if !retained_outputs.is_empty() {
            programs.compute = programs
                .compute
                .project_compute_output_paths(&retained_outputs)
                .map_err(|error| compute_planning_error(error.message))?;
        }

        let bind_imports = |program: CanonicalSourceProgram| -> MResult<CanonicalSourceProgram> {
            let values = program
                .program()
                .inputs
                .iter()
                .enumerate()
                .filter_map(|(ordinal, input)| {
                    imported
                        .get(&input.name)
                        .map(|value| (ordinal as u32, value.to_value()))
                })
                .collect::<Vec<_>>();
            program
                .bind_input_constants(&values)
                .map_err(|error| canonical_compilation_error(error.to_string()))
        };
        programs.compute = bind_imports(programs.compute)?;
        programs.compute_initializers = bind_imports(programs.compute_initializers)?;

        let initial_inputs = if external_input_names.is_empty() {
            BTreeMap::new()
        } else {
            let compute_initializer_artifact = programs
                .compute_initializers
                .compile_artifact_with_external_contracts(
                    &ResidentExternalContractResolver::new(self.resources),
                )?;
            execute_named_canonical_outputs(
                &compute_initializer_artifact,
                &self.function_catalog,
                &external_input_names,
            )?
        };
        let compute_artifact = programs.compute.compile_artifact_with_external_contracts(
            &ResidentExternalContractResolver::new(self.resources),
        )?;
        let compute = assemble_compute_region(
            ProgramArtifactCompilationProduct::from_artifact(compute_artifact),
            initial_inputs,
            &programs.region_name,
        )?;

        if !retained_outputs.is_empty() {
            let published = compute
                .interface
                .outputs
                .iter()
                .map(|port| port.name.as_ref())
                .collect::<BTreeSet<_>>();
            let retained = retained_outputs.iter().map(String::as_str).collect();
            if published != retained {
                if let Some(name) = retained_outputs
                    .iter()
                    .find(|name| !published.contains(name.as_str()))
                {
                    return Err(compute_planning_error(format!(
                        "unknown sampled compute output `{name}`"
                    )));
                }
                return Err(compute_planning_error(
                    "compute output projection exposed an undeclared sampled output",
                ));
            }
        }

        // A declared compute capability is part of the interface contract even
        // when no executable coordinator expression reads it. Validate every
        // literal path here so misspelled telemetry cannot become conditionally
        // valid based on source reachability.
        for path in &declared_compute_reads {
            let key = RuntimeResourceKey::new("compute://declared/kernel", path)?;
            plan_compute_read(&compute.interface, &key).map_err(classify_source_planning)?;
        }

        let mut compute_read_schemas = BTreeMap::new();
        for (name, request) in &resource_reads {
            if is_compute_kernel_base(&request.base_uri) {
                let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
                let value = plan_compute_read(&compute.interface, &key)
                    .map_err(classify_source_planning)?;
                compute_read_schemas.insert(name.clone(), canonical_planned_read_schema(&value)?);
                planned_reads.insert(name.clone(), value);
            }
        }
        let coordinator = bind_imports(
            programs
                .coordinator
                .compile(compute_read_schemas)
                .map_err(|error| canonical_compilation_error(error.to_string()))?,
        )?;

        let read_bindings = coordinator
            .program()
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(ordinal, input)| {
                planned_reads
                    .get(&input.name)
                    .cloned()
                    .map(|value| (ordinal as u32, value))
            })
            .collect::<Vec<_>>();
        let planning_coordinator = coordinator
            .clone()
            .bind_input_constants(&read_bindings)
            .map_err(|error| canonical_compilation_error(error.to_string()))?;
        let compute_contracts = CompilerExternalContractResolver {
            providers: ResidentExternalContractResolver::new(self.resources),
            compute: true,
        };
        let planning_artifact =
            planning_coordinator.compile_artifact_with_external_contracts(&compute_contracts)?;
        let activation_inputs = capture_canonical_compute_activation_inputs(
            &planning_artifact,
            &self.function_catalog,
            &compute.interface,
            self.resources,
        )?;

        let mut coordinator = coordinator;
        for (name, request) in resource_reads {
            // The source index also sees reads in local function bodies. Those
            // bodies contribute inputs only when inlined, so bind only reads
            // that survived canonical coordinator lowering.
            if coordinator
                .program()
                .inputs
                .iter()
                .any(|input| input.name == name)
            {
                coordinator = coordinator
                    .bind_resource_input(&name, request)
                    .map_err(|error| canonical_compilation_error(error.to_string()))?;
            }
        }
        let coordinator =
            coordinator.compile_artifact_with_external_contracts(&compute_contracts)?;
        Ok(MixedProgramCompilation {
            coordinator: ProgramArtifactCompilationProduct::from_artifact(coordinator),
            compute,
            activation_inputs,
            retained_outputs,
            source_dependencies: BTreeMap::new(),
        })
    }

    /// Resolve context imports for inline source through the same configured
    /// host-interface and module-manifest owners used by rooted compilation.
    /// Inline compilation has no persisted module record on which to attach
    /// these declarations, so keep the materialized declarations local to the
    /// compiler session.
    fn materialize_inline_context_imports(
        &self,
        imports: &[SourceImportDeclaration],
        contexts: &mut Vec<crate::SourceContextDeclaration>,
    ) -> MResult<()> {
        for import in imports {
            let Some(SourceImportAlias::Context(alias)) = &import.alias else {
                continue;
            };
            if contexts.iter().any(|context| context.name == *alias) {
                return Err(mech_core::MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "compile_source",
                        reason: format!(
                            "context import duplicates an existing context binding `{alias}`"
                        ),
                    },
                    None,
                ));
            }
            let module = import
                .module
                .as_deref()
                .ok_or_else(|| invalid_context_import(&import.specifier, "module"))?;
            let item = import
                .item
                .as_deref()
                .ok_or_else(|| invalid_context_import(&import.specifier, "item"))?;
            let target = format!("{module}/{item}");
            let (base_uri, operations) =
                if let Some(export) = self.host_interfaces.resolve_optional(&target)? {
                    (export.base_uri.clone(), export.operations.clone())
                } else {
                    let export = self.module_manifests.context_export(module, item)?;
                    (export.base_uri.clone(), export.operations.clone())
                };
            contexts.push(crate::SourceContextDeclaration {
                name: alias.clone(),
                base: crate::SourceContextBase::ResourceUri(base_uri),
                capabilities: operations
                    .into_iter()
                    .map(|operation| crate::SourceContextCapability {
                        operation,
                        scope: crate::SourceContextCapabilityScope::Wildcard,
                    })
                    .collect(),
            });
        }
        Ok(())
    }
}

fn preflight_canonical_effect_payloads(
    prepared: &mech_engine::resident::PreparedResidentTurn<'_>,
    artifact: &ProgramArtifact,
    resources: &RuntimeResourceRegistry,
) -> MResult<()> {
    for effect in prepared.effect_intents() {
        let Some(ApplicationRequirement::Resource(request)) =
            artifact.requirements().get(effect.requirement)
        else {
            continue;
        };
        #[cfg(feature = "compute")]
        if is_compute_kernel_base(&request.base_uri) {
            continue;
        }
        let intent = match request.intent {
            ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
            ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
            ResourceIntent::Read => continue,
        };
        resources.plan_write(RuntimeResourceWriteCommand {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: RuntimeCapabilityOperation::from_name(request.operation.clone())?,
            value: prepared.materialize_effect_payload(effect.ordinal)?,
            intent,
        })?;
    }
    Ok(())
}

fn execute_named_canonical_outputs(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
    names: &BTreeSet<String>,
) -> MResult<BTreeMap<String, RuntimeHostInputValue>> {
    let mut instance = activate_external(
        ReactiveInstanceId::new(0x4349_4e49, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        canonical_compilation_error(format!(
            "canonical initializer activation failed: {error:?}"
        ))
    })?;
    let prepared = instance.prepare_initial_turn(&[]).map_err(|error| {
        canonical_compilation_error(format!("canonical initializer execution failed: {error:?}"))
    })?;
    let values = names
        .iter()
        .map(|name| {
            let output = artifact
                .outputs()
                .iter()
                .position(|output| {
                    output.name == mech_engine::encode_interactive_symbol_output_name(name)
                })
                .or_else(|| {
                    artifact
                        .outputs()
                        .iter()
                        .position(|output| output.name == *name)
                })
                .ok_or_else(|| {
                    canonical_compilation_error(format!(
                        "canonical initializer {name} was not published"
                    ))
                })?;
            let value = prepared.copied_output(output).map_err(|error| {
                canonical_compilation_error(format!(
                    "canonical initializer {name} could not be materialized: {error:?}"
                ))
            })?;
            RuntimeHostInputValue::from_numeric_value(&value).map(|value| (name.clone(), value))
        })
        .collect::<MResult<BTreeMap<_, _>>>()?;
    prepared.abort();
    Ok(values)
}

#[cfg(feature = "compute")]
fn capture_canonical_compute_activation_inputs(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
    interface: &ComputeRegionInterface,
    resources: &RuntimeResourceRegistry,
) -> MResult<BTreeMap<String, ComputeValue>> {
    let mut instance = activate_external(
        ReactiveInstanceId::new(0x4343_4f4f, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        canonical_compilation_error(format!(
            "canonical compute coordinator activation failed: {error:?}"
        ))
    })?;
    let prepared = instance.prepare_initial_turn(&[]).map_err(|error| {
        canonical_compilation_error(format!(
            "canonical compute coordinator planning failed: {error:?}"
        ))
    })?;
    preflight_canonical_effect_payloads(&prepared, artifact, resources)?;
    let effects = prepared
        .effect_intents()
        .map(|intent| (intent.ordinal, intent.requirement))
        .collect::<Vec<_>>();
    let mut activation_inputs = BTreeMap::new();
    for (ordinal, requirement) in effects {
        let Some(ApplicationRequirement::Resource(request)) =
            artifact.requirements().get(requirement)
        else {
            continue;
        };
        if !is_compute_kernel_base(&request.base_uri) {
            continue;
        }
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        let payload = prepared.materialize_effect_payload(ordinal)?;
        if let Some((name, value)) =
            plan_compute_write(interface, &key, RuntimeResourceWriteIntent::Send, &payload)?
        {
            activation_inputs.insert(name, value);
        }
    }
    prepared.abort();
    Ok(activation_inputs)
}

#[cfg(feature = "compute")]
fn canonical_declared_compute_inputs(index: &SourceIndex) -> MResult<BTreeSet<String>> {
    canonical_declared_compute_paths(index, "write", "input/")
}

#[cfg(feature = "compute")]
fn canonical_declared_compute_outputs(index: &SourceIndex) -> MResult<BTreeSet<String>> {
    canonical_declared_compute_paths(index, "read", "sample/")
}

#[cfg(feature = "compute")]
fn canonical_declared_compute_reads(index: &SourceIndex) -> MResult<BTreeSet<String>> {
    canonical_declared_compute_paths(index, "read", "")
}

#[cfg(feature = "compute")]
fn canonical_declared_compute_paths(
    index: &SourceIndex,
    operation: &str,
    prefix: &str,
) -> MResult<BTreeSet<String>> {
    index.validate_address_targets()?;
    Ok(index
        .all_contexts()
        .into_iter()
        .filter(|context| {
            matches!(
                &context.base,
                SourceContextBase::ResourceUri(uri)
                    if uri.starts_with("compute://") && uri.ends_with("/kernel")
            )
        })
        .flat_map(|context| context.capabilities)
        .filter(|capability| capability.operation == operation)
        .filter_map(|capability| match capability.scope {
            SourceContextCapabilityScope::Path(path) => (!path.contains('*'))
                .then(|| path.strip_prefix(prefix).map(str::to_owned))
                .flatten(),
            SourceContextCapabilityScope::Wildcard => None,
        })
        .collect())
}

#[cfg(feature = "compute")]
fn assemble_compute_region(
    compute: ProgramArtifactCompilationProduct,
    initial_inputs: BTreeMap<String, RuntimeHostInputValue>,
    expected_region_name: &str,
) -> MResult<ComputeRegionCompilation> {
    let artifact = compute.into_artifact();
    if artifact.compute_regions().len() != 1
        || artifact.compute_regions()[0].name.as_ref() != expected_region_name
    {
        return Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "compile_mixed_program",
                reason: format!(
                    "isolated section `{expected_region_name}` did not produce exactly that compute region"
                ),
            },
            None,
        ));
    }
    let declaration = artifact.compute_regions()[0].clone();
    let interface =
        build_compute_region_interface(&artifact, Some(&declaration)).map_err(|error| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "compile_mixed_program",
                    reason: format!("compute interface construction failed: {error}"),
                },
                None,
            )
        })?;
    let initializers = compute_initializers(&interface, initial_inputs)?;
    Ok(ComputeRegionCompilation {
        declaration,
        artifact,
        interface,
        initializers,
    })
}

#[cfg(feature = "compute")]
fn compute_initializers(
    interface: &ComputeRegionInterface,
    mut values: BTreeMap<String, RuntimeHostInputValue>,
) -> MResult<ComputeInitializerSet> {
    let mut initializers = BTreeMap::new();
    for port in &interface.inputs {
        let value = values.remove(port.name.as_ref()).ok_or_else(|| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "compile_mixed_program",
                    reason: format!(
                        "compute input `{}` has no declaration-time initializer",
                        port.name
                    ),
                },
                None,
            )
        })?;
        let value = match value {
            RuntimeHostInputValue::F32(value) => ComputeValue::ScalarF32(value),
            RuntimeHostInputValue::F64(value) => {
                ComputeValue::ScalarF32(narrow_compute_input_f64(port.name.as_ref(), value)?)
            }
            RuntimeHostInputValue::F32Matrix {
                rows,
                columns,
                values,
            } => ComputeValue::TensorF32 {
                dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values: Arc::from(values),
            },
            RuntimeHostInputValue::F64Matrix {
                rows,
                columns,
                values,
            } => ComputeValue::TensorF32 {
                dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
                layout: TensorLayout::RowMajor,
                values: values
                    .into_iter()
                    .map(|value| narrow_compute_input_f64(port.name.as_ref(), value))
                    .collect::<MResult<Vec<_>>>()?
                    .into(),
            },
            value => {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "compile_mixed_program",
                        reason: format!(
                            "compute input `{}` must be fixed-shape f32 data, found {value:?}",
                            port.name
                        ),
                    },
                    None,
                ));
            }
        };
        let normalized = port.normalize_value(value).map_err(|error| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "compile_mixed_program",
                    reason: format!("compute input `{}` is invalid: {error}", port.name),
                },
                None,
            )
        })?;
        initializers.insert(port.id, normalized);
    }
    if !values.is_empty() {
        return Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "compile_mixed_program",
                reason: format!(
                    "source declares compute inputs that are not connected to the region interface: {}",
                    values.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            },
            None,
        ));
    }
    Ok(ComputeInitializerSet::new(initializers))
}

#[cfg(feature = "compute")]
fn narrow_compute_input_f64(port: &str, value: f64) -> MResult<f32> {
    mech_compute::narrow_compute_f64(value).map_err(|value| {
        MechError::new(
            RuntimeInvalidOperationError {
                operation: "compile_mixed_program",
                reason: format!(
                    "compute input `{port}` contains f64 value {value} outside the f32 range"
                ),
            },
            None,
        )
    })
}

/// Preserve caller order for independent roots while promoting any requested
/// dependency ahead of its consumer. That lets an explicit dependency execute
/// exactly once in the shared program. Caller-visible outputs are published
/// separately after planning so this topological order cannot reorder them.

fn resolve_canonical_context_bindings(
    contexts: &[crate::SourceContextDeclaration],
) -> MResult<BTreeMap<String, (String, String)>> {
    let mut declarations = BTreeMap::new();
    for context in contexts {
        if declarations
            .insert(context.name.clone(), &context.base)
            .is_some()
        {
            return Err(canonical_compilation_error(format!(
                "canonical context `@{}` is declared more than once",
                context.name,
            )));
        }
    }

    fn resolve(
        name: &str,
        declarations: &BTreeMap<String, &SourceContextBase>,
        resolved: &mut BTreeMap<String, (String, String)>,
        active: &mut BTreeSet<String>,
    ) -> MResult<(String, String)> {
        if let Some(binding) = resolved.get(name) {
            return Ok(binding.clone());
        }
        if !active.insert(name.to_owned()) {
            return Err(canonical_compilation_error(format!(
                "canonical context inheritance contains a cycle at `@{name}`",
            )));
        }
        let base = declarations.get(name).ok_or_else(|| {
            canonical_compilation_error(format!(
                "canonical context `@{name}` references an unknown base context",
            ))
        })?;
        let binding = match base {
            SourceContextBase::ResourceUri(base_uri) => {
                if base_uri.is_empty() {
                    return Err(canonical_compilation_error(format!(
                        "canonical context `@{name}` has an empty resource URI",
                    )));
                }
                let context_name = base_uri
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(base_uri)
                    .to_owned();
                (context_name, (*base_uri).clone())
            }
            SourceContextBase::Context(base) => resolve(base, declarations, resolved, active)?,
        };
        active.remove(name);
        resolved.insert(name.to_owned(), binding.clone());
        Ok(binding)
    }

    let mut resolved = BTreeMap::new();
    for name in declarations.keys() {
        resolve(name, &declarations, &mut resolved, &mut BTreeSet::new())?;
    }
    Ok(resolved)
}

fn canonical_resource_send_operations(
    contexts: &[crate::SourceContextDeclaration],
    bindings: &BTreeMap<String, (String, String)>,
) -> Vec<CompiledResourceSendOperation> {
    contexts
        .iter()
        .filter_map(|context| {
            bindings.get(&context.name).map(|(_, base_uri)| {
                context.capabilities.iter().filter_map(move |capability| {
                    (capability.operation != "read").then(|| CompiledResourceSendOperation {
                        base_uri: base_uri.clone(),
                        path: match &capability.scope {
                            crate::SourceContextCapabilityScope::Path(path) => Some(path.clone()),
                            crate::SourceContextCapabilityScope::Wildcard => None,
                        },
                        operation: capability.operation.clone(),
                    })
                })
            })
        })
        .flatten()
        .collect()
}

fn resource_send_path_specificity(declared: Option<&str>, requested: &str) -> Option<u8> {
    match declared {
        None => Some(0),
        Some(path) if path == requested => Some(2),
        Some(path) => {
            let prefix = path.strip_suffix("/*")?;
            requested
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('/'))
                .then_some(1)
        }
    }
}

fn declared_resource_send_operation<'a>(
    request: &ExecutionResourceRequest,
    operations: &'a [CompiledResourceSendOperation],
) -> MResult<Option<&'a str>> {
    let mut selected = None;
    for specificity in [2, 1, 0] {
        for declaration in operations.iter().filter(|declaration| {
            declaration.base_uri == request.base_uri
                && resource_send_path_specificity(declaration.path.as_deref(), &request.path)
                    == Some(specificity)
        }) {
            match selected {
                None => selected = Some(declaration.operation.as_str()),
                Some(operation) if operation == declaration.operation => {}
                Some(operation) => {
                    return Err(route_failure(
                        ResidentRouteFailureClass::InvalidArtifact,
                        format!(
                            "resource send `{}/{}` resolves to both `{operation}` and `{}`",
                            request.base_uri, request.path, declaration.operation,
                        ),
                    ));
                }
            }
        }
        if selected.is_some() {
            break;
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod resource_send_scope_tests {
    use super::resource_send_path_specificity;

    #[test]
    fn prefix_wildcard_resource_paths_match_descendants_with_lower_specificity() {
        assert_eq!(
            resource_send_path_specificity(Some("messages/42"), "messages/42"),
            Some(2)
        );
        assert_eq!(
            resource_send_path_specificity(Some("messages/*"), "messages/42"),
            Some(1)
        );
        assert_eq!(
            resource_send_path_specificity(Some("messages/*"), "messages/42/body"),
            Some(1)
        );
        assert_eq!(
            resource_send_path_specificity(Some("messages/*"), "messages"),
            None
        );
        assert_eq!(
            resource_send_path_specificity(Some("other/*"), "messages/42"),
            None
        );
        assert_eq!(resource_send_path_specificity(None, "messages/42"), Some(0));
    }
}

fn invalid_context_import(specifier: &str, missing: &'static str) -> mech_core::MechError {
    mech_core::MechError::new(
        RuntimeInvalidOperationError {
            operation: "compile_root",
            reason: format!("context import `{specifier}` is missing {missing} metadata"),
        },
        None,
    )
}

#[cfg(feature = "compute")]
struct CompilerExternalContractResolver<'a> {
    providers: ResidentExternalContractResolver<'a>,
    compute: bool,
}

#[cfg(feature = "compute")]
impl mech_engine::ExternalRequirementContractResolver for CompilerExternalContractResolver<'_> {
    fn resolve_external_contract(
        &self,
        requirement: &ApplicationRequirement,
    ) -> MResult<Option<&'static OperationContractDeclaration>> {
        let ApplicationRequirement::Resource(request) = requirement else {
            return mech_engine::ExternalRequirementContractResolver::resolve_external_contract(
                &self.providers,
                requirement,
            );
        };
        if self.compute && is_compute_kernel_base(&request.base_uri) {
            return Ok(match request.intent {
                ResourceIntent::Read => Some(crate::resource_observation_contract()),
                ResourceIntent::Send => Some(crate::compute_effect_contract()),
                ResourceIntent::Assign => None,
            });
        }
        mech_engine::ExternalRequirementContractResolver::resolve_external_contract(
            &self.providers,
            requirement,
        )
    }
}

fn canonical_planned_read_schema(value: &Value) -> MResult<mech_core::SchemaBody> {
    let schemas = value
        .schemas()
        .ok_or_else(|| canonical_compilation_error("planned resource read has no schema owner"))?;
    let schema = schemas.get(value.schema()).ok_or_else(|| {
        canonical_compilation_error("planned resource read schema is unavailable")
    })?;
    // Resolve provider/interface shape parameters before crossing schema arenas.
    schema.closed_body(value.shape())
}

#[cfg(feature = "compute")]
fn is_compute_kernel_base(base_uri: &str) -> bool {
    base_uri
        .strip_prefix("compute://")
        .is_some_and(|instance| !instance.is_empty() && instance.ends_with("/kernel"))
}

#[cfg(feature = "compute")]
fn plan_compute_read(
    interface: &ComputeRegionInterface,
    key: &RuntimeResourceKey,
) -> MResult<Value> {
    if let Some(name) = key.path.strip_prefix("sample/") {
        let port = interface
            .outputs
            .iter()
            .find(|port| port.name.as_ref() == name)
            .ok_or_else(|| {
                compute_planning_error(format!("unknown sampled compute output `{name}`"))
            })?;
        let elements = port.elements().map_err(|error| {
            compute_planning_error(format!(
                "sampled compute output `{name}` has an invalid shape: {error}"
            ))
        })?;
        return match port.dimensions.as_ref() {
            [] => RuntimeHostInputValue::F64(0.0).into_value(),
            [columns] => RuntimeHostInputValue::F64Matrix {
                rows: 1,
                columns: *columns as usize,
                values: vec![0.0; elements],
            }
            .into_value(),
            [rows, columns] => RuntimeHostInputValue::F64Matrix {
                rows: *rows as usize,
                columns: *columns as usize,
                values: vec![0.0; elements],
            }
            .into_value(),
            dimensions => Err(compute_planning_error(format!(
                "sampled compute output `{name}` has unsupported rank {}",
                dimensions.len(),
            ))),
        };
    }
    match key.path.as_str() {
        "backend" | "last-fault" => RuntimeHostInputValue::String(String::new()).into_value(),
        "turns" | "dispatch-ms" | "fault-count" => RuntimeHostInputValue::F64(0.0).into_value(),
        path => Err(compute_planning_error(format!(
            "unknown compute telemetry path `{path}`"
        ))),
    }
}

#[cfg(feature = "compute")]
fn plan_compute_write(
    interface: &ComputeRegionInterface,
    key: &RuntimeResourceKey,
    intent: RuntimeResourceWriteIntent,
    value: &Value,
) -> MResult<Option<(String, ComputeValue)>> {
    if intent != RuntimeResourceWriteIntent::Send {
        return Err(compute_planning_error(
            "compute inputs and turns are effects; use <-",
        ));
    }
    if key.path == "turn" {
        return Ok(None);
    }
    let name = key.path.strip_prefix("input/").ok_or_else(|| {
        compute_planning_error(format!("unknown compute input path `{}`", key.path))
    })?;
    let port = interface.input_named(name).ok_or_else(|| {
        compute_planning_error(format!("unknown compute input path `{}`", key.path))
    })?;
    let value = RuntimeHostInputValue::from_numeric_value(value).and_then(|value| match value {
        RuntimeHostInputValue::F32(value) => Ok(ComputeValue::ScalarF32(value)),
        RuntimeHostInputValue::F64(value) => Ok(ComputeValue::ScalarF32(narrow_compute_input_f64(
            port.name.as_ref(),
            value,
        )?)),
        RuntimeHostInputValue::F32Matrix {
            rows,
            columns,
            values,
        } => Ok(ComputeValue::TensorF32 {
            dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
            layout: TensorLayout::RowMajor,
            values: Arc::from(values),
        }),
        RuntimeHostInputValue::F64Matrix {
            rows,
            columns,
            values,
        } => Ok(ComputeValue::TensorF32 {
            dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
            layout: TensorLayout::RowMajor,
            values: values
                .into_iter()
                .map(|value| narrow_compute_input_f64(port.name.as_ref(), value))
                .collect::<MResult<Vec<_>>>()?
                .into(),
        }),
        value => Err(compute_planning_error(format!(
            "compute input `{}` requires fixed-shape f32 data, found `{value:?}`",
            port.name
        ))),
    })?;
    let (value, _) = port
        .normalize_broadcast_value(value, None)
        .map_err(|error| {
            compute_planning_error(format!("compute input `{}` is invalid: {error}", port.name))
        })?;
    Ok(Some((name.to_owned(), value)))
}

#[cfg(feature = "compute")]
fn compute_planning_error(message: impl Into<String>) -> MechError {
    route_failure(
        ResidentRouteFailureClass::InvalidArtifact,
        format!("compute boundary planning failed: {}", message.into()),
    )
}

fn classify_source_planning(error: mech_core::MechError) -> mech_core::MechError {
    if error.kind_as::<ResidentRouteFailure>().is_some() {
        return error;
    }
    let class = if error.kind_as::<RuntimeResourceProviderNotFound>().is_some() {
        ResidentRouteFailureClass::ProviderUnavailable
    } else if error
        .kind_as::<ReactiveComprehensionStructureUnsupported>()
        .is_some()
    {
        ResidentRouteFailureClass::SemanticUnsupported
    } else {
        ResidentRouteFailureClass::InvalidArtifact
    };
    route_failure(class, format!("resident source planning failed: {error:?}"))
}
