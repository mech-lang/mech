use super::{
  identifier_from_str,
  resolve_runtime_value,
  single_code_program,
  RuntimeProgramTarget,
};
use crate::event::RuntimeEventKind;
use crate::resolver::SourceScope;
use crate::runtime::{
  MechRuntime,
  RuntimeInvalidOperationError,
  RuntimeProgramBusy,
};
use crate::{
  ResourceBudgetExceededError,
  RuntimeContext,
  RuntimeValueSnapshot,
};
use mech_core::{
  hash_str,
  MResult,
  MechError,
  MechSourceCode,
  ValRef,
  Value,
};
use mech_program::MechProgram;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;

impl MechRuntime {
  pub(super) fn run_tree_on_program(
    &mut self,
    context: &mut RuntimeContext,
    target: &mut RuntimeProgramTarget<'_>,
    tree: &mech_core::Program,
    scope_hint: Option<&SourceScope>,
  ) -> MResult<Value> {
    let direct_document_run = scope_hint.is_none();
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
                target,
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
                target,
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
          mech_core::SectionElement::FencedMechCode(fenced) if direct_document_run => {
            pending.push(mech_core::SectionElement::FencedMechCode(fenced.clone()));
          }
          _ => {}
        }
      }
    }

    self.flush_direct_execution(
      context,
      target,
      &mut pending,
      &mut result,
    )?;
    Ok(result)
  }

  pub(super) fn evaluate_expression_on_program(
    &mut self,
    context: &mut RuntimeContext,
    target: &mut RuntimeProgramTarget<'_>,
    expression: &mech_core::Expression,
  ) -> MResult<Value> {
    let single = single_code_program(mech_core::MechCode::Expression(expression.clone()), None);
    self
      .execute_program_target_tree(context, target, &single)
      .map(resolve_runtime_value)
  }

  pub(super) fn bind_persistent_send_value_on_program(
    &mut self,
    context: &mut RuntimeContext,
    target: &mut RuntimeProgramTarget<'_>,
    expression: mech_core::Expression,
  ) -> MResult<ValRef> {
    let name = format!("mech-internal-persistent-send-{}", self.persistent_sends.len());
    let id = hash_str(&name);
    let var_def = mech_core::VariableDefine {
      mutable: false,
      var: mech_core::Var {
        name: identifier_from_str(&name),
        context: None,
        kind: None,
      },
      expression,
    };
    let single = single_code_program(
      mech_core::MechCode::Statement(mech_core::Statement::VariableDefine(var_def)),
      None,
    );
    self.execute_program_target_tree(
      context,
      target,
      &single,
    )?;
    self
      .program_target_ref(target)
      .interpreter()
      .symbols()
      .borrow()
      .get(id)
      .ok_or_else(|| MechError::new(RuntimeInvalidOperationError {
        operation: "persistent_context_send",
        reason: "failed to bind persistent send expression to an output cell".to_string(),
      }, None))
  }

  pub fn run_string(
    &mut self,
    source: &str,
  ) -> MResult<RuntimeValueSnapshot> {
    let mut context = self.runtime_context()?;
    self.run_string_with_context(&mut context, source)
  }

  pub fn run_string_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<RuntimeValueSnapshot> {
    self.run_string_with_context_map(
      context,
      source,
      |value| RuntimeValueSnapshot::try_capture(&value),
    )
  }

  pub(crate) fn run_string_value_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<Value> {
    self.run_string_with_context_map(
      context,
      source,
      Ok,
    )
  }

  fn run_string_with_context_map<T>(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
    finish: impl FnOnce(Value) -> MResult<T>,
  ) -> MResult<T> {
    let turn_started = Instant::now();
    let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
    let result = self.with_atomic_program_operation(
      context,
      "run_string_with_context",
      |runtime, context| {
        let source_bytes = u64::try_from(source.as_bytes().len()).map_err(|_| {
          MechError::new(
            ResourceBudgetExceededError {
              resource: "source_bytes",
              used: u64::MAX,
              requested: 1,
              max: None,
            },
            None,
          )
        })?;
        runtime.enforce_source_byte_count(context, source_bytes)?;
        let value = runtime.run_string_operation(
          context,
          source,
          turn_started,
        )?;
        finish(value)
      },
    );
    if let Err(error) = &result {
      self.emit_program_failure_audit(
        context,
        error,
        profile_started,
      );
    }
    result
  }

  fn run_string_operation(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
    turn_started: Instant,
  ) -> MResult<Value> {
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let result = match mech_syntax::parser::parse(source.trim()) {
      Ok(tree) => match self.preflight_context_capabilities(context, &tree, &SourceScope::Program) {
        Ok(()) => {
          self.register_retained_program_host_functions(context)?;
          self.run_tree_on_program(
            context,
            &mut RuntimeProgramTarget::Retained,
            &tree,
            None,
          )
        }
        Err(error) => Err(error),
      },
      Err(error) => Err(error),
    };

    let result = result.and_then(|value| { self.enforce_turn_duration(turn_started)?; Ok(value) });
    if result.is_ok() {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::ProgramCompleted {
          task_id: context.task,
        },
      )?;
      if let Some(started) = profile_started {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramProfiled {
            task_id: context.task,
            duration_ns: started.elapsed().as_nanos(),
          },
        )?;
      }
    }

    result
  }

  fn emit_program_failure_audit(
    &mut self,
    context: &mut RuntimeContext,
    error: &MechError,
    profile_started: Option<Instant>,
  ) {
    let _ = self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramFailed {
        task_id: context.task,
        message: format!("{:?}", error),
      },
    );
    if let Some(started) = profile_started {
      let _ = self.emit_event_to_context(
        context,
        RuntimeEventKind::ProgramProfiled {
          task_id: context.task,
          duration_ns: started.elapsed().as_nanos(),
        },
      );
    }
  }

  pub fn run_bytecode_with_context(
    &mut self,
    context: &mut RuntimeContext,
    bytecode: &[u8],
  ) -> MResult<RuntimeValueSnapshot> {
    self.run_bytecode_with_context_map(
      context,
      bytecode,
      |value| RuntimeValueSnapshot::try_capture(&value),
    )
  }

  fn run_bytecode_value_with_context(
    &mut self,
    context: &mut RuntimeContext,
    bytecode: &[u8],
  ) -> MResult<Value> {
    self.run_bytecode_with_context_map(
      context,
      bytecode,
      Ok,
    )
  }

  fn run_bytecode_with_context_map<T>(
    &mut self,
    context: &mut RuntimeContext,
    bytecode: &[u8],
    finish: impl FnOnce(Value) -> MResult<T>,
  ) -> MResult<T> {
    let turn_started = Instant::now();
    self.validate_context_for_runtime(context)?;
    let source_bytes = u64::try_from(bytecode.len()).map_err(|_| {
      MechError::new(
        ResourceBudgetExceededError {
          resource: "source_bytes",
          used: u64::MAX,
          requested: 1,
          max: None,
        },
        None,
      )
    })?;
    self.enforce_source_byte_count(context, source_bytes)?;
    self.run_bytecode_with_context_inner_map(
      context,
      bytecode,
      turn_started,
      finish,
    )
  }

  fn run_bytecode_with_context_inner_map<T>(
    &mut self,
    context: &mut RuntimeContext,
    bytecode: &[u8],
    turn_started: Instant,
    finish: impl FnOnce(Value) -> MResult<T>,
  ) -> MResult<T> {
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let mut bytecode_program =
      MechProgram::new(self.program.config.clone());

    let live_state_before = self.live_state_snapshot();
    let result = (|| {
      self.register_runtime_program_host_functions(
        context,
        &mut bytecode_program,
      )?;

      bytecode_program.run_bytecode(bytecode)
    })();

    let result = result.and_then(|value| {
      self.enforce_turn_duration(turn_started)?;
      finish(value)
    });

    // Runtime bytecode execution is one-shot. Direct MechProgram
    // bytecode loading is the persistent installation path.
    self.restore_live_state(live_state_before);
    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
        if let Some(started) = profile_started {
          self.emit_event_to_context(
            context,
            RuntimeEventKind::ProgramProfiled {
              task_id: context.task,
              duration_ns: started.elapsed().as_nanos(),
            },
          )?;
        }
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
        if let Some(started) = profile_started {
          self.emit_event_to_context(
            context,
            RuntimeEventKind::ProgramProfiled {
              task_id: context.task,
              duration_ns: started.elapsed().as_nanos(),
            },
          )?;
        }
      }
    }

    result
  }

  pub fn run_source_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &MechSourceCode,
  ) -> MResult<RuntimeValueSnapshot> {
    self.run_source_with_context_map(
      context,
      source,
      |value| RuntimeValueSnapshot::try_capture(&value),
    )
  }

  pub fn run_source(
    &mut self,
    source: &MechSourceCode,
  ) -> MResult<RuntimeValueSnapshot> {
    let mut context = self.runtime_context()?;
    self.run_source_with_context(&mut context, source)
  }

  pub(crate) fn run_source_value_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &MechSourceCode,
  ) -> MResult<Value> {
    self.run_source_with_context_map(
      context,
      source,
      Ok,
    )
  }

  fn run_source_with_context_map<T>(
    &mut self,
    context: &mut RuntimeContext,
    source: &MechSourceCode,
    finish: impl FnOnce(Value) -> MResult<T>,
  ) -> MResult<T> {
    let turn_started = Instant::now();
    if let MechSourceCode::ByteCode(bytes) = source {
      return self.run_bytecode_with_context_map(
        context,
        bytes,
        finish,
      );
    }

    let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
    let result = self.with_atomic_program_operation(
      context,
      "run_source_with_context",
      |runtime, context| {
        runtime.enforce_source_limits(context, source)?;
        let value = runtime.run_source_operation(
          context,
          source,
          turn_started,
        )?;
        finish(value)
      },
    );
    if let Err(error) = &result {
      self.emit_program_failure_audit(
        context,
        error,
        profile_started,
      );
    }
    result
  }

  fn run_source_operation(
    &mut self,
    context: &mut RuntimeContext,
    source: &MechSourceCode,
    turn_started: Instant,
  ) -> MResult<Value> {
    match source {
      MechSourceCode::String(source) => self.run_string_operation(context, source, turn_started),
      MechSourceCode::Tree(tree) => self.run_tree_operation(context, tree, turn_started),
      MechSourceCode::ByteCode(bytes) => {
        self.run_bytecode_with_context_inner_map(
          context,
          bytes,
          turn_started,
          Ok,
        )
      }
      MechSourceCode::Program(sources) => {
        let mut value = Value::Empty;
        for source in sources {
          value = self.run_source_operation(context, source, turn_started)?;
        }
        Ok(value)
      }
      unsupported => Err(MechError::new(RuntimeInvalidOperationError { operation: "run_source", reason: format!("unsupported program source: {:?}", unsupported) }, None)),
    }
  }

  pub fn run_tree(
    &mut self,
    tree: &mech_core::Program,
  ) -> MResult<RuntimeValueSnapshot> {
    let mut context = self.runtime_context()?;
    self.run_tree_with_context(&mut context, tree)
  }

  pub fn run_tree_with_context(
    &mut self,
    context: &mut RuntimeContext,
    tree: &mech_core::Program,
  ) -> MResult<RuntimeValueSnapshot> {
    self.run_tree_with_context_map(
      context,
      tree,
      |value| RuntimeValueSnapshot::try_capture(&value),
    )
  }

  pub(crate) fn run_tree_value_with_context(
    &mut self,
    context: &mut RuntimeContext,
    tree: &mech_core::Program,
  ) -> MResult<Value> {
    self.run_tree_with_context_map(
      context,
      tree,
      Ok,
    )
  }

  fn run_tree_with_context_map<T>(
    &mut self,
    context: &mut RuntimeContext,
    tree: &mech_core::Program,
    finish: impl FnOnce(Value) -> MResult<T>,
  ) -> MResult<T> {
    let turn_started = Instant::now();
    let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
    let result = self.with_atomic_program_operation(
      context,
      "run_tree_with_context",
      |runtime, context| {
        let value = runtime.run_tree_operation(
          context,
          tree,
          turn_started,
        )?;
        finish(value)
      },
    );
    if let Err(error) = &result {
      self.emit_program_failure_audit(
        context,
        error,
        profile_started,
      );
    }
    result
  }

  fn run_tree_operation(
    &mut self,
    context: &mut RuntimeContext,
    tree: &mech_core::Program,
    turn_started: Instant,
  ) -> MResult<Value> {
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let result = match self.preflight_context_capabilities(context, tree, &SourceScope::Program) {
      Ok(()) => {
        self.register_retained_program_host_functions(context)?;
        self.run_tree_on_program(
          context,
          &mut RuntimeProgramTarget::Retained,
          tree,
          None,
        )
      }
      Err(error) => Err(error),
    };

    let result = result.and_then(|value| { self.enforce_turn_duration(turn_started)?; Ok(value) });
    if result.is_ok() {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::ProgramCompleted {
          task_id: context.task,
        },
      )?;
      if let Some(started) = profile_started {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramProfiled {
            task_id: context.task,
            duration_ns: started.elapsed().as_nanos(),
          },
        )?;
      }
    }

    result
  }

  #[cfg(feature = "compiler")]
  pub fn compile_program_bytecode(&mut self) -> MResult<Vec<u8>> {
    self.ensure_runtime_mutation_allowed("compile_program_bytecode")?;
    self.reject_program_operation_reentrancy("compile_program_bytecode")?;
    if let Some(transaction_id) = self.program_transaction_owner {
      return Err(MechError::new(RuntimeProgramBusy {
        operation: "compile_program_bytecode",
        owner: transaction_id,
        requester: None,
      }, None));
    }
    self.program.compile_bytecode()
  }
}
