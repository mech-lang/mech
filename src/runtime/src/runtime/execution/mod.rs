// ---------------------------------------------------------------------------
// Program execution
// ---------------------------------------------------------------------------

// These are the main methods responsible for executing Mech programs within the runtime. They handle the orchestration of program execution, including setting up the execution context, managing module imports and dependencies, emitting events for diagnostics, and ensuring that execution adheres to the runtime's limits and policies.

// There are two main entry points for execution:

// - `run_string`: Executes a string of Mech source code directly. This is for lightweight execution of ad-hoc code snippets, scripts, documents, configuration files, etc.
// - `run_module`: Executes a module by its version ID, handling the resolution of dependencies and the construction of the import environment. This is for executing more complex, modular code that depends on other modules and is part of the larger program structure.

// Both methods have corresponding _with_context versions that accept a mutable reference to a RuntimeContext, allowing for execution within the context of an active transaction. This ensures that any changes made during execution are properly staged within the transaction that if an error occurs, the transaction can be rolled back to maintain consistency.

mod activation_effects;
mod context_preflight;
mod query;
mod reactive;
mod source;

pub(super) use activation_effects::{
  ActivationEffectBarrierCompiler,
  ActivationEffectPayloadCaptureCompiler,
  ACTIVATION_EFFECT_BARRIER_NAME,
  ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
};

use super::*;
use super::reactive_transaction::PreparedRuntimeHostInput;
use crate::{
  RuntimeModuleResult, RuntimeResourceKey, RuntimeValueSnapshot, SourceDeclaration,
  SourceImportDeclaration, SourceIndex,
};
use crate::RuntimeResourceWritePreflightRequest;

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAddressTarget {
  Interpreter(SourceScope),
  Context(RuntimeContextBinding),
  Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectContextEffectPlacement {
  TopLevel,
  ActivationScope,
  FunctionBody,
  FsmTransition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AddressedReadPreflight {
  RequireContextBinding,
  AllowModuleAddressTargets,
}

struct PreparedModuleScopeExecution {
  record: ModuleVersionRecord,
  source: MechSourceCode,
  environment: HashMap<String, ValRef>,
}

struct ProgramEnvironmentOverlay;

fn imports_for_scope<'a>(
  record: &'a ModuleVersionRecord,
  scope: &SourceScope,
) -> &'a [SourceImportDeclaration] {
  if let Some(metadata) = record.scopes.iter().find(|metadata| &metadata.scope == scope) {
    return metadata.imports.as_slice();
  }

  if matches!(scope, SourceScope::Program) {
    return record.imports.as_slice();
  }

  &[]
}

fn import_has_source_edge(
  record: &ModuleVersionRecord,
  scope: &SourceScope,
  import: &SourceImportDeclaration,
) -> bool {
  record
    .import_edges
    .iter()
    .any(|edge| &edge.scope == scope && &edge.import == import)
}

