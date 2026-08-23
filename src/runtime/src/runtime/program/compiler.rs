//! Isolated compiler workspace for resident source products.
//!
//! `CompilerPlanningProgram` and `LegacyValue` are private compilation
//! coordinates. They never become runtime owners: each planning program is
//! local to one compilation and is dropped after finalization. Only immutable
//! compilation products and detached typed initialization values
//! escape this module.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};

#[cfg(feature = "compute")]
use mech_compute::{
    ComputeInitializerSet, ComputeRegionInterface, ComputeValue, TensorLayout,
    build_compute_region_interface,
};
use mech_core::{
    ApplicationRequirement, ExecutionHostFunctionRequest, ExecutionResourceRequest, LegacyValue,
    MResult, MechError, MechErrorKind, MechExecutionServices, MechSourceCode,
    ModuleManifestCatalog, OperationContractDeclaration, Ref, ResourceIntent,
};
#[cfg(feature = "compute")]
use mech_core::{Body, MechCode, Program, Section, SectionElement};
use mech_engine::{
    CompiledResourceSendOperation, CompilerPlanningConfig, CompilerPlanningProgram,
    ProgramArtifactCompilationProduct, ProgramCompilationProduct, root_document_output_ids,
};
#[cfg(feature = "compute")]
use mech_engine::{ComputeRegionDeclaration, ProgramArtifact};

use crate::{
    CapabilityRequest, HostInterfaceCatalog, ModuleBuildOptions, ModuleBuilder, ModuleVersionId,
    ResidentExternalContractResolver, ResolvedSource, RuntimeCapabilityOperation,
    RuntimeHostInputValue, RuntimeInvalidOperationError, RuntimeModuleDependencyCycleError,
    RuntimeModuleDependencyMissingError, RuntimeModuleExportNotFound, RuntimeModuleImportConflict,
    RuntimeResourceKey, RuntimeResourceProviderNotFound, RuntimeResourceReadRequest,
    RuntimeResourceRegistry, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
    SourceExportDeclaration, SourceImportAlias, SourceImportDeclaration, SourceImportKind,
    SourceIndex, SourceRequest, SourceResolver, import_may_resolve_source_dependency,
    import_requires_source_dependency, module_namespace_for_import, source_request_for_import,
};
#[cfg(feature = "compute")]
use crate::{SourceContextBase, SourceContextCapabilityScope};

use super::{ResidentRouteFailure, ResidentRouteFailureClass, route_failure, unsupported_route};

#[cfg(not(feature = "compute"))]
type ComputeRegionInterface = ();

#[derive(Clone, Copy)]
enum ProgramBytecodeEncoding {
    Semantic,
    FrozenV1ImplementationIdentities,
}

#[derive(Clone, Copy)]
enum RootOutputProjection {
    ObservableResults,
    ObservableResultsAndSymbols,
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

    /// Reproduces the checked-in bytecode-v1 compatibility corpus. Runtime
    /// products must otherwise use [`Self::compile_source`].
    #[doc(hidden)]
    pub fn compile_source_frozen_v1(&mut self, source: &str) -> MResult<ProgramCompilationProduct> {
        self.view().compile_source_frozen_v1(source)
    }

    pub fn compile_tree(
        &mut self,
        tree: &mech_core::Program,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_tree(tree)
    }

    /// Compiles an already parsed interactive document without requiring a
    /// formatter round-trip through source text.
    pub fn compile_interactive_tree(
        &mut self,
        tree: &mech_core::Program,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_interactive_tree(tree)
    }

    /// Compiles source for immediate artifact activation without returning or
    /// retaining a duplicate durable bytecode container.
    pub fn compile_source_artifact(
        &mut self,
        source: &str,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        let tree = mech_syntax::parser::parse(source.trim())?;
        self.compile_tree_artifact(&tree)
    }

    pub fn compile_tree_artifact(
        &mut self,
        tree: &mech_core::Program,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        self.view()
            .compile_tree_artifact_with_inputs(tree, &BTreeMap::new(), &BTreeSet::new())
    }

    /// Compiles one parsed tree with detached planning values and live input
    /// declarations supplied by the enclosing compute boundary.
    ///
    /// Planning values let the compiler specialize types and shapes; they do
    /// not implicitly become live ports. Only names in `external_input_names`
    /// remain replaceable at activation. This keeps initialization data and
    /// the live host interface separate and explicit.
    pub fn compile_tree_artifact_with_inputs(
        &mut self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        self.view()
            .compile_tree_artifact_with_inputs(tree, inputs, external_input_names)
    }

