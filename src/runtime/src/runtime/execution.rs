// ---------------------------------------------------------------------------
// Program execution
// ---------------------------------------------------------------------------

// These are the main methods responsible for executing Mech programs within the runtime. They handle the orchestration of program execution, including setting up the execution context, managing module imports and dependencies, emitting events for diagnostics, and ensuring that execution adheres to the runtime's limits and policies.

// There are two main entry points for execution:

// - `run_string`: Executes a string of Mech source code directly. This is for lightweight execution of ad-hoc code snippets, scripts, documents, configuration files, etc.
// - `run_module`: Executes a module by its version ID, handling the resolution of dependencies and the construction of the import environment. This is for executing more complex, modular code that depends on other modules and is part of the larger program structure.

// Both methods have corresponding _with_context versions that accept a mutable reference to a RuntimeContext, allowing for execution within the context of an active transaction. This ensures that any changes made during execution are properly staged within the transaction that if an error occurs, the transaction can be rolled back to maintain consistency.

use super::*;
use crate::SourceIndex;

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAddressTarget {
  Interpreter(SourceScope),
  Context(RuntimeContextBinding),
  Unknown,
}

struct ActiveRuntimeProgramHostGuard {
  previous: Option<RuntimeProgramHostTarget>,
}

impl ActiveRuntimeProgramHostGuard {
  fn install(runtime: *mut MechRuntime, context: *mut RuntimeContext) -> Self {
    let previous = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget { runtime, context }))
    });
    Self { previous }
  }
}

impl Drop for ActiveRuntimeProgramHostGuard {
  fn drop(&mut self) {
    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(self.previous.take());
    });
  }
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