fn materialize_function_imports_for_scope(
  program: &mut MechProgram,
  record: &ModuleVersionRecord,
  scope: &SourceScope,
) -> MResult<()> {
  for import in imports_for_scope(record, scope) {
    if matches!(import.alias, Some(SourceImportAlias::Context(_))) {
      continue;
    }

    if import_has_source_edge(record, scope, import) {
      continue;
    }

    let module = import.module.as_deref().ok_or_else(|| {
      MechError::new(
        RuntimeInvalidOperationError {
          operation: "materialize_function_import",
          reason: format!(
            "non-source import `{}` is missing module metadata",
            import.specifier
          ),
        },
        None,
      )
    })?;

    match &import.kind {
      SourceImportKind::Namespace => program.load_function_module(module)?,
      SourceImportKind::Single { .. } => {
        let item = import.item.as_deref().ok_or_else(|| {
          MechError::new(
            RuntimeInvalidOperationError {
              operation: "materialize_function_import",
              reason: format!(
                "item import `{}` is missing item metadata",
                import.specifier
              ),
            },
            None,
          )
        })?;

        match &import.alias {
          Some(SourceImportAlias::Value(alias)) => {
            program.import_function_module_item_as(module, item, alias.as_str())?;
          }
          Some(SourceImportAlias::Context(_)) => {
            unreachable!("context imports were filtered above")
          }
          None => program.import_function_module_item(module, item)?,
        }
      }
      SourceImportKind::Wildcard => program.import_function_module_glob(module)?,
      SourceImportKind::DependencyOnly => {
        return Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "materialize_function_import",
            reason: format!(
              "required source dependency `{}` has no module import edge",
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

impl ProgramEnvironmentOverlay {
  fn install(program: &mut MechProgram, environment: &HashMap<String, ValRef>) -> MResult<()> {
    let mut ids = HashMap::new();
    for name in environment.keys() {
      let id = hash_str(name);
      if let Some(first) = ids.insert(id, name.clone()) {
        if first != *name {
          return Err(MechError::new(RuntimeModuleImportConflict {
            binding: name.clone(),
            first_import: first,
            second_import: name.clone(),
          }, None));
        }
      }
    }

    let symbols = program.interpreter_mut().symbols();
    let mut symbols_brrw = symbols.borrow_mut();
    for (name, value_ref) in environment {
      let id = hash_str(name);
      let previous_dictionary = symbols_brrw.dictionary.borrow().get(&id).cloned();
      if let Some(previous_name) = &previous_dictionary {
        if previous_name != name && !environment.contains_key(previous_name) {
          return Err(MechError::new(RuntimeModuleImportConflict {
            binding: name.clone(),
            first_import: previous_name.clone(),
            second_import: name.clone(),
          }, None));
        }
      }
      symbols_brrw.symbols.insert(id, value_ref.clone());
      symbols_brrw.dictionary.borrow_mut().insert(id, name.clone());
    }

    Ok(())
  }
}

impl AddressedReadPreflight {
  fn requires_context_binding(self) -> bool {
    matches!(self, AddressedReadPreflight::RequireContextBinding)
  }
}

impl DirectContextEffectPlacement {
  fn description(self) -> &'static str {
    match self {
      DirectContextEffectPlacement::TopLevel => "module top level",
      DirectContextEffectPlacement::ActivationScope => "an activation scope",
      DirectContextEffectPlacement::FunctionBody => "a function body",
      DirectContextEffectPlacement::FsmTransition => "an FSM transition",
    }
  }

}

enum RuntimeProgramTarget<'a> {
  Retained,
  Isolated(&'a mut MechProgram),
}

#[derive(Debug, Clone)]
pub struct RuntimeAddressedAssignmentUnsupported {
  pub target: String,
}

impl MechErrorKind for RuntimeAddressedAssignmentUnsupported {
  fn name(&self) -> &str { "RuntimeAddressedAssignmentUnsupported" }

  fn message(&self) -> String {
    format!("addressed assignment is not supported for `{}`", self.target)
  }
}


#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedContextResourceRequest {
  provider_base_uri: String,
  provider_path: String,
  context_path: String,
}

fn identifier_from_str(name: &str) -> mech_core::Identifier {
  mech_core::Identifier {
    name: mech_core::Token::new(
      mech_core::TokenKind::Identifier,
      mech_core::SourceRange::default(),
      name.chars().collect(),
    ),
  }
}


fn execution_scope_for_extracted_module_source(scope: &SourceScope) -> SourceScope {
  match scope {
    SourceScope::Program => SourceScope::Program,
    SourceScope::Interpreter(_) => SourceScope::Program,
  }
}

fn resolve_runtime_address_target(
  record: &ModuleVersionRecord,
  scope: &SourceScope,
  context_registry: &RuntimeContextRegistry,
  target: &str,
) -> RuntimeAddressTarget {
  if !matches!(scope, SourceScope::Program) {
    if let Some(binding) = context_registry.get(target) {
      return RuntimeAddressTarget::Context(binding.clone());
    }
  }

  for metadata in &record.scopes {
    if let SourceScope::Interpreter(interpreter) = &metadata.scope {
      if interpreter.namespace_str == target {
        return RuntimeAddressTarget::Interpreter(metadata.scope.clone());
      }
    }
  }

  if let Some(binding) = context_registry.get(target) {
    return RuntimeAddressTarget::Context(binding.clone());
  }

  RuntimeAddressTarget::Unknown
}

fn context_registry_for_scope(
  record: &ModuleVersionRecord,
  scope: &SourceScope,
) -> MResult<RuntimeContextRegistry> {
  let declarations = record
    .scopes
    .iter()
    .find(|metadata| &metadata.scope == scope)
    .map(|metadata| metadata.contexts.as_slice())
    .unwrap_or(&[]);
  RuntimeContextRegistry::from_declarations(scope.clone(), declarations)
}

fn runtime_context_base_uri(binding: &RuntimeContextBinding) -> String {
  match &binding.base {
    RuntimeContextBase::ResourceUri(uri) => uri.clone(),
  }
}

fn runtime_context_allows_operation(
  binding: &RuntimeContextBinding,
  operation: &str,
  path: &str,
) -> bool {
  binding.capabilities.iter().any(|capability| {
    if capability.operation != operation {
      return false;
    }

    match &capability.scope {
      RuntimeContextCapabilityScope::Wildcard => true,
      RuntimeContextCapabilityScope::Path(exact) => {
        if exact == path {
          return true;
        }
        if let Some(prefix) = exact.strip_suffix("/*") {
          let required_prefix = format!("{}/", prefix);
          return path.starts_with(&required_prefix);
        }
        false
      }
    }
  })
}

fn runtime_context_exposes_operation(
  binding: &RuntimeContextBinding,
  operation: &str,
) -> bool {
  binding.capabilities.iter().any(|capability| capability.operation == operation)
}

fn runtime_context_allows_read(
  binding: &RuntimeContextBinding,
  path: &str,
) -> bool {
  runtime_context_allows_operation(binding, "read", path)
}

fn runtime_context_allows_write(
  binding: &RuntimeContextBinding,
  path: &str,
) -> bool {
  runtime_context_allows_operation(binding, "write", path)
}

fn first_context_send_segment(path: &str) -> MResult<&str> {
  path
    .split('/')
    .next()
    .filter(|segment| !segment.is_empty())
    .ok_or_else(|| MechError::new(RuntimeInvalidOperationError {
      operation: "context_send",
      reason: "context send target path must start with an operation name".to_string(),
    }, None))
}

fn reserved_context_send_operation_error(operation: &str, path: &str) -> MechError {
  MechError::new(RuntimeInvalidOperationError {
    operation: "context_send",
    reason: format!(
      "context send target path `{path}` starts with reserved operation `{operation}`; use assignment for writes or a custom send operation"
    ),
  }, None)
}

fn context_send_operation(
  binding: &RuntimeContextBinding,
  path: &str,
) -> MResult<RuntimeCapabilityOperation> {
  let candidate = first_context_send_segment(path)?;
  if candidate == "read" {
    return Err(reserved_context_send_operation_error(candidate, path));
  }
  if candidate == "write" {
    if runtime_context_allows_write(binding, path) {
      return Ok(RuntimeCapabilityOperation::Write);
    }
    return Err(reserved_context_send_operation_error(candidate, path));
  }
  if runtime_context_allows_operation(binding, candidate, path) {
    return RuntimeCapabilityOperation::from_name(candidate.to_string());
  }
  if runtime_context_exposes_operation(binding, candidate) {
    return RuntimeCapabilityOperation::from_name(candidate.to_string());
  }
  if runtime_context_allows_write(binding, path) {
    return Ok(RuntimeCapabilityOperation::Write);
  }
  RuntimeCapabilityOperation::from_name(candidate.to_string())
}

fn context_write_operation(
  binding: &RuntimeContextBinding,
  intent: RuntimeResourceWriteIntent,
  path: &str,
) -> MResult<RuntimeCapabilityOperation> {
  match intent {
    RuntimeResourceWriteIntent::Assign => Ok(RuntimeCapabilityOperation::Write),
    RuntimeResourceWriteIntent::Send => context_send_operation(binding, path),
  }
}


fn direct_context_target(context_name: &str, path: &str) -> String {
  format!("@{}/{}", context_name, path)
}

fn undeclared_direct_context_target_error(context_name: &str) -> MechError {
  MechError::new(RuntimeInvalidOperationError {
    operation: "direct_context_target",
    reason: format!("context target `@{context_name}` is not declared or imported"),
  }, None)
}



fn single_code_program(
  code: mech_core::MechCode,
  comment: Option<mech_core::Comment>,
) -> mech_core::Program {
  mech_core::Program {
    title: None,
    body: mech_core::Body {
      sections: vec![mech_core::Section {
        subtitle: None,
        elements: vec![mech_core::SectionElement::MechCode(vec![(code, comment)])],
      }],
    },
  }
}

fn resolve_runtime_value(value: Value) -> Value {
  match value {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  }
}

// Activation effects are released after the reactive turn succeeds. Keep the
// body-time payload separate from any live source cells so a register commit
// cannot change the value that the selected arm staged for its send.
fn snapshot_runtime_value(value: &Value) -> Value {
  value.deep_snapshot()
}



impl MechRuntime {
  fn with_live_registration_mode<T>(
    &mut self,
    mode: crate::runtime::LiveRegistrationMode,
    f: impl FnOnce(&mut Self) -> MResult<T>,
  ) -> MResult<T> {
    let previous = std::mem::replace(&mut self.live_registration_mode, mode);
    let result = f(self);
    self.live_registration_mode = previous;
    result
  }

  #[cfg(test)]
  pub(super) fn run_string_with_isolated_registration_for_test(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<Value> {
    self.with_live_registration_mode(
      crate::runtime::LiveRegistrationMode::IsolatedSnapshot,
      |runtime| {
        runtime.run_string_value_with_context(context, source)
      },
    )
  }








































    fn reject_runtime_context_reads_in_statement(
        &self,
        registry: &RuntimeContextRegistry,
        statement: &mech_core::Statement,
    ) -> MResult<()> {
        match statement {
            mech_core::Statement::VariableDefine(var_def) => {
                self.reject_runtime_context_reads_in_expression(registry, &var_def.expression)
            }
            mech_core::Statement::VariableAssign(assign) => {
                self.reject_runtime_context_reads_in_slice_ref(registry, &assign.target)?;
                self.reject_runtime_context_reads_in_expression(registry, &assign.expression)
            }
            mech_core::Statement::OpAssign(op_assign) => {
                self.reject_runtime_context_reads_in_slice_ref(registry, &op_assign.target)?;
                self.reject_runtime_context_reads_in_expression(registry, &op_assign.expression)
            }
            mech_core::Statement::ContextSend(send) => {
                self.reject_runtime_context_reads_in_expression(registry, &send.expression)
            }
            mech_core::Statement::TupleDestructure(tuple_destructure) => self
                .reject_runtime_context_reads_in_expression(
                    registry,
                    &tuple_destructure.expression,
                ),
            mech_core::Statement::FsmDeclare(fsm) => {
                if let Some(args) = &fsm.pipe.start.args {
                    for (_, expression) in args {
                        self.reject_runtime_context_reads_in_expression(registry, expression)?;
                    }
                }
                for transition in &fsm.pipe.transitions {
                    self.reject_runtime_context_reads_in_transition(registry, transition)?;
                }
                Ok(())
            }
            #[cfg(feature = "invariant_define")]
            mech_core::Statement::InvariantDefine(invariant) => {
                self.reject_runtime_context_reads_in_expression(registry, &invariant.expression)
            }
            _ => Ok(()),
        }
    }

    fn reject_runtime_context_reads_in_transition(
        &self,
        registry: &RuntimeContextRegistry,
        transition: &mech_core::Transition,
    ) -> MResult<()> {
        match transition {
            mech_core::Transition::Async(pattern)
            | mech_core::Transition::Next(pattern)
            | mech_core::Transition::Output(pattern) => {
                self.reject_runtime_context_reads_in_pattern(registry, pattern)
            }
            mech_core::Transition::CodeBlock(code_items) => {
                for (code, _) in code_items {
                    if let mech_core::MechCode::Statement(statement) = code {
                        self.reject_runtime_context_reads_in_statement(registry, statement)?;
                    } else if let mech_core::MechCode::Expression(expression) = code {
                        self.reject_runtime_context_reads_in_expression(registry, expression)?;
                    }
                }
                Ok(())
            }
            mech_core::Transition::Statement(statement) => {
                self.reject_runtime_context_reads_in_statement(registry, statement)
            }
        }
    }

    fn reject_runtime_context_reads_in_pattern(
        &self,
        registry: &RuntimeContextRegistry,
        pattern: &mech_core::Pattern,
    ) -> MResult<()> {
        match pattern {
            mech_core::Pattern::Expression(expression) => {
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::Pattern::TupleStruct(tuple_struct) => {
                for pattern in &tuple_struct.patterns {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                Ok(())
            }
            mech_core::Pattern::Tuple(tuple) => {
                for pattern in &tuple.0 {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                Ok(())
            }
            mech_core::Pattern::Array(array) => {
                for pattern in &array.prefix {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                if let Some(spread) = &array.spread {
                    if let Some(binding) = &spread.binding {
                        self.reject_runtime_context_reads_in_pattern(registry, binding)?;
                    }
                }
                for pattern in &array.suffix {
                    self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                }
                Ok(())
            }
            mech_core::Pattern::Wildcard => Ok(()),
        }
    }

    fn reject_runtime_context_reads_in_expression(
        &self,
        registry: &RuntimeContextRegistry,
        expression: &mech_core::Expression,
    ) -> MResult<()> {
        match expression {
            mech_core::Expression::Var(var) => {
                if let Some(context_name) = &var.context {
                    if registry.contains(&context_name.to_string()) {
                        self.reject_function_context_read(context_name, &var.name)?;
                    }
                }
                Ok(())
            }
            mech_core::Expression::Formula(factor) => {
                self.reject_runtime_context_reads_in_factor(registry, factor)
            }
            mech_core::Expression::FunctionCall(call) => {
                for (_, expression) in &call.args {
                    self.reject_runtime_context_reads_in_expression(registry, expression)?;
                }
                Ok(())
            }
            mech_core::Expression::FsmPipe(pipe) => {
                if let Some(args) = &pipe.start.args {
                    for (_, expression) in args {
                        self.reject_runtime_context_reads_in_expression(registry, expression)?;
                    }
                }
                for transition in &pipe.transitions {
                    self.reject_runtime_context_reads_in_transition(registry, transition)?;
                }
                Ok(())
            }
            mech_core::Expression::Literal(_) => Ok(()),
            mech_core::Expression::Range(range) => {
                self.reject_runtime_context_reads_in_factor(registry, &range.start)?;
                if let Some((_, increment)) = &range.increment {
                    self.reject_runtime_context_reads_in_factor(registry, increment)?;
                }
                self.reject_runtime_context_reads_in_factor(registry, &range.terminal)
            }
            mech_core::Expression::Structure(structure) => {
                self.reject_runtime_context_reads_in_structure(registry, structure)
            }
            mech_core::Expression::Match(match_expression) => {
                self.reject_runtime_context_reads_in_expression(
                    registry,
                    &match_expression.source,
                )?;
                for arm in &match_expression.arms {
                    self.reject_runtime_context_reads_in_pattern(registry, &arm.pattern)?;
                    if let Some(guard) = &arm.guard {
                        self.reject_runtime_context_reads_in_expression(registry, guard)?;
                    }
                    self.reject_runtime_context_reads_in_expression(registry, &arm.expression)?;
                }
                Ok(())
            }
            mech_core::Expression::Slice(slice) => {
                self.reject_runtime_context_reads_in_slice(registry, slice)
            }
            mech_core::Expression::SetComprehension(comprehension) => {
                self.reject_runtime_context_reads_in_expression(
                    registry,
                    &comprehension.expression,
                )?;
                for qualifier in &comprehension.qualifiers {
                    self.reject_runtime_context_reads_in_comprehension_qualifier(
                        registry, qualifier,
                    )?;
                }
                Ok(())
            }
            mech_core::Expression::MatrixComprehension(comprehension) => {
                self.reject_runtime_context_reads_in_expression(
                    registry,
                    &comprehension.expression,
                )?;
                for qualifier in &comprehension.qualifiers {
                    self.reject_runtime_context_reads_in_comprehension_qualifier(
                        registry, qualifier,
                    )?;
                }
                Ok(())
            }
        }
    }

    fn reject_runtime_context_reads_in_factor(
        &self,
        registry: &RuntimeContextRegistry,
        factor: &mech_core::Factor,
    ) -> MResult<()> {
        match factor {
            mech_core::Factor::Expression(expression) => {
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::Factor::Negate(factor)
            | mech_core::Factor::Not(factor)
            | mech_core::Factor::Parenthetical(factor)
            | mech_core::Factor::Transpose(factor) => {
                self.reject_runtime_context_reads_in_factor(registry, factor)
            }
            mech_core::Factor::Term(term) => {
                self.reject_runtime_context_reads_in_factor(registry, &term.lhs)?;
                for (_, factor) in &term.rhs {
                    self.reject_runtime_context_reads_in_factor(registry, factor)?;
                }
                Ok(())
            }
        }
    }

    fn reject_runtime_context_reads_in_slice(
        &self,
        registry: &RuntimeContextRegistry,
        slice: &mech_core::Slice,
    ) -> MResult<()> {
        if let Some(context_name) = &slice.context {
            if registry.contains(&context_name.to_string()) {
                self.reject_function_context_read(context_name, &slice.name)?;
            }
        }
        for subscript in &slice.subscript {
            self.reject_runtime_context_reads_in_subscript(registry, subscript)?;
        }
        Ok(())
    }

    fn reject_runtime_context_reads_in_slice_ref(
        &self,
        registry: &RuntimeContextRegistry,
        target: &mech_core::SliceRef,
    ) -> MResult<()> {
        if let Some(subscripts) = &target.subscript {
            for subscript in subscripts {
                self.reject_runtime_context_reads_in_subscript(registry, subscript)?;
            }
        }
        Ok(())
    }

    fn reject_runtime_context_reads_in_subscript(
        &self,
        registry: &RuntimeContextRegistry,
        subscript: &mech_core::Subscript,
    ) -> MResult<()> {
        match subscript {
            mech_core::Subscript::Brace(subscripts) | mech_core::Subscript::Bracket(subscripts) => {
                for subscript in subscripts {
                    self.reject_runtime_context_reads_in_subscript(registry, subscript)?;
                }
                Ok(())
            }
            mech_core::Subscript::Formula(factor) => {
                self.reject_runtime_context_reads_in_factor(registry, factor)
            }
            mech_core::Subscript::Range(range) => {
                self.reject_runtime_context_reads_in_factor(registry, &range.start)?;
                if let Some((_, increment)) = &range.increment {
                    self.reject_runtime_context_reads_in_factor(registry, increment)?;
                }
                self.reject_runtime_context_reads_in_factor(registry, &range.terminal)
            }
            _ => Ok(()),
        }
    }

    fn reject_runtime_context_reads_in_structure(
        &self,
        registry: &RuntimeContextRegistry,
        structure: &mech_core::Structure,
    ) -> MResult<()> {
        match structure {
            mech_core::Structure::Map(map) => {
                for mapping in &map.elements {
                    self.reject_runtime_context_reads_in_expression(registry, &mapping.key)?;
                    self.reject_runtime_context_reads_in_expression(registry, &mapping.value)?;
                }
            }
            mech_core::Structure::Set(set) => {
                for expression in &set.elements {
                    self.reject_runtime_context_reads_in_expression(registry, expression)?;
                }
            }
            mech_core::Structure::Matrix(matrix) => {
                for row in &matrix.rows {
                    for column in &row.columns {
                        self.reject_runtime_context_reads_in_expression(registry, &column.element)?;
                    }
                }
            }
            mech_core::Structure::Record(record) => {
                for binding in &record.bindings {
                    self.reject_runtime_context_reads_in_expression(registry, &binding.value)?;
                }
            }
            mech_core::Structure::Table(table) => {
                for row in &table.rows {
                    for column in &row.columns {
                        self.reject_runtime_context_reads_in_expression(registry, &column.element)?;
                    }
                }
            }
            mech_core::Structure::Tuple(tuple) => {
                for expression in &tuple.elements {
                    self.reject_runtime_context_reads_in_expression(registry, expression)?;
                }
            }
            mech_core::Structure::TupleStruct(tuple_struct) => {
                self.reject_runtime_context_reads_in_expression(registry, &tuple_struct.value)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn reject_runtime_context_reads_in_comprehension_qualifier(
        &self,
        registry: &RuntimeContextRegistry,
        qualifier: &mech_core::ComprehensionQualifier,
    ) -> MResult<()> {
        match qualifier {
            mech_core::ComprehensionQualifier::Generator((pattern, expression)) => {
                self.reject_runtime_context_reads_in_pattern(registry, pattern)?;
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::ComprehensionQualifier::Filter(expression) => {
                self.reject_runtime_context_reads_in_expression(registry, expression)
            }
            mech_core::ComprehensionQualifier::Let(var_def) => {
                self.reject_runtime_context_reads_in_expression(registry, &var_def.expression)
            }
        }
    }

    fn preflight_fsm_implementation_context_capabilities(
    &self,
    context: &RuntimeContext,
    registry: &RuntimeContextRegistry,
    fsm: &mech_core::FsmImplementation,
    addressed_read_preflight: AddressedReadPreflight,
  ) -> MResult<()> {
    self.preflight_pattern_context_reads(context, registry, &fsm.start, addressed_read_preflight)?;
    for arm in &fsm.arms {
      match arm {
        mech_core::FsmArm::Guard(pattern, guards) => {
          self.preflight_pattern_context_reads(context, registry, pattern, addressed_read_preflight)?;
          for guard in guards {
            self.preflight_pattern_context_reads(context, registry, &guard.condition, addressed_read_preflight)?;
            for transition in &guard.transitions {
              self.preflight_transition_context_capabilities(context, registry, transition, addressed_read_preflight)?;
            }
          }
        }
        mech_core::FsmArm::Transition(pattern, transitions) => {
          self.preflight_pattern_context_reads(context, registry, pattern, addressed_read_preflight)?;
          for transition in transitions {
            self.preflight_transition_context_capabilities(context, registry, transition, addressed_read_preflight)?;
          }
        }
        mech_core::FsmArm::Comment(_) => {}
      }
    }
    Ok(())
  }







































  pub fn run_module(
    &mut self,
    version: ModuleVersionId,
  ) -> MResult<RuntimeModuleResult> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_with_context(&mut context, version)
  }

  pub fn run_module_scope(
    &mut self,
    version: ModuleVersionId,
    scope: SourceScope,
  ) -> MResult<RuntimeModuleResult> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_scope_with_context(&mut context, version, scope)
  }

  pub fn run_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
  ) -> MResult<RuntimeModuleResult> {
    self.run_module_scope_with_context(context, version, SourceScope::Program)
  }

  pub fn run_module_scope_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: SourceScope,
  ) -> MResult<RuntimeModuleResult> {
    self.with_atomic_program_operation(
      context,
      "run_module_scope_with_context",
      |runtime, context| {
        let turn_started = Instant::now();
        let mut preflight_seen = HashSet::new();
        runtime.preflight_module_graph_for_scope(
          context,
          version,
          &scope,
          &mut preflight_seen,
        )?;

        let mut seen = HashSet::new();
        let mut module_instances = HashMap::new();

        let instance = runtime.execute_module_isolated_for_scope(
          context,
          version,
          &scope,
          &mut seen,
          &mut module_instances,
        )?;

        runtime.enforce_turn_duration(turn_started)?;
        Ok(instance.detached_result())
      },
    )
  }

  pub(super) fn preflight_module_graph_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: &SourceScope,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
  ) -> MResult<()> {
    self.validate_context_for_runtime(context)?;
    let instance_key = (version, scope.clone());
    if !seen.insert(instance_key) {
      return Ok(());
    }

    let Some(record) =
      self.get_module_version_visible(context, version)?
    else {
      return Err(MechError::new(RuntimeRecordNotFoundError { record_type: "module_version", id: version.to_string() }, None));
    };
    validate_module_import_edges(&record)?;
    for requirement in &record.capability_requirements {
      let mut requirement = requirement.clone();
      requirement.subject = context.subject.clone();
      self.check_capability_with_context(
        context,
        &requirement,
      )?;
    }
    let Some(source) = record.source.clone() else {
      return Err(MechError::new(RuntimeInvalidOperationError { operation: "run_module", reason: "module version has no source".to_string() }, None));
    };

    let context_registry = context_registry_for_scope(&record, scope)?;
    let scoped_source = module_source_for_scope(&source, scope)?;
    let execution_scope = execution_scope_for_extracted_module_source(scope);
    self.preflight_module_source(context, &context_registry, &scoped_source, &execution_scope)?;

    for edge in &record.import_edges {
      if &edge.scope != scope {
        continue;
      }

      self.preflight_module_graph_for_scope(
        context,
        edge.dependency,
        &SourceScope::Program,
        seen,
      )?;
    }

    let scoped_refs = record
      .scopes
      .iter()
      .find(|metadata| &metadata.scope == scope)
      .map(|metadata| metadata.address_references.as_slice())
      .unwrap_or(&[]);

    for reference in scoped_refs {
      match resolve_runtime_address_target(&record, scope, &context_registry, &reference.target) {
        RuntimeAddressTarget::Interpreter(interpreter_scope) => {
          if !matches!(scope, SourceScope::Program) {
            return Err(MechError::new(UnknownAddressTarget { target: reference.target.clone() }, None));
          }
          self.preflight_module_graph_for_scope(
            context,
            version,
            &interpreter_scope,
            seen,
          )?;
        }
        RuntimeAddressTarget::Context(_) => {}
        RuntimeAddressTarget::Unknown => {
          return Err(MechError::new(UnknownAddressTarget { target: reference.target.clone() }, None));
        }
      }
    }

    Ok(())
  }

  fn preflight_module_source(
    &self,
    context: &RuntimeContext,
    registry: &RuntimeContextRegistry,
    source: &MechSourceCode,
    scope: &SourceScope,
  ) -> MResult<()> {
    match source {
      MechSourceCode::String(source) => {
        let tree = mech_syntax::parser::parse(source.trim())?;
        self.preflight_context_capabilities_with_registry(context, registry, &tree, scope, AddressedReadPreflight::AllowModuleAddressTargets)
      }
      MechSourceCode::Tree(tree) => {
        self.preflight_context_capabilities_with_registry(context, registry, tree, scope, AddressedReadPreflight::AllowModuleAddressTargets)
      }
      MechSourceCode::Program(sources) => {
        for source in sources {
          self.preflight_module_source(context, registry, source, scope)?;
        }
        Ok(())
      }
      MechSourceCode::Html(_) | MechSourceCode::ByteCode(_) => Ok(()),
      MechSourceCode::Image(_, _) => Err(MechError::new(RuntimeInvalidOperationError {
        operation: "run_module_preflight",
        reason: "unsupported executable source kind for provider preflight: image".to_string(),
      }, None)),
    }
  }

  fn prepare_module_scope_execution(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: &SourceScope,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<PreparedModuleScopeExecution> {
    let Some(record) =
      self.get_module_version_visible(context, version)?
    else {
      return Err(MechError::new(RuntimeRecordNotFoundError { record_type: "module_version", id: version.to_string() }, None));
    };
    validate_module_import_edges(&record)?;
    let Some(source) = record.source.clone() else {
      return Err(MechError::new(RuntimeInvalidOperationError { operation: "run_module", reason: "module version has no source".to_string() }, None));
    };

    let context_registry = context_registry_for_scope(&record, scope)?;

    for edge in &record.import_edges {
      if &edge.scope != scope {
        continue;
      }

      self.execute_module_isolated_for_scope(
        context,
        edge.dependency,
        &SourceScope::Program,
        seen,
        module_instances,
      )?;
    }

    let mut environment = self.build_import_environment_for_scope(
      context,
      &record,
      scope,
      module_instances,
    )?;

    let address_environment = self.build_address_environment_for_scope(
      context,
      version,
      &record,
      scope,
      &context_registry,
      seen,
      module_instances,
    )?;
    merge_module_environment(
      &mut environment,
      address_environment,
      "import environment",
      "address environment",
    )?;

    let scoped_source = module_source_for_scope(&source, scope)?;

    Ok(PreparedModuleScopeExecution { record, source: scoped_source, environment })
  }

  fn execute_module_isolated_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: &SourceScope,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<ModuleInstance> {
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;

    let instance_key = (version, scope.clone());

    if let Some(instance) = module_instances.get(&instance_key).cloned() {
      return Ok(instance);
    }

    if seen.contains(&instance_key) {
      return Ok(ModuleInstance { version, exports: HashMap::new(), result: Value::Empty });
    }
    seen.insert(instance_key.clone());

    let prepared = self.prepare_module_scope_execution(
      context,
      version,
      scope,
      seen,
      module_instances,
    )?;
    let mut module_program = MechProgram::new(MechProgramConfig {
      name: self.config.name.clone(),
      environment: MechProgramEnvironment {
        trace_enabled: self.config.diagnostics.trace_enabled,
        debug_enabled: self.config.diagnostics.debug_enabled,
        profile_enabled: self.config.diagnostics.profile_enabled,
        rounds_per_step: self.config.limits.max_steps_per_turn_as_usize()?,
      },
    });

    materialize_function_imports_for_scope(&mut module_program, &prepared.record, scope)?;

    {
      let symbols = module_program.interpreter_mut().symbols();
      let mut symbols_brrw = symbols.borrow_mut();
      for (name, value_ref) in &prepared.environment {
        let id = hash_str(&name);
        symbols_brrw.symbols.insert(id, value_ref.clone());
        symbols_brrw.dictionary.borrow_mut().insert(id, name.clone());
      }
    }

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleExecutionStarted { module_version: version },
    )?;
    let result = self.run_module_source_on_program(
      context,
      &mut RuntimeProgramTarget::Isolated(
        &mut module_program,
      ),
      &prepared.source,
      scope,
      crate::runtime::LiveRegistrationMode::IsolatedSnapshot,
    );
    let result = match result {
      Ok(value) => value,
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ModuleExecutionFailed {
            module_version: version,
            message: format!("{:?}", error),
          },
        )?;
        return Err(error);
      }
    };
    #[cfg(feature = "invariant_define")]
    if let Err(error) = module_program.validate_integrity_constraints() {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::ModuleExecutionFailed {
          module_version: version,
          message: format!("{:?}", error),
        },
      )?;
      return Err(error);
    }
    let mut exports = HashMap::new();
    {
      let symbols = module_program.interpreter_mut().symbols();
      let symbols_brrw = symbols.borrow();
      for export in exports_for_scope(&prepared.record, scope) {
        let id = hash_str(&export.name);
        let Some(value_ref) = symbols_brrw.get(id) else {
          let error = MechError::new(RuntimeModuleExportNotFound { dependency: prepared.record.id.to_string(), export: export.name.clone() }, None);
          self.emit_event_to_context(
            context,
            RuntimeEventKind::ModuleExecutionFailed {
              module_version: version,
              message: format!("{:?}", error),
            },
          )?;
          return Err(error);
        };
        exports.insert(export.name.clone(), value_ref.clone());
      }
    }

    let instance = ModuleInstance { version, exports, result };
    module_instances.insert(instance_key, instance.clone());
    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleExecutionCompleted { module_version: version },
    )?;
    Ok(instance)
  }

  pub(super) fn execute_module_retained_root_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: &SourceScope,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
    turn_started: Instant,
  ) -> MResult<Value> {
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;

    if !matches!(scope, SourceScope::Program) {
      return Err(MechError::new(RuntimeInvalidOperationError {
        operation: "run_root_module",
        reason: "retained root module execution only supports program scope".to_string(),
      }, None));
    }

    let instance_key = (version, scope.clone());
    if seen.contains(&instance_key) {
      return Ok(Value::Empty);
    }
    seen.insert(instance_key);

    let prepared = self.prepare_module_scope_execution(
      context,
      version,
      scope,
      seen,
      module_instances,
    )?;

    let result = (|| -> MResult<Value> {
      materialize_function_imports_for_scope(
        &mut self.program,
        &prepared.record,
        scope,
      )?;
      ProgramEnvironmentOverlay::install(
        &mut self.program,
        &prepared.environment,
      )?;

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ModuleExecutionStarted { module_version: version },
      )?;

      let result = self
        .run_module_source_on_program(
          context,
          &mut RuntimeProgramTarget::Retained,
          &prepared.source,
          scope,
          crate::runtime::LiveRegistrationMode::RetainedRoot,
        )
        .and_then(|value| {
          self.enforce_turn_duration(turn_started)?;
          Ok(value)
        });

      result
    })();

    match result {
      Ok(value) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ModuleExecutionCompleted { module_version: version },
        )?;
        Ok(value)
      }
      Err(error) => Err(error),
    }
  }

  fn run_module_source_on_program(
    &mut self,
    context: &mut RuntimeContext,
    target: &mut RuntimeProgramTarget<'_>,
    source: &MechSourceCode,
    scope: &SourceScope,
    registration_mode: crate::runtime::LiveRegistrationMode,
  ) -> MResult<Value> {
    match target {
      RuntimeProgramTarget::Retained => {
        self.register_retained_program_host_functions(context)?;
      }
      RuntimeProgramTarget::Isolated(program) => {
        self.register_runtime_program_host_functions(
          context,
          program,
        )?;
      }
    }

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    // Named interpreter scopes are extracted to bare source by module_source_for_scope.
    // Once extracted, their declarations are indexed as SourceScope::Program in the
    // reparsed tree, so direct context handling must use Program scope for that tree.
    // The surrounding module execution still uses the original scope for imports,
    // exports, address environment, and ModuleInstance identity.
    let execution_scope = execution_scope_for_extracted_module_source(scope);

    let result = self.with_live_registration_mode(registration_mode, |runtime| {
      match source {
      MechSourceCode::String(source) => match mech_syntax::parser::parse(source.trim()) {
        Ok(tree) => {
          let registry = runtime.direct_context_registry_for_scope(&tree, &execution_scope)?;
          match runtime.preflight_context_capabilities_with_registry(
            context,
            &registry,
            &tree,
            &execution_scope,
            AddressedReadPreflight::AllowModuleAddressTargets,
          ) {
            Ok(()) => runtime.run_tree_on_program(context, target, &tree, Some(&execution_scope)),
            Err(error) => Err(error),
          }
        },
        Err(error) => Err(error),
      },
      MechSourceCode::Tree(tree) => {
        let registry = runtime.direct_context_registry_for_scope(tree, &execution_scope)?;
        match runtime.preflight_context_capabilities_with_registry(
          context,
          &registry,
          tree,
          &execution_scope,
          AddressedReadPreflight::AllowModuleAddressTargets,
        ) {
          Ok(()) => runtime.run_tree_on_program(context, target, tree, Some(&execution_scope)),
          Err(error) => Err(error),
        }
      },
      MechSourceCode::Program(sources) => {
        let mut result = Ok(Value::Empty);
        for source in sources {
          result = runtime.run_module_source_on_program(
            context,
            target,
            source,
            scope,
            registration_mode,
          );
          if result.is_err() {
            break;
          }
        }
        result
      }
      MechSourceCode::Html(_) | MechSourceCode::ByteCode(_) => {
        runtime.execute_program_target_source(
          context,
          target,
          source,
        )
      }
      MechSourceCode::Image(_, _) => Err(MechError::new(RuntimeInvalidOperationError {
        operation: "run_module_preflight",
        reason: "unsupported executable source kind for provider preflight: image".to_string(),
      }, None)),
      }
    });

    if result.is_ok() {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::ProgramCompleted {
          task_id: context.task,
        },
      )?;
    }

    result
  }

  fn build_address_environment_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    record: &ModuleVersionRecord,
    scope: &SourceScope,
    context_registry: &RuntimeContextRegistry,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<HashMap<String, mech_core::ValRef>> {
    fn address_binding_key(target: &str, name: &str) -> String {
      format!("@{target}/{name}")
    }

    let scoped_refs = record
      .scopes
      .iter()
      .find(|metadata| &metadata.scope == scope)
      .map(|metadata| metadata.address_references.as_slice())
      .unwrap_or(&[]);

    let mut requested_by_scope: HashMap<SourceScope, Vec<SourceAddressReference>> = HashMap::new();
    let mut bindings = HashMap::new();
    for reference in scoped_refs {
      match resolve_runtime_address_target(record, scope, context_registry, &reference.target) {
        RuntimeAddressTarget::Interpreter(interpreter_scope) => {
          if !matches!(scope, SourceScope::Program) {
            return Err(MechError::new(UnknownAddressTarget { target: reference.target.clone() }, None));
          }
          requested_by_scope.entry(interpreter_scope).or_default().push(reference.clone());
        }
        RuntimeAddressTarget::Context(_) => {
          // Context-addressed resource reads are resolved while executing source
          // statements so reads observe earlier writes in the same module scope.
        }
        RuntimeAddressTarget::Unknown => {
          return Err(MechError::new(UnknownAddressTarget { target: reference.target.clone() }, None));
        }
      }
    }

    for (interpreter_scope, requested_refs) in requested_by_scope {
      let instance = self.execute_module_isolated_for_scope(
        context,
        version,
        &interpreter_scope,
        seen,
        module_instances,
      )?;

      for reference in requested_refs {
        let Some(export_value) = instance.exports.get(&reference.name) else {
          return Err(MechError::new(RuntimeModuleExportNotFound { dependency: record.id.to_string(), export: reference.name.clone() }, None));
        };
        bindings.insert(address_binding_key(&reference.target, &reference.name), export_value.clone());
      }
    }

    Ok(bindings)
  }

  fn build_import_environment_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    importer: &ModuleVersionRecord,
    scope: &SourceScope,
    module_instances: &HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<HashMap<String, mech_core::ValRef>> {
    let mut bindings = HashMap::new();
    let mut ownership: HashMap<String, String> = HashMap::new();

    for edge in &importer.import_edges {
      if &edge.scope != scope {
        continue;
      }

      let import = &edge.import;
      let dependency = edge.dependency;

      let dependency_key = (dependency, SourceScope::Program);
      let Some(dependency_instance) = module_instances.get(&dependency_key) else {
        return Err(MechError::new(RuntimeInvalidOperationError { operation: "build_import_environment", reason: format!("dependency instance missing for {}", dependency) }, None));
      };

      match &import.kind {
        SourceImportKind::DependencyOnly | SourceImportKind::Namespace => {
          let Some(namespace) = module_namespace_for_import(import) else { continue; };
          for (export_name, export_value) in &dependency_instance.exports {
            let binding = format!("{}/{}", namespace, export_name);
            if let Some(first) = ownership.insert(binding.clone(), import.specifier.clone()) {
              return Err(MechError::new(RuntimeModuleImportConflict { binding, first_import: first, second_import: import.specifier.clone() }, None));
            }
            bindings.insert(format!("{}/{}", namespace, export_name), export_value.clone());
          }
        }
        SourceImportKind::Single { name } => {
          if matches!(import.alias, Some(crate::resolver::SourceImportAlias::Context(_))) {
            continue;
          }
          let Some(export_value) = dependency_instance.exports.get(name) else {
            return Err(MechError::new(RuntimeModuleExportNotFound { dependency: import.specifier.clone(), export: name.clone() }, None));
          };

          let binding = match &import.alias {
            Some(crate::resolver::SourceImportAlias::Value(alias)) => alias.clone(),
            Some(crate::resolver::SourceImportAlias::Context(_)) => continue,
            None => name.clone(),
          };

          if let Some(first) = ownership.insert(binding.clone(), import.specifier.clone()) {
            return Err(MechError::new(RuntimeModuleImportConflict { binding: binding.clone(), first_import: first, second_import: import.specifier.clone() }, None));
          }
          bindings.insert(binding, export_value.clone());
        }
        SourceImportKind::Wildcard => {
          for (export_name, export_value) in &dependency_instance.exports {
            if let Some(first) = ownership.insert(export_name.clone(), import.specifier.clone()) {
              return Err(MechError::new(RuntimeModuleImportConflict { binding: export_name.clone(), first_import: first, second_import: import.specifier.clone() }, None));
            }
            bindings.insert(export_name.clone(), export_value.clone());
          }
        }
      }

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ModuleImportLinked {
          importer: importer.id,
          dependency,
          specifier: import.specifier.clone(),
        },
      )?;
    }

    Ok(bindings)
  }
}



