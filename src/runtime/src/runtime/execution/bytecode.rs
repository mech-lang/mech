use crate::event::RuntimeEventKind;
use crate::runtime::MechRuntime;
use crate::runtime::live_state::LiveRegistrationMode;
use crate::{ResourceBudgetExceededError, RuntimeContext, RuntimeValueSnapshot};
use mech_core::{LegacyValue, MResult, MechError};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

impl MechRuntime {
    pub fn evaluate_bytecode_once_with_context(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
    ) -> MResult<RuntimeValueSnapshot> {
        self.evaluate_bytecode_once_with_context_map(context, bytecode, |value| {
            RuntimeValueSnapshot::try_capture(&value)
        })
    }

    pub(super) fn evaluate_bytecode_once_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
    ) -> MResult<T> {
        let turn_started = Instant::now();
        let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
        let result = self.with_atomic_program_operation(
            context,
            "evaluate_bytecode_once_with_context",
            |runtime, context| {
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
                runtime.enforce_source_byte_count(context, source_bytes)?;
                runtime.evaluate_bytecode_once_with_context_inner_map(
                    context,
                    bytecode,
                    turn_started,
                    finish,
                )
            },
        );
        if let Err(error) = &result {
            self.emit_program_failure_audit(context, error, profile_started);
        }
        result
    }

    pub(super) fn evaluate_bytecode_once_with_context_inner_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
        turn_started: Instant,
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
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

        let mut bytecode_program = self.new_program(self.program.config.clone());

        let live_state_before = self.live_state_snapshot();
        let result =
            self.with_live_registration_mode(LiveRegistrationMode::IsolatedSnapshot, |runtime| {
                runtime.register_runtime_program_host_functions(context, &mut bytecode_program)?;
                runtime.with_isolated_program_execution_session(
                    context,
                    &mut bytecode_program,
                    |program, services| program.run_bytecode_with_services(bytecode, services),
                )
            });

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

    /// Installs bytecode as the retained root program.
    ///
    /// This API is available with runtime support alone; source parsing is not
    /// required.
    ///
    /// ```
    /// use mech_runtime::MechRuntime;
    ///
    /// let mut runtime = MechRuntime::builder().build().unwrap();
    /// let mut context = runtime.runtime_context().unwrap();
    /// assert!(runtime
    ///     .install_bytecode_with_context(&mut context, b"not-bytecode")
    ///     .is_err());
    /// ```
    pub fn install_bytecode_with_context(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
    ) -> MResult<RuntimeValueSnapshot> {
        let turn_started = Instant::now();
        let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
        let result = self.with_atomic_program_operation(
            context,
            "install_bytecode_with_context",
            |runtime, context| {
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
                runtime.enforce_source_byte_count(context, source_bytes)?;
                context.charge_step()?;
                runtime.emit_event_to_context(
                    context,
                    RuntimeEventKind::ProgramStarted {
                        task_id: context.task,
                    },
                )?;

                // A retained bytecode install replaces the root plan. Its live
                // bindings must therefore replace, rather than append to, the
                // targets retained by the previous root plan. The surrounding
                // atomic operation restores both collections on every failure.
                runtime.live_input_bindings.clear();
                runtime.live_context_template = None;
                let mut replacement = runtime.new_program(runtime.program.config.clone());
                runtime.register_runtime_program_host_functions(context, &mut replacement)?;
                let value = runtime.with_isolated_program_execution_session(
                    context,
                    &mut replacement,
                    |program, services| program.run_bytecode_with_services(bytecode, services),
                )?;
                runtime.enforce_turn_duration(turn_started)?;
                if runtime.has_live_input_bindings() {
                    runtime.commit_live_context_candidate(context);
                }
                let snapshot = RuntimeValueSnapshot::try_capture(&value)?;
                let transaction_id = Self::context_transaction_id(context)?;
                runtime.replace_retained_program(transaction_id, replacement)?;

                runtime.emit_event_to_context(
                    context,
                    RuntimeEventKind::ProgramCompleted {
                        task_id: context.task,
                    },
                )?;
                if let Some(started) = profile_started {
                    runtime.emit_event_to_context(
                        context,
                        RuntimeEventKind::ProgramProfiled {
                            task_id: context.task,
                            duration_ns: started.elapsed().as_nanos(),
                        },
                    )?;
                }
                #[cfg(feature = "resident-routing")]
                runtime.stage_legacy_program_owner(context)?;
                Ok(snapshot)
            },
        );
        if let Err(error) = &result {
            self.emit_program_failure_audit(context, error, profile_started);
        }
        result
    }
}