fn join_resource_paths(prefix: &str, child: &str) -> String {
  let prefix = prefix.trim_matches('/');
  let child = child.trim_matches('/');
  match (prefix.is_empty(), child.is_empty()) {
    (true, true) => String::new(),
    (true, false) => child.to_string(),
    (false, true) => prefix.to_string(),
    (false, false) => format!("{}/{}", prefix, child),
  }
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

fn runtime_context_allows_read(
  binding: &RuntimeContextBinding,
  path: &str,
) -> bool {
  binding.capabilities.iter().any(|capability| {
    if capability.operation != "read" {
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

#[allow(dead_code)]
fn runtime_context_allows_write(
  binding: &RuntimeContextBinding,
  path: &str,
) -> bool {
  binding.capabilities.iter().any(|capability| {
    if capability.operation != "write" {
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

fn bind_runtime_value_on_program(
  program: &mut MechProgram,
  var: &mech_core::Var,
  value: Value,
  mutable: bool,
) -> MResult<()> {
  if var.context.is_some() {
    return Err(MechError::new(
      RuntimeAddressedAssignmentUnsupported { target: var.name.to_string() },
      None,
    ).with_compiler_loc().with_tokens(var.tokens()));
  }
  let (id, name) = (var.name.hash(), var.name.to_string());
  let symbols = program.interpreter_mut().symbols();
  let mut symbols = symbols.borrow_mut();
  symbols.insert(id, value, mutable);
  symbols.dictionary.borrow_mut().insert(id, name);
  Ok(())
}

fn resolve_runtime_value(value: Value) -> Value {
  match value {
    Value::MutableReference(value) => value.borrow().clone(),
    other => other,
  }
}



impl MechRuntime {
  fn is_manifest_context_import(import: &mech_core::ModuleImport) -> bool {
    matches!(import.alias, Some(mech_core::ModuleImportAlias::Context(_)))
  }


  fn context_declarations_from_index_scope(
    &self,
    index: &SourceIndex,
    scope: &SourceScope,
  ) -> MResult<Vec<crate::SourceContextDeclaration>> {
    let mut declarations = index
      .contexts
      .iter()
      .filter(|ctx| &ctx.occurrence.scope == scope)
      .map(|ctx| ctx.declaration.clone())
      .collect::<Vec<_>>();

    for import in index.imports.iter().filter(|import| &import.occurrence.scope == scope) {
      let Some(SourceImportAlias::Context(alias)) = &import.declaration.alias else {
        continue;
      };
      let module = import.declaration.module.as_deref().ok_or_else(|| {
        MechError::new(RuntimeInvalidOperationError {
          operation: "materialize_direct_manifest_context_imports",
          reason: format!("context import `{}` is missing module metadata", import.declaration.specifier),
        }, None)
      })?;
      let item = import.declaration.item.as_deref().ok_or_else(|| {
        MechError::new(RuntimeInvalidOperationError {
          operation: "materialize_direct_manifest_context_imports",
          reason: format!("context import `{}` is missing item metadata", import.declaration.specifier),
        }, None)
      })?;
      let export = self.module_manifests.context_export(module, item)?;
      declarations.push(crate::SourceContextDeclaration {
        name: alias.clone(),
        base: crate::SourceContextBase::ResourceUri(export.base_uri.clone()),
        capabilities: export.operations.iter().map(|operation| crate::SourceContextCapability {
          operation: operation.clone(),
          scope: crate::SourceContextCapabilityScope::Wildcard,
        }).collect(),
      });
    }

    Ok(declarations)
  }

  fn direct_context_registry_for_scope(
    &self,
    tree: &mech_core::Program,
    scope: &SourceScope,
  ) -> MResult<RuntimeContextRegistry> {
    let index = SourceIndex::from_program(tree);
    let declarations = self.context_declarations_from_index_scope(&index, scope)?;
    RuntimeContextRegistry::from_declarations(scope.clone(), &declarations)
  }

  fn resolve_context_resource_request(
    &self,
    binding: &RuntimeContextBinding,
    requested_path: &str,
  ) -> MResult<ResolvedContextResourceRequest> {
    let context_base_uri = runtime_context_base_uri(binding).trim_end_matches('/').to_string();
    let provider_base_uri = self
      .resources
      .provider_base_uri_for(&context_base_uri)?
      .unwrap_or_else(|| context_base_uri.clone());
    let context_root = context_base_uri
      .strip_prefix(&provider_base_uri)
      .unwrap_or_default()
      .trim_matches('/');
    Ok(ResolvedContextResourceRequest {
      provider_base_uri,
      provider_path: join_resource_paths(context_root, requested_path),
      context_path: requested_path.trim_matches('/').to_string(),
    })
  }

  fn read_context_resource(
    &self,
    context: &RuntimeContext,
    binding: &RuntimeContextBinding,
    path: &str,
  ) -> MResult<Value> {
    let resolved = self.resolve_context_resource_request(binding, path)?;
    if !runtime_context_allows_read(binding, &resolved.context_path) {
      return Err(MechError::new(RuntimeResourceCapabilityDenied {
        context_name: binding.name.clone(),
        operation: "read".to_string(),
        path: resolved.context_path,
      }, None));
    }
    if !self.grants.allows(
      &context.subject,
      &resolved.provider_base_uri,
      &RuntimeCapabilityOperation::Read,
      &resolved.provider_path,
    ) {
      return Err(MechError::new(RuntimeCapabilityGrantDenied {
        subject: context.subject.clone(),
        resource: resolved.provider_base_uri,
        operation: RuntimeCapabilityOperation::Read,
        path: resolved.provider_path,
      }, None));
    }
    self.resources.read(RuntimeResourceReadRequest {
      base_uri: resolved.provider_base_uri,
      path: resolved.provider_path,
      context_name: binding.name.clone(),
    })
  }

  fn write_context_resource(
    &mut self,
    context: &RuntimeContext,
    binding: &RuntimeContextBinding,
    path: &str,
    value: Value,
  ) -> MResult<()> {
    let resolved = self.resolve_context_resource_request(binding, path)?;
    if !runtime_context_allows_write(binding, &resolved.context_path) {
      return Err(MechError::new(RuntimeResourceCapabilityDenied {
        context_name: binding.name.clone(),
        operation: "write".to_string(),
        path: resolved.context_path,
      }, None));
    }
    if !self.grants.allows(
      &context.subject,
      &resolved.provider_base_uri,
      &RuntimeCapabilityOperation::Write,
      &resolved.provider_path,
    ) {
      return Err(MechError::new(RuntimeCapabilityGrantDenied {
        subject: context.subject.clone(),
        resource: resolved.provider_base_uri,
        operation: RuntimeCapabilityOperation::Write,
        path: resolved.provider_path,
      }, None));
    }
    self.resources.write(RuntimeResourceWriteRequest {
      base_uri: resolved.provider_base_uri,
      path: resolved.provider_path,
      context_name: binding.name.clone(),
      value,
    })
  }

  fn bind_context_read_temp(
    &self,
    program: &mut MechProgram,
    target: &str,
    path: &str,
    value: Value,
  ) -> MResult<mech_core::Expression> {
    let name = format!("mech-internal-context-{}-{}", hash_str(target), hash_str(path));
    let var = mech_core::Var {
      name: identifier_from_str(&name),
      context: None,
      kind: None,
    };
    bind_runtime_value_on_program(program, &var, resolve_runtime_value(value), false)?;
    Ok(mech_core::Expression::Var(var))
  }

  fn resolve_context_reads_in_expression(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    expression: &mech_core::Expression,
  ) -> MResult<mech_core::Expression> {
    match expression {
      mech_core::Expression::Var(var) => {
        let Some(var_context) = &var.context else {
          return Ok(expression.clone());
        };
        let target = var_context.to_string();
        let Some(binding) = registry.get(&target) else {
          return Ok(expression.clone());
        };
        let path = var.name.to_string();
        let value = self.read_context_resource(context, binding, &path)?;
        self.bind_context_read_temp(program, &target, &path, value)
      }
      mech_core::Expression::Formula(factor) => Ok(mech_core::Expression::Formula(
        self.resolve_context_reads_in_factor(context, program, registry, factor)?,
      )),
      mech_core::Expression::FunctionCall(call) => {
        let args = call.args.iter().map(|(name, expression)| {
          Ok((name.clone(), self.resolve_context_reads_in_expression(context, program, registry, expression)?))
        }).collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::FunctionCall(mech_core::FunctionCall { name: call.name.clone(), args }))
      }
      mech_core::Expression::FsmPipe(pipe) => {
        let mut pipe = pipe.clone();
        if let Some(args) = &pipe.start.args {
          pipe.start.args = Some(args.iter().map(|(name, expression)| {
            Ok((name.clone(), self.resolve_context_reads_in_expression(context, program, registry, expression)?))
          }).collect::<MResult<Vec<_>>>()?);
        }
        Ok(mech_core::Expression::FsmPipe(pipe))
      }
      mech_core::Expression::Literal(_) => Ok(expression.clone()),
      mech_core::Expression::Match(match_expression) => {
        let mut match_expression = match_expression.as_ref().clone();
        match_expression.source = self.resolve_context_reads_in_expression(context, program, registry, &match_expression.source)?;
        match_expression.arms = match_expression.arms.iter().map(|arm| {
          Ok(mech_core::MatchArm {
            pattern: arm.pattern.clone(),
            guard: arm.guard.as_ref().map(|guard| self.resolve_context_reads_in_expression(context, program, registry, guard)).transpose()?,
            expression: self.resolve_context_reads_in_expression(context, program, registry, &arm.expression)?,
          })
        }).collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::Match(Box::new(match_expression)))
      }
      mech_core::Expression::Range(range) => {
        let mut range = range.as_ref().clone();
        range.start = self.resolve_context_reads_in_factor(context, program, registry, &range.start)?;
        range.increment = match &range.increment {
          Some((operator, increment)) => Some((operator.clone(), self.resolve_context_reads_in_factor(context, program, registry, increment)?)),
          None => None,
        };
        range.terminal = self.resolve_context_reads_in_factor(context, program, registry, &range.terminal)?;
        Ok(mech_core::Expression::Range(Box::new(range)))
      }
      mech_core::Expression::Slice(slice) => {
        if slice
          .context
          .as_ref()
          .is_some_and(|context| registry.contains(&context.to_string()))
        {
          return Err(MechError::new(RuntimeInvalidOperationError {
            operation: "context_read",
            reason: "context-addressed slices are not supported".to_string(),
          }, None));
        }
        Ok(mech_core::Expression::Slice(
          self.resolve_context_reads_in_slice(context, program, registry, slice)?,
        ))
      }
      mech_core::Expression::Structure(structure) => Ok(mech_core::Expression::Structure(
        self.resolve_context_reads_in_structure(context, program, registry, structure)?,
      )),
      mech_core::Expression::SetComprehension(comprehension) => {
        let mut comprehension = comprehension.as_ref().clone();
        comprehension.expression = self.resolve_context_reads_in_expression(context, program, registry, &comprehension.expression)?;
        comprehension.qualifiers = comprehension.qualifiers.iter().map(|qualifier| self.resolve_context_reads_in_comprehension_qualifier(context, program, registry, qualifier)).collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::SetComprehension(Box::new(comprehension)))
      }
      mech_core::Expression::MatrixComprehension(comprehension) => {
        let mut comprehension = comprehension.as_ref().clone();
        comprehension.expression = self.resolve_context_reads_in_expression(context, program, registry, &comprehension.expression)?;
        comprehension.qualifiers = comprehension.qualifiers.iter().map(|qualifier| self.resolve_context_reads_in_comprehension_qualifier(context, program, registry, qualifier)).collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Expression::MatrixComprehension(Box::new(comprehension)))
      }
    }
  }

  fn resolve_context_reads_in_factor(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    factor: &mech_core::Factor,
  ) -> MResult<mech_core::Factor> {
    match factor {
      mech_core::Factor::Expression(expression) => Ok(mech_core::Factor::Expression(Box::new(self.resolve_context_reads_in_expression(context, program, registry, expression)?))),
      mech_core::Factor::Negate(factor) => Ok(mech_core::Factor::Negate(Box::new(self.resolve_context_reads_in_factor(context, program, registry, factor)?))),
      mech_core::Factor::Not(factor) => Ok(mech_core::Factor::Not(Box::new(self.resolve_context_reads_in_factor(context, program, registry, factor)?))),
      mech_core::Factor::Parenthetical(factor) => Ok(mech_core::Factor::Parenthetical(Box::new(self.resolve_context_reads_in_factor(context, program, registry, factor)?))),
      mech_core::Factor::Transpose(factor) => Ok(mech_core::Factor::Transpose(Box::new(self.resolve_context_reads_in_factor(context, program, registry, factor)?))),
      mech_core::Factor::Term(term) => {
        let rhs = term.rhs.iter().map(|(operator, factor)| {
          Ok((operator.clone(), self.resolve_context_reads_in_factor(context, program, registry, factor)?))
        }).collect::<MResult<Vec<_>>>()?;
        Ok(mech_core::Factor::Term(Box::new(mech_core::Term {
          lhs: self.resolve_context_reads_in_factor(context, program, registry, &term.lhs)?,
          rhs,
        })))
      }
    }
  }

  fn resolve_context_reads_in_slice(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    slice: &mech_core::Slice,
  ) -> MResult<mech_core::Slice> {
    Ok(mech_core::Slice {
      name: slice.name.clone(),
      context: slice.context.clone(),
      subscript: slice.subscript.iter().map(|subscript| self.resolve_context_reads_in_subscript(context, program, registry, subscript)).collect::<MResult<Vec<_>>>()?,
    })
  }

  fn resolve_context_reads_in_subscript(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    subscript: &mech_core::Subscript,
  ) -> MResult<mech_core::Subscript> {
    match subscript {
      mech_core::Subscript::Brace(subscripts) => Ok(mech_core::Subscript::Brace(subscripts.iter().map(|subscript| self.resolve_context_reads_in_subscript(context, program, registry, subscript)).collect::<MResult<Vec<_>>>()?)),
      mech_core::Subscript::Bracket(subscripts) => Ok(mech_core::Subscript::Bracket(subscripts.iter().map(|subscript| self.resolve_context_reads_in_subscript(context, program, registry, subscript)).collect::<MResult<Vec<_>>>()?)),
      mech_core::Subscript::Formula(factor) => Ok(mech_core::Subscript::Formula(self.resolve_context_reads_in_factor(context, program, registry, factor)?)),
      mech_core::Subscript::Range(range) => {
        let mut range = range.clone();
        range.start = self.resolve_context_reads_in_factor(context, program, registry, &range.start)?;
        range.increment = match &range.increment {
          Some((operator, increment)) => Some((operator.clone(), self.resolve_context_reads_in_factor(context, program, registry, increment)?)),
          None => None,
        };
        range.terminal = self.resolve_context_reads_in_factor(context, program, registry, &range.terminal)?;
        Ok(mech_core::Subscript::Range(range))
      }
      _ => Ok(subscript.clone()),
    }
  }

  fn resolve_context_reads_in_structure(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    structure: &mech_core::Structure,
  ) -> MResult<mech_core::Structure> {
    match structure {
      mech_core::Structure::Empty => Ok(mech_core::Structure::Empty),
      mech_core::Structure::Map(map) => Ok(mech_core::Structure::Map(mech_core::Map {
        elements: map.elements.iter().map(|mapping| Ok(mech_core::Mapping {
          key: self.resolve_context_reads_in_expression(context, program, registry, &mapping.key)?,
          value: self.resolve_context_reads_in_expression(context, program, registry, &mapping.value)?,
        })).collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Matrix(matrix) => Ok(mech_core::Structure::Matrix(mech_core::nodes::Matrix {
        rows: matrix.rows.iter().map(|row| Ok(mech_core::MatrixRow {
          columns: row.columns.iter().map(|column| Ok(mech_core::MatrixColumn {
            element: self.resolve_context_reads_in_expression(context, program, registry, &column.element)?,
          })).collect::<MResult<Vec<_>>>()?,
        })).collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Record(record) => Ok(mech_core::Structure::Record(mech_core::Record {
        bindings: record.bindings.iter().map(|binding| Ok(mech_core::Binding {
          name: binding.name.clone(),
          kind: binding.kind.clone(),
          value: self.resolve_context_reads_in_expression(context, program, registry, &binding.value)?,
        })).collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Set(set) => Ok(mech_core::Structure::Set(mech_core::Set {
        elements: set.elements.iter().map(|expression| self.resolve_context_reads_in_expression(context, program, registry, expression)).collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Table(table) => Ok(mech_core::Structure::Table(mech_core::Table {
        header: table.header.clone(),
        rows: table.rows.iter().map(|row| Ok(mech_core::TableRow {
          columns: row.columns.iter().map(|column| Ok(mech_core::TableColumn {
            element: self.resolve_context_reads_in_expression(context, program, registry, &column.element)?,
          })).collect::<MResult<Vec<_>>>()?,
        })).collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::Tuple(tuple) => Ok(mech_core::Structure::Tuple(mech_core::Tuple {
        elements: tuple.elements.iter().map(|expression| self.resolve_context_reads_in_expression(context, program, registry, expression)).collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Structure::TupleStruct(tuple_struct) => Ok(mech_core::Structure::TupleStruct(mech_core::TupleStruct {
        name: tuple_struct.name.clone(),
        value: Box::new(self.resolve_context_reads_in_expression(context, program, registry, &tuple_struct.value)?),
      })),
    }
  }

  fn resolve_context_reads_in_comprehension_qualifier(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    qualifier: &mech_core::ComprehensionQualifier,
  ) -> MResult<mech_core::ComprehensionQualifier> {
    match qualifier {
      mech_core::ComprehensionQualifier::Generator((pattern, expression)) => Ok(mech_core::ComprehensionQualifier::Generator((pattern.clone(), self.resolve_context_reads_in_expression(context, program, registry, expression)?))),
      mech_core::ComprehensionQualifier::Filter(expression) => Ok(mech_core::ComprehensionQualifier::Filter(self.resolve_context_reads_in_expression(context, program, registry, expression)?)),
      mech_core::ComprehensionQualifier::Let(var_def) => {
        let mut var_def = var_def.clone();
        var_def.expression = self.resolve_context_reads_in_expression(context, program, registry, &var_def.expression)?;
        Ok(mech_core::ComprehensionQualifier::Let(var_def))
      }
    }
  }

  fn flush_direct_execution(
    &mut self,
    program: &mut MechProgram,
    pending: &mut Vec<mech_core::SectionElement>,
    result: &mut Value,
  ) -> MResult<()> {
    if pending.is_empty() {
      return Ok(());
    }
    let tree = mech_core::Program {
      title: None,
      body: mech_core::Body {
        sections: vec![mech_core::Section {
          subtitle: None,
          elements: std::mem::take(pending),
        }],
      },
    };
    *result = program.run_tree(&tree)?;
    Ok(())
  }

  fn executable_fence_for_scope(
    fenced: &mech_core::FencedMechCode,
    scope: &SourceScope,
  ) -> bool {
    match scope {
      SourceScope::Program => fenced.config.namespace_str.is_empty(),
      SourceScope::Interpreter(interpreter) => fenced.config.namespace_str == interpreter.namespace_str,
    }
  }

  fn resolve_context_reads_in_pattern(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    pattern: &mech_core::Pattern,
  ) -> MResult<mech_core::Pattern> {
    match pattern {
      mech_core::Pattern::Expression(expression) => Ok(mech_core::Pattern::Expression(
        self.resolve_context_reads_in_expression(context, program, registry, expression)?,
      )),
      mech_core::Pattern::TupleStruct(tuple_struct) => Ok(mech_core::Pattern::TupleStruct(mech_core::PatternTupleStruct {
        name: tuple_struct.name.clone(),
        patterns: tuple_struct.patterns.iter()
          .map(|pattern| self.resolve_context_reads_in_pattern(context, program, registry, pattern))
          .collect::<MResult<Vec<_>>>()?,
      })),
      mech_core::Pattern::Tuple(tuple) => Ok(mech_core::Pattern::Tuple(mech_core::PatternTuple(
        tuple.0.iter()
          .map(|pattern| self.resolve_context_reads_in_pattern(context, program, registry, pattern))
          .collect::<MResult<Vec<_>>>()?,
      ))),
      mech_core::Pattern::Array(array) => {
        let spread = if let Some(spread) = &array.spread {
          Some(mech_core::PatternArraySpread {
            kind: spread.kind.clone(),
            binding: spread.binding.as_ref()
              .map(|binding| self.resolve_context_reads_in_pattern(context, program, registry, binding).map(Box::new))
              .transpose()?,
          })
        } else {
          None
        };
        Ok(mech_core::Pattern::Array(mech_core::PatternArray {
          prefix: array.prefix.iter()
            .map(|pattern| self.resolve_context_reads_in_pattern(context, program, registry, pattern))
            .collect::<MResult<Vec<_>>>()?,
          spread,
          suffix: array.suffix.iter()
            .map(|pattern| self.resolve_context_reads_in_pattern(context, program, registry, pattern))
            .collect::<MResult<Vec<_>>>()?,
        }))
      }
      mech_core::Pattern::Wildcard => Ok(mech_core::Pattern::Wildcard),
    }
  }

  fn resolve_context_reads_in_transition(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    transition: &mech_core::Transition,
  ) -> MResult<mech_core::Transition> {
    match transition {
      mech_core::Transition::Async(pattern) => Ok(mech_core::Transition::Async(
        self.resolve_context_reads_in_pattern(context, program, registry, pattern)?,
      )),
      mech_core::Transition::CodeBlock(code_items) => Ok(mech_core::Transition::CodeBlock(
        code_items.iter()
          .map(|(code, comment)| Ok((self.resolve_context_reads_in_mech_code(context, program, registry, code)?, comment.clone())))
          .collect::<MResult<Vec<_>>>()?,
      )),
      mech_core::Transition::Next(pattern) => Ok(mech_core::Transition::Next(
        self.resolve_context_reads_in_pattern(context, program, registry, pattern)?,
      )),
      mech_core::Transition::Output(pattern) => Ok(mech_core::Transition::Output(
        self.resolve_context_reads_in_pattern(context, program, registry, pattern)?,
      )),
      mech_core::Transition::Statement(statement) => Ok(mech_core::Transition::Statement(
        self.resolve_context_reads_in_statement(context, program, registry, statement)?,
      )),
    }
  }

  fn resolve_context_reads_in_fsm_implementation(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    fsm: &mech_core::FsmImplementation,
  ) -> MResult<mech_core::FsmImplementation> {
    let arms = fsm.arms.iter().map(|arm| {
      match arm {
        mech_core::FsmArm::Guard(pattern, guards) => Ok(mech_core::FsmArm::Guard(
          self.resolve_context_reads_in_pattern(context, program, registry, pattern)?,
          guards.iter().map(|guard| Ok(mech_core::Guard {
            condition: self.resolve_context_reads_in_pattern(context, program, registry, &guard.condition)?,
            transitions: guard.transitions.iter()
              .map(|transition| self.resolve_context_reads_in_transition(context, program, registry, transition))
              .collect::<MResult<Vec<_>>>()?,
          })).collect::<MResult<Vec<_>>>()?,
        )),
        mech_core::FsmArm::Transition(pattern, transitions) => Ok(mech_core::FsmArm::Transition(
          self.resolve_context_reads_in_pattern(context, program, registry, pattern)?,
          transitions.iter()
            .map(|transition| self.resolve_context_reads_in_transition(context, program, registry, transition))
            .collect::<MResult<Vec<_>>>()?,
        )),
        mech_core::FsmArm::Comment(comment) => Ok(mech_core::FsmArm::Comment(comment.clone())),
      }
    }).collect::<MResult<Vec<_>>>()?;

    Ok(mech_core::FsmImplementation {
      name: fsm.name.clone(),
      input: fsm.input.clone(),
      start: self.resolve_context_reads_in_pattern(context, program, registry, &fsm.start)?,
      arms,
    })
  }

  fn resolve_context_reads_in_function_define(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    function: &mech_core::FunctionDefine,
  ) -> MResult<mech_core::FunctionDefine> {
    Ok(mech_core::FunctionDefine {
      name: function.name.clone(),
      input: function.input.clone(),
      output: function.output.clone(),
      statements: function.statements.iter()
        .map(|statement| self.resolve_context_reads_in_statement(context, program, registry, statement))
        .collect::<MResult<Vec<_>>>()?,
      match_arms: function.match_arms.iter().map(|arm| Ok(mech_core::FunctionMatchArm {
        pattern: self.resolve_context_reads_in_pattern(context, program, registry, &arm.pattern)?,
        expression: self.resolve_context_reads_in_expression(context, program, registry, &arm.expression)?,
      })).collect::<MResult<Vec<_>>>()?,
    })
  }

  fn resolve_context_reads_in_mech_code(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    code: &mech_core::MechCode,
  ) -> MResult<mech_core::MechCode> {
    match code {
      mech_core::MechCode::Statement(statement) => Ok(mech_core::MechCode::Statement(
        self.resolve_context_reads_in_statement(context, program, registry, statement)?,
      )),
      mech_core::MechCode::Expression(expression) => Ok(mech_core::MechCode::Expression(
        self.resolve_context_reads_in_expression(context, program, registry, expression)?,
      )),
      mech_core::MechCode::FsmImplementation(fsm) => Ok(mech_core::MechCode::FsmImplementation(
        self.resolve_context_reads_in_fsm_implementation(context, program, registry, fsm)?,
      )),
      mech_core::MechCode::FsmSpecification(spec) => Ok(mech_core::MechCode::FsmSpecification(spec.clone())),
      mech_core::MechCode::FunctionDefine(function) => Ok(mech_core::MechCode::FunctionDefine(
        self.resolve_context_reads_in_function_define(context, program, registry, function)?,
      )),
      mech_core::MechCode::Import(_)
      | mech_core::MechCode::Comment(_)
      | mech_core::MechCode::Error(_, _) => Ok(code.clone()),
    }
  }

  fn resolve_context_reads_in_statement(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    statement: &mech_core::Statement,
  ) -> MResult<mech_core::Statement> {
    match statement {
      mech_core::Statement::VariableDefine(var_def) => {
        let mut var_def = var_def.clone();
        var_def.expression = self.resolve_context_reads_in_expression(context, program, registry, &var_def.expression)?;
        Ok(mech_core::Statement::VariableDefine(var_def))
      }
      mech_core::Statement::VariableAssign(assign) => {
        let mut assign = assign.clone();
        assign.expression = self.resolve_context_reads_in_expression(context, program, registry, &assign.expression)?;
        Ok(mech_core::Statement::VariableAssign(assign))
      }
      mech_core::Statement::OpAssign(op_assign) => {
        let mut op_assign = op_assign.clone();
        op_assign.expression = self.resolve_context_reads_in_expression(context, program, registry, &op_assign.expression)?;
        Ok(mech_core::Statement::OpAssign(op_assign))
      }
      mech_core::Statement::TupleDestructure(tuple_destructure) => {
        let mut tuple_destructure = tuple_destructure.clone();
        tuple_destructure.expression = self.resolve_context_reads_in_expression(context, program, registry, &tuple_destructure.expression)?;
        Ok(mech_core::Statement::TupleDestructure(tuple_destructure))
      }
      #[cfg(feature = "invariant_define")]
      mech_core::Statement::InvariantDefine(invariant) => {
        let mut invariant = invariant.clone();
        invariant.expression = self.resolve_context_reads_in_expression(context, program, registry, &invariant.expression)?;
        Ok(mech_core::Statement::InvariantDefine(invariant))
      }
      _ => Ok(statement.clone()),
    }
  }

  fn push_direct_code(
    &mut self,
    context: &RuntimeContext,
    program: &mut MechProgram,
    registry: &RuntimeContextRegistry,
    pending: &mut Vec<mech_core::SectionElement>,
    pending_codes: &mut Vec<(mech_core::MechCode, Option<mech_core::Comment>)>,
    result: &mut Value,
    skip_non_context_imports: bool,
    code: &mech_core::MechCode,
    comment: &Option<mech_core::Comment>,
  ) -> MResult<()> {
    match code {
      mech_core::MechCode::Import(import) if Self::is_manifest_context_import(import) => Ok(()),
      mech_core::MechCode::Import(_) if skip_non_context_imports => Ok(()),
      mech_core::MechCode::Statement(mech_core::Statement::ImportDeclaration(_))
      | mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(_)) => Ok(()),
      mech_core::MechCode::Statement(mech_core::Statement::ExportDeclaration(export)) => {
        if !pending_codes.is_empty() {
          pending.push(mech_core::SectionElement::MechCode(std::mem::take(pending_codes)));
        }
        self.flush_direct_execution(program, pending, result)?;
        let id = hash_str(&export.name.to_string());
        if let Some(value) = program.interpreter().symbols().borrow().get(id) {
          *result = resolve_runtime_value(value.borrow().clone());
        } else {
          *result = Value::Empty;
        }
        Ok(())
      }
      mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)) => {
        if let Some(context_name) = &var_def.var.context {
          let target = context_name.to_string();
          if let Some(binding) = registry.get(&target).cloned() {
            if !pending_codes.is_empty() {
              pending.push(mech_core::SectionElement::MechCode(std::mem::take(pending_codes)));
            }
            self.flush_direct_execution(program, pending, result)?;
            return Err(MechError::new(RuntimeInvalidOperationError {
              operation: "direct_context_define",
              reason: format!(
                "context-addressed path `@{}/{}` cannot be defined with `:=`; use `=` for context writes",
                binding.name,
                var_def.var.name.to_string()
              ),
            }, None));
          }
        }
        let code = self.resolve_context_reads_in_mech_code(
          context,
          program,
          registry,
          &mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def.clone())),
        )?;
        pending_codes.push((code, comment.clone()));
        Ok(())
      }
      mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign)) => {
        if let Some(context_name) = &assign.target.context {
          let target = context_name.to_string();
          if let Some(binding) = registry.get(&target).cloned() {
            if !pending_codes.is_empty() {
              pending.push(mech_core::SectionElement::MechCode(std::mem::take(pending_codes)));
            }
            self.flush_direct_execution(program, pending, result)?;
            let expression = self.resolve_context_reads_in_expression(context, program, registry, &assign.expression)?;
            let value = resolve_runtime_value(self.evaluate_expression_on_program(program, &expression)?);
            self.write_context_resource(context, &binding, &assign.target.name.to_string(), value.clone())?;
            *result = value;
            return Ok(());
          }
        }
        let code = self.resolve_context_reads_in_mech_code(
          context,
          program,
          registry,
          &mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign.clone())),
        )?;
        pending_codes.push((code, comment.clone()));
        Ok(())
      }
      _ => {
        let code = self.resolve_context_reads_in_mech_code(context, program, registry, code)?;
        pending_codes.push((code, comment.clone()));
        Ok(())
      }
    }
  }

  fn run_tree_on_program(
    &mut self,
    context: &mut RuntimeContext,
    program: &mut MechProgram,
    tree: &mech_core::Program,
    scope_hint: Option<&SourceScope>,
  ) -> MResult<Value> {
    let execution_scope = scope_hint.unwrap_or(&SourceScope::Program);
    let skip_non_context_imports = scope_hint.is_some();
    let registry = self.direct_context_registry_for_scope(tree, execution_scope)?;
    let mut result = Value::Empty;
    let mut pending = Vec::new();

    for section in &tree.body.sections {
      for element in &section.elements {
        match element {
          mech_core::SectionElement::MechCode(codes) => {
            let mut pending_codes = Vec::new();
            for (code, comment) in codes {
              self.push_direct_code(
                context,
                program,
                &registry,
                &mut pending,
                &mut pending_codes,
                &mut result,
                skip_non_context_imports,
                code,
                comment,
              )?;
            }
            if !pending_codes.is_empty() {
              pending.push(mech_core::SectionElement::MechCode(pending_codes));
            }
          }
          mech_core::SectionElement::FencedMechCode(fenced)
            if Self::executable_fence_for_scope(fenced, execution_scope) =>
          {
            let mut pending_codes = Vec::new();
            for (code, comment) in &fenced.code {
              self.push_direct_code(
                context,
                program,
                &registry,
                &mut pending,
                &mut pending_codes,
                &mut result,
                skip_non_context_imports,
                code,
                comment,
              )?;
            }
            if !pending_codes.is_empty() {
              let mut fenced = fenced.clone();
              fenced.code = pending_codes;
              pending.push(mech_core::SectionElement::FencedMechCode(fenced));
            }
          }
          _ => {}
        }
      }
    }

    self.flush_direct_execution(program, &mut pending, &mut result)?;
    Ok(result)
  }

  fn evaluate_expression_on_program(
    &mut self,
    program: &mut MechProgram,
    expression: &mech_core::Expression,
  ) -> MResult<Value> {
    let single = single_code_program(mech_core::MechCode::Expression(expression.clone()), None);
    program.run_tree(&single).map(resolve_runtime_value)
  }

  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let mut context = self.runtime_context()?;
    self.run_string_with_context(&mut context, source)
  }
  pub fn run_string_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<Value> {
    context.validate()?;
    context.charge_step()?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let tree = mech_syntax::parser::parse(source.trim())?;
    let program_config = self.program.config.clone();
    let mut program = std::mem::replace(
      &mut self.program,
      MechProgram::new(program_config),
    );

    self.register_runtime_program_host_functions(
      context,
      &mut program,
    )?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;
    let _host_guard = ActiveRuntimeProgramHostGuard::install(runtime_ptr, context_ptr);

    let result = self.run_tree_on_program(context, &mut program, &tree, None);

    self.program = program;

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }


  pub fn run_tree(&mut self, tree: &mech_core::Program) -> MResult<Value> {
    let mut context = self.runtime_context()?;
    self.run_tree_with_context(&mut context, tree)
  }

  pub fn run_tree_with_context(
    &mut self,
    context: &mut RuntimeContext,
    tree: &mech_core::Program,
  ) -> MResult<Value> {
    context.validate()?;
    context.charge_step()?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let program_config = self.program.config.clone();
    let mut program = std::mem::replace(
      &mut self.program,
      MechProgram::new(program_config),
    );

    self.register_runtime_program_host_functions(
      context,
      &mut program,
    )?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;
    let _host_guard = ActiveRuntimeProgramHostGuard::install(runtime_ptr, context_ptr);

    let result = self.run_tree_on_program(context, &mut program, tree, None);

    self.program = program;

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }

  pub fn out_string(&self) -> String {
    self.program.out_string()
  }

  pub fn has_interpreter(&self, interpreter_id: u64) -> bool {
    self.program.has_interpreter(interpreter_id)
  }

  pub fn output_value_for_interpreter(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<Value> {
    self.program.output_value_for_interpreter(interpreter_id, output_id)
  }

  pub fn symbol_name_for_interpreter_output(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<String> {
    self.program.symbol_name_for_interpreter_output(interpreter_id, output_id)
  }

  pub fn symbol_values_for_interpreter(
    &self,
    interpreter_id: u64,
    names: &[String],
  ) -> Option<Vec<(String, Value)>> {
    self.program.symbol_values_for_interpreter(interpreter_id, names)
  }

  pub fn bind_ans_for_interpreter(
    &mut self,
    interpreter_id: u64,
    value: &Value,
  ) -> MResult<()> {
    if self.program.bind_ans_for_interpreter(interpreter_id, value) {
      return Ok(());
    }

    Err(MechError::new(
      RuntimeInvalidOperationError {
        operation: "bind_ans_for_interpreter",
        reason: format!("interpreter id {} not found", interpreter_id),
      },
      None,
    ))
  }

  #[cfg(feature = "functions")]
  pub fn step(&mut self, count: u64) -> MResult<()> {
    self.program.step(count)
  }

  pub fn run_module(&mut self, version: ModuleVersionId) -> MResult<Value> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_with_context(&mut context, version)
  }

  pub fn run_module_scope(
    &mut self,
    version: ModuleVersionId,
    scope: SourceScope,
  ) -> MResult<Value> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_scope_with_context(&mut context, version, scope)
  }

  pub fn run_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
  ) -> MResult<Value> {
    self.run_module_scope_with_context(context, version, SourceScope::Program)
  }

  pub fn run_module_scope_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: SourceScope,
  ) -> MResult<Value> {
    let mut seen = HashSet::new();
    let mut module_instances = HashMap::new();

    let instance = self.execute_module_isolated_for_scope(
      context,
      version,
      &scope,
      &mut seen,
      &mut module_instances,
    )?;

    Ok(instance.result)
  }

  fn execute_module_isolated_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    scope: &SourceScope,
    seen: &mut HashSet<(ModuleVersionId, SourceScope)>,
    module_instances: &mut HashMap<(ModuleVersionId, SourceScope), ModuleInstance>,
  ) -> MResult<ModuleInstance> {
    context.validate()?;
    context.charge_step()?;

    let instance_key = (version, scope.clone());

    if let Some(instance) = module_instances.get(&instance_key).cloned() {
      return Ok(instance);
    }

    if seen.contains(&instance_key) {
      return Ok(ModuleInstance { version, exports: HashMap::new(), result: Value::Empty });
    }
    seen.insert(instance_key.clone());

    let Some(record) = self.store.get_module_version(version)? else {
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

    let mut import_environment = self.build_import_environment_for_scope(
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
    import_environment.extend(address_environment);
    let mut module_program = MechProgram::new(MechProgramConfig {
      name: self.config.name.clone(),
      environment: MechProgramEnvironment {
        trace_enabled: self.config.diagnostics.trace_enabled,
        debug_enabled: self.config.diagnostics.debug_enabled,
        profile_enabled: self.config.diagnostics.profile_enabled,
        rounds_per_step: self.config.limits.max_steps_per_turn.unwrap_or(10_000) as usize,
      },
    });

    {
      let symbols = module_program.interpreter_mut().symbols();
      let mut symbols_brrw = symbols.borrow_mut();
      for (name, value_ref) in import_environment {
        let id = hash_str(&name);
        symbols_brrw.symbols.insert(id, value_ref);
        symbols_brrw.dictionary.borrow_mut().insert(id, name);
      }
    }

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleExecutionStarted { module_version: version },
    )?;
    let scoped_source = module_source_for_scope(&source, scope)?;
    let result = self.run_module_source_on_program(context, &mut module_program, &scoped_source, scope);
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
    let mut exports = HashMap::new();
    {
      let symbols = module_program.interpreter_mut().symbols();
      let symbols_brrw = symbols.borrow();
      for export in exports_for_scope(&record, scope) {
        let id = hash_str(&export.name);
        let Some(value_ref) = symbols_brrw.get(id) else {
          let error = MechError::new(RuntimeModuleExportNotFound { dependency: record.id.to_string(), export: export.name.clone() }, None);
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

  fn run_module_source_on_program(
    &mut self,
    context: &mut RuntimeContext,
    program: &mut MechProgram,
    source: &MechSourceCode,
    scope: &SourceScope,
  ) -> MResult<Value> {
    self.register_runtime_program_host_functions(context, program)?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;
    let _host_guard = ActiveRuntimeProgramHostGuard::install(runtime_ptr, context_ptr);

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

    let result = match source {
      MechSourceCode::String(source) => {
        mech_syntax::parser::parse(source.trim())
          .and_then(|tree| self.run_tree_on_program(context, program, &tree, Some(&execution_scope)))
      }
      other => program.run_source(other),
    };

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
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




#[cfg(test)]
mod tests {
  use super::*;
  use mech_core::Ref;

  #[test]
  fn runtime_has_interpreter_finds_root_interpreter() {
    let runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    assert!(runtime.has_interpreter(0));
  }

  #[test]
  fn runtime_output_value_for_interpreter_returns_value_after_run_string() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let source = "```mech
1
```";
    let _ = runtime.run_string(source).unwrap();
    let root_id = runtime.program().interpreter().id;
    let output_id = {
      let out_values = runtime.program().interpreter().out_values.borrow();
      *out_values.keys().next().expect("expected output value after run_string")
    };
    let output = runtime.output_value_for_interpreter(root_id, output_id);
    assert!(output.is_some());
  }

  #[test]
  fn runtime_bind_ans_for_interpreter_binds_ans() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let value = Value::U64(Ref::new(42));
    runtime.bind_ans_for_interpreter(0, &value).unwrap();
    let ans_id = hash_str("ans");
    let bound = runtime
      .program()
      .interpreter()
      .symbols()
      .borrow()
      .get(ans_id)
      .map(|value| value.borrow().clone());
    assert_eq!(bound, Some(value));
  }
}
