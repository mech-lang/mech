use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[cfg(feature = "functions")]
use mech_core::FunctionCatalog;
use mech_core::{
    ApplicationRequirement, FunctionSpecializer, LegacyValue, MResult, MechError, MechErrorKind,
    MechFunction, ResourceIntent, ValueKind, compare_application_requirements, hash_str,
    validate_application_requirement, validate_stable_value_update,
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
    published_outputs: Vec<LegacyValue>,
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
        specializer: Arc<dyn FunctionSpecializer>,
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
        function: impl Fn(Vec<LegacyValue>) -> MResult<LegacyValue> + Send + Sync + 'static,
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
    ) -> MResult<LegacyValue> {
        self.interpreter.interpret_with_services(tree, services)
    }

    #[cfg(test)]
    pub(crate) fn plan_source_for_test(&mut self, source: &str) -> MResult<LegacyValue> {
        let tree = mech_syntax::parser::parse(source.trim())?;
        let mut services = NoMechExecutionServices;
        self.plan_tree_with_services(&tree, &mut services)
    }

    pub fn compiler_root_symbol_value(&self, name: &str) -> MResult<LegacyValue> {
        let mut values = self.compiler_root_symbol_values(&[name])?;
        Ok(values.remove(0).1)
    }

    pub fn compiler_root_symbol_values(
        &self,
        names: &[&str],
    ) -> MResult<Vec<(String, LegacyValue)>> {
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
            values.push(((*name).to_string(), value_ref.borrow().clone()));
        }
        Ok(values)
    }

    pub fn compiler_root_symbol_values_all(&self) -> Vec<(String, LegacyValue)> {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut values = symbol_rows(&symbols_brrw, &[]);
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values
    }

    /// Resolves stable formatted-document output addresses inside the
    /// short-lived compiler planner. Product runtimes only receive the
    /// resulting artifact output ordinals; the planner map never escapes.
    pub fn compiler_document_output_values(&self, output_ids: &[u64]) -> MResult<Vec<LegacyValue>> {
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
    pub fn publish_compiler_root_output(&mut self, value: LegacyValue) {
        self.published_outputs.push(value);
    }

    /// Installs one compiler-resolved source import in the ephemeral planning
    /// symbol table. This coordinate is consumed during compilation and is not
    /// retained by the runtime artifact.
    pub fn install_compiler_symbol(&mut self, name: &str, value: Ref<LegacyValue>) -> MResult<()> {
        let id = hash_str(name);
        let symbols = self.interpreter.symbols();
        let mut symbols = symbols.borrow_mut();
        symbols.symbols.insert(id, value);
        symbols.dictionary.borrow_mut().insert(id, name.to_owned());
        Ok(())
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
        let artifact = compile_executable_program_artifact_with_outputs(
            &compiled.bytecode,
            &compiled.published_outputs,
            self.interpreter.function_catalog().as_ref(),
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
        Ok(ProgramCompilationProduct { artifact, bytecode })
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
/// finalized. Keeping this boundary separate makes the adapter's legacy input
/// explicit without exposing it through `ProgramArtifact`.
#[cfg(all(feature = "semantic-compiler", feature = "invariant_define"))]
struct RetainedIntegrityMarkerMetadata {
    result_register: Register,
    name: LegacyValue,
    expression: LegacyValue,
    operator: LegacyValue,
    lhs: LegacyValue,
    rhs: LegacyValue,
}

#[cfg(feature = "semantic-compiler")]
struct CompilerPlanningBytecode {
    bytecode: CompiledBytecode,
    published_outputs: Box<[Register]>,
}

#[cfg(feature = "semantic-compiler")]
fn compile_bytecode(program: &mut CompilerPlanningProgram) -> MResult<CompilerPlanningBytecode> {
    let state = program.interpreter.state.borrow();
    let plan = state.plan.borrow();
    let mut context = CompileCtx::new();

    for step in plan.iter() {
        context.begin_plan_node_with_contract(
            match step.reactive_node_kind() {
                ReactiveNodeKind::Combinational => CompiledNodeKind::Combinational,
                ReactiveNodeKind::Register => CompiledNodeKind::Register,
            },
            step.semantic_operation_contract(),
        )?;
        let compile_result = step.compile(&mut context);
        context.end_plan_node();
        compile_result?;
    }

    #[cfg(feature = "invariant_define")]
    let retained_integrity_metadata = if !state.integrity_constraints.is_empty() {
        let mut metadata = Vec::with_capacity(state.integrity_constraints.len());
        for constraint in state.integrity_constraints.values() {
            let result = LegacyValue::MutableReference(constraint.result.clone());
            let result_register = context.resolve_value_register(&result)?;
            context.record_integrity_constraint(result_register)?;
            let name = LegacyValue::String(Ref::new(constraint.name.clone()));
            let expression = LegacyValue::String(Ref::new(constraint.expression.clone()));
            let operator = match &constraint.operator {
                Some(FormulaOperator::Comparison(ComparisonOp::Equal)) => {
                    LegacyValue::String(Ref::new("eq".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::NotEqual)) => {
                    LegacyValue::String(Ref::new("neq".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::LessThan)) => {
                    LegacyValue::String(Ref::new("lt".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::LessThanEqual)) => {
                    LegacyValue::String(Ref::new("lte".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::GreaterThan)) => {
                    LegacyValue::String(Ref::new("gt".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual)) => {
                    LegacyValue::String(Ref::new("gte".to_owned()))
                }
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
                None => LegacyValue::Empty,
            };
            let lhs = constraint
                .lhs
                .as_ref()
                .map(|value| LegacyValue::MutableReference(value.clone()))
                .unwrap_or(LegacyValue::Empty);
            let rhs = constraint
                .rhs
                .as_ref()
                .map(|value| LegacyValue::MutableReference(value.clone()))
                .unwrap_or(LegacyValue::Empty);
            metadata.push(RetainedIntegrityMarkerMetadata {
                result_register,
                name,
                expression,
                operator,
                lhs,
                rhs,
            });
        }
        metadata
    } else {
        Vec::new()
    };

    #[cfg(feature = "invariant_define")]
    let retained_integrity_marker_output =
        (!retained_integrity_metadata.is_empty()).then(|| LegacyValue::Bool(Ref::new(false)));

    #[cfg(feature = "invariant_define")]
    if let Some(marker_output) = retained_integrity_marker_output.as_ref() {
        let marker_register = context.resolve_value_register(marker_output)?;
        for marker in &retained_integrity_metadata {
            let metadata = vec![
                marker.result_register,
                context.resolve_value_register(&marker.name)?,
                context.resolve_value_register(&marker.expression)?,
                context.resolve_value_register(&marker.operator)?,
                context.resolve_value_register(&marker.lhs)?,
                context.resolve_value_register(&marker.rhs)?,
            ];
            context.emit_integrity_marker(
                hash_str("integrity/constraint"),
                marker_register,
                metadata,
            );
        }
    }

    let published_outputs = program
        .published_outputs
        .iter()
        .map(|value| context.resolve_value_register(value))
        .collect::<MResult<Vec<_>>>()?
        .into_boxed_slice();
    let return_register = context.resolve_value_register(&program.interpreter.out)?;
    let bytecode = context.finish_program(return_register)?;

    #[cfg(feature = "invariant_define")]
    {
        drop(retained_integrity_marker_output);
        drop(retained_integrity_metadata);
    }

    Ok(CompilerPlanningBytecode {
        bytecode,
        published_outputs,
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
) -> Vec<(String, LegacyValue)> {
    let dictionary = symbol_table.dictionary.borrow();
    let mut rows = Vec::new();

    if !names.is_empty() {
        for target_name in names {
            for (id, name) in dictionary.iter() {
                if name == target_name {
                    if let Some(value_ref) = symbol_table.symbols.get(id) {
                        let value = value_ref.borrow();
                        rows.push((name.clone(), value.clone()));
                    }
                    break;
                }
            }
        }
    } else {
        for (id, value_ref) in symbol_table.symbols.iter() {
            if let Some(name) = dictionary.get(id) {
                let value = value_ref.borrow();
                rows.push((name.clone(), value.clone()));
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

#[derive(Debug, Clone)]
pub struct UnsupportedProgramSourceError {
    pub source_kind: String,
}

impl MechErrorKind for UnsupportedProgramSourceError {
    fn name(&self) -> &str {
        "UnsupportedProgramSource"
    }

    fn message(&self) -> String {
        format!("Unsupported program source: {}", self.source_kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "functions")]
    use mech_core::FunctionCatalogBuilder;
    use mech_core::Ref;
    #[cfg(all(
        feature = "semantic-compiler",
        feature = "source",
        feature = "invariant_define",
        feature = "compare_default",
        feature = "f64"
    ))]
    use mech_core::{BytecodeInstruction, ParsedProgram, Register};

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
            .register_native_closure("", |_| Ok(LegacyValue::Empty))
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
            .register_native_closure("host/source-only", |_| Ok(LegacyValue::F64(Ref::new(4.0))))
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
        assert_eq!(descriptor.lhs.as_ref().unwrap().addr(), target.addr());
        assert!(descriptor.rhs.is_some());
        if let LegacyValue::F64(value) = &*target.borrow() {
            *value.borrow_mut() = 3.0;
        } else {
            panic!("target must be f64");
        }
        if let LegacyValue::F64(value) = &*descriptor.lhs.unwrap().borrow() {
            assert_eq!(*value.borrow(), 3.0);
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
                match &constants[*constant as usize] {
                    LegacyValue::String(value) => value.borrow().clone(),
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

#[cfg(test)]
mod live_input_tests {
    use super::*;
    #[cfg(all(feature = "matrix", feature = "f64"))]
    use mech_core::structures::matrix::Matrix as MechMatrix;
    use mech_core::{Ref, hash_str};

    fn f64_value(value: &LegacyValue) -> f64 {
        match value {
            LegacyValue::F64(value) => *value.borrow(),
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_preserves_f64_reference() {
        let sink = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::F64(value) => value.as_ptr(),
            other => panic!("expected f64, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(9.0))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::F64(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 9.0);
            }
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[cfg(feature = "i64")]
    #[test]
    fn stable_value_update_preserves_i64_reference() {
        let sink = Ref::new(LegacyValue::I64(Ref::new(1)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::I64(value) => value.as_ptr(),
            other => panic!("expected i64, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::I64(Ref::new(9))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::I64(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 9);
            }
            other => panic!("expected i64, got {other:?}"),
        }
    }

    #[cfg(feature = "bool")]
    #[test]
    fn stable_value_update_preserves_bool_reference() {
        let sink = Ref::new(LegacyValue::Bool(Ref::new(false)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Bool(value) => value.as_ptr(),
            other => panic!("expected bool, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::Bool(Ref::new(true))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Bool(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert!(*value.borrow());
            }
            other => panic!("expected bool, got {other:?}"),
        }
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    #[test]
    fn stable_value_update_preserves_string_reference() {
        let sink = Ref::new(LegacyValue::String(Ref::new("old".to_string())));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::String(value) => value.as_ptr(),
            other => panic!("expected string, got {other:?}"),
        };
        apply_stable_value_update(
            sink.clone(),
            LegacyValue::String(Ref::new("new".to_string())),
        )
        .unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::String(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(&*value.borrow(), "new");
            }
            other => panic!("expected string, got {other:?}"),
        }
    }

    #[test]
    fn stable_value_update_preserves_index_reference() {
        let sink = Ref::new(LegacyValue::Index(Ref::new(1)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Index(value) => value.as_ptr(),
            other => panic!("expected index, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::Index(Ref::new(9))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Index(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 9);
            }
            other => panic!("expected index, got {other:?}"),
        }
    }

    #[cfg(all(feature = "f64", any(feature = "string", feature = "variable_define")))]
    #[test]
    fn stable_value_update_rejects_incompatible_kind() {
        let sink = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::F64(value) => value.as_ptr(),
            other => panic!("expected f64, got {other:?}"),
        };
        assert!(
            apply_stable_value_update(
                sink.clone(),
                LegacyValue::String(Ref::new("bad".to_string()))
            )
            .is_err()
        );
        match &*sink.borrow() {
            LegacyValue::F64(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 1.0);
            }
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[cfg(all(feature = "matrixd", feature = "f64"))]
    #[test]
    fn stable_value_update_preserves_dynamic_matrix_storage() {
        let sink_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            2,
            2,
            vec![1.0, 2.0, 3.0, 4.0],
        )));
        let source_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            2,
            2,
            vec![5.0, 6.0, 7.0, 8.0],
        )));
        let sink = Ref::new(LegacyValue::MatrixF64(sink_matrix));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => value.addr(),
            other => panic!("expected f64 matrix, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::MatrixF64(source_matrix.clone()))
            .unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value, &source_matrix);
            }
            other => panic!("expected f64 matrix, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_preserves_typed_scalar_reference() {
        let sink = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            ValueKind::F64,
        ));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => value.as_ptr(),
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        };

        apply_stable_value_update(
            sink.clone(),
            LegacyValue::Typed(Box::new(LegacyValue::F64(Ref::new(9.0))), ValueKind::F64),
        )
        .unwrap();

        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => {
                        assert_eq!(inner_pointer, value.as_ptr());
                        assert_eq!(*value.borrow(), 9.0);
                    }
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_rejects_different_typed_annotation() {
        let sink = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            ValueKind::F64,
        ));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Typed(inner, _) => match inner.as_ref() {
                LegacyValue::F64(value) => value.as_ptr(),
                other => panic!("expected typed f64 inner, got {other:?}"),
            },
            other => panic!("expected typed value, got {other:?}"),
        };

        let result = apply_stable_value_update(
            sink.clone(),
            LegacyValue::Typed(Box::new(LegacyValue::F64(Ref::new(9.0))), ValueKind::String),
        );
        assert!(
            format!("{:?}", result.unwrap_err()).contains("StableValueUpdateContractViolation")
        );

        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => {
                        assert_eq!(inner_pointer, value.as_ptr());
                        assert_eq!(*value.borrow(), 1.0);
                    }
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_rejects_typed_to_untyped() {
        let sink = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            ValueKind::F64,
        ));
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Typed(inner, _) => match inner.as_ref() {
                LegacyValue::F64(value) => value.as_ptr(),
                other => panic!("expected typed f64 inner, got {other:?}"),
            },
            other => panic!("expected typed value, got {other:?}"),
        };

        let result = apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(9.0)));
        assert!(
            format!("{:?}", result.unwrap_err()).contains("StableValueUpdateContractViolation")
        );

        match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => {
                        assert_eq!(inner_pointer, value.as_ptr());
                        assert_eq!(*value.borrow(), 1.0);
                    }
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        }
    }

    #[cfg(feature = "semantic-compiler")]
    #[test]
    fn empty_stable_assignment_bytecode_compile_returns_error() {
        let assignment =
            compile_stable_value_update(Ref::new(LegacyValue::Empty), LegacyValue::Empty).unwrap();
        let mut ctx = CompileCtx::new();
        let error = assignment.compile(&mut ctx).unwrap_err();
        let rendered = format!("{error:?}");
        assert!(
            rendered.contains("EmptyAssignmentNotBytecodeCompilable"),
            "{rendered}"
        );
    }

    #[test]
    fn stable_value_update_accepts_empty_to_empty() {
        let sink = Ref::new(LegacyValue::Empty);
        compile_stable_value_update(sink.clone(), LegacyValue::Empty).unwrap();
        apply_stable_value_update(sink.clone(), LegacyValue::Empty).unwrap();
        assert_eq!(&*sink.borrow(), &LegacyValue::Empty);
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_rejects_empty_to_value() {
        let sink = Ref::new(LegacyValue::Empty);
        let result = apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(1.0)));
        assert!(
            format!("{:?}", result.unwrap_err()).contains("StableValueUpdateContractViolation")
        );
        assert_eq!(&*sink.borrow(), &LegacyValue::Empty);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_dynamic_matrix_shape_change() {
        let sink_matrix = MechMatrix::from_vec((1..=25).map(|x| x as f64).collect(), 5, 5);
        let source_matrix = MechMatrix::from_vec((1..=36).map(|x| x as f64).collect(), 6, 6);
        let expected = sink_matrix.clone();
        let sink = Ref::new(LegacyValue::MatrixF64(sink_matrix));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => value.addr(),
            other => panic!("expected f64 matrix, got {other:?}"),
        };

        let error = apply_stable_value_update(sink.clone(), LegacyValue::MatrixF64(source_matrix))
            .unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");

        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value.shape(), vec![5, 5]);
                assert_eq!(value, &expected);
            }
            other => panic!("expected f64 matrix, got {other:?}"),
        }
    }

    #[cfg(all(feature = "matrixd", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_equal_length_dynamic_shape_change() {
        let sink_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            2,
            3,
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )));
        let source_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            3,
            2,
            vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
        )));
        let expected = sink_matrix.clone();
        let sink = Ref::new(LegacyValue::MatrixF64(sink_matrix));
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => value.addr(),
            other => panic!("expected f64 matrix, got {other:?}"),
        };

        let error = apply_stable_value_update(sink.clone(), LegacyValue::MatrixF64(source_matrix))
            .unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");

        match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value.shape(), vec![2, 3]);
                assert_eq!(value, &expected);
            }
            other => panic!("expected f64 matrix, got {other:?}"),
        }
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_matrix_value_sink() {
        let matrix_value = MechMatrix::from_vec(vec![LegacyValue::F64(Ref::new(1.0))], 1, 1);
        let sink = Ref::new(LegacyValue::MatrixValue(matrix_value));
        let result = apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(9.0)));
        let rendered = format!("{:?}", result.unwrap_err());
        assert!(
            rendered.contains("StableValueUpdateContractViolation"),
            "{rendered}"
        );
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_matrix_value_source() {
        let sink = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let matrix_value = MechMatrix::from_vec(vec![LegacyValue::F64(Ref::new(9.0))], 1, 1);
        let result =
            apply_stable_value_update(sink.clone(), LegacyValue::MatrixValue(matrix_value));
        let rendered = format!("{:?}", result.unwrap_err());
        assert!(
            rendered.contains("StableValueUpdateContractViolation"),
            "{rendered}"
        );
    }

    #[cfg(feature = "matrix")]
    #[test]
    fn stable_value_update_preserves_matrix_index_reference() {
        let matrix = MechMatrix::from_vec(vec![1usize, 2, 3, 4], 2, 2);
        let sink = Ref::new(LegacyValue::MatrixIndex(matrix));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixIndex(value) => value.addr(),
            other => panic!("expected index matrix, got {other:?}"),
        };
        apply_stable_value_update(
            sink.clone(),
            LegacyValue::MatrixIndex(MechMatrix::from_vec(vec![5usize, 6, 7, 8], 2, 2)),
        )
        .unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::MatrixIndex(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value.shape(), vec![2, 2]);
                assert_eq!(value, &MechMatrix::from_vec(vec![5usize, 6, 7, 8], 2, 2));
            }
            other => panic!("expected index matrix, got {other:?}"),
        }
    }

    #[cfg(all(
        feature = "record",
        feature = "map",
        feature = "set",
        feature = "table",
        feature = "tuple",
        feature = "f64",
        any(feature = "string", feature = "variable_define")
    ))]
    #[test]
    fn stable_value_update_publishes_every_composite_without_replacing_its_cell() {
        let record = |value| {
            LegacyValue::Record(Ref::new(mech_core::MechRecord::new(vec![(
                "value",
                LegacyValue::F64(Ref::new(value)),
            )])))
        };
        let map = |value| {
            LegacyValue::Map(Ref::new(mech_core::MechMap::from_typed_vec(
                ValueKind::String,
                ValueKind::F64,
                1,
                vec![(
                    LegacyValue::String(Ref::new("key".to_owned())),
                    LegacyValue::F64(Ref::new(value)),
                )],
            )))
        };
        let set = |value| {
            LegacyValue::Set(Ref::new(mech_core::MechSet::from_vec(vec![
                LegacyValue::F64(Ref::new(value)),
            ])))
        };
        let table = |value| {
            LegacyValue::Table(Ref::new(
                mech_core::MechTable::from_records(vec![mech_core::MechRecord::new(vec![(
                    "value",
                    LegacyValue::F64(Ref::new(value)),
                )])])
                .unwrap(),
            ))
        };
        let tuple = |value| {
            LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
                LegacyValue::F64(Ref::new(value)),
            ])))
        };
        let composite_addr = |value: &LegacyValue| match value {
            LegacyValue::Record(value) => value.addr(),
            LegacyValue::Map(value) => value.addr(),
            LegacyValue::Set(value) => value.addr(),
            LegacyValue::Table(value) => value.addr(),
            LegacyValue::Tuple(value) => value.addr(),
            other => panic!("expected composite, got {other:?}"),
        };
        let nested_f64 = |value: &LegacyValue| -> Option<Ref<f64>> {
            let nested = match value {
                LegacyValue::Record(value) => value.borrow().data.values().next().cloned(),
                LegacyValue::Map(value) => value.borrow().map.values().next().cloned(),
                LegacyValue::Table(value) => value
                    .borrow()
                    .data
                    .values()
                    .next()
                    .and_then(|(_, values)| values.as_vec().into_iter().next()),
                LegacyValue::Tuple(value) => value
                    .borrow()
                    .elements
                    .first()
                    .map(|value| value.as_ref().clone()),
                LegacyValue::Set(_) => None,
                other => panic!("expected composite, got {other:?}"),
            };
            nested.map(|value| match value {
                LegacyValue::F64(value) => value,
                other => panic!("expected nested F64, got {other:?}"),
            })
        };

        for (current, incoming) in [
            (record(1.0), record(2.0)),
            (map(1.0), map(2.0)),
            (set(1.0), set(2.0)),
            (table(1.0), table(2.0)),
            (tuple(1.0), tuple(2.0)),
        ] {
            let expected = incoming.clone();
            let inner_pointer = composite_addr(&current);
            let nested = nested_f64(&current);
            let nested_pointer = nested.as_ref().map(Ref::addr);
            let sink = Ref::new(current);

            compile_stable_value_update(sink.clone(), incoming)
                .unwrap()
                .stage_register()
                .unwrap()
                .commit();

            assert_eq!(composite_addr(&sink.borrow()), inner_pointer);
            assert_eq!(*sink.borrow(), expected);
            if let Some(nested) = nested {
                let updated = nested_f64(&sink.borrow()).expect("nested cell must remain present");
                assert_eq!(Some(updated.addr()), nested_pointer);
                assert_eq!(updated.addr(), nested.addr());
                assert_eq!(*nested.borrow(), 2.0);
            }
        }

        let shared = Ref::new(1.0);
        let aliased = LegacyValue::Record(Ref::new(mech_core::MechRecord::new(vec![
            ("left", LegacyValue::F64(shared.clone())),
            ("right", LegacyValue::F64(shared.clone())),
        ])));
        let distinct = LegacyValue::Record(Ref::new(mech_core::MechRecord::new(vec![
            ("left", LegacyValue::F64(Ref::new(2.0))),
            ("right", LegacyValue::F64(Ref::new(3.0))),
        ])));
        let aliased = Ref::new(aliased);
        let error = match compile_stable_value_update(aliased, distinct)
            .unwrap()
            .stage_register()
        {
            Ok(_) => panic!("aliased structured update unexpectedly staged"),
            Err(error) => error,
        };
        assert!(
            error
                .kind_message()
                .contains("cannot preserve aliased child cell")
        );
        assert_eq!(*shared.borrow(), 1.0);
    }
}

#[cfg(all(test, feature = "source"))]
mod root_symbol_snapshot_tests {
    use super::*;
    use mech_core::Ref;

    fn f64_value(value: &LegacyValue) -> f64 {
        match value {
            LegacyValue::F64(value) => *value.borrow(),
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
        let rows = program.compiler_root_symbol_values_all();
        let names: Vec<_> = rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["a", "ans", "b", "c"]);
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
