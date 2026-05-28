// ---------------------------------------------------------------------------
// Program execution
// ---------------------------------------------------------------------------

// These are the main methods responsible for executing Mech programs within the runtime. They handle the orchestration of program execution, including setting up the execution context, managing module imports and dependencies, emitting events for diagnostics, and ensuring that execution adheres to the runtime's limits and policies.

// There are two main entry points for execution:

// - `run_string`: Executes a string of Mech source code directly. This is for lightweight execution of ad-hoc code snippets, scripts, documents, configuration files, etc.
// - `run_module`: Executes a module by its version ID, handling the resolution of dependencies and the construction of the import environment. This is for executing more complex, modular code that depends on other modules and is part of the larger program structure.

// Both methods have corresponding _with_context versions that accept a mutable reference to a RuntimeContext, allowing for execution within the context of an active transaction. This ensures that any changes made during execution are properly staged within the transaction that if an error occurs, the transaction can be rolled back to maintain consistency.

use super::*;

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

  pub fn run_module(&mut self, version: ModuleVersionId) -> MResult<Value> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_with_context(&mut context, version)
  }

  pub fn run_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
  ) -> MResult<Value> {
    let mut seen = HashSet::new();
    let mut module_instances = HashMap::new();
    let instance = self.execute_module_isolated(context, version, &mut seen, &mut module_instances)?;
    Ok(instance.result)
  }

  fn execute_module_isolated(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
    seen: &mut HashSet<ModuleVersionId>,
    module_instances: &mut HashMap<ModuleVersionId, ModuleInstance>,
  ) -> MResult<ModuleInstance> {
    context.validate()?;
    context.charge_step()?;

    if let Some(instance) = module_instances.get(&version).cloned() {
      return Ok(instance);
    }

    if seen.contains(&version) {
      return Ok(ModuleInstance { version, exports: HashMap::new(), result: Value::Empty });
    }
    seen.insert(version);

    let Some(record) = self.store.get_module_version(version)? else {
      return Err(MechError::new(RuntimeRecordNotFoundError { record_type: "module_version", id: version.to_string() }, None));
    };
    validate_module_import_edges(&record)?;
    let Some(source) = record.source.clone() else {
      return Err(MechError::new(RuntimeInvalidOperationError { operation: "run_module", reason: "module version has no source".to_string() }, None));
    };

    for edge in &record.import_edges {
      self.execute_module_isolated(context, edge.dependency, seen, module_instances)?;
    }

    let import_environment = self.build_import_environment_for_scope(
      context,
      &record,
      &SourceScope::Program,
      module_instances,
    )?;
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
    let result = self.run_module_source_on_program(context, &mut module_program, &source);
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
      for export in &record.exports {
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
    module_instances.insert(version, instance.clone());
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

  fn build_import_environment_for_scope(
    &mut self,
    context: &mut RuntimeContext,
    importer: &ModuleVersionRecord,
    scope: &SourceScope,
    module_instances: &HashMap<ModuleVersionId, ModuleInstance>,
  ) -> MResult<HashMap<String, mech_core::ValRef>> {
    let mut bindings = HashMap::new();
    let mut ownership: HashMap<String, String> = HashMap::new();

    for edge in &importer.import_edges {
      if &edge.scope != scope {
        continue;
      }

      let import = &edge.import;
      let dependency = edge.dependency;

      let Some(dependency_instance) = module_instances.get(&dependency) else {
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

pub fn strip_module_declarations_for_execution(source: &str) -> String {
  source
    .lines()
    .filter(|line| {
      let trimmed = line.trim_start();
      !(trimmed.starts_with("+>") || trimmed.starts_with("<+"))
    })
    .collect::<Vec<_>>()
    .join("\n")
}