fn module_source_for_scope(
  source: &MechSourceCode,
  scope: &SourceScope,
) -> MResult<MechSourceCode> {
  match scope {
    SourceScope::Program => Ok(source.clone()),
    SourceScope::Interpreter(interpreter) => {
      let MechSourceCode::String(source_text) = source else {
        return Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "run_module_scope",
            reason: "interpreter scope execution requires string source".to_string(),
          },
          None,
        ));
      };

      let tree = mech_syntax::parser::parse(source_text.trim())?;

      if let Some(source) = source_from_parsed_fenced_blocks(&tree, interpreter.namespace)? {
        return Ok(MechSourceCode::String(source));
      }

      Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "run_module_scope",
          reason: format!("interpreter scope `{}` not found", interpreter.namespace_str),
        },
        None,
      ))
    }
  }
}

fn source_from_parsed_fenced_blocks(
  tree: &mech_core::Program,
  namespace: u64,
) -> MResult<Option<String>> {
  let mut blocks = Vec::new();

  for section in &tree.body.sections {
    for element in &section.elements {
      if let mech_core::SectionElement::FencedMechCode(fenced) = element {
        if fenced.config.namespace == namespace {
          let block = source_from_parsed_fenced_code(fenced)?;
          blocks.push(block.trim_end().to_string());
        }
      }
    }
  }

  if blocks.is_empty() {
    Ok(None)
  } else {
    Ok(Some(blocks.join("\n")))
  }
}

