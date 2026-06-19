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

const DEFAULT_RESOURCE_SUBJECT: &str = "task://main";


#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAddressTarget {
  Interpreter(SourceScope),
  Context(RuntimeContextBinding),
  Unknown,
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



fn program_codes(
  tree: &mech_core::Program,
) -> impl Iterator<Item = (&mech_core::MechCode, &Option<mech_core::Comment>)> {
  tree.body.sections.iter().flat_map(|section| {
    section.elements.iter().flat_map(|element| match element {
      mech_core::SectionElement::MechCode(codes) => codes
        .iter()
        .map(|(code, comment)| (code, comment))
        .collect::<Vec<_>>(),
      mech_core::SectionElement::FencedMechCode(fenced) => fenced
        .code
        .iter()
        .map(|(code, comment)| (code, comment))
        .collect::<Vec<_>>(),
      _ => Vec::new(),
    })
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

  fn read_context_resource(
    &self,
    binding: &RuntimeContextBinding,
    path: &str,
  ) -> MResult<Value> {
    let base_uri = runtime_context_base_uri(binding);
    if !runtime_context_allows_read(binding, path) {
      return Err(MechError::new(RuntimeResourceCapabilityDenied {
        context_name: binding.name.clone(),
        operation: "read".to_string(),
        path: path.to_string(),
      }, None));
    }
    self.resources.read(RuntimeResourceReadRequest {
      base_uri,
      path: path.to_string(),
      context_name: binding.name.clone(),
    })
  }

  fn write_context_resource(
    &mut self,
    binding: &RuntimeContextBinding,
    path: &str,
    value: Value,
  ) -> MResult<()> {
    let base_uri = runtime_context_base_uri(binding);
    if !runtime_context_allows_write(binding, path) {
      return Err(MechError::new(RuntimeResourceCapabilityDenied {
        context_name: binding.name.clone(),
        operation: "write".to_string(),
        path: path.to_string(),
      }, None));
    }
    self.resources.write(RuntimeResourceWriteRequest {
      base_uri,
      path: path.to_string(),
      context_name: binding.name.clone(),
      value,
    })
  }

  fn expression_context_read(
    &self,
    registry: &RuntimeContextRegistry,
    expression: &mech_core::Expression,
  ) -> MResult<Option<Value>> {
    let mech_core::Expression::Var(var) = expression else {
      return Ok(None);
    };
    let Some(context) = &var.context else {
      return Ok(None);
    };
    let target = context.to_string();
    let Some(binding) = registry.get(&target) else {
      return Ok(None);
    };
    Ok(Some(resolve_runtime_value(self.read_context_resource(binding, &var.name.to_string())?)))
  }

  fn run_tree_on_program(
    &mut self,
    _context: &mut RuntimeContext,
    program: &mut MechProgram,
    tree: &mech_core::Program,
  ) -> MResult<Value> {
    let registry = self.direct_context_registry_for_scope(tree, &SourceScope::Program)?;
    let mut result = Value::Empty;
    for (code, comment) in program_codes(tree) {
      match code {
        mech_core::MechCode::Import(_)
        | mech_core::MechCode::Statement(mech_core::Statement::ImportDeclaration(_))
        | mech_core::MechCode::Statement(mech_core::Statement::ExportDeclaration(_))
        | mech_core::MechCode::Statement(mech_core::Statement::ContextDeclaration(_)) => {
          result = Value::Empty;
        }
        mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)) => {
          if let Some(context) = &var_def.var.context {
            let target = context.to_string();
            if let Some(binding) = registry.get(&target).cloned() {
              let value = resolve_runtime_value(self.evaluate_expression_on_program(program, &var_def.expression)?);
              self.write_context_resource(&binding, &var_def.var.name.to_string(), value.clone())?;
              result = value;
            } else {
              let single = single_code_program(code.clone(), comment.clone());
              result = program.run_tree(&single)?;
            }
          } else if let Some(value) = self.expression_context_read(&registry, &var_def.expression)? {
            bind_runtime_value_on_program(program, &var_def.var, value.clone(), var_def.mutable)?;
            result = value;
          } else {
            let single = single_code_program(code.clone(), comment.clone());
            result = program.run_tree(&single)?;
          }
        }
        mech_core::MechCode::Statement(mech_core::Statement::VariableAssign(assign)) => {
          if let Some(context) = &assign.target.context {
            let target = context.to_string();
            if let Some(binding) = registry.get(&target).cloned() {
              let value = resolve_runtime_value(self.evaluate_expression_on_program(program, &assign.expression)?);
              self.write_context_resource(&binding, &assign.target.name.to_string(), value.clone())?;
              result = value;
            } else {
              let single = single_code_program(code.clone(), comment.clone());
              result = program.run_tree(&single)?;
            }
          } else {
            let single = single_code_program(code.clone(), comment.clone());
            result = program.run_tree(&single)?;
          }
        }
        mech_core::MechCode::Expression(expression) => {
          if let Some(value) = self.expression_context_read(&registry, expression)? {
            result = value;
          } else {
            let single = single_code_program(code.clone(), comment.clone());
            result = program.run_tree(&single)?;
          }
        }
        _ => {
          let single = single_code_program(code.clone(), comment.clone());
          result = program.run_tree(&single)?;
        }
      }
    }
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

    let previous_target = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget {
        runtime: runtime_ptr,
        context: context_ptr,
      }))
    });

    let result = self.run_tree_on_program(context, &mut program, &tree);

    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(previous_target);
    });

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

    let previous_target = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget {
        runtime: runtime_ptr,
        context: context_ptr,
      }))
    });

    let result = self.run_tree_on_program(context, &mut program, tree);

    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(previous_target);
    });

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
    let result = self.run_module_source_on_program(context, &mut module_program, &scoped_source);
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
  ) -> MResult<Value> {
    self.register_runtime_program_host_functions(context, program)?;

    let runtime_ptr: *mut MechRuntime = self;
    let context_ptr: *mut RuntimeContext = context;

    let previous_target = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(Some(RuntimeProgramHostTarget {
        runtime: runtime_ptr,
        context: context_ptr,
      }))
    });

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let result = match source {
      MechSourceCode::String(source) => {
        let tree = mech_syntax::parser::parse(source.trim())?;
        self.run_tree_on_program(context, program, &tree)
      }
      other => program.run_source(other),
    };

    ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      slot.replace(previous_target);
    });

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
        RuntimeAddressTarget::Context(binding) => {
          let base_uri = runtime_context_base_uri(&binding);
          if !runtime_context_allows_read(&binding, &reference.name) {
            return Err(MechError::new(
              RuntimeResourceCapabilityDenied {
                context_name: binding.name.clone(),
                operation: "read".to_string(),
                path: reference.name.clone(),
              },
              None,
            ));
          }
          if !self.grants.allows(
            DEFAULT_RESOURCE_SUBJECT,
            &base_uri,
            &RuntimeCapabilityOperation::Read,
            &reference.name,
          ) {
            return Err(MechError::new(
              RuntimeCapabilityGrantDenied {
                subject: DEFAULT_RESOURCE_SUBJECT.to_string(),
                resource: base_uri,
                operation: RuntimeCapabilityOperation::Read,
                path: reference.name.clone(),
              },
              None,
            ));
          }
          let value = self.resources.read(RuntimeResourceReadRequest {
            base_uri,
            path: reference.name.clone(),
            context_name: binding.name.clone(),
          })?;
          bindings.insert(
            address_binding_key(&reference.target, &reference.name),
            Ref::new(value),
          );
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
