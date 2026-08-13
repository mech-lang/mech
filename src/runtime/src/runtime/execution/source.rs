use super::RuntimeProgramTarget;
use crate::event::RuntimeEventKind;
use crate::resolver::SourceScope;
#[cfg(feature = "compiler")]
use crate::runtime::RuntimeProgramBusy;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError};
use crate::{ResourceBudgetExceededError, RuntimeContext, RuntimeValueSnapshot};
use mech_core::{LegacyValue, MResult, MechError, MechSourceCode};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

impl MechRuntime {
    pub(in crate::runtime) fn run_tree_on_program(
        &mut self,
        context: &mut RuntimeContext,
        target: &mut RuntimeProgramTarget<'_>,
        tree: &mech_core::Program,
        scope_hint: Option<&SourceScope>,
        render_document_presentation: bool,
    ) -> MResult<LegacyValue> {
        let direct_document_run = render_document_presentation;
        let execution_scope = scope_hint.unwrap_or(&SourceScope::Program);
        let skip_non_context_imports = scope_hint.is_some();
        let registry = self.direct_context_registry_for_scope(tree, execution_scope)?;
        let mut result = LegacyValue::Empty;
        let mut pending = Vec::new();
        let mut document_inline_nodes = Vec::new();

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
                    // Evaluate these presentation nodes after direct document code. This
                    // preserves the source operation's result while making inline Mech
                    // output available to document renderers once declarations are ready.
                    inline @ (mech_core::SectionElement::Comment(_)
                    | mech_core::SectionElement::FigureTable(_)
                    | mech_core::SectionElement::Paragraph(_)
                    | mech_core::SectionElement::Table(_))
                        if direct_document_run =>
                    {
                        document_inline_nodes.push(inline.clone());
                    }
                    _ => {}
                }
            }
        }

        self.flush_direct_execution(context, target, &mut pending, &mut result)?;
        // Inline presentation evaluation produces output records, not the source
        // operation result. Keep it separate so trailing prose cannot replace the
        // final value from executable Mech source.
        let mut inline_result = LegacyValue::Empty;
        self.flush_direct_execution(
            context,
            target,
            &mut document_inline_nodes,
            &mut inline_result,
        )?;
        Ok(result)
    }

    pub fn run_string(&mut self, source: &str) -> MResult<RuntimeValueSnapshot> {
        let mut context = self.runtime_context()?;
        self.run_string_with_context(&mut context, source)
    }

    pub fn run_string_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
    ) -> MResult<RuntimeValueSnapshot> {
        self.run_string_with_context_map(context, source, |value| {
            RuntimeValueSnapshot::try_capture(&value)
        })
    }

    pub(crate) fn run_string_value_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
    ) -> MResult<LegacyValue> {
        self.run_string_with_context_map(context, source, Ok)
    }

    fn run_string_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
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
                let value = runtime.run_string_operation(context, source, turn_started)?;
                finish(value)
            },
        );
        if let Err(error) = &result {
            self.emit_program_failure_audit(context, error, profile_started);
        }
        result
    }

    fn run_string_operation(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
        turn_started: Instant,
    ) -> MResult<LegacyValue> {
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
            Ok(tree) => {
                match self.preflight_context_capabilities(context, &tree, &SourceScope::Program) {
                    Ok(()) => {
                        self.register_retained_program_host_functions(context)?;
                        self.run_tree_on_program(
                            context,
                            &mut RuntimeProgramTarget::Retained,
                            &tree,
                            None,
                            true,
                        )
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };

        let result = result.and_then(|value| {
            self.enforce_turn_duration(turn_started)?;
            if self.has_live_input_bindings() {
                self.validate_live_context_candidate(context)?;
                self.commit_live_context_candidate(context);
            }
            Ok(value)
        });
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

    pub fn run_source_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
    ) -> MResult<RuntimeValueSnapshot> {
        self.run_source_with_context_map(context, source, |value| {
            RuntimeValueSnapshot::try_capture(&value)
        })
    }

    pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<RuntimeValueSnapshot> {
        let mut context = self.runtime_context()?;
        self.run_source_with_context(&mut context, source)
    }

    pub(crate) fn run_source_value_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
    ) -> MResult<LegacyValue> {
        self.run_source_with_context_map(context, source, Ok)
    }

    fn run_source_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
    ) -> MResult<T> {
        let turn_started = Instant::now();
        if let MechSourceCode::ByteCode(bytes) = source {
            return self.evaluate_bytecode_once_with_context_map(context, bytes, finish);
        }

        let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
        let result = self.with_atomic_program_operation(
            context,
            "run_source_with_context",
            |runtime, context| {
                runtime.enforce_source_limits(context, source)?;
                let value = runtime.run_source_operation(context, source, turn_started)?;
                finish(value)
            },
        );
        if let Err(error) = &result {
            self.emit_program_failure_audit(context, error, profile_started);
        }
        result
    }

    fn run_source_operation(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
        turn_started: Instant,
    ) -> MResult<LegacyValue> {
        match source {
            MechSourceCode::String(source) => {
                self.run_string_operation(context, source, turn_started)
            }
            MechSourceCode::Tree(tree) => self.run_tree_operation(context, tree, turn_started),
            MechSourceCode::ByteCode(bytes) => {
                self.evaluate_bytecode_once_with_context_inner_map(context, bytes, turn_started, Ok)
            }
            MechSourceCode::Program(sources) => {
                let mut value = LegacyValue::Empty;
                for source in sources {
                    value = self.run_source_operation(context, source, turn_started)?;
                }
                Ok(value)
            }
            unsupported => Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "run_source",
                    reason: format!("unsupported program source: {:?}", unsupported),
                },
                None,
            )),
        }
    }

    pub fn run_tree(&mut self, tree: &mech_core::Program) -> MResult<RuntimeValueSnapshot> {
        let mut context = self.runtime_context()?;
        self.run_tree_with_context(&mut context, tree)
    }

    pub fn run_tree_with_context(
        &mut self,
        context: &mut RuntimeContext,
        tree: &mech_core::Program,
    ) -> MResult<RuntimeValueSnapshot> {
        self.run_tree_with_context_map(context, tree, |value| {
            RuntimeValueSnapshot::try_capture(&value)
        })
    }

    pub(crate) fn run_tree_value_with_context(
        &mut self,
        context: &mut RuntimeContext,
        tree: &mech_core::Program,
    ) -> MResult<LegacyValue> {
        self.run_tree_with_context_map(context, tree, Ok)
    }

    fn run_tree_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        tree: &mech_core::Program,
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
    ) -> MResult<T> {
        let turn_started = Instant::now();
        let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);
        let result = self.with_atomic_program_operation(
            context,
            "run_tree_with_context",
            |runtime, context| {
                let value = runtime.run_tree_operation(context, tree, turn_started)?;
                finish(value)
            },
        );
        if let Err(error) = &result {
            self.emit_program_failure_audit(context, error, profile_started);
        }
        result
    }

    fn run_tree_operation(
        &mut self,
        context: &mut RuntimeContext,
        tree: &mech_core::Program,
        turn_started: Instant,
    ) -> MResult<LegacyValue> {
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        let profile_started = self.config.diagnostics.profile_enabled.then(Instant::now);

        self.emit_event_to_context(
            context,
            RuntimeEventKind::ProgramStarted {
                task_id: context.task,
            },
        )?;

        let result = match self.preflight_context_capabilities(context, tree, &SourceScope::Program)
        {
            Ok(()) => {
                self.register_retained_program_host_functions(context)?;
                self.run_tree_on_program(
                    context,
                    &mut RuntimeProgramTarget::Retained,
                    tree,
                    None,
                    true,
                )
            }
            Err(error) => Err(error),
        };

        let result = result.and_then(|value| {
            self.enforce_turn_duration(turn_started)?;
            if self.has_live_input_bindings() {
                self.validate_live_context_candidate(context)?;
                self.commit_live_context_candidate(context);
            }
            Ok(value)
        });
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
            return Err(MechError::new(
                RuntimeProgramBusy {
                    operation: "compile_program_bytecode",
                    owner: transaction_id,
                    requester: None,
                },
                None,
            ));
        }
        self.program.compile_bytecode()
    }
}