fn source_from_parsed_fenced_code(
  fenced: &mech_core::FencedMechCode,
) -> MResult<String> {
  source_from_tokens(std::slice::from_ref(&fenced.source))
}

fn source_from_tokens(tokens: &[mech_core::Token]) -> MResult<String> {
  if tokens.is_empty() {
    return Ok(String::new());
  }

  if tokens.iter().any(|token| token.src_range.start.row == 0 || token.src_range.start.col == 0) {
    return Ok(tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>().join(" "));
  }

  let mut source = String::new();
  let mut row = tokens[0].src_range.start.row;
  let mut col = tokens[0].src_range.start.col;

  for token in tokens {
    let start = &token.src_range.start;
    while row < start.row {
      source.push('\n');
      row += 1;
      col = 1;
    }
    while col < start.col {
      source.push(' ');
      col += 1;
    }

    let token_text = token.to_string();
    for ch in token_text.chars() {
      source.push(ch);
      if ch == '\n' {
        row += 1;
        col = 1;
      } else {
        col += 1;
      }
    }
  }

  Ok(source)
}

fn exports_for_scope<'a>(
  record: &'a ModuleVersionRecord,
  scope: &SourceScope,
) -> &'a [SourceExportDeclaration] {
  record
    .scopes
    .iter()
    .find(|metadata| &metadata.scope == scope)
    .map(|metadata| metadata.exports.as_slice())
    .unwrap_or(&[])
}