    /// Compiles one parsed tree and captures the declaration-time values of
    /// its explicitly live inputs during the same short-lived planning pass.
    ///
    /// The returned values are detached snapshots. Compiler cells and the
    /// planning graph are dropped before this method returns.
    pub fn compile_tree_artifact_with_input_initializers(
        &mut self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<(
        ProgramArtifactCompilationProduct,
        BTreeMap<String, RuntimeHostInputValue>,
    )> {
        self.view().compile_tree_artifact_with_input_initializers(
            tree,
            inputs,
            external_input_names,
        )
    }

    /// Evaluates source-defined activation values in the short-lived compiler
    /// and returns detached typed snapshots for the requested names.
    ///
    /// This is initialization, not resident execution: no planner object,
    /// reactive cell, transaction, or live reference escapes the call.
    pub fn evaluate_static_tree_symbols(
        &mut self,
        tree: &mech_core::Program,
        names: &[&str],
    ) -> MResult<BTreeMap<String, RuntimeHostInputValue>> {
        self.evaluate_static_tree_symbols_with_inputs(tree, &BTreeMap::new(), names)
    }

    pub fn evaluate_static_tree_symbols_with_inputs(
        &mut self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        names: &[&str],
    ) -> MResult<BTreeMap<String, RuntimeHostInputValue>> {
        self.view()
            .evaluate_static_tree_symbols(tree, inputs, names)
    }

    pub fn compile_root(
        &mut self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_root(request, options)
    }

    /// Compile a rooted source graph while retaining every live root symbol
    /// needed by an interactive document host.
    pub fn compile_interactive_root(
        &mut self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_interactive_root(request, options)
    }

    /// Compiles ordered build roots into one resident artifact. Roots share
    /// one compiler-owned program so later roots can consume definitions from
    /// earlier roots, matching the retained `mech build` caller-order contract.
    pub fn compile_roots(
        &mut self,
        requests: &[SourceRequest],
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.view().compile_roots(requests, options)
    }

    /// Compiles an inline one-document application into its ordinary resident
    /// coordinator and one backend-neutral compute region.
    #[cfg(feature = "compute")]
    pub fn compile_mixed_tree(&mut self, tree: &Program) -> MResult<MixedProgramCompilation> {
        self.view().compile_mixed_tree(tree)
    }

    /// Resolves a rooted application once, then compiles both mixed-program
    /// products from that same resolver-owned module graph.
    #[cfg(feature = "compute")]
    pub fn compile_mixed_root(
        &mut self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<MixedProgramCompilation> {
        self.view().compile_mixed_root(request, options)
    }

    fn view(&self) -> ProgramCompilerView<'_> {
        ProgramCompilerView::new(
            Arc::clone(&self.function_catalog),
            self.source_resolver.as_ref(),
            &self.resources,
            &self.module_builder,
            &self.host_interfaces,
            &self.module_manifests,
            self.program_config.clone(),
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
    program_config: CompilerPlanningConfig,
}

#[derive(Clone)]
struct CompilerModule {
    source: ResolvedSource,
    import_edges: Vec<(SourceImportDeclaration, String)>,
    module_version: ModuleVersionId,
}

#[derive(Clone)]
struct CompilerModuleInstance {
    exports: HashMap<String, CompilerExportValue>,
    document_outputs: Vec<LegacyValue>,
    result: LegacyValue,
}

#[derive(Clone)]
struct CompilerExportValue {
    module: String,
    export: String,
    value: LegacyValue,
}

impl CompilerExportValue {
    fn fresh_reference(&self) -> MResult<Ref<LegacyValue>> {
        self.value
            .try_deep_snapshot()
            .map(Ref::new)
            .map_err(|_| unsupported_compiler_import(self))
    }
}

#[derive(Clone, Debug)]
pub struct CompilerImportValueUnsupported {
    pub module: String,
    pub export: String,
    pub kind: String,
}

impl MechErrorKind for CompilerImportValueUnsupported {
    fn name(&self) -> &str {
        "CompilerImportValueUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "compiler import `{}` from module `{}` cannot snapshot value kind `{}`",
            self.export, self.module, self.kind,
        )
    }
}

fn unsupported_compiler_import(value: &CompilerExportValue) -> MechError {
    MechError::new(
        CompilerImportValueUnsupported {
            module: value.module.clone(),
            export: value.export.clone(),
            kind: value.value.kind().to_string(),
        },
        None,
    )
}

impl<'a> ProgramCompilerView<'a> {
    pub(crate) fn new(
        function_catalog: Arc<mech_core::FunctionCatalog>,
        source_resolver: &'a dyn SourceResolver,
        resources: &'a RuntimeResourceRegistry,
        module_builder: &'a ModuleBuilder,
        host_interfaces: &'a HostInterfaceCatalog,
        module_manifests: &'a ModuleManifestCatalog,
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

    pub(crate) fn compile_source(&self, source: &str) -> MResult<ProgramCompilationProduct> {
        let tree = mech_syntax::parser::parse(source.trim())?;
        self.compile_tree(&tree)
    }

    pub(crate) fn compile_interactive_source(
        &self,
        source: &str,
    ) -> MResult<ProgramCompilationProduct> {
        let tree = mech_syntax::parser::parse(source.trim())?;
        self.compile_tree_with_bytecode_encoding(
            &tree,
            ProgramBytecodeEncoding::Semantic,
            RootOutputProjection::ObservableResultsAndSymbols,
        )
    }

    fn compile_source_frozen_v1(&self, source: &str) -> MResult<ProgramCompilationProduct> {
        let tree = mech_syntax::parser::parse(source.trim())?;
        self.compile_tree_with_bytecode_encoding(
            &tree,
            ProgramBytecodeEncoding::FrozenV1ImplementationIdentities,
            RootOutputProjection::ObservableResults,
        )
    }

    pub(crate) fn compile_tree(
        &self,
        tree: &mech_core::Program,
    ) -> MResult<ProgramCompilationProduct> {
        self.compile_tree_with_bytecode_encoding(
            tree,
            ProgramBytecodeEncoding::Semantic,
            RootOutputProjection::ObservableResults,
        )
    }

    pub(crate) fn compile_interactive_tree(
        &self,
        tree: &mech_core::Program,
    ) -> MResult<ProgramCompilationProduct> {
        self.compile_tree_with_bytecode_encoding(
            tree,
            ProgramBytecodeEncoding::Semantic,
            RootOutputProjection::ObservableResultsAndSymbols,
        )
    }

    fn compile_tree_with_bytecode_encoding(
        &self,
        tree: &mech_core::Program,
        bytecode_encoding: ProgramBytecodeEncoding,
        output_projection: RootOutputProjection,
    ) -> MResult<ProgramCompilationProduct> {
        let mut program = self.new_program();
        let document_output_ids = root_document_output_ids(tree);
        let index = SourceIndex::from_program(tree);
        index.validate_address_targets()?;
        let imports = index.all_imports();
        let mut contexts = index.all_contexts();
        self.materialize_inline_context_imports(&imports, &mut contexts)?;
        install_function_imports(&mut program, &imports, &[])?;
        install_context_imports(&mut program, &imports, &contexts)?;
        let operations = compiled_resource_send_operations(&contexts);
        let mut services = CompilerPlanningServices {
            providers: self.resources,
            resource_send_operations: &operations,
            compute_interface: None,
        };
        let result = program
            .plan_tree_with_services(&sanitize_tree(tree.clone())?, &mut services)
            .map_err(classify_source_planning)?;
        publish_document_and_root_outputs(&mut program, &document_output_ids, &result)?;
        if matches!(
            output_projection,
            RootOutputProjection::ObservableResultsAndSymbols
        ) {
            program.publish_compiler_root_symbols();
        }
        self.finalize(program, &operations, bytecode_encoding)
    }

    fn compile_tree_artifact_with_inputs(
        &self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        self.compile_tree_artifact_with_input_initializers(tree, inputs, external_input_names)
            .map(|(product, _)| product)
    }

    fn compile_tree_artifact_with_input_initializers(
        &self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
    ) -> MResult<(
        ProgramArtifactCompilationProduct,
        BTreeMap<String, RuntimeHostInputValue>,
    )> {
        self.compile_tree_artifact_with_input_initializers_and_compute(
            tree,
            inputs,
            external_input_names,
            None,
        )
    }

    fn compile_tree_artifact_with_input_initializers_and_compute(
        &self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        external_input_names: &BTreeSet<String>,
        compute_interface: Option<&ComputeRegionInterface>,
    ) -> MResult<(
        ProgramArtifactCompilationProduct,
        BTreeMap<String, RuntimeHostInputValue>,
    )> {
        let mut program = self.new_program();
        for (name, value) in inputs {
            program.install_compiler_symbol(name, Ref::new(value.clone().into_mech_value()?))?;
        }
        let document_output_ids = root_document_output_ids(tree);
        let index = SourceIndex::from_program(tree);
        index.validate_address_targets()?;
        let imports = index.all_imports();
        let mut contexts = index.all_contexts();
        self.materialize_inline_context_imports(&imports, &mut contexts)?;
        install_function_imports(&mut program, &imports, &[])?;
        install_context_imports(&mut program, &imports, &contexts)?;
        let operations = compiled_resource_send_operations(&contexts);
        let mut services = CompilerPlanningServices {
            providers: self.resources,
            resource_send_operations: &operations,
            compute_interface,
        };
        let result = program
            .plan_tree_with_services(&sanitize_tree(tree.clone())?, &mut services)
            .map_err(classify_source_planning)?;
        publish_document_and_root_outputs(&mut program, &document_output_ids, &result)?;
        let input_names = external_input_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let initial_inputs = program
            .compiler_root_symbol_values(&input_names)?
            .into_iter()
            .map(|(name, value)| {
                RuntimeHostInputValue::from_numeric_mech_value(&value).map(|value| (name, value))
            })
            .collect::<MResult<BTreeMap<_, _>>>()?;
        let resolver = CompilerExternalContractResolver {
            providers: ResidentExternalContractResolver::new(self.resources),
            compute: compute_interface.is_some(),
        };
        let product = program
            .compile_program_artifact_product_with_resource_send_operations(
                &resolver,
                &operations,
                external_input_names,
            )
            .map_err(|error| {
                route_failure(
                    ResidentRouteFailureClass::InvalidArtifact,
                    format!("resident ProgramArtifact finalization failed: {error:?}"),
                )
            })?;
        Ok((product, initial_inputs))
    }

    #[cfg(feature = "compute")]
    fn compile_mixed_tree(&self, tree: &Program) -> MResult<MixedProgramCompilation> {
        let partition = partition_mixed_program(tree)?;
        let external_input_names = source_declared_compute_inputs(tree)?;
        let (compute, initial_inputs) = self.compile_tree_artifact_with_input_initializers(
            &partition.compute,
            &BTreeMap::new(),
            &external_input_names,
        )?;
        let compute = assemble_compute_region(compute, initial_inputs, &partition.name)?;
        let coordinator = self
            .compile_tree_artifact_with_input_initializers_and_compute(
                &partition.coordinator,
                &BTreeMap::new(),
                &BTreeSet::new(),
                Some(&compute.interface),
            )?
            .0;
        Ok(MixedProgramCompilation {
            coordinator,
            compute,
        })
    }

    #[cfg(feature = "compute")]
    fn compile_mixed_root(
        &self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<MixedProgramCompilation> {
        request.validate()?;
        let mut modules = HashMap::new();
        let mut stack = Vec::new();
        let root = self.resolve_module(request, options, &mut modules, &mut stack)?;
        let tree = declaration_tree(&modules[&root].source.source)?;
        let partition = partition_mixed_program(&tree)?;
        let external_input_names = source_declared_compute_inputs(&tree)?;
        let (compute, initial_inputs) = self.compile_resolved_tree_artifact(
            &root,
            &modules,
            partition.compute,
            &external_input_names,
            None,
        )?;
        let compute = assemble_compute_region(compute, initial_inputs, &partition.name)?;
        let (coordinator, _) = self.compile_resolved_tree_artifact(
            &root,
            &modules,
            partition.coordinator,
            &BTreeSet::new(),
            Some(&compute.interface),
        )?;
        Ok(MixedProgramCompilation {
            coordinator,
            compute,
        })
    }

    #[cfg(feature = "compute")]
    fn compile_resolved_tree_artifact(
        &self,
        root: &str,
        resolved_modules: &HashMap<String, CompilerModule>,
        tree: Program,
        external_input_names: &BTreeSet<String>,
        compute_interface: Option<&ComputeRegionInterface>,
    ) -> MResult<(
        ProgramArtifactCompilationProduct,
        BTreeMap<String, RuntimeHostInputValue>,
    )> {
        let mut modules = resolved_modules.clone();
        replace_root_tree(
            modules
                .get_mut(root)
                .expect("resolved root is retained by the compiler session"),
            tree,
        )?;

        let mut instances = HashMap::new();
        let mut active = Vec::new();
        let mut root_program = Some(self.new_program());
        self.execute_module(
            root,
            &modules,
            &mut instances,
            &mut active,
            Some(&mut root_program),
            compute_interface,
        )?;
        let program = root_program
            .as_mut()
            .expect("root compiler program is retained until finalization");
        publish_module_outputs(
            program,
            instances
                .get(root)
                .expect("the requested root was executed"),
        );
        let input_names = external_input_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let initial_inputs = program
            .compiler_root_symbol_values(&input_names)?
            .into_iter()
            .map(|(name, value)| {
                RuntimeHostInputValue::from_numeric_mech_value(&value).map(|value| (name, value))
            })
            .collect::<MResult<BTreeMap<_, _>>>()?;
        let operations = modules
            .values()
            .flat_map(|module| compiled_resource_send_operations(&module.source.contexts))
            .collect::<Vec<_>>();
        let resolver = CompilerExternalContractResolver {
            providers: ResidentExternalContractResolver::new(self.resources),
            compute: compute_interface.is_some(),
        };
        let product = root_program
            .expect("root compiler program is retained until finalization")
            .compile_program_artifact_product_with_resource_send_operations(
                &resolver,
                &operations,
                external_input_names,
            )
            .map_err(|error| {
                route_failure(
                    ResidentRouteFailureClass::InvalidArtifact,
                    format!("resident ProgramArtifact finalization failed: {error:?}"),
                )
            })?;
        Ok((product, initial_inputs))
    }

    fn evaluate_static_tree_symbols(
        &self,
        tree: &mech_core::Program,
        inputs: &BTreeMap<String, RuntimeHostInputValue>,
        names: &[&str],
    ) -> MResult<BTreeMap<String, RuntimeHostInputValue>> {
        let mut program = self.new_program();
        for (name, value) in inputs {
            program.install_compiler_symbol(name, Ref::new(value.clone().into_mech_value()?))?;
        }
        let index = SourceIndex::from_program(tree);
        index.validate_address_targets()?;
        let imports = index.all_imports();
        let mut contexts = index.all_contexts();
        self.materialize_inline_context_imports(&imports, &mut contexts)?;
        install_function_imports(&mut program, &imports, &[])?;
        install_context_imports(&mut program, &imports, &contexts)?;
        let operations = compiled_resource_send_operations(&contexts);
        let mut services = CompilerPlanningServices {
            providers: self.resources,
            resource_send_operations: &operations,
            compute_interface: None,
        };
        program
            .plan_tree_with_services(&sanitize_tree(tree.clone())?, &mut services)
            .map_err(classify_source_planning)?;
        program
            .compiler_root_symbol_values(names)?
            .into_iter()
            .map(|(name, value)| {
                RuntimeHostInputValue::from_numeric_mech_value(&value).map(|value| (name, value))
            })
            .collect()
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

    pub(crate) fn compile_root(
        &self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.compile_root_with_projection(request, options, RootOutputProjection::ObservableResults)
    }

    pub(crate) fn compile_resolved_root(
        &self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.compile_resolved_root_with_projection(
            resolved,
            options,
            RootOutputProjection::ObservableResults,
        )
    }

    pub(crate) fn compile_interactive_root(
        &self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.compile_root_with_projection(
            request,
            options,
            RootOutputProjection::ObservableResultsAndSymbols,
        )
    }

    pub(crate) fn compile_interactive_resolved_root(
        &self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        self.compile_resolved_root_with_projection(
            resolved,
            options,
            RootOutputProjection::ObservableResultsAndSymbols,
        )
    }

    fn compile_root_with_projection(
        &self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
        output_projection: RootOutputProjection,
    ) -> MResult<ProgramCompilationProduct> {
        request.validate()?;
        let resolved = self.source_resolver.resolve(&request)?.ok_or_else(|| {
            route_failure(
                ResidentRouteFailureClass::InvalidArtifact,
                format!(
                    "root or imported source `{}` was not found",
                    request.specifier
                ),
            )
        })?;
        self.compile_resolved_root_with_projection(resolved, options, output_projection)
    }

    fn compile_resolved_root_with_projection(
        &self,
        resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
        output_projection: RootOutputProjection,
    ) -> MResult<ProgramCompilationProduct> {
        let mut modules = HashMap::new();
        let mut stack = Vec::new();
        let root = self.resolve_resolved_module(resolved, options, &mut modules, &mut stack)?;
        let mut instances = HashMap::new();
        let mut active = Vec::new();
        let mut root_program = Some(self.new_program());
        self.execute_module(
            &root,
            &modules,
            &mut instances,
            &mut active,
            Some(&mut root_program),
            None,
        )?;
        let program = root_program
            .as_mut()
            .expect("root compiler program is retained until finalization");
        let instance = instances
            .get(&root)
            .expect("the requested root was executed");
        publish_module_outputs(program, instance);
        if matches!(
            output_projection,
            RootOutputProjection::ObservableResultsAndSymbols
        ) {
            program.publish_compiler_root_symbols();
        }
        let operations = modules
            .values()
            .flat_map(|module| compiled_resource_send_operations(&module.source.contexts))
            .collect::<Vec<_>>();
        self.finalize(
            root_program.expect("root compiler program is retained until finalization"),
            &operations,
            ProgramBytecodeEncoding::Semantic,
        )
    }

    pub(crate) fn compile_roots(
        &self,
        requests: &[SourceRequest],
        options: ModuleBuildOptions<'_>,
    ) -> MResult<ProgramCompilationProduct> {
        if requests.is_empty() {
            return Err(route_failure(
                ResidentRouteFailureClass::SemanticUnsupported,
                "resident source compilation requires at least one root",
            ));
        }
        let mut modules = HashMap::new();
        let mut stack = Vec::new();
        let mut roots = Vec::with_capacity(requests.len());
        for request in requests {
            request.validate()?;
            roots.push(self.resolve_module(request.clone(), options, &mut modules, &mut stack)?);
        }
        let plan_order = explicit_root_plan_order(&roots, &modules);
        let mut instances = HashMap::new();
        let mut active = Vec::new();
        let mut root_program = Some(self.new_program());
        for root in plan_order {
            self.execute_module(
                &root,
                &modules,
                &mut instances,
                &mut active,
                Some(&mut root_program),
                None,
            )?;
        }
        let program = root_program
            .as_mut()
            .expect("root compiler program is retained until finalization");
        for root in &roots {
            publish_module_outputs(
                program,
                instances
                    .get(root)
                    .expect("every requested root was executed"),
            );
        }
        let operations = modules
            .values()
            .flat_map(|module| compiled_resource_send_operations(&module.source.contexts))
            .collect::<Vec<_>>();
        self.finalize(
            root_program.expect("root compiler program is retained until finalization"),
            &operations,
            ProgramBytecodeEncoding::Semantic,
        )
    }

    fn new_program(&self) -> CompilerPlanningProgram {
        CompilerPlanningProgram::with_function_catalog(
            self.program_config.clone(),
            Arc::clone(&self.function_catalog),
        )
    }

    fn finalize(
        &self,
        mut program: CompilerPlanningProgram,
        operations: &[CompiledResourceSendOperation],
        bytecode_encoding: ProgramBytecodeEncoding,
    ) -> MResult<ProgramCompilationProduct> {
        let resolver = ResidentExternalContractResolver::new(self.resources);
        let product = match bytecode_encoding {
            ProgramBytecodeEncoding::Semantic => {
                program.compile_program_product_with_resource_send_operations(&resolver, operations)
            }
            ProgramBytecodeEncoding::FrozenV1ImplementationIdentities => program
                .compile_frozen_v1_program_product_with_resource_send_operations(
                    &resolver, operations,
                ),
        };
        product.map_err(|error| {
            route_failure(
                ResidentRouteFailureClass::InvalidArtifact,
                format!("resident ProgramArtifact finalization failed: {error:?}"),
            )
        })
    }

    fn resolve_module(
        &self,
        request: SourceRequest,
        options: ModuleBuildOptions<'_>,
        modules: &mut HashMap<String, CompilerModule>,
        stack: &mut Vec<String>,
    ) -> MResult<String> {
        let resolved = self.source_resolver.resolve(&request)?.ok_or_else(|| {
            route_failure(
                ResidentRouteFailureClass::InvalidArtifact,
                format!(
                    "root or imported source `{}` was not found",
                    request.specifier
                ),
            )
        })?;
        self.resolve_resolved_module(resolved, options, modules, stack)
    }

    fn resolve_resolved_module(
        &self,
        mut resolved: ResolvedSource,
        options: ModuleBuildOptions<'_>,
        modules: &mut HashMap<String, CompilerModule>,
        stack: &mut Vec<String>,
    ) -> MResult<String> {
        index_source(&mut resolved)?;
        resolved.capability_requirements.extend(
            options
                .capability_requirements
                .iter()
                .map(|resource| CapabilityRequest::from_keys("compiler", "use", *resource)),
        );
        let canonical_uri = resolved.canonical_uri.clone();
        if modules.contains_key(&canonical_uri) {
            return Ok(canonical_uri);
        }
        if stack.contains(&canonical_uri) {
            let mut cycle = stack.clone();
            cycle.push(canonical_uri);
            return Err(mech_core::MechError::new(
                RuntimeModuleDependencyCycleError { cycle },
                None,
            ));
        }
        stack.push(canonical_uri.clone());

        let mut import_edges = Vec::new();
        for import in resolved.imports.clone() {
            if !import_may_resolve_source_dependency(&import) {
                continue;
            }
            let dependency_request =
                source_request_for_import(&import, Some(&resolved.canonical_uri));
            match self.source_resolver.resolve(&dependency_request)? {
                Some(dependency_source) => {
                    let dependency =
                        self.resolve_resolved_module(dependency_source, options, modules, stack)?;
                    import_edges.push((import, dependency));
                }
                None if import_requires_source_dependency(&import) => {
                    return Err(mech_core::MechError::new(
                        RuntimeModuleDependencyMissingError {
                            module: resolved.canonical_uri.clone(),
                            specifier: dependency_request.specifier,
                            referrer: dependency_request.referrer,
                        },
                        None,
                    ));
                }
                None => {}
            }
        }

        // Keep ModuleBuilder in the compiler boundary as the validation and
        // deterministic module-identity authority, without persisting records
        // into the runtime store. Dependency identities remain part of the
        // root identity even though the records themselves are ephemeral.
        let dependency_versions = import_edges
            .iter()
            .map(|(_, dependency)| {
                modules
                    .get(dependency)
                    .expect("resolved dependency is retained by the compiler session")
                    .module_version
            })
            .collect::<Vec<_>>();
        let feature_flags = options
            .feature_flags
            .iter()
            .map(|flag| (*flag).to_owned())
            .collect::<Vec<_>>();
        let mut builder = self.module_builder.clone();
        let mut record = builder.build_resolved_source(
            resolved.clone(),
            options.compiler_version,
            options.language_edition,
            options.target,
            &feature_flags,
            &dependency_versions,
            &resolved.capability_requirements,
        )?;
        self.materialize_manifest_context_imports(&mut record)?;
        let module_version = record.module_version;
        resolved.contexts = record.contexts;
        resolved.scopes = record.scopes;

        stack.pop();
        modules.insert(
            canonical_uri.clone(),
            CompilerModule {
                source: resolved,
                import_edges,
                module_version,
            },
        );
        Ok(canonical_uri)
    }

    fn materialize_manifest_context_imports(
        &self,
        record: &mut crate::RuntimeModuleRecord,
    ) -> MResult<()> {
        for scope in &mut record.scopes {
            let context_imports = scope
                .imports
                .iter()
                .filter_map(|import| match &import.alias {
                    Some(SourceImportAlias::Context(alias)) => Some((import, alias.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>();

            for (import, alias) in context_imports {
                if scope.contexts.iter().any(|context| context.name == alias) {
                    return Err(mech_core::MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "compile_root",
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
                let declaration = crate::SourceContextDeclaration {
                    name: alias,
                    base: crate::SourceContextBase::ResourceUri(base_uri),
                    capabilities: operations
                        .iter()
                        .map(|operation| crate::SourceContextCapability {
                            operation: operation.clone(),
                            scope: crate::SourceContextCapabilityScope::Wildcard,
                        })
                        .collect(),
                };
                scope.contexts.push(declaration.clone());
                record.contexts.push(declaration);
            }
        }
        Ok(())
    }

    fn execute_module(
        &self,
        canonical_uri: &str,
        modules: &HashMap<String, CompilerModule>,
        instances: &mut HashMap<String, CompilerModuleInstance>,
        active: &mut Vec<String>,
        root_program: Option<&mut Option<CompilerPlanningProgram>>,
        compute_interface: Option<&ComputeRegionInterface>,
    ) -> MResult<()> {
        if instances.contains_key(canonical_uri) {
            return Ok(());
        }
        if active.iter().any(|uri| uri == canonical_uri) {
            let mut cycle = active.clone();
            cycle.push(canonical_uri.to_owned());
            return Err(mech_core::MechError::new(
                RuntimeModuleDependencyCycleError { cycle },
                None,
            ));
        }
        active.push(canonical_uri.to_owned());
        let module = modules.get(canonical_uri).ok_or_else(|| {
            route_failure(
                ResidentRouteFailureClass::InternalFailure,
                format!("compiler module `{canonical_uri}` was not retained"),
            )
        })?;
        for (_, dependency) in &module.import_edges {
            self.execute_module(
                dependency,
                modules,
                instances,
                active,
                None,
                compute_interface,
            )?;
        }

        let mut owned_program;
        let program = if let Some(slot) = root_program {
            slot.as_mut().expect("root program is present")
        } else {
            owned_program = self.new_program();
            &mut owned_program
        };
        install_function_imports(program, &module.source.imports, &module.import_edges)?;
        install_context_imports(program, &module.source.imports, &module.source.contexts)?;
        let environment = build_import_environment(module, instances)?;
        install_environment(program, &environment)?;
        let tree = executable_resolved_tree(&module.source)?;
        let resource_send_operations = compiled_resource_send_operations(&module.source.contexts);
        let mut services = CompilerPlanningServices {
            providers: self.resources,
            resource_send_operations: &resource_send_operations,
            compute_interface,
        };
        let result = program
            .plan_tree_with_services(&tree, &mut services)
            .map_err(classify_source_planning)?;
        let document_output_ids = root_document_output_ids(&tree);
        let document_outputs = program.compiler_document_output_values(&document_output_ids)?;

        let mut exports = HashMap::new();
        for export in source_exports(&module.source) {
            let name = export.name.clone();
            let value = program.compiler_root_symbol_value(&name).map_err(|_| {
                mech_core::MechError::new(
                    RuntimeModuleExportNotFound {
                        dependency: canonical_uri.to_owned(),
                        export: name.clone(),
                    },
                    None,
                )
            })?;
            let snapshot = value.try_deep_snapshot().map_err(|_| {
                MechError::new(
                    CompilerImportValueUnsupported {
                        module: canonical_uri.to_owned(),
                        export: name.clone(),
                        kind: value.kind().to_string(),
                    },
                    None,
                )
            })?;
            exports.insert(
                name.clone(),
                CompilerExportValue {
                    module: canonical_uri.to_owned(),
                    export: name,
                    value: snapshot,
                },
            );
        }
        instances.insert(
            canonical_uri.to_owned(),
            CompilerModuleInstance {
                exports,
                document_outputs,
                result,
            },
        );
        active.pop();
        Ok(())
    }
}

#[cfg(feature = "compute")]
struct MixedProgramPartition {
    name: String,
    coordinator: Program,
    compute: Program,
}

#[cfg(feature = "compute")]
fn partition_mixed_program(tree: &Program) -> MResult<MixedProgramPartition> {
    let mut regions = Vec::new();
    for (index, section) in tree.body.sections.iter().enumerate() {
        if mech_engine::section_compute_placement(section)?.is_some() {
            regions.push(index);
        }
    }
    if regions.len() != 1 {
        return Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "compile_mixed_program",
                reason: format!(
                    "v0.4 mixed programs require exactly one executable compute region, found {}",
                    regions.len(),
                ),
            },
            None,
        ));
    }
    let region_index = regions[0];
    let region = &tree.body.sections[region_index];
    let name = region
        .subtitle
        .as_ref()
        .map(|subtitle| subtitle.to_string().trim().to_owned())
        .unwrap_or_default();
    if name.is_empty() {
        return Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "compile_mixed_program",
                reason: "the executable compute region must have a nonempty section name"
                    .to_owned(),
            },
            None,
        ));
    }

    let excluded_imports = region
        .elements
        .iter()
        .filter_map(import_element)
        .collect::<Vec<_>>();
    let mut coordinator_sections = Vec::with_capacity(tree.body.sections.len());
    if !excluded_imports.is_empty() {
        coordinator_sections.push(Section {
            subtitle: None,
            annotations: Vec::new(),
            elements: excluded_imports,
        });
    }
    coordinator_sections.extend(
        tree.body
            .sections
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != region_index)
            .map(|(_, section)| section)
            .cloned(),
    );
    let imports = import_prelude(tree);
    let mut compute_sections = Vec::with_capacity(2);
    if !imports.is_empty() {
        compute_sections.push(Section {
            subtitle: None,
            annotations: Vec::new(),
            elements: imports,
        });
    }
    compute_sections.push(region.clone());

    Ok(MixedProgramPartition {
        name,
        coordinator: Program {
            title: tree.title.clone(),
            body: Body {
                sections: coordinator_sections,
            },
        },
        compute: Program {
            title: None,
            body: Body {
                sections: compute_sections,
            },
        },
    })
}

#[cfg(feature = "compute")]
fn import_prelude(tree: &Program) -> Vec<SectionElement> {
    let title_imports = tree
        .title
        .as_ref()
        .map(|title| {
            title
                .imports
                .iter()
                .cloned()
                .map(|(import, comment)| (MechCode::Import(import), comment))
                .collect::<Vec<_>>()
        })
        .filter(|imports| !imports.is_empty())
        .map(SectionElement::MechCode);
    title_imports
        .into_iter()
        .chain(
            tree.body
                .sections
                .iter()
                .flat_map(|section| &section.elements)
                .filter_map(import_element),
        )
        .collect()
}

#[cfg(feature = "compute")]
fn import_element(element: &SectionElement) -> Option<SectionElement> {
    let SectionElement::MechCode(code) = element else {
        return None;
    };
    let imports = code
        .iter()
        .filter(|(code, _)| matches!(code, MechCode::Import(_)))
        .cloned()
        .collect::<Vec<_>>();
    (!imports.is_empty()).then_some(SectionElement::MechCode(imports))
}

#[cfg(feature = "compute")]
fn source_declared_compute_inputs(tree: &Program) -> MResult<BTreeSet<String>> {
    let index = SourceIndex::from_program(tree);
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
        .filter(|capability| capability.operation == "write")
        .filter_map(|capability| match capability.scope {
            SourceContextCapabilityScope::Path(path) => (!path.contains('*'))
                .then(|| path.strip_prefix("input/").map(str::to_owned))
                .flatten(),
            SourceContextCapabilityScope::Wildcard => None,
        })
        .collect())
}

#[cfg(feature = "compute")]
fn replace_root_tree(module: &mut CompilerModule, tree: Program) -> MResult<()> {
    let inherited_contexts = module.source.contexts.clone();
    module.source.replace_source(MechSourceCode::Tree(tree));
    index_source(&mut module.source)?;
    for context in inherited_contexts {
        if !module
            .source
            .contexts
            .iter()
            .any(|candidate| candidate.name == context.name)
        {
            module.source.contexts.push(context);
        }
    }
    Ok(())
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
            RuntimeHostInputValue::F32Matrix {
                rows,
                columns,
                values,
            } => ComputeValue::TensorF32 {
                dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
                layout: TensorLayout::ColumnMajor,
                values: Arc::from(values),
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

/// Preserve caller order for independent roots while promoting any requested
/// dependency ahead of its consumer. That lets an explicit dependency execute
/// exactly once in the shared program. Caller-visible outputs are published
/// separately after planning so this topological order cannot reorder them.
fn explicit_root_plan_order(
    roots: &[String],
    modules: &HashMap<String, CompilerModule>,
) -> Vec<String> {
    fn visit(
        root: &str,
        requested: &HashSet<&str>,
        modules: &HashMap<String, CompilerModule>,
        visited: &mut HashSet<String>,
        ordered: &mut Vec<String>,
    ) {
        if !visited.insert(root.to_owned()) {
            return;
        }
        if let Some(module) = modules.get(root) {
            for (_, dependency) in &module.import_edges {
                if requested.contains(dependency.as_str()) {
                    visit(dependency, requested, modules, visited, ordered);
                }
            }
        }
        ordered.push(root.to_owned());
    }

    let requested = roots.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut visited = HashSet::new();
    let mut ordered = Vec::with_capacity(requested.len());
    for root in roots {
        visit(root, &requested, modules, &mut visited, &mut ordered);
    }
    ordered
}

fn publish_document_and_root_outputs(
    program: &mut CompilerPlanningProgram,
    document_output_ids: &[u64],
    root: &LegacyValue,
) -> MResult<()> {
    for output in program.compiler_document_output_values(document_output_ids)? {
        program.publish_compiler_document_output(output);
    }
    program.publish_compiler_root_output(root.clone());
    Ok(())
}

fn publish_module_outputs(
    program: &mut CompilerPlanningProgram,
    instance: &CompilerModuleInstance,
) {
    for output in &instance.document_outputs {
        program.publish_compiler_document_output(output.clone());
    }
    program.publish_compiler_root_output(instance.result.clone());
}

fn compiled_resource_send_operations(
    contexts: &[crate::SourceContextDeclaration],
) -> Vec<CompiledResourceSendOperation> {
    contexts
        .iter()
        .filter_map(|context| {
            let crate::SourceContextBase::ResourceUri(base_uri) = &context.base else {
                return None;
            };
            Some(context.capabilities.iter().filter_map(move |capability| {
                (capability.operation != "read").then(|| CompiledResourceSendOperation {
                    base_uri: base_uri.clone(),
                    path: match &capability.scope {
                        crate::SourceContextCapabilityScope::Path(path) => Some(path.clone()),
                        crate::SourceContextCapabilityScope::Wildcard => None,
                    },
                    operation: capability.operation.clone(),
                })
            }))
        })
        .flatten()
        .collect()
}

fn declared_resource_send_operation<'a>(
    request: &ExecutionResourceRequest,
    operations: &'a [CompiledResourceSendOperation],
) -> MResult<Option<&'a str>> {
    let mut selected = None;
    for exact in [true, false] {
        for declaration in operations.iter().filter(|declaration| {
            declaration.base_uri == request.base_uri
                && declaration.path.is_some() == exact
                && declaration
                    .path
                    .as_deref()
                    .map_or(!exact, |path| path == request.path)
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

fn install_context_imports(
    program: &mut CompilerPlanningProgram,
    imports: &[SourceImportDeclaration],
    contexts: &[crate::SourceContextDeclaration],
) -> MResult<()> {
    for import in imports {
        let Some(SourceImportAlias::Context(alias)) = &import.alias else {
            continue;
        };
        let declaration = contexts
            .iter()
            .find(|context| context.name == *alias)
            .ok_or_else(|| invalid_context_import(&import.specifier, "materialized declaration"))?;
        let crate::SourceContextBase::ResourceUri(base_uri) = &declaration.base else {
            return Err(invalid_context_import(
                &import.specifier,
                "resolved resource URI",
            ));
        };
        program.install_compiler_context(alias, base_uri);
    }
    Ok(())
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

fn index_source(resolved: &mut ResolvedSource) -> MResult<()> {
    if !resolved.scopes.is_empty() {
        return Ok(());
    }
    let parsed;
    let tree = match resolved.syntax_tree.as_deref() {
        Some(tree) => tree,
        None => {
            parsed = source_tree(&resolved.source)?;
            let Some(tree) = parsed.as_ref() else {
                return Ok(());
            };
            tree
        }
    };
    let index = SourceIndex::from_program(tree);
    index.validate_address_targets()?;
    resolved.imports = index.all_imports();
    resolved.exports = index.all_exports();
    resolved.contexts = index.all_contexts();
    resolved.address_references = index.all_address_references();
    resolved.scopes = index.module_scopes();
    Ok(())
}

fn executable_resolved_tree(source: &ResolvedSource) -> MResult<mech_core::Program> {
    match source.syntax_tree.as_deref() {
        Some(tree) => sanitize_tree(tree.clone()),
        None => executable_tree(&source.source),
    }
}

fn source_tree(source: &MechSourceCode) -> MResult<Option<mech_core::Program>> {
    match source {
        MechSourceCode::String(source) => Ok(Some(mech_syntax::parser::parse(source.trim())?)),
        MechSourceCode::Tree(tree) => Ok(Some(tree.clone())),
        MechSourceCode::Program(_) => Ok(None),
        MechSourceCode::ByteCode(_) | MechSourceCode::Html(_) => Ok(None),
        MechSourceCode::Image(_, _) => Err(unsupported_route(
            "image source cannot be compiled as a resident program",
        )),
    }
}

#[cfg(feature = "compute")]
fn declaration_tree(source: &MechSourceCode) -> MResult<Program> {
    match source {
        MechSourceCode::String(source) => mech_syntax::parser::parse(source.trim()),
        MechSourceCode::Tree(tree) => Ok(tree.clone()),
        MechSourceCode::Program(sources) => {
            let mut sections = Vec::new();
            for source in sources {
                sections.extend(declaration_tree(source)?.body.sections);
            }
            Ok(Program {
                title: None,
                body: Body { sections },
            })
        }
        MechSourceCode::ByteCode(_) | MechSourceCode::Html(_) => Err(unsupported_route(
            "root bytecode and HTML cannot be partitioned as a mixed source program",
        )),
        MechSourceCode::Image(_, _) => Err(unsupported_route(
            "image source cannot be partitioned as a mixed source program",
        )),
    }
}

fn executable_tree(source: &MechSourceCode) -> MResult<mech_core::Program> {
    match source {
        MechSourceCode::String(source) => sanitize_tree(mech_syntax::parser::parse(source.trim())?),
        MechSourceCode::Tree(tree) => sanitize_tree(tree.clone()),
        MechSourceCode::Program(sources) => {
            let mut sections = Vec::new();
            for source in sources {
                sections.extend(executable_tree(source)?.body.sections);
            }
            Ok(mech_core::Program {
                title: None,
                body: mech_core::Body { sections },
            })
        }
        MechSourceCode::ByteCode(_) | MechSourceCode::Html(_) => Err(unsupported_route(
            "root bytecode and HTML are handled outside the source compiler session",
        )),
        MechSourceCode::Image(_, _) => Err(unsupported_route(
            "image source cannot be compiled as a resident program",
        )),
    }
}

fn sanitize_tree(mut tree: mech_core::Program) -> MResult<mech_core::Program> {
    if let Some(title) = &mut tree.title {
        title.imports.clear();
    }
    for section in &mut tree.body.sections {
        for element in &mut section.elements {
            let mech_core::SectionElement::MechCode(codes) = element else {
                continue;
            };
            codes.retain(|(code, _)| {
                !matches!(
                    code,
                    mech_core::MechCode::Import(_)
                        | mech_core::MechCode::Statement(
                            mech_core::Statement::ImportDeclaration(_)
                                | mech_core::Statement::ExportDeclaration(_)
                        )
                )
            });
        }
    }
    Ok(tree)
}

fn source_exports(source: &ResolvedSource) -> Vec<SourceExportDeclaration> {
    source
        .scopes
        .iter()
        .find(|scope| matches!(scope.scope, crate::SourceScope::Program))
        .map(|scope| scope.exports.clone())
        .unwrap_or_else(|| source.exports.clone())
}

fn install_function_imports(
    program: &mut CompilerPlanningProgram,
    imports: &[SourceImportDeclaration],
    source_edges: &[(SourceImportDeclaration, String)],
) -> MResult<()> {
    for import in imports {
        if matches!(import.alias, Some(SourceImportAlias::Context(_)))
            || source_edges.iter().any(|(edge, _)| edge == import)
        {
            continue;
        }
        let module = import.module.as_deref().unwrap_or(&import.specifier);
        match &import.kind {
            SourceImportKind::Namespace => program.install_function_module(module)?,
            SourceImportKind::Single { name } => match &import.alias {
                Some(SourceImportAlias::Value(alias)) => {
                    program.import_compiler_function_item_as(module, name, alias)?;
                }
                Some(SourceImportAlias::Context(_)) => {}
                None => program.import_compiler_function_item(module, name)?,
            },
            SourceImportKind::Wildcard => program.import_compiler_function_glob(module)?,
            SourceImportKind::DependencyOnly => {
                return Err(mech_core::MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "compile_root",
                        reason: format!(
                            "required source dependency `{}` was not resolved",
                            import.specifier
                        ),
                    },
                    None,
                ));
            }
        }
    }
    Ok(())
}

fn build_import_environment(
    module: &CompilerModule,
    instances: &HashMap<String, CompilerModuleInstance>,
) -> MResult<HashMap<String, CompilerExportValue>> {
    let mut environment = HashMap::new();
    let mut ownership = HashMap::<String, String>::new();
    for (import, dependency) in &module.import_edges {
        let instance = instances.get(dependency).ok_or_else(|| {
            route_failure(
                ResidentRouteFailureClass::InternalFailure,
                format!("dependency `{dependency}` was not compiled"),
            )
        })?;
        match &import.kind {
            SourceImportKind::DependencyOnly | SourceImportKind::Namespace => {
                let Some(namespace) = module_namespace_for_import(import) else {
                    continue;
                };
                for (name, value) in &instance.exports {
                    insert_import(
                        &mut environment,
                        &mut ownership,
                        format!("{namespace}/{name}"),
                        value.clone(),
                        &import.specifier,
                    )?;
                }
            }
            SourceImportKind::Single { name } => {
                let value = instance.exports.get(name).ok_or_else(|| {
                    mech_core::MechError::new(
                        RuntimeModuleExportNotFound {
                            dependency: dependency.clone(),
                            export: name.clone(),
                        },
                        None,
                    )
                })?;
                let binding = match &import.alias {
                    Some(SourceImportAlias::Value(alias)) => alias.clone(),
                    Some(SourceImportAlias::Context(_)) => continue,
                    None => name.clone(),
                };
                insert_import(
                    &mut environment,
                    &mut ownership,
                    binding,
                    value.clone(),
                    &import.specifier,
                )?;
            }
            SourceImportKind::Wildcard => {
                for (name, value) in &instance.exports {
                    insert_import(
                        &mut environment,
                        &mut ownership,
                        name.clone(),
                        value.clone(),
                        &import.specifier,
                    )?;
                }
            }
        }
    }
    Ok(environment)
}

fn insert_import(
    environment: &mut HashMap<String, CompilerExportValue>,
    ownership: &mut HashMap<String, String>,
    binding: String,
    value: CompilerExportValue,
    source: &str,
) -> MResult<()> {
    if let Some(first) = ownership.insert(binding.clone(), source.to_owned()) {
        return Err(mech_core::MechError::new(
            RuntimeModuleImportConflict {
                binding,
                first_import: first,
                second_import: source.to_owned(),
            },
            None,
        ));
    }
    environment.insert(binding, value);
    Ok(())
}

fn install_environment(
    program: &mut CompilerPlanningProgram,
    environment: &HashMap<String, CompilerExportValue>,
) -> MResult<()> {
    for (name, value) in environment {
        program.install_compiler_symbol(name, value.fresh_reference()?)?;
    }
    Ok(())
}

struct CompilerPlanningServices<'a> {
    providers: &'a RuntimeResourceRegistry,
    resource_send_operations: &'a [CompiledResourceSendOperation],
    compute_interface: Option<&'a ComputeRegionInterface>,
}

struct CompilerExternalContractResolver<'a> {
    providers: ResidentExternalContractResolver<'a>,
    compute: bool,
}

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

impl MechExecutionServices for CompilerPlanningServices<'_> {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        _arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
        Err(unsupported_route(format!(
            "resident source requires host function `{}`",
            request.name,
        )))
    }

    fn plan_resource_read_output(
        &mut self,
        request: &ExecutionResourceRequest,
    ) -> MResult<LegacyValue> {
        self.plan_read(request)
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        self.plan_read(request)
    }

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        value: &LegacyValue,
    ) -> MResult<()> {
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        let intent = match request.intent {
            ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
            ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
            ResourceIntent::Read => {
                return Err(route_failure(
                    ResidentRouteFailureClass::InvalidArtifact,
                    "resident source attempted a read through the write planning path",
                ));
            }
        };
        let operation = if request.intent == ResourceIntent::Send {
            declared_resource_send_operation(request, self.resource_send_operations)?
                .unwrap_or(&request.operation)
        } else {
            &request.operation
        };
        #[cfg(feature = "compute")]
        if let Some(interface) = self
            .compute_interface
            .filter(|_| is_compute_kernel_base(&key.base_uri))
        {
            return plan_compute_write(interface, &key, intent, value);
        }
        self.providers.plan_write(RuntimeResourceWriteRequest {
            base_uri: key.base_uri,
            path: key.path,
            context_name: request.context_name.clone(),
            operation: RuntimeCapabilityOperation::from_name(operation.to_owned())?,
            value: value.try_deep_snapshot()?,
            intent,
        })
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        _request: &ExecutionResourceRequest,
        _target: Ref<LegacyValue>,
    ) -> MResult<()> {
        Ok(())
    }
}

impl CompilerPlanningServices<'_> {
    fn plan_read(&self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        #[cfg(feature = "compute")]
        if self
            .compute_interface
            .is_some_and(|_| is_compute_kernel_base(&key.base_uri))
        {
            return plan_compute_read(&key);
        }
        self.providers.plan_read(RuntimeResourceReadRequest {
            base_uri: key.base_uri,
            path: key.path,
            context_name: request.context_name.clone(),
        })
    }
}

