use super::compiler_value_source::CompilerValueSource;
use crate::*;
use std::collections::BTreeSet;
use std::sync::Arc;

#[cfg(feature = "functions")]
use mech_core::FunctionCatalog;
use mech_core::{
    ApplicationRequirement, CanonicalFunctionSpecializer, MResult, MechError, MechErrorKind,
    ResourceIntent, compare_application_requirements, hash_str, validate_application_requirement,
};

#[cfg(feature = "semantic-compiler")]
use mech_core::Register;

#[cfg(all(feature = "source", feature = "native"))]
use crate::ClosureFunctionSpecializer;
use crate::Interpreter;

#[derive(Debug, Clone)]
pub struct CompilerPlanningLimits {
    pub max_planning_steps: usize,
}

impl Default for CompilerPlanningLimits {
    fn default() -> Self {
        Self {
            max_planning_steps: 10_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilerPlanningConfig {
    pub name: String,
    pub limits: CompilerPlanningLimits,
}

impl Default for CompilerPlanningConfig {
    fn default() -> Self {
        Self {
            name: "program".into(),
            limits: CompilerPlanningLimits::default(),
        }
    }
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
pub struct ProgramCompilationProduct {
    artifact: ProgramArtifact,
    bytecode: Vec<u8>,
    instruction_type_bindings: Vec<Option<mech_core::BoundCall>>,
}

/// Immutable source-compilation product for hosts that immediately activate
/// an artifact and do not need a second durable bytecode representation.
#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
pub struct ProgramArtifactCompilationProduct {
    artifact: ProgramArtifact,
}

#[cfg(feature = "semantic-compiler")]
impl ProgramArtifactCompilationProduct {
    pub const fn artifact(&self) -> &ProgramArtifact {
        &self.artifact
    }

    pub fn into_artifact(self) -> ProgramArtifact {
        self.artifact
    }
}

/// Compiler-only instruction for preserving a source-declared custom send
/// operation in the canonical application requirement. It does not grant
/// authority or change interpreter execution.
#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledResourceSendOperation {
    pub base_uri: String,
    pub path: Option<String>,
    pub operation: String,
}

#[cfg(feature = "semantic-compiler")]
impl ProgramCompilationProduct {
    pub const fn artifact(&self) -> &ProgramArtifact {
        &self.artifact
    }

    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    pub fn into_parts(self) -> (ProgramArtifact, Vec<u8>) {
        (self.artifact, self.bytecode)
    }

    pub fn instruction_type_bindings(&self) -> &[Option<mech_core::BoundCall>] {
        &self.instruction_type_bindings
    }

    pub fn into_native_parts(
        self,
    ) -> (ProgramArtifact, Vec<u8>, Vec<Option<mech_core::BoundCall>>) {
        (self.artifact, self.bytecode, self.instruction_type_bindings)
    }
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramArtifactCompilationError {
    pub reason: String,
}

#[cfg(feature = "semantic-compiler")]
impl MechErrorKind for ProgramArtifactCompilationError {
    fn name(&self) -> &str {
        "ProgramArtifactCompilationError"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

pub struct CompilerPlanningProgram {
    pub config: CompilerPlanningConfig,
    pub(crate) interpreter: Interpreter,
    published_outputs: Vec<PublishedPlanningOutput>,
}

struct PublishedPlanningOutput {
    name: Option<String>,
    source: CompilerValueSource,
}

impl CompilerPlanningProgram {
    pub fn new(config: CompilerPlanningConfig) -> Self {
        #[cfg(feature = "functions")]
        {
            Self::with_function_catalog(config, crate::empty_function_catalog())
        }
        #[cfg(not(feature = "functions"))]
        {
            let id = hash_str(&format!("program/{}", config.name));
            let interpreter = Interpreter::new(id, config.limits.max_planning_steps);

            Self {
                config,
                interpreter,
                published_outputs: Vec::new(),
            }
        }
    }

    #[cfg(feature = "functions")]
    pub fn with_function_catalog(
        config: CompilerPlanningConfig,
        catalog: Arc<FunctionCatalog>,
    ) -> Self {
        let id = hash_str(&format!("program/{}", config.name));
        let interpreter =
            Interpreter::with_function_catalog(id, config.limits.max_planning_steps, catalog);

        Self {
            config,
            interpreter,
            published_outputs: Vec::new(),
        }
    }

    #[cfg(feature = "functions")]
    pub fn function_catalog(&self) -> &Arc<FunctionCatalog> {
        self.interpreter.function_catalog()
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn install_function_module(&mut self, module: &str) -> MResult<()> {
        crate::load_module(&self.interpreter, module)?;
        Ok(())
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn import_compiler_function_item(&mut self, module: &str, item: &str) -> MResult<()> {
        crate::import_module_item(&self.interpreter, module, item)
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn import_compiler_function_item_as(
        &mut self,
        module: &str,
        item: &str,
        alias: &str,
    ) -> MResult<()> {
        crate::import_module_item_as(&self.interpreter, module, item, alias)
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn import_compiler_function_glob(&mut self, module: &str) -> MResult<()> {
        crate::import_module_glob(&self.interpreter, module)
    }

    /// Installs one context import that the source compiler already resolved
    /// through its host-interface/module-manifest catalogs.
    #[cfg(feature = "source")]
    pub fn install_compiler_context(&mut self, alias: &str, base_uri: &str) {
        let context_name = base_uri
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(base_uri)
            .to_owned();
        self.interpreter.context_bindings.borrow_mut().insert(
            hash_str(alias),
            RuntimeContextBinding {
                context_name,
                base_uri: base_uri.to_owned(),
            },
        );
    }

    pub fn bind_compiler_catalog_export(
        &mut self,
        export: &FunctionExport,
        canonical_name: &str,
    ) -> MResult<()> {
        self.interpreter
            .state
            .borrow_mut()
            .function_environment
            .bind_catalog_export(export, canonical_name)
    }

    pub fn register_function_extension(
        &mut self,
        name: impl Into<String>,
        specializer: Arc<dyn CanonicalFunctionSpecializer>,
    ) -> MResult<()> {
        let canonical_name = name.into();
        let entry = FunctionExtensionEntry::new(canonical_name.clone(), specializer);
        let extension = entry.id;

        // Registration spans two checkpointed stores. Validate and apply it to
        // clones first so a collision cannot leave either store half-mutated.
        let mut state = self.interpreter.state.borrow_mut();
        let mut extensions = state.function_extensions.clone();
        let mut environment = state.function_environment.clone();
        extensions.insert_or_replace(entry)?;
        environment.bind_extension(&canonical_name, &canonical_name, extension)?;
        state.function_extensions = extensions;
        state.function_environment = environment;
        Ok(())
    }

    #[cfg(all(feature = "source", feature = "native"))]
    pub fn register_native_closure(
        &mut self,
        name: impl Into<String>,
        function: impl Fn(Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
    ) -> MResult<()> {
        let name = name.into();

        self.register_function_extension(
            name.clone(),
            Arc::new(ClosureFunctionSpecializer::new(name, function)),
        )
    }

    /// Plans a parsed source tree inside the compiler's short-lived mutable
    /// workspace. The resulting value is a compiler coordinate only; product
    /// callers receive `ProgramCompilationProduct` after finalization.
    pub fn plan_tree_with_services(
        &mut self,
        tree: &mech_core::Program,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<Option<ValueCell>> {
        self.interpreter.interpret_with_services(tree, services)
    }

    /// Plans source for an executable artifact, retaining user-function call
    /// graphs in the caller plan so artifact lowering can inline their ordinary
    /// operations and preserve dependencies on caller-owned inputs.
    ///
    /// Interactive evaluation keeps the normal call-local plan boundary; only
    /// compilation needs the expanded graph after the function frame returns.
    pub fn plan_artifact_tree_with_services(
        &mut self,
        tree: &mech_core::Program,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<Option<ValueCell>> {
        let _persistent_user_function_plan =
            crate::function::PersistentUserFunctionPlanScope::enter(&self.interpreter);
        self.interpreter.interpret_with_services(tree, services)
    }

    #[cfg(test)]
    pub(crate) fn plan_source_for_test(&mut self, source: &str) -> MResult<Option<ValueCell>> {
        let tree = mech_syntax::parser::parse(source.trim())?;
        let mut services = NoMechExecutionServices;
        self.plan_tree_with_services(&tree, &mut services)
    }

    /// Returns a host-inspection snapshot of a root symbol.
    ///
    /// Compiler ownership paths must use `compiler_root_symbol_cell` instead.
    pub fn compiler_root_symbol_value(&self, name: &str) -> MResult<Value> {
        let mut values = self.compiler_root_symbol_values(&[name])?;
        Ok(values.remove(0).1)
    }

    /// Returns host-inspection snapshots of selected root symbols.
    ///
    /// These cloned values do not carry compiler register ownership.
    pub fn compiler_root_symbol_values(&self, names: &[&str]) -> MResult<Vec<(String, Value)>> {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut values = Vec::with_capacity(names.len());
        for name in names {
            let symbol_id = hash_str(name);
            let Some(value_ref) = symbols_brrw.get(symbol_id) else {
                return Err(MechError::new(
                    ProgramOutputNotFound {
                        name: (*name).to_string(),
                    },
                    None,
                ));
            };
            values.push(((*name).to_string(), value_ref.snapshot()?));
        }
        Ok(values)
    }

    /// Returns host-inspection snapshots of every root symbol.
    pub fn compiler_root_symbol_values_all(&self) -> MResult<Vec<(String, Value)>> {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut values = symbol_rows(&symbols_brrw, &[])?;
        values.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(values)
    }

    /// Returns the original compiler-owned cell for one root symbol.
    pub fn compiler_root_symbol_cell(&self, name: &str) -> MResult<ValueCell> {
        let mut cells = self.compiler_root_symbol_cells(&[name])?;
        Ok(cells.remove(0).1)
    }

    /// Returns the lexical root symbol that owns `cell`, when the program's
    /// final expression is a direct symbol projection. This preserves the
    /// source name when a multi-root compiler moves that projection into the
    /// shared artifact program.
    pub fn compiler_root_symbol_name_for_cell(&self, cell: &ValueCell) -> Option<String> {
        let symbols = self.interpreter.symbols();
        let symbols = symbols.borrow();
        let mut names = symbol_cell_rows(&symbols, &[])
            .into_iter()
            .filter_map(|(name, candidate)| candidate.same_cell(cell).then_some(name))
            .collect::<Vec<_>>();
        names.sort();
        names.into_iter().next()
    }

    /// Returns the original compiler-owned cells for selected root symbols.
    pub fn compiler_root_symbol_cells(&self, names: &[&str]) -> MResult<Vec<(String, ValueCell)>> {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut cells = Vec::with_capacity(names.len());
        for name in names {
            let symbol_id = hash_str(name);
            let Some(cell) = symbols_brrw.get(symbol_id) else {
                return Err(MechError::new(
                    ProgramOutputNotFound {
                        name: (*name).to_string(),
                    },
                    None,
                ));
            };
            cells.push(((*name).to_string(), cell));
        }
        Ok(cells)
    }

    /// Resolves stable formatted-document output addresses inside the
    /// short-lived compiler planner. Product runtimes only receive the
    /// resulting artifact output ordinals; the planner map never escapes.
    pub fn compiler_document_output_cells(
        &self,
        output_ids: &[u64],
    ) -> MResult<Vec<Option<ValueCell>>> {
        let outputs = self.interpreter.out_values.borrow();
        output_ids
            .iter()
            .map(|output_id| {
                outputs.get(output_id).cloned().ok_or_else(|| {
                    MechError::new(
                        ProgramOutputNotFound {
                            name: format!("document output {output_id}"),
                        },
                        None,
                    )
                })
            })
            .collect()
    }

    /// Retains the observable result of one explicitly requested source root.
    /// Dependency-only planning never calls this method, so multi-root
    /// products publish each requested root exactly once in caller order.
    pub fn publish_compiler_root_output(&mut self, cell: ValueCell) {
        self.published_outputs.push(PublishedPlanningOutput {
            name: None,
            source: CompilerValueSource::Cell(cell),
        });
    }

    /// Retain a document-addressed output whose ordinal is part of the host
    /// rendering contract, even when its register aliases a named symbol.
    pub fn publish_compiler_document_output(&mut self, cell: Option<ValueCell>) {
        self.published_outputs.push(PublishedPlanningOutput {
            name: None,
            source: cell
                .map(CompilerValueSource::Cell)
                .unwrap_or(CompilerValueSource::Absent),
        });
    }

    /// Retain every named root symbol as a live artifact output for an
    /// interactive host. The ordinary result remains first, while explicit
    /// names may alias one shared register without losing lexical identity.
    pub fn publish_compiler_root_symbols(&mut self) {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut cells = symbol_cell_rows(&symbols_brrw, &[]);
        cells.sort_by(|left, right| left.0.cmp(&right.0));
        self.published_outputs
            .extend(
                cells
                    .into_iter()
                    .map(|(name, cell)| PublishedPlanningOutput {
                        name: Some(name),
                        source: CompilerValueSource::Cell(cell),
                    }),
            );
    }

    /// Installs one compiler-resolved source import in the ephemeral planning
    /// symbol table. This coordinate is consumed during compilation and is not
    /// retained by the runtime artifact.
    pub fn install_compiler_symbol(&mut self, name: &str, value: ValueCell) -> MResult<()> {
        let id = hash_str(name);
        let symbols = self.interpreter.symbols();
        let mut symbols = symbols.borrow_mut();
        symbols.symbols.insert(id, value);
        symbols.dictionary.borrow_mut().insert(id, name.to_owned());
        Ok(())
    }

    /// Compiles the in-memory executable artifact and its compute-region
    /// metadata without also serializing a durable bytecode container.
    ///
    /// Hosts that immediately lower an artifact for the current process do not
    /// need the duplicate bytecode representation. This is especially
    /// important for large initialized device state, which would otherwise be
    /// copied into both representations before execution begins.
    #[cfg(feature = "semantic-compiler")]
    pub fn compile_program_artifact(&mut self) -> MResult<ProgramArtifact> {
        let compiled = compile_bytecode(self)?;
        compile_executable_program_artifact_with_named_outputs_and_external_inputs(
            &compiled.bytecode,
            &compiled.published_outputs,
            &compiled.published_output_names,
            self.interpreter.function_catalog().as_ref(),
            &BTreeSet::new(),
        )
        .map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to finalize source ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })
    }

    /// Finalizes an activation-only artifact while preserving configured
    /// external contracts and explicit compute-boundary input declarations.
    #[cfg(feature = "semantic-compiler")]
    pub fn compile_program_artifact_product_with_resource_send_operations(
        &mut self,
        resolver: &dyn ExternalRequirementContractResolver,
        operations: &[CompiledResourceSendOperation],
        external_input_names: &BTreeSet<String>,
    ) -> MResult<ProgramArtifactCompilationProduct> {
        let mut compiled = compile_bytecode(self)?;
        preserve_compiled_resource_send_operations(&mut compiled.bytecode, operations)?;
        resolve_compiled_external_contracts(&mut compiled.bytecode, resolver)?;
        let artifact = compile_executable_program_artifact_with_named_outputs_and_external_inputs(
            &compiled.bytecode,
            &compiled.published_outputs,
            &compiled.published_output_names,
            self.interpreter.function_catalog().as_ref(),
            external_input_names,
        )
        .map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to finalize source ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        Ok(ProgramArtifactCompilationProduct { artifact })
    }

    #[cfg(feature = "semantic-compiler")]
    pub fn compile_program_product(&mut self) -> MResult<ProgramCompilationProduct> {
        let compiled = compile_bytecode(self)?;
        self.finalize_program_product(compiled)
    }

    #[cfg(feature = "semantic-compiler")]
    pub fn compile_program_product_with_external_contracts(
        &mut self,
        resolver: &dyn ExternalRequirementContractResolver,
    ) -> MResult<ProgramCompilationProduct> {
        let mut compiled = compile_bytecode(self)?;
        resolve_compiled_external_contracts(&mut compiled.bytecode, resolver)?;
        self.finalize_program_product(compiled)
    }

    /// Finalizes a source compiler product after replacing generic send names
    /// with the operation explicitly declared by the source context. The
    /// transformed requirements are re-canonicalized before artifact and
    /// bytecode-v1 encoding, so both routes retain identical authority data.
    #[cfg(feature = "semantic-compiler")]
    pub fn compile_program_product_with_resource_send_operations(
        &mut self,
        resolver: &dyn ExternalRequirementContractResolver,
        operations: &[CompiledResourceSendOperation],
    ) -> MResult<ProgramCompilationProduct> {
        let mut compiled = compile_bytecode(self)?;
        preserve_compiled_resource_send_operations(&mut compiled.bytecode, operations)?;
        resolve_compiled_external_contracts(&mut compiled.bytecode, resolver)?;
        self.finalize_program_product(compiled)
    }

    #[cfg(feature = "semantic-compiler")]
    fn finalize_program_product(
        &self,
        compiled: CompilerPlanningBytecode,
    ) -> MResult<ProgramCompilationProduct> {
        let instruction_type_bindings = compiled.bytecode.instruction_type_bindings.clone();
        let artifact = compile_executable_program_artifact_with_named_outputs_and_external_inputs(
            &compiled.bytecode,
            &compiled.published_outputs,
            &compiled.published_output_names,
            self.interpreter.function_catalog().as_ref(),
            &BTreeSet::new(),
        )
        .map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to finalize source ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let sections = encode_program_artifact_sections(&artifact).map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to encode source ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let bytecode = write_bytecode_with_artifact(&compiled.bytecode.program, &sections)?;
        Ok(ProgramCompilationProduct {
            artifact,
            bytecode,
            instruction_type_bindings,
        })
    }
}

#[cfg(feature = "semantic-compiler")]
fn preserve_compiled_resource_send_operations(
    compiled: &mut CompiledBytecode,
    operations: &[CompiledResourceSendOperation],
) -> MResult<()> {
    if operations.is_empty() {
        return Ok(());
    }

    let mut rewritten = compiled.program.requirements.clone();
    for requirement in &mut rewritten {
        let ApplicationRequirement::Resource(request) = requirement else {
            continue;
        };
        if request.intent != ResourceIntent::Send {
            continue;
        }
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
                        return Err(MechError::new(
                            GenericError {
                                msg: format!(
                                    "Resource send `{}/{}` resolves to both `{operation}` and `{}`",
                                    request.base_uri, request.path, declaration.operation,
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                }
            }
            if selected.is_some() {
                break;
            }
        }
        if let Some(operation) = selected {
            request.operation = operation.to_owned();
            validate_application_requirement(requirement)?;
        }
    }

    let mut canonical = rewritten.clone();
    canonical.sort_by(compare_application_requirements);
    canonical.dedup();
    let remap = rewritten
        .iter()
        .map(|requirement| {
            canonical
                .binary_search_by(|candidate| {
                    compare_application_requirements(candidate, requirement)
                })
                .map(|index| index as u32)
                .map_err(|_| {
                    MechError::new(
                        GenericError {
                            msg: "Unable to canonicalize rewritten resource requirement".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })
        })
        .collect::<MResult<Vec<_>>>()?;
    for instruction in &mut compiled.program.instructions {
        let requirement = match instruction {
            mech_core::BytecodeInstruction::HostCall { requirement, .. }
            | mech_core::BytecodeInstruction::ResourceRead { requirement, .. }
            | mech_core::BytecodeInstruction::ResourceWrite { requirement, .. }
            | mech_core::BytecodeInstruction::ResourceSend { requirement, .. } => requirement,
            _ => continue,
        };
        *requirement = *remap.get(*requirement as usize).ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: "Resource requirement remap index is out of range".to_owned(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
    }
    compiled.program.requirements = canonical;
    Ok(())
}

/// Builds the executable compiler product before the C3 semantic artifact is
/// finalized. Keeping this boundary separate makes intermediate compiler
/// metadata explicit without exposing it through `ProgramArtifact`.
#[cfg(all(feature = "semantic-compiler", feature = "invariant_define"))]
struct RetainedIntegrityMarkerMetadata {
    result: ValueCell,
    name: ValueCell,
    expression: ValueCell,
    operator: Option<ValueCell>,
    lhs: Option<ValueCell>,
    rhs: Option<ValueCell>,
}

#[cfg(feature = "semantic-compiler")]
struct CompilerPlanningBytecode {
    bytecode: CompiledBytecode,
    published_outputs: Box<[Register]>,
    published_output_names: Box<[Option<String>]>,
}

#[cfg(feature = "semantic-compiler")]
fn compile_bytecode(program: &mut CompilerPlanningProgram) -> MResult<CompilerPlanningBytecode> {
    let state = program.interpreter.state.borrow();
    let plan = state.plan.borrow();
    let mut context = CompileCtx::new();

    {
        let symbols = state.symbol_table.borrow();
        for (name, cell) in symbol_cell_rows(&symbols, &[]) {
            context.retain_compiler_symbol_cell(&name, &cell)?;
        }
    }
    for output in &program.published_outputs {
        output.source.retain_cells(&mut context)?;
    }
    #[cfg(feature = "invariant_define")]
    for constraint in state.integrity_constraints.values() {
        context.retain_compiler_value_cell(&constraint.result)?;
        if let Some(lhs) = &constraint.lhs {
            context.retain_compiler_value_cell(lhs)?;
        }
        if let Some(rhs) = &constraint.rhs {
            context.retain_compiler_value_cell(rhs)?;
        }
    }
    for step in plan.iter() {
        for cell in step.compiler_owned_value_cells() {
            context.retain_compiler_value_cell(&cell)?;
        }
    }

    // State declarations retain their declaration-time initializer even when
    // source execution has already advanced the corresponding reactive cell.
    for step in plan.iter() {
        step.reserve_bytecode_registers(&mut context)?;
    }

    for step in plan.iter() {
        context.begin_plan_node_with_type_binding(
            match step.reactive_node_kind() {
                ReactiveNodeKind::Combinational => CompiledNodeKind::Combinational,
                ReactiveNodeKind::Register => CompiledNodeKind::Register,
            },
            step.semantic_operation_name(),
            step.semantic_operation_contract(),
            step.bound_call(),
        )?;
        let compile_result = step.compile(&mut context);
        context.end_plan_node();
        compile_result?;
    }

    context.associate_retained_symbol_cells_with_existing_value_registers()?;

    for region in &state.compute_regions {
        let start = u32::try_from(region.plan_nodes.start).map_err(|_| {
            MechError::new(
                BytecodeValidationError {
                    reason: format!(
                        "compute region `{}` start exceeds source node identity space",
                        region.name,
                    ),
                },
                None,
            )
        })?;
        let end = u32::try_from(region.plan_nodes.end).map_err(|_| {
            MechError::new(
                BytecodeValidationError {
                    reason: format!(
                        "compute region `{}` end exceeds source node identity space",
                        region.name,
                    ),
                },
                None,
            )
        })?;
        context.record_compute_region(region.name.clone(), region.placement, start..end)?;
    }

    #[cfg(feature = "invariant_define")]
    let retained_integrity_metadata = if !state.integrity_constraints.is_empty() {
        let mut metadata = Vec::with_capacity(state.integrity_constraints.len());
        for constraint in state.integrity_constraints.values() {
            let result_register =
                mech_core::compile_value_cell_register(&constraint.result, &mut context)?;
            context.record_integrity_constraint(constraint.name.clone(), result_register)?;
            let name = ValueCell::from_exact(constraint.name.clone())?;
            let expression = ValueCell::from_exact(constraint.expression.clone())?;
            let operator = match &constraint.operator {
                Some(FormulaOperator::Comparison(ComparisonOp::Equal)) => Some("eq"),
                Some(FormulaOperator::Comparison(ComparisonOp::NotEqual)) => Some("neq"),
                Some(FormulaOperator::Comparison(ComparisonOp::LessThan)) => Some("lt"),
                Some(FormulaOperator::Comparison(ComparisonOp::LessThanEqual)) => Some("lte"),
                Some(FormulaOperator::Comparison(ComparisonOp::GreaterThan)) => Some("gt"),
                Some(FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual)) => Some("gte"),
                Some(other) => {
                    return Err(MechError::new(
                        BytecodeValidationError {
                            reason: format!(
                                "integrity constraint {:?} has unsupported operator {other:?}",
                                constraint.name
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                None => None,
            }
            .map(|operator| ValueCell::from_exact(operator.to_owned()))
            .transpose()?;
            metadata.push(RetainedIntegrityMarkerMetadata {
                result: constraint.result.clone(),
                name,
                expression,
                operator,
                lhs: constraint.lhs.clone(),
                rhs: constraint.rhs.clone(),
            });
        }
        metadata
    } else {
        Vec::new()
    };

    #[cfg(feature = "invariant_define")]
    let retained_integrity_marker_output = if retained_integrity_metadata.is_empty() {
        None
    } else {
        Some(ValueCell::from_exact(false)?)
    };

    #[cfg(feature = "invariant_define")]
    if let Some(marker_output) = retained_integrity_marker_output.as_ref() {
        let marker_register = mech_core::compile_value_cell_register(marker_output, &mut context)?;
        for marker in &retained_integrity_metadata {
            let result = mech_core::compile_value_cell_register(&marker.result, &mut context)?;
            let lhs = match &marker.lhs {
                Some(cell) => mech_core::compile_value_cell_register(cell, &mut context)?,
                None => mech_core::compile_absent_register(&mut context)?,
            };
            let rhs = match &marker.rhs {
                Some(cell) => mech_core::compile_value_cell_register(cell, &mut context)?,
                None => mech_core::compile_absent_register(&mut context)?,
            };
            let metadata = vec![
                result,
                mech_core::compile_value_cell_register(&marker.name, &mut context)?,
                mech_core::compile_value_cell_register(&marker.expression, &mut context)?,
                match &marker.operator {
                    Some(operator) => {
                        mech_core::compile_value_cell_register(operator, &mut context)?
                    }
                    None => mech_core::compile_absent_register(&mut context)?,
                },
                lhs,
                rhs,
            ];
            context.emit_integrity_marker(
                hash_str("integrity/constraint"),
                marker_register,
                metadata,
            );
        }
    }

    let mut resolved_outputs = Vec::with_capacity(program.published_outputs.len());
    for output in &program.published_outputs {
        resolved_outputs.push((
            output.name.clone(),
            output.source.compile_register(&mut context)?,
        ));
    }
    let mut published_outputs = Vec::with_capacity(resolved_outputs.len());
    let mut published_output_names = Vec::with_capacity(resolved_outputs.len());
    for (name, register) in &resolved_outputs {
        published_outputs.push(*register);
        published_output_names.push(name.clone());
    }
    let published_outputs = published_outputs.into_boxed_slice();
    let published_output_names = published_output_names.into_boxed_slice();
    // Materializing a composite return can emit CompositePack. Give any such
    // instruction normal source-node metadata instead of leaving it outside
    // the compiler's semantic plan boundary.
    context.begin_plan_node(CompiledNodeKind::Combinational)?;
    let return_result = match &program.interpreter.out {
        Some(cell) => mech_core::compile_value_cell_register(cell, &mut context),
        None => mech_core::compile_absent_register(&mut context),
    };
    context.end_plan_node();
    let return_register = return_result?;
    let bytecode = context.finish_program(return_register)?;

    #[cfg(feature = "invariant_define")]
    {
        drop(retained_integrity_marker_output);
        drop(retained_integrity_metadata);
    }

    Ok(CompilerPlanningBytecode {
        bytecode,
        published_outputs,
        published_output_names,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramOutputNotFound {
    pub name: String,
}
impl MechErrorKind for ProgramOutputNotFound {
    fn name(&self) -> &str {
        "ProgramOutputNotFound"
    }
    fn message(&self) -> String {
        format!("program output symbol `{}` was not found", self.name)
    }
}

fn symbol_rows(
    symbol_table: &mech_core::SymbolTable,
    names: &[String],
) -> MResult<Vec<(String, Value)>> {
    let dictionary = symbol_table.dictionary.borrow();
    let mut rows = Vec::new();

    if !names.is_empty() {
        for target_name in names {
            for (id, name) in dictionary.iter() {
                if name == target_name {
                    if let Some(value_ref) = symbol_table.symbols.get(id) {
                        rows.push((name.clone(), value_ref.snapshot()?));
                    }
                    break;
                }
            }
        }
    } else {
        for (id, value_ref) in symbol_table.symbols.iter() {
            if let Some(name) = dictionary.get(id) {
                rows.push((name.clone(), value_ref.snapshot()?));
            }
        }
    }

    Ok(rows)
}

fn symbol_cell_rows(
    symbol_table: &mech_core::SymbolTable,
    names: &[String],
) -> Vec<(String, ValueCell)> {
    let dictionary = symbol_table.dictionary.borrow();
    let mut rows = Vec::new();

    if !names.is_empty() {
        for target_name in names {
            for (id, name) in dictionary.iter() {
                if name == target_name {
                    if let Some(cell) = symbol_table.symbols.get(id) {
                        rows.push((name.clone(), cell.clone()));
                    }
                    break;
                }
            }
        }
    } else {
        for (id, cell) in symbol_table.symbols.iter() {
            if let Some(name) = dictionary.get(id) {
                rows.push((name.clone(), cell.clone()));
            }
        }
    }

    rows
}

#[cfg(test)]
fn test_mech_program(config: CompilerPlanningConfig) -> CompilerPlanningProgram {
    CompilerPlanningProgram::with_function_catalog(
        config,
        crate::test_support::catalog::function_catalog(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "functions")]
    use mech_core::FunctionCatalogBuilder;
    #[cfg(all(
        feature = "semantic-compiler",
        feature = "source",
        feature = "invariant_define",
        feature = "compare_default",
        feature = "f64"
    ))]
    use mech_core::{BytecodeInstruction, ParsedProgram, Register};
    use std::collections::BTreeMap;

    #[cfg(all(
        feature = "source",
        feature = "functions",
        feature = "program",
        feature = "f64",
        feature = "matrix",
        feature = "compare_default",
        feature = "invariant_define"
    ))]
    fn assert_ordinary_source_artifact_parity(
        artifact_a: &ProgramArtifact,
        artifact_b: &ProgramArtifact,
    ) {
        assert_eq!(artifact_a.contracts(), artifact_b.contracts());
        assert_eq!(artifact_a.inputs(), artifact_b.inputs());
        assert_eq!(artifact_a.slots(), artifact_b.slots());
        assert_eq!(artifact_a.bindings(), artifact_b.bindings());
        assert_eq!(artifact_a.outputs(), artifact_b.outputs());
        assert_eq!(artifact_a.constraints(), artifact_b.constraints());
        assert_eq!(artifact_a.schemas().len(), artifact_b.schemas().len());
        assert_eq!(artifact_a.constants().len(), artifact_b.constants().len());
        assert_eq!(artifact_a.nodes().len(), artifact_b.nodes().len());
        assert_eq!(artifact_a.revision(), artifact_b.revision());
    }

    #[cfg(all(
        feature = "source",
        feature = "functions",
        feature = "program",
        feature = "f64",
        feature = "matrix",
        feature = "compare_default",
        feature = "invariant_define"
    ))]
    #[test]
    fn ordinary_mech_sources_emit_equivalent_program_artifacts_in_bytecode_v1() -> MResult<()> {
        let mut executable_node_count = 0;
        for source in [
            include_str!("../../tests/fixtures/program-artifact/scalar-alias.mec"),
            include_str!("../../tests/fixtures/program-artifact/state-register.mec"),
            include_str!("../../tests/fixtures/program-artifact/matrix-literal.mec"),
            include_str!("../../tests/fixtures/program-artifact/comparison-output.mec"),
            include_str!("../../tests/fixtures/program-artifact/integrity-constraint.mec"),
        ] {
            let mut program = test_mech_program(CompilerPlanningConfig::default());
            program.plan_source_for_test(source)?;
            let product = program.compile_program_product()?;
            let artifact_a = product.artifact();
            let parsed = ParsedProgram::from_bytes(product.bytecode())?;
            let artifact_b = decode_program_artifact_sections(&parsed.artifact)
                .expect("ordinary source bytecode-v1 artifact sections must decode");

            assert_ordinary_source_artifact_parity(artifact_a, &artifact_b);
            assert!(!artifact_a.schemas().is_empty());
            executable_node_count += artifact_a.nodes().len();
            assert!(artifact_a.nodes().iter().all(|node| matches!(
                artifact_a.contracts().get(node.contract),
                Some(mech_core::ResolvedOperationContract::Declared(_))
            )));
        }
        assert!(executable_node_count > 0);
        Ok(())
    }

    #[cfg(all(
        feature = "source",
        feature = "functions",
        feature = "program",
        feature = "f64",
        feature = "matrix",
        feature = "compare_default",
        feature = "invariant_define"
    ))]
    fn compile_source_artifact(source: &str) -> MResult<ProgramArtifact> {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program.plan_source_for_test(source)?;
        Ok(program.compile_program_product()?.into_parts().0)
    }

    #[cfg(all(
        feature = "source",
        feature = "functions",
        feature = "program",
        feature = "f64",
        feature = "matrix",
        feature = "compare_default",
        feature = "invariant_define"
    ))]
    #[test]
    fn source_composites_and_mutable_state_keep_exact_artifact_semantics() -> MResult<()> {
        for source in [
            "(1.0, 2.0)",
            "first := 1.0\nsecond := 2.0\npair := (first, second)\npair",
        ] {
            let artifact = compile_source_artifact(source)?;
            let composite = artifact
                .nodes()
                .iter()
                .find(|node| {
                    node.operation.module_path.as_ref() == ["core"]
                        && node.operation.operation_name == "composite-pack"
                })
                .expect("source tuple must retain a reactive composite-pack node");
            assert!(composite.input_bindings.len() >= 2);
            assert_eq!(composite.output_bindings.len(), 1);
            assert_eq!(artifact.outputs().len(), 1);
        }

        let matrix = compile_source_artifact(
            "~state := [1.0 2.0; 3.0 4.0]\nreplacement := [0.0 0.0; 0.0 0.0]\nstate = replacement\nstate",
        )?;
        let state = matrix
            .slots()
            .iter()
            .find(|slot| slot.role == SlotRole::State)
            .expect("mutable matrix must retain a state slot");
        let InitializerReference::Constant(initializer) = state
            .initializer
            .expect("mutable matrix state must retain its declaration initializer");
        let ValueData::Matrix(initializer) = matrix.constants().get(initializer).unwrap().data()
        else {
            panic!("mutable matrix initializer must remain a matrix")
        };
        let mech_core::snapshot::SequenceView::F64(values) = initializer.elements() else {
            panic!("mutable matrix initializer must retain f64 elements")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0, 4.0]
        );

        let equal =
            compile_source_artifact("input := 1.0\n~state := 1.0\nstate = input\noutput := state")?;
        let equal_state = equal
            .slots()
            .iter()
            .find(|slot| slot.role == SlotRole::State)
            .expect("equal constants must not erase the state role");
        assert!(equal_state.initializer.is_some());
        Ok(())
    }

    #[cfg(all(
        feature = "source",
        feature = "functions",
        feature = "program",
        feature = "f64",
        feature = "matrix",
        feature = "compare_default",
        feature = "invariant_define"
    ))]
    #[test]
    fn source_artifact_rejects_multiple_full_state_writers() -> MResult<()> {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program.plan_source_for_test(
            "~state := 1.0\nlimit := 2.0\nbefore := state < limit\nstate = limit\nstate = 3.0\nstate",
        )?;
        let error = program.compile_program_product().unwrap_err();
        assert_eq!(error.kind_name(), "ProgramArtifactCompilationError");
        assert!(error.kind_message().contains("InvalidStateWriterChain"));
        Ok(())
    }

    #[cfg(feature = "functions")]
    #[test]
    fn program_uses_and_retains_an_explicit_function_catalog() {
        let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
        let mut program = CompilerPlanningProgram::with_function_catalog(
            CompilerPlanningConfig::default(),
            Arc::clone(&catalog),
        );

        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));

        program.interpreter.clear();

        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));
    }

    #[cfg(all(feature = "functions", feature = "native"))]
    #[test]
    fn native_closure_registration_rejects_invalid_entries_atomically() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        let extension_count = program.interpreter.state.borrow().function_extensions.len();

        let error = program
            .register_native_closure("", |_| Ok(ValueCell::unit().snapshot().unwrap()))
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionExtensionInvalidEntry");
        assert_eq!(
            program.interpreter.state.borrow().function_extensions.len(),
            extension_count,
        );
    }

    #[cfg(all(
        feature = "functions",
        feature = "native",
        feature = "semantic-compiler",
        feature = "f64"
    ))]
    #[test]
    fn native_closure_bytecode_rejection_remains_structured() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program
            .register_native_closure("host/source-only", |_| {
                ValueCell::from_exact(4.0_f64)?.snapshot()
            })
            .unwrap();
        program
            .plan_source_for_test("source-only := host/source-only()")
            .unwrap();

        let error = program.compile_program_product().unwrap_err();
        assert_eq!(
            error.kind_name(),
            "ClosureNativeFunctionNotBytecodeCompilable",
        );
    }

    #[cfg(feature = "invariant_define")]
    fn constraint<'a>(
        program: &'a CompilerPlanningProgram,
        name: &str,
    ) -> mech_core::IntegrityConstraint {
        program
            .interpreter
            .state
            .borrow()
            .integrity_constraints
            .get(&hash_str(name))
            .unwrap()
            .clone()
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_declarations_create_live_descriptors_without_enforcement() {
        for (name, expression) in [
            ("true!", "1.0 <= 2.0"),
            ("false!", "2.0 <= 1.0"),
            ("number!", "42.0"),
        ] {
            let mut program = test_mech_program(CompilerPlanningConfig::default());
            program
                .plan_source_for_test(&format!("{name} := {expression}"))
                .unwrap();
            let descriptor = constraint(&program, name);
            assert_eq!(descriptor.id, hash_str(name));
            assert_eq!(descriptor.name, name);
            assert!(!descriptor.expression.is_empty());
            assert!(!descriptor.tokens.is_empty());
            assert_eq!(
                program
                    .interpreter
                    .state
                    .borrow()
                    .integrity_constraints
                    .len(),
                1,
            );
        }
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_direct_operands_remain_live() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program
            .plan_source_for_test("target := 1.0\nmaximum := 2.0\nsafe! := target <= maximum")
            .unwrap();

        let descriptor = constraint(&program, "safe!");
        let target = program
            .interpreter
            .symbols()
            .borrow()
            .get(hash_str("target"))
            .unwrap();
        assert!(descriptor.lhs.as_ref().unwrap().same_cell(&target));
        assert!(descriptor.rhs.is_some());
        target
            .replace(&ValueCell::from_exact(3.0_f64).unwrap().snapshot().unwrap())
            .unwrap();
        let lhs = descriptor.lhs.unwrap().snapshot().unwrap();
        if let ValueData::F64(value) = lhs.data() {
            assert_eq!(value.to_f64(), 3.0);
        } else {
            panic!("captured lhs must remain the target cell");
        }
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_diagnostics_do_not_recompile_complex_operands() {
        let source = "target := 1.0\nmaximum := 3.0";
        let expression = "target + 1.0 <= maximum";
        let mut ordinary = test_mech_program(CompilerPlanningConfig::default());
        ordinary
            .plan_source_for_test(&format!("{source}\ncandidate := {expression}"))
            .unwrap();
        let ordinary_plan_len = ordinary.interpreter.plan_len();

        let mut constrained = test_mech_program(CompilerPlanningConfig::default());
        constrained
            .plan_source_for_test(&format!("{source}\nsafe! := {expression}"))
            .unwrap();
        let descriptor = constraint(&constrained, "safe!");

        assert_eq!(constrained.interpreter.plan_len(), ordinary_plan_len);
        assert!(descriptor.lhs.is_none());
        assert!(descriptor.operator.is_some());
        assert!(descriptor.rhs.is_some());
    }

    #[cfg(all(
        feature = "semantic-compiler",
        feature = "source",
        feature = "invariant_define",
        feature = "compare_default",
        feature = "f64"
    ))]
    #[test]
    fn integrity_marker_metadata_lives_through_bytecode_finalization() -> MResult<()> {
        let source = concat!(
            "finite-candidate! := 1.0 <= 2.0\n",
            "positive-covariance! := 2.0 <= 3.0\n",
            "symmetric-covariance! := 3.0 <= 4.0",
        );
        let expected_names = BTreeSet::from([
            "finite-candidate!".to_owned(),
            "positive-covariance!".to_owned(),
            "symmetric-covariance!".to_owned(),
        ]);

        for _ in 0..64 {
            let mut program = test_mech_program(CompilerPlanningConfig::default());
            program.plan_source_for_test(source)?;
            let expected_expressions = program
                .interpreter
                .state
                .borrow()
                .integrity_constraints
                .values()
                .map(|constraint| (constraint.name.clone(), constraint.expression.clone()))
                .collect::<BTreeMap<_, _>>();
            let product = program.compile_program_product()?;
            let parsed = ParsedProgram::from_bytes(product.bytecode())?;
            let constants = parsed.decode_constants()?;
            let constant_registers = parsed
                .instructions
                .iter()
                .filter_map(|instruction| match instruction {
                    BytecodeInstruction::ConstLoad { dst, constant } => Some((*dst, *constant)),
                    _ => None,
                })
                .collect::<BTreeMap<Register, u32>>();
            let string_at = |register: Register| -> String {
                let constant = constant_registers
                    .get(&register)
                    .expect("marker String metadata must be loaded from a constant");
                match constants[*constant as usize].data() {
                    mech_core::ValueData::String(value) => value.to_string(),
                    other => {
                        panic!("marker metadata register {register} must be String, got {other:?}")
                    }
                }
            };
            let markers = parsed
                .instructions
                .iter()
                .filter_map(|instruction| match instruction {
                    BytecodeInstruction::RuntimeVariadic {
                        function,
                        arguments,
                        ..
                    } if *function == hash_str("integrity/constraint") => Some(arguments),
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(markers.len(), 3);
            let mut decoded_names = BTreeSet::new();
            let mut distinct_string_registers = BTreeSet::new();
            for arguments in markers {
                assert_eq!(arguments.len(), 6);
                let name = string_at(arguments[1]);
                let expression = string_at(arguments[2]);
                assert_eq!(
                    parsed.symbols.get(&hash_str(&name)),
                    Some(&arguments[0]),
                    "integrity marker must remain associated with its named result register",
                );
                assert_eq!(expected_expressions.get(&name), Some(&expression));
                assert!(decoded_names.insert(name));
                assert!(distinct_string_registers.insert(arguments[1]));
                assert!(distinct_string_registers.insert(arguments[2]));
            }
            assert_eq!(decoded_names, expected_names);
            assert_eq!(distinct_string_registers.len(), 6);
        }

        Ok(())
    }
}

#[cfg(all(test, feature = "source"))]
mod root_symbol_snapshot_tests {
    use super::*;

    fn f64_value(value: &Value) -> f64 {
        match value.data() {
            ValueData::F64(value) => value.to_f64(),
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[test]
    fn root_symbol_value_returns_value() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program.plan_source_for_test("answer := 42.0").unwrap();
        assert_eq!(
            f64_value(&program.compiler_root_symbol_value("answer").unwrap()),
            42.0
        );
    }

    #[test]
    fn root_symbol_values_preserve_order() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program
            .plan_source_for_test("a := 1.0\nb := 2.0\nc := 3.0")
            .unwrap();
        let rows = program
            .compiler_root_symbol_values(&["c", "a", "b"])
            .unwrap();
        let names: Vec<_> = rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["c", "a", "b"]);
    }

    #[test]
    fn root_symbol_cells_preserve_order_and_original_identity() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program.plan_source_for_test("a := 1.0\nb := 2.0").unwrap();
        let symbols = program.interpreter.symbols();
        let a = symbols.borrow().get(hash_str("a")).unwrap();

        let rows = program.compiler_root_symbol_cells(&["b", "a"]).unwrap();

        assert_eq!(rows[0].0, "b");
        assert_eq!(rows[1].0, "a");
        assert!(rows[1].1.same_cell(&a));
        assert!(
            program
                .compiler_root_symbol_cell("a")
                .unwrap()
                .same_cell(&a)
        );
    }

    #[test]
    fn root_symbol_publication_retains_cells_and_lexical_aliases() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        let shared = ValueCell::from_exact(7.0_f64).unwrap();
        program
            .install_compiler_symbol("left", shared.clone())
            .unwrap();
        program
            .install_compiler_symbol("right", shared.clone())
            .unwrap();

        program.publish_compiler_root_symbols();

        let alias_outputs = program
            .published_outputs
            .iter()
            .filter(|output| matches!(output.name.as_deref(), Some("left" | "right")))
            .collect::<Vec<_>>();
        assert_eq!(alias_outputs.len(), 2);
        for output in alias_outputs {
            let CompilerValueSource::Cell(cell) = &output.source else {
                panic!("root symbols always retain canonical cells")
            };
            assert!(cell.same_cell(&shared));
        }
        let compiled = compile_bytecode(&mut program).unwrap();
        let left = compiled
            .published_output_names
            .iter()
            .position(|name| name.as_deref() == Some("left"))
            .unwrap();
        let right = compiled
            .published_output_names
            .iter()
            .position(|name| name.as_deref() == Some("right"))
            .unwrap();
        assert_ne!(left, right);
        assert_eq!(
            compiled.published_outputs[left],
            compiled.published_outputs[right]
        );
    }

    #[test]
    fn root_symbol_values_snapshot_multiple_values() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program.plan_source_for_test("a := 1.0\nb := 2.0").unwrap();
        let rows = program.compiler_root_symbol_values(&["a", "b"]).unwrap();
        assert_eq!(f64_value(&rows[0].1), 1.0);
        assert_eq!(f64_value(&rows[1].1), 2.0);
    }

    #[test]
    fn root_symbol_values_all_are_sorted_by_name() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program
            .plan_source_for_test("c := 3.0\na := 1.0\nb := 2.0")
            .unwrap();
        let rows = program.compiler_root_symbol_values_all().unwrap();
        let names: Vec<_> = rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["a", "ans", "b", "c"]);
    }

    #[cfg(all(
        feature = "semantic-compiler",
        feature = "source",
        feature = "table",
        feature = "record",
        feature = "kind_annotation",
        feature = "string",
        feature = "f64"
    ))]
    #[test]
    fn dynamic_table_columns_compile_inside_records_without_schema_id_collisions() -> MResult<()> {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program
            .plan_source_for_test(
                r#"
scene :=
  | fill<*> stroke<*> |
  | "none"  4.0       |
  | 3.0     "none"    |
"#,
            )
            .expect("dynamic table source planning succeeds");
        let result = program
            .plan_source_for_test("presentation := { circles: scene }\npresentation")
            .expect("record source planning succeeds");
        program.publish_compiler_root_output(result.expect("source returns the presentation"));

        let artifact = program.compile_program_artifact()?;
        assert_eq!(artifact.outputs().len(), 1);
        Ok(())
    }

    #[test]
    fn missing_root_symbol_returns_structured_error() {
        let program = test_mech_program(CompilerPlanningConfig::default());
        let err = program.compiler_root_symbol_value("missing").unwrap_err();
        assert!(format!("{:?}", err).contains("ProgramOutputNotFound"));
    }

    #[test]
    fn snapshot_does_not_hold_symbol_table_borrow() {
        let mut program = test_mech_program(CompilerPlanningConfig::default());
        program.plan_source_for_test("answer := 42.0").unwrap();
        let _snapshot = program.compiler_root_symbol_value("answer").unwrap();
        let symbols = program.interpreter.symbols();
        let _mutable_borrow = symbols.borrow_mut();
    }
}