fn merge_module_environment(
  target: &mut HashMap<String, ValRef>,
  source: HashMap<String, ValRef>,
  first_import: &str,
  second_import: &str,
) -> MResult<()> {
  for (binding, value) in source {
    if target.contains_key(&binding) {
      return Err(MechError::new(RuntimeModuleImportConflict {
        binding,
        first_import: first_import.to_string(),
        second_import: second_import.to_string(),
      }, None));
    }
    target.insert(binding, value);
  }
  Ok(())
}




#[cfg(test)]
mod tests;

impl MechRuntime {
  pub fn ingress(&self) -> crate::RuntimeIngress {
    crate::RuntimeIngress::new(self.host_input_queue.clone())
  }

  pub fn has_pending_host_inputs(&self) -> MResult<bool> {
    Ok(self.pending_host_input_count()? > 0)
  }

  pub fn persistent_send_count(&self) -> usize {
    self.persistent_sends.len()
  }

  pub fn input_driver_count(&self) -> usize {
    self.attached_input_driver_count
  }

  pub fn has_input_drivers(&self) -> bool {
    self.input_driver_count() > 0
  }

  pub fn live_input_binding_count(&self) -> usize {
    self.live_input_bindings.values().map(Vec::len).sum()
  }

  pub fn has_live_input_bindings(&self) -> bool {
    self.live_input_binding_count() > 0
  }

