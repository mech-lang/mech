use super::{
  context_registry_for_scope,
  execution_scope_for_extracted_module_source,
  exports_for_scope,
  materialize_function_imports_for_scope,
  merge_module_environment,
  module_source_for_scope,
  resolve_runtime_address_target,
  AddressedReadPreflight,
  PreparedModuleScopeExecution,
  ProgramEnvironmentOverlay,
  RuntimeAddressTarget,
  RuntimeProgramTarget,
};
use crate::context::RuntimeContextRegistry;
use crate::event::RuntimeEventKind;
use crate::id::ModuleVersionId;
use crate::resolver::{
  module_namespace_for_import,
  SourceAddressReference,
  SourceImportKind,
  SourceScope,
};
use crate::runtime::{
  MechRuntime,
  ModuleInstance,
  RuntimeInvalidOperationError,
  RuntimeModuleExportNotFound,
  RuntimeModuleImportConflict,
  RuntimeRecordNotFoundError,
  UnknownAddressTarget,
  validate_module_import_edges,
};
use crate::store::ModuleVersionRecord;
use crate::{
  RuntimeContext,
  RuntimeModuleResult,
};
use mech_core::{
  hash_str,
  MResult,
  MechError,
  MechSourceCode,
  Value,
};
use mech_program::{
  MechProgram,
  MechProgramConfig,
  MechProgramEnvironment,
};
use std::collections::{
  HashMap,
  HashSet,
};
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;

impl MechRuntime {
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

  pub(in crate::runtime) fn preflight_module_graph_for_scope(
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

  pub(in crate::runtime) fn execute_module_retained_root_for_scope(
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
