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
mod host_input;
mod input_drivers;
mod live_registration;
mod module;
mod module_environment;
mod persistent_send;
mod query;
mod reactive;
mod source;
mod source_reconstruction;

pub(super) use activation_effects::{
  ActivationEffectBarrierCompiler,
  ActivationEffectPayloadCaptureCompiler,
  ACTIVATION_EFFECT_BARRIER_NAME,
  ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
};
use module_environment::{
  context_registry_for_scope,
  execution_scope_for_extracted_module_source,
  exports_for_scope,
  materialize_function_imports_for_scope,
  merge_module_environment,
  resolve_runtime_address_target,
  PreparedModuleScopeExecution,
  ProgramEnvironmentOverlay,
  RuntimeAddressTarget,
};
use source_reconstruction::module_source_for_scope;

use super::*;
use super::reactive_transaction::PreparedRuntimeHostInput;
use crate::{
  RuntimeModuleResult, RuntimeResourceKey, RuntimeValueSnapshot, SourceDeclaration,
  SourceImportDeclaration, SourceIndex,
};
use crate::RuntimeResourceWritePreflightRequest;

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


















































}



#[cfg(test)]
mod tests;

impl MechRuntime {

















}