  pub fn driven_live_input_binding_count(&self) -> MResult<usize> {
    let mut count = 0;
    for (source, bindings) in &self.live_input_bindings {
      let mut driven = false;
      for driver in &self.input_drivers[..self.attached_input_driver_count] {
        if extension::invoke_extension_value(
          "host input driver",
          "drives",
          || driver.drives(source),
        )? {
          driven = true;
          break;
        }
      }
      if driven {
        count += bindings.len();
      }
    }
    Ok(count)
  }

  pub fn has_driven_live_input_bindings(&self) -> MResult<bool> {
    Ok(self.driven_live_input_binding_count()? > 0)
  }

  pub fn pending_host_input_count(&self) -> MResult<usize> {
    let guard = self.host_input_queue.lock().map_err(|_| crate::input::input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
    Ok(guard.queue.len())
  }

  pub fn apply_host_input(&mut self, input: crate::RuntimeHostInput) -> MResult<crate::RuntimeHostInputOutcome> {
    self.ensure_runtime_mutation_allowed("apply_host_input")?;
    self.reject_program_operation_reentrancy("apply_host_input")?;
    let prepared = self.prepare_runtime_host_input(&input)?;
    if prepared.binding_count == 0 {
      return Ok(crate::RuntimeHostInputOutcome {
        update_count: prepared.update_count,
        ignored_update_count: prepared.ignored_update_count,
        binding_count: 0,
        turn: None,
      });
    }

    let mut context = self.live_turn_context()?;
    self.validate_context_for_runtime(&context)?;
    self.validate_live_turn_context(&context)?;
    self.apply_prepared_host_input_with_context(
      &mut context,
      prepared,
    )
  }

  pub fn apply_host_input_with_context(
    &mut self,
    context: &mut RuntimeContext,
    input: crate::RuntimeHostInput,
  ) -> MResult<crate::RuntimeHostInputOutcome> {
    self.ensure_runtime_mutation_allowed(
      "apply_host_input_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    self.reject_program_operation_reentrancy(
      "apply_host_input_with_context",
    )?;
    self.validate_live_turn_context(context)?;
    let prepared = self.prepare_runtime_host_input(&input)?;

    if prepared.binding_count == 0 {
      return Ok(crate::RuntimeHostInputOutcome {
        update_count: prepared.update_count,
        ignored_update_count: prepared.ignored_update_count,
        binding_count: 0,
        turn: None,
      });
    }

    self.apply_prepared_host_input_with_context(context, prepared)
  }

  fn apply_prepared_host_input_with_context(
    &mut self,
    context: &mut RuntimeContext,
    prepared: PreparedRuntimeHostInput,
  ) -> MResult<crate::RuntimeHostInputOutcome> {
    let turn_started = Instant::now();
    let turn = self.with_atomic_reactive_turn(
      context,
      "apply_host_input_with_context",
      |program, services, finalize| {
        program.update_inputs_and_advance_turn_coordinated(
          &prepared.updates,
          services,
          |turn| finalize(turn),
        )
      },
      move |runtime, context, turn| {
        runtime.enforce_turn_duration(turn_started)?;
        runtime.execute_persistent_sends(context, turn)?;
        Ok(())
      },
    )?;
    Ok(crate::RuntimeHostInputOutcome {
      update_count: prepared.update_count,
      ignored_update_count: prepared.ignored_update_count,
      binding_count: prepared.binding_count,
      turn: Some(turn),
    })
  }

  fn execute_persistent_sends(&mut self, context: &mut RuntimeContext, turn: &mech_program::ProgramInputTurnOutcome) -> MResult<()> {
    for send in self.persistent_sends.clone() {
      let should_send = match send.schedule {
        RuntimePersistentSendSchedule::EveryAcceptedTurn => true,
        RuntimePersistentSendSchedule::Activation { interpreter_id, barrier_node_id } => turn.interpreter_turns.iter().find(|outcome| outcome.interpreter_id == interpreter_id).map(|outcome| {
          outcome.turn.before_commit.executed_nodes.contains(&barrier_node_id) || outcome.turn.after_commit.executed_nodes.contains(&barrier_node_id)
        }).unwrap_or(false),
      };
      if !should_send { continue; }
      let value = resolve_runtime_value(send.value.borrow().clone());
      self.write_context_resource(context, &send.binding, &send.path, value, RuntimeResourceWriteIntent::Send)?;
    }
    Ok(())
  }

  pub fn drain_host_inputs(&mut self, max_inputs: usize) -> MResult<Vec<crate::RuntimeHostInputOutcome>> {
    let mut outcomes = Vec::new();
    for _ in 0..max_inputs {
      let input = {
        let mut guard = self.host_input_queue.lock().map_err(|_| crate::input::input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
        guard.queue.pop_front()
      };
      let Some(input) = input else { break; };
      outcomes.push(self.apply_host_input(input)?);
    }
    Ok(outcomes)
  }

  pub fn close_ingress(&mut self) -> MResult<()> {
    let mut guard = self.host_input_queue.lock().map_err(|_| crate::input::input_error("RuntimeIngressUnavailable", "host input queue lock is poisoned"))?;
    guard.closed = true;
    Ok(())
  }

  pub fn start_input_drivers(&mut self) -> MResult<()> {
    if self.ingress().is_closed()? {
      return Err(crate::input::input_error("RuntimeIngressClosed", "cannot start input drivers after ingress is closed"));
    }
    let mut started = vec![false; self.attached_input_driver_count];
    for index in 0..self.attached_input_driver_count {
      if extension::invoke_extension_value(
        "host input driver",
        "is_live",
        || self.input_drivers[index].is_live(),
      )? {
        continue;
      }
      let has_driven_input = {
        let driver = &self.input_drivers[index];
        self
          .live_input_bindings
          .iter()
          .try_fold(false, |driven, (source, bindings)| {
            if driven || bindings.is_empty() {
              return Ok(driven);
            }
            extension::invoke_extension_value(
              "host input driver",
              "drives",
              || driver.drives(source),
            )
          })?
      };
      if !has_driven_input { continue; }
      if let Err(error) = extension::invoke_extension(
        "host input driver",
        "start",
        || self.input_drivers[index].start(),
      ) {
        let mut cleanup_failures = Vec::new();
        for cleanup_index in (0..self.attached_input_driver_count).rev() {
          if !started[cleanup_index] { continue; }
          if let Err(cleanup_error) = extension::invoke_extension(
            "host input driver",
            "stop",
            || self.input_drivers[cleanup_index].stop(),
          ) {
            cleanup_failures.push(format!(
              "input driver {} stop failed: {:?}",
              cleanup_index,
              cleanup_error,
            ));
          }
        }
        if !cleanup_failures.is_empty() {
          return Err(self.poison_program_operation(
            "start_input_drivers",
            None,
            format!("{:?}", error),
            cleanup_failures,
          ));
        }
        return Err(error);
      }
      started[index] = true;
    }
    Ok(())
  }

  pub fn stop_input_drivers(&mut self) -> MResult<()> {
    let mut first_error = None;
    let mut panic_failures = Vec::new();
    for driver in self.input_drivers[..self.attached_input_driver_count].iter_mut().rev() {
      if let Err(error) = extension::invoke_extension(
        "host input driver",
        "stop",
        || driver.stop(),
      ) {
        if error.kind_name() == "RuntimeExtensionPanicked" {
          panic_failures.push(format!("{:?}", error));
        }
        if first_error.is_none() { first_error = Some(error); }
      }
    }
    if !panic_failures.is_empty() {
      return Err(self.poison_program_operation(
        "stop_input_drivers",
        None,
        first_error
          .as_ref()
          .map(|error| format!("{:?}", error))
          .unwrap_or_else(|| "input driver cleanup panicked".to_string()),
        panic_failures,
      ));
    }
    if let Some(error) = first_error { return Err(error); }
    Ok(())
  }
}
