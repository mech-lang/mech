// ---------------------------------------------------------------------------
// Program execution
// ---------------------------------------------------------------------------

// These are the main methods responsible for executing Mech programs within the runtime. They handle the orchestration of program execution, including setting up the execution context, managing module imports and dependencies, emitting events for diagnostics, and ensuring that execution adheres to the runtime's limits and policies.

// There are two main entry points for execution:

// - `run_string`: Executes a string of Mech source code directly. This is for lightweight execution of ad-hoc code snippets, scripts, documents, configuration files, etc.
// - `run_module`: Executes a module by its version ID, handling the resolution of dependencies and the construction of the import environment. This is for executing more complex, modular code that depends on other modules and is part of the larger program structure.

// Both methods have corresponding _with_context versions that accept a mutable reference to a RuntimeContext, allowing for execution within the context of an active transaction. This ensures that any changes made during execution are properly staged within the transaction that if an error occurs, the transaction can be rolled back to maintain consistency.

use super::*;

const DEFAULT_RESOURCE_SUBJECT: &str = "task://main";


#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeAddressTarget {
  Interpreter(SourceScope),
  Context(RuntimeContextBinding),
  Unknown,
}

fn resolve_runtime_address_target(
  record: &ModuleVersionRecord,
  _scope: &SourceScope,
  context_registry: &RuntimeContextRegistry,
  target: &str,
) -> RuntimeAddressTarget {
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

impl MechRuntime {

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

    let result = program.run_string(source);

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

    let result = program.run_tree(tree);

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
    self.program.interpreter().out.to_string()
  }

  pub fn has_interpreter(&self, interpreter_id: u64) -> bool {
    with_interpreter(self.program.interpreter(), interpreter_id, &mut |_| ()).is_some()
  }

  pub fn output_value_for_interpreter(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<Value> {
    with_interpreter(self.program.interpreter(), interpreter_id, &mut |interpreter| {
      interpreter.out_values.borrow().get(&output_id).cloned()
    })
    .flatten()
  }

  pub fn symbol_name_for_interpreter_output(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<String> {
    with_interpreter(self.program.interpreter(), interpreter_id, &mut |interpreter| {
      interpreter.symbols().borrow().get_symbol_name_by_id(output_id)
    })
    .flatten()
  }

  pub fn symbol_values_for_interpreter(
    &self,
    interpreter_id: u64,
    names: &[String],
  ) -> Option<Vec<(String, Value)>> {
    with_interpreter(self.program.interpreter(), interpreter_id, &mut |interpreter| {
      let symbols = interpreter.symbols();
      let symbols_brrw = symbols.borrow();
      symbol_rows(&symbols_brrw, names)
    })
  }

  pub fn bind_ans_for_interpreter(
    &mut self,
    interpreter_id: u64,
    value: &Value,
  ) -> MResult<()> {
    if bind_ans_recursive(self.program.interpreter_mut(), interpreter_id, value) {
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
    self.program.interpreter_mut().step(count as usize, 1)?;
    Ok(())
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
        let stripped = strip_module_declarations_for_execution(source);
        program.run_source(&MechSourceCode::String(stripped))
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
            format!("{}@{}", reference.name, reference.target),
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
        bindings.insert(format!("{}@{}", reference.name, reference.target), export_value.clone());
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
          let Some(export_value) = dependency_instance.exports.get(name) else {
            return Err(MechError::new(RuntimeModuleExportNotFound { dependency: import.specifier.clone(), export: name.clone() }, None));
          };
          if let Some(first) = ownership.insert(name.clone(), import.specifier.clone()) {
            return Err(MechError::new(RuntimeModuleImportConflict { binding: name.clone(), first_import: first, second_import: import.specifier.clone() }, None));
          }
          bindings.insert(name.clone(), export_value.clone());
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


fn with_interpreter<T>(
  interpreter: &mech_interpreter::Interpreter,
  interpreter_id: u64,
  f: &mut impl FnMut(&mech_interpreter::Interpreter) -> T,
) -> Option<T> {
  if interpreter_id == 0 || interpreter.id == interpreter_id {
    return Some(f(interpreter));
  }

  let sub_interpreters = interpreter.sub_interpreters.borrow();
  for sub_interpreter in sub_interpreters.values() {
    if let Some(result) = with_interpreter(sub_interpreter.as_ref(), interpreter_id, f) {
      return Some(result);
    }
  }

  None
}

fn bind_ans_recursive(
  interpreter: &mut mech_interpreter::Interpreter,
  interpreter_id: u64,
  value: &Value,
) -> bool {
  if interpreter_id == 0 || interpreter.id == interpreter_id {
    bind_ans_on_interpreter(interpreter, value);
    return true;
  }

  let child_ids = {
    let sub_interpreters = interpreter.sub_interpreters.borrow();
    sub_interpreters.keys().copied().collect::<Vec<_>>()
  };

  for child_id in child_ids {
    let mut sub_interpreters = interpreter.sub_interpreters.borrow_mut();
    let Some(child) = sub_interpreters.get_mut(&child_id) else {
      continue;
    };
    if bind_ans_recursive(child.as_mut(), interpreter_id, value) {
      return true;
    }
  }

  false
}

fn bind_ans_on_interpreter(
  interpreter: &mut mech_interpreter::Interpreter,
  value: &Value,
) {
  let resolved_value = match value {
    Value::MutableReference(reference) => reference.borrow().clone(),
    _ => value.clone(),
  };
  let ans_id = hash_str("ans");
  let symbols = interpreter.symbols();
  let mut symbols_brrw = symbols.borrow_mut();
  symbols_brrw.insert(ans_id, resolved_value, false);
  symbols_brrw.dictionary.borrow_mut().insert(ans_id, "ans".to_string());
  interpreter.dictionary().borrow_mut().insert(ans_id, "ans".to_string());
}

fn symbol_rows(symbol_table: &mech_core::SymbolTable, names: &[String]) -> Vec<(String, Value)> {
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

pub fn strip_module_declarations_for_execution(source: &str) -> String {
  source
    .lines()
    .filter(|line| {
      let trimmed = line.trim_start();
      !(trimmed.starts_with("+>") || trimmed.starts_with("<+") || trimmed.starts_with("@"))
    })
    .collect::<Vec<_>>()
    .join("\n")
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
  fn runtime_has_interpreter_finds_nested_interpreter() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let nested_id = 4242;
    let child_id = 2424;
    let mut child = runtime.program().interpreter().clone();
    child.clear();
    child.id = child_id;
    let mut nested = runtime.program().interpreter().clone();
    nested.clear();
    nested.id = nested_id;
    child
      .sub_interpreters
      .borrow_mut()
      .insert(nested_id, Box::new(nested));
    runtime
      .program_mut()
      .interpreter_mut()
      .sub_interpreters
      .borrow_mut()
      .insert(child_id, Box::new(child));

    assert!(runtime.has_interpreter(nested_id));
  }

  #[test]
  fn runtime_output_value_for_interpreter_finds_nested_interpreter() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let nested_id = 4242;
    let child_id = 2424;
    let output_id = 101;
    let mut child = runtime.program().interpreter().clone();
    child.clear();
    child.id = child_id;
    let mut nested = runtime.program().interpreter().clone();
    nested.clear();
    nested.id = nested_id;
    nested
      .out_values
      .borrow_mut()
      .insert(output_id, Value::U64(Ref::new(42)));
    child
      .sub_interpreters
      .borrow_mut()
      .insert(nested_id, Box::new(nested));
    runtime
      .program_mut()
      .interpreter_mut()
      .sub_interpreters
      .borrow_mut()
      .insert(child_id, Box::new(child));

    assert!(runtime.output_value_for_interpreter(nested_id, output_id).is_some());
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