fn is_compute_kernel_base(base_uri: &str) -> bool {
    base_uri
        .strip_prefix("compute://")
        .is_some_and(|instance| !instance.is_empty() && instance.ends_with("/kernel"))
}

fn plan_compute_read(key: &RuntimeResourceKey) -> MResult<LegacyValue> {
    match key.path.as_str() {
        "backend" | "last-fault" => RuntimeHostInputValue::String(String::new()).into_mech_value(),
        "turns" | "dispatch-ms" | "fault-count" => {
            RuntimeHostInputValue::F64(0.0).into_mech_value()
        }
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
    value: &LegacyValue,
) -> MResult<()> {
    if intent != RuntimeResourceWriteIntent::Send {
        return Err(compute_planning_error(
            "compute inputs and turns are effects; use <-",
        ));
    }
    if key.path == "turn" {
        return Ok(());
    }
    let name = key.path.strip_prefix("input/").ok_or_else(|| {
        compute_planning_error(format!("unknown compute input path `{}`", key.path))
    })?;
    let port = interface.input_named(name).ok_or_else(|| {
        compute_planning_error(format!("unknown compute input path `{}`", key.path))
    })?;
    let value =
        RuntimeHostInputValue::from_numeric_mech_value(value).and_then(|value| match value {
            RuntimeHostInputValue::F32(value) => Ok(ComputeValue::ScalarF32(value)),
            RuntimeHostInputValue::F32Matrix {
                rows,
                columns,
                values,
            } => Ok(ComputeValue::TensorF32 {
                dimensions: vec![rows as u64, columns as u64].into_boxed_slice(),
                layout: TensorLayout::ColumnMajor,
                values: Arc::from(values),
            }),
            value => Err(compute_planning_error(format!(
                "compute input `{}` requires fixed-shape f32 data, found `{value:?}`",
                port.name
            ))),
        })?;
    port.normalize_value(value).map_err(|error| {
        compute_planning_error(format!("compute input `{}` is invalid: {error}", port.name))
    })?;
    Ok(())
}

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
    } else {
        ResidentRouteFailureClass::InvalidArtifact
    };
    route_failure(class, format!("resident source planning failed: {error:?}"))
}
