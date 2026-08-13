use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[cfg(feature = "runtime_bench_probes")]
thread_local! {
    static LAST_REACTIVE_JOURNAL_CELL_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[cfg(feature = "runtime_bench_probes")]
#[doc(hidden)]
pub fn reset_last_reactive_journal_cell_count() {
    LAST_REACTIVE_JOURNAL_CELL_COUNT.with(|count| count.set(0));
}

#[cfg(feature = "runtime_bench_probes")]
#[doc(hidden)]
pub fn take_last_reactive_journal_cell_count() -> usize {
    LAST_REACTIVE_JOURNAL_CELL_COUNT.with(|count| count.replace(0))
}

#[cfg(feature = "runtime_bench_probes")]
fn record_reactive_journal_cell_count(count: usize) {
    LAST_REACTIVE_JOURNAL_CELL_COUNT.with(|slot| slot.set(count));
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown",))]
use web_time::Instant;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown",)))]
use std::time::Instant;

#[cfg(feature = "functions")]
use mech_core::FunctionCatalog;
use mech_core::{
    FunctionSpecializer, LegacyValue, MResult, MechError, MechErrorKind, MechFunction,
    MechSourceCode, ParsedProgram, ReactiveCellId, ReactiveJournalParticipant, ReactiveTurnOutcome,
    ValRef, ValueKind, hash_str, val_ref_reactive_cell_ids, validate_stable_value_update,
    with_reactive_journal_participant,
};

#[cfg(feature = "compiler")]
use mech_bytecode::{CompileCtx, CompiledBytecode, CompiledNodeKind};
#[cfg(all(feature = "compiler", feature = "invariant_define"))]
use mech_core::Register;

use crate::{Interpreter, InterpreterCheckpoint, InterpreterReactiveTurnCheckpoint};
#[cfg(feature = "source")]
use mech_syntax::parser;

#[cfg(all(feature = "source", feature = "native"))]
use crate::ClosureFunctionSpecializer;

pub fn compile_stable_value_update(
    sink: ValRef,
    source: LegacyValue,
) -> MResult<Box<dyn MechFunction>> {
    {
        let current = sink.borrow();
        validate_stable_value_update(&current, &source)?;
    }

    crate::AssignValue {}.specialize(&[LegacyValue::MutableReference(sink), source])
}

pub fn apply_stable_value_update(sink: ValRef, source: LegacyValue) -> MResult<LegacyValue> {
    {
        let current = sink.borrow();
        validate_stable_value_update(&current, &source)?;
    }
    let update =
        crate::AssignValue {}.specialize(&[LegacyValue::MutableReference(sink.clone()), source])?;
    update.solve_result()?;
    Ok(sink.borrow().clone())
}

#[derive(Debug, Clone)]
pub struct MechProgramEnvironment {
    pub trace_enabled: bool,
    pub debug_enabled: bool,
    pub profile_enabled: bool,
    pub rounds_per_step: usize,
}

impl Default for MechProgramEnvironment {
    fn default() -> Self {
        Self {
            trace_enabled: false,
            debug_enabled: false,
            profile_enabled: false,
            rounds_per_step: 10_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MechProgramConfig {
    pub name: String,
    pub environment: MechProgramEnvironment,
}

impl Default for MechProgramConfig {
    fn default() -> Self {
        Self {
            name: "program".into(),
            environment: MechProgramEnvironment::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramInputId {
    pub interpreter_id: u64,
    pub symbol_id: u64,
}

#[derive(Clone, Debug)]
pub struct ProgramInputUpdate {
    pub input: ProgramInputId,
    pub value: LegacyValue,
}

#[derive(Clone)]
pub struct ProgramCellUpdate {
    pub interpreter_id: u64,
    pub target: ValRef,
    pub value: LegacyValue,
}

#[derive(Clone, Debug)]
pub struct ProgramInputUpdateOutcome {
    pub updated_count: usize,
    pub dirty_cells: Vec<ReactiveCellId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramInterpreterTurnOutcome {
    pub interpreter_id: u64,
    pub dirty_cells: Vec<ReactiveCellId>,
    pub turn: ReactiveTurnOutcome,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramInputTurnOutcome {
    pub updated_count: usize,
    pub interpreter_turns: Vec<ProgramInterpreterTurnOutcome>,
}

struct PreparedProgramCellBatch {
    assignments: Vec<Box<dyn MechFunction>>,
    targets: Vec<ValRef>,
    dirty_cells: Vec<ReactiveCellId>,
    dirty_cells_by_interpreter: BTreeMap<u64, Vec<ReactiveCellId>>,
}

#[derive(Clone, Debug)]
pub struct ProgramSolveOutcome {
    pub value: LegacyValue,
    pub plan_len: usize,
}

#[cfg(feature = "compiler")]
pub struct ProgramCompilationProduct {
    artifact: ProgramArtifact,
    bytecode: Vec<u8>,
}

#[cfg(feature = "compiler")]
impl ProgramCompilationProduct {
    pub const fn artifact(&self) -> &ProgramArtifact {
        &self.artifact
    }

    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    pub fn into_parts(self) -> (ProgramArtifact, Vec<u8>) {
        (self.artifact, self.bytecode)
    }
}

#[cfg(feature = "compiler")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramArtifactCompilationError {
    pub reason: String,
}

#[cfg(feature = "compiler")]
impl MechErrorKind for ProgramArtifactCompilationError {
    fn name(&self) -> &str {
        "ProgramArtifactCompilationError"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

pub struct MechProgram {
    pub config: MechProgramConfig,
    interpreter: Interpreter,
}

/// The ephemeral before-state for exactly one program-local reactive operation.
///
/// This is deliberately private. Safe callers choose a finalization outcome;
/// only the program decides whether the journal is retained or applied.
struct ProgramReactiveTurnJournal<'journal> {
    participant: ReactiveJournalParticipant<'journal>,
    interpreters: BTreeMap<u64, InterpreterReactiveTurnCheckpoint>,
    used: bool,
}

impl<'journal> ProgramReactiveTurnJournal<'journal> {
    fn new(participant: ReactiveJournalParticipant<'journal>) -> Self {
        Self {
            participant,
            interpreters: BTreeMap::new(),
            used: false,
        }
    }

    fn begin_operation(&mut self, operation: &'static str) -> MResult<()> {
        if self.used {
            return Err(
                MechError::new(ProgramReactiveTurnJournalAlreadyUsed { operation }, None)
                    .with_compiler_loc(),
            );
        }
        self.used = true;
        Ok(())
    }

    fn cell_count(&self) -> usize {
        self.participant.cell_count()
    }

    fn interpreter_count(&self) -> usize {
        self.interpreters.len()
    }

    fn is_empty(&self) -> bool {
        self.participant.is_empty() && self.interpreters.is_empty()
    }

    fn commit(self) {
        #[cfg(feature = "runtime_bench_probes")]
        record_reactive_journal_cell_count(self.cell_count());
        self.participant.commit();
    }
}

pub enum ProgramTurnFinalization {
    Commit,
    Rollback(MechError),
    CommitWithError(MechError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramReactiveTurnJournalAlreadyUsed {
    pub operation: &'static str,
}

impl MechErrorKind for ProgramReactiveTurnJournalAlreadyUsed {
    fn name(&self) -> &str {
        "ProgramReactiveTurnJournalAlreadyUsed"
    }

    fn message(&self) -> String {
        format!(
            "The reactive-turn journal has already been used and cannot begin `{}`.",
            self.operation,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramReactiveTurnRollbackFailed {
    pub operation: &'static str,
    pub original_error: String,
    pub rollback_error: String,
}

impl MechErrorKind for ProgramReactiveTurnRollbackFailed {
    fn name(&self) -> &str {
        "ProgramReactiveTurnRollbackFailed"
    }

    fn message(&self) -> String {
        format!(
            "Program reactive operation `{}` failed with {}; rollback also failed with {}.",
            self.operation, self.original_error, self.rollback_error,
        )
    }
}

/// An explicit, retained snapshot of a complete [`MechProgram`].
///
/// The checkpoint is intentionally opaque: it can only be created by
/// [`MechProgram::checkpoint`] and consumed by [`MechProgram::restore`].
/// It is a process-local savepoint, not a serializable durable-history record
/// or a source of stable transaction identities.
///
/// This checkpoint supports append-only plan elaboration. Removing or
/// replacing a function object that existed at capture time invalidates
/// restoration.
#[derive(Clone)]
pub struct MechProgramCheckpoint {
    config: MechProgramConfig,
    interpreter: InterpreterCheckpoint,
}

impl MechProgram {
    #[cfg(feature = "functions")]
    fn capture_reactive_interpreter(
        &self,
        interpreter_id: u64,
        journal: &mut ProgramReactiveTurnJournal<'_>,
    ) -> MResult<()> {
        let Some((actual_id, checkpoint)) =
            with_interpreter(&self.interpreter, interpreter_id, &mut |interpreter| {
                (
                    interpreter.id,
                    if journal.interpreters.contains_key(&interpreter.id) {
                        None
                    } else {
                        Some(interpreter.reactive_turn_checkpoint())
                    },
                )
            })
        else {
            return Err(MechError::new(
                ProgramInputError {
                    reason: format!("missing interpreter {interpreter_id}"),
                },
                None,
            ));
        };
        if let Some(checkpoint) = checkpoint {
            journal.interpreters.insert(actual_id, checkpoint?);
        }
        Ok(())
    }

    #[cfg(feature = "functions")]
    fn preflight_reactive_turn_rollback(
        &self,
        journal: &ProgramReactiveTurnJournal<'_>,
    ) -> MResult<()> {
        for (interpreter_id, checkpoint) in &journal.interpreters {
            let Some(result) =
                with_interpreter(&self.interpreter, *interpreter_id, &mut |interpreter| {
                    interpreter.preflight_restore_reactive_turn(checkpoint)
                })
            else {
                return Err(MechError::new(
                    ProgramInputError {
                        reason: format!(
                            "missing interpreter {} during reactive-turn rollback",
                            interpreter_id,
                        ),
                    },
                    None,
                ));
            };
            result?;
        }
        journal.participant.preflight_restore_before()
    }

    #[cfg(feature = "functions")]
    fn rollback_reactive_turn(&mut self, journal: ProgramReactiveTurnJournal<'_>) -> MResult<()> {
        #[cfg(feature = "runtime_bench_probes")]
        record_reactive_journal_cell_count(journal.cell_count());
        self.preflight_reactive_turn_rollback(&journal)?;
        let ProgramReactiveTurnJournal {
            participant,
            interpreters,
            ..
        } = journal;
        participant.apply_restore_before();
        for (interpreter_id, checkpoint) in &interpreters {
            with_interpreter_mut(&mut self.interpreter, *interpreter_id, &mut |interpreter| {
                interpreter.apply_restore_reactive_turn(checkpoint)
            })
            .expect("reactive-turn rollback preflight located every interpreter");
        }
        Ok(())
    }

    #[cfg(feature = "functions")]
    fn finish_failed_reactive_operation<T>(
        &mut self,
        operation: &'static str,
        journal: ProgramReactiveTurnJournal<'_>,
        original_error: MechError,
    ) -> MResult<T> {
        match self.rollback_reactive_turn(journal) {
            Ok(()) => Err(original_error),
            Err(rollback_error) => Err(MechError::new(
                ProgramReactiveTurnRollbackFailed {
                    operation,
                    original_error: format!("{:?}", original_error),
                    rollback_error: format!("{:?}", rollback_error),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }

    pub fn new(config: MechProgramConfig) -> Self {
        #[cfg(feature = "functions")]
        {
            Self::with_function_catalog(config, crate::empty_function_catalog())
        }
        #[cfg(not(feature = "functions"))]
        {
            let id = hash_str(&format!("program/{}", config.name));
            let mut interpreter = Interpreter::new(id, config.environment.rounds_per_step);

            interpreter.set_trace_enabled(config.environment.trace_enabled);

            Self {
                config,
                interpreter,
            }
        }
    }

    #[cfg(feature = "functions")]
    pub fn with_function_catalog(config: MechProgramConfig, catalog: Arc<FunctionCatalog>) -> Self {
        let id = hash_str(&format!("program/{}", config.name));
        let mut interpreter =
            Interpreter::with_function_catalog(id, config.environment.rounds_per_step, catalog);

        interpreter.set_trace_enabled(config.environment.trace_enabled);

        Self {
            config,
            interpreter,
        }
    }

    #[cfg(feature = "functions")]
    pub fn function_catalog(&self) -> &Arc<FunctionCatalog> {
        self.interpreter.function_catalog()
    }

    /// Captures the complete structural and value state of this program.
    ///
    /// Capture is explicit and has no effect on ordinary interpretation or
    /// reactive turns.
    pub fn checkpoint(&self) -> MResult<MechProgramCheckpoint> {
        Ok(MechProgramCheckpoint {
            config: self.config.clone(),
            interpreter: self.interpreter.checkpoint()?,
        })
    }

    /// Restores a retained checkpoint without executing program functions,
    /// callbacks, solvers, or providers.
    pub fn restore(&mut self, checkpoint: MechProgramCheckpoint) -> MResult<()> {
        self.interpreter.restore(checkpoint.interpreter)?;
        self.config = checkpoint.config;
        Ok(())
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn load_function_module(&mut self, module: &str) -> MResult<()> {
        crate::load_module(&self.interpreter, module)?;
        Ok(())
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn import_function_module_item(&mut self, module: &str, item: &str) -> MResult<()> {
        crate::import_module_item(&self.interpreter, module, item)
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn import_function_module_item_as(
        &mut self,
        module: &str,
        item: &str,
        alias: &str,
    ) -> MResult<()> {
        crate::import_module_item_as(&self.interpreter, module, item, alias)
    }

    #[cfg(all(feature = "source", feature = "functions"))]
    pub fn import_function_module_glob(&mut self, module: &str) -> MResult<()> {
        crate::import_module_glob(&self.interpreter, module)
    }

    pub fn register_function_extension(
        &mut self,
        name: impl Into<String>,
        specializer: Arc<dyn FunctionSpecializer>,
    ) -> MResult<()> {
        let canonical_name = name.into();
        let entry = FunctionExtensionEntry::new(canonical_name.clone(), specializer);
        let extension = entry.id;

        // Registration spans two checkpointed stores. Validate and apply it to
        // clones first so a collision cannot leave either store half-mutated.
        let mut state = self.interpreter.state.borrow_mut();
        let mut extensions = state.function_extensions.clone();
        let mut environment = state.function_environment.clone();
        extensions.insert_or_replace(entry)?;
        environment.bind_extension(&canonical_name, &canonical_name, extension)?;
        state.function_extensions = extensions;
        state.function_environment = environment;
        Ok(())
    }

    #[cfg(all(feature = "source", feature = "native"))]
    pub fn register_native_closure(
        &mut self,
        name: impl Into<String>,
        function: impl Fn(Vec<LegacyValue>) -> MResult<LegacyValue> + Send + Sync + 'static,
    ) -> MResult<()> {
        let name = name.into();

        self.register_function_extension(
            name.clone(),
            Arc::new(ClosureFunctionSpecializer::new(name, function)),
        )
    }

    pub fn from_environment(name: impl Into<String>, environment: MechProgramEnvironment) -> Self {
        Self::new(MechProgramConfig {
            name: name.into(),
            environment,
        })
    }

    pub fn environment(&self) -> &MechProgramEnvironment {
        &self.config.environment
    }

    pub fn set_environment(&mut self, environment: MechProgramEnvironment) {
        self.config.environment = environment;
        self.apply_environment();
    }

    pub fn configure(
        &mut self,
        debug_enabled: bool,
        trace_enabled: bool,
        profile_enabled: bool,
        rounds_per_step: usize,
    ) {
        self.set_environment(MechProgramEnvironment {
            trace_enabled,
            debug_enabled,
            profile_enabled,
            rounds_per_step,
        });
    }

    fn apply_environment(&mut self) {
        self.interpreter.max_steps = self.config.environment.rounds_per_step;
        self.interpreter
            .set_trace_enabled(self.config.environment.trace_enabled);
    }

    pub fn interpreter(&self) -> &Interpreter {
        &self.interpreter
    }

    pub fn interpreter_mut(&mut self) -> &mut Interpreter {
        &mut self.interpreter
    }

    pub fn into_interpreter(self) -> Interpreter {
        self.interpreter
    }

    #[cfg(feature = "source")]
    pub fn run_string(&mut self, source: &str) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_string_with_services(source, &mut services)
    }

    #[cfg(feature = "source")]
    pub fn run_string_with_services(
        &mut self,
        source: &str,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        let tree = parser::parse(source.trim())?;
        self.run_tree_with_services(&tree, services)
    }

    #[cfg(feature = "source")]
    pub fn run_tree(&mut self, tree: &mech_core::Program) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_tree_with_services(tree, &mut services)
    }

    #[cfg(feature = "source")]
    pub fn run_tree_with_services(
        &mut self,
        tree: &mech_core::Program,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        self.interpreter.interpret_with_services(tree, services)
    }

    pub fn run_bytecode(&mut self, bytecode: &[u8]) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_bytecode_with_services(bytecode, &mut services)
    }

    pub fn run_bytecode_with_services(
        &mut self,
        bytecode: &[u8],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        let parsed = ParsedProgram::from_bytes(bytecode)?;
        self.run_bytecode_program_with_services(&parsed, services)
    }

    pub fn run_bytecode_program(&mut self, program: &ParsedProgram) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_bytecode_program_with_services(program, &mut services)
    }

    pub fn run_bytecode_program_with_services(
        &mut self,
        program: &ParsedProgram,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        self.interpreter
            .run_program_with_services(program, services)
    }

    #[cfg(feature = "source")]
    pub fn run_program(&mut self, source: &str) -> MResult<LegacyValue> {
        self.run_profiled_string(source)
    }

    #[cfg(feature = "source")]
    pub fn run_profiled_string(&mut self, source: &str) -> MResult<LegacyValue> {
        let now = Instant::now();
        let result = self.run_string(source);

        if self.config.environment.profile_enabled {
            let cycle_duration = now.elapsed().as_nanos() as f64;
            println!("Cycle Time: {} ns", cycle_duration);
        }

        result
    }

    pub fn out_string(&self) -> String {
        self.interpreter.out.to_string()
    }

    pub fn has_interpreter(&self, interpreter_id: u64) -> bool {
        with_interpreter(&self.interpreter, interpreter_id, &mut |_| ()).is_some()
    }

    pub fn output_value_for_interpreter(
        &self,
        interpreter_id: u64,
        output_id: u64,
    ) -> Option<LegacyValue> {
        with_interpreter(&self.interpreter, interpreter_id, &mut |interpreter| {
            interpreter.out_values.borrow().get(&output_id).cloned()
        })
        .flatten()
    }

    pub fn symbol_name_for_interpreter_output(
        &self,
        interpreter_id: u64,
        output_id: u64,
    ) -> Option<String> {
        with_interpreter(&self.interpreter, interpreter_id, &mut |interpreter| {
            interpreter
                .symbols()
                .borrow()
                .get_symbol_name_by_id(output_id)
        })
        .flatten()
    }

    pub fn symbol_values_for_interpreter(
        &self,
        interpreter_id: u64,
        names: &[String],
    ) -> Option<Vec<(String, LegacyValue)>> {
        with_interpreter(&self.interpreter, interpreter_id, &mut |interpreter| {
            let symbols = interpreter.symbols();
            let symbols_brrw = symbols.borrow();
            symbol_rows(&symbols_brrw, names)
        })
    }

    pub fn root_symbol_value(&self, name: &str) -> MResult<LegacyValue> {
        let mut values = self.root_symbol_values(&[name])?;
        Ok(values.remove(0).1)
    }

    pub fn root_symbol_values(&self, names: &[&str]) -> MResult<Vec<(String, LegacyValue)>> {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut values = Vec::with_capacity(names.len());
        for name in names {
            let symbol_id = hash_str(name);
            let Some(value_ref) = symbols_brrw.get(symbol_id) else {
                return Err(MechError::new(
                    ProgramOutputNotFound {
                        name: (*name).to_string(),
                    },
                    None,
                ));
            };
            values.push(((*name).to_string(), value_ref.borrow().clone()));
        }
        Ok(values)
    }

    pub fn root_symbol_values_all(&self) -> Vec<(String, LegacyValue)> {
        let symbols = self.interpreter.symbols();
        let symbols_brrw = symbols.borrow();
        let mut values = symbol_rows(&symbols_brrw, &[]);
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values
    }

    pub fn bind_ans_for_interpreter(&mut self, interpreter_id: u64, value: &LegacyValue) -> bool {
        bind_ans_recursive(&mut self.interpreter, interpreter_id, value)
    }

    #[cfg(feature = "functions")]
    pub fn step(&mut self, step_id: u64) -> MResult<()> {
        let mut services = NoMechExecutionServices;
        self.step_with_services(step_id, &mut services)
    }

    #[cfg(feature = "functions")]
    pub fn step_with_services(
        &mut self,
        step_id: u64,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        self.step_coordinated(step_id, services, || ProgramTurnFinalization::Commit)
    }

    #[cfg(feature = "functions")]
    pub fn step_coordinated(
        &mut self,
        step_id: u64,
        services: &mut dyn MechExecutionServices,
        finalize: impl FnOnce() -> ProgramTurnFinalization,
    ) -> MResult<()> {
        with_reactive_journal_participant(|participant| {
            let mut journal = ProgramReactiveTurnJournal::new(participant);
            let execution = self.step_with_reactive_turn_journal(step_id, &mut journal, services);
            match execution {
                Ok(()) => {
                    #[cfg(feature = "invariant_define")]
                    if let Err(error) = self.validate_integrity_constraints() {
                        return self.finish_failed_reactive_operation("step", journal, error);
                    }
                    match finalize() {
                        ProgramTurnFinalization::Commit => {
                            journal.commit();
                            Ok(())
                        }
                        ProgramTurnFinalization::Rollback(error) => {
                            self.finish_failed_reactive_operation("step", journal, error)
                        }
                        ProgramTurnFinalization::CommitWithError(error) => {
                            journal.commit();
                            Err(error)
                        }
                    }
                }
                Err(error) => self.finish_failed_reactive_operation("step", journal, error),
            }
        })
    }

    #[cfg(feature = "functions")]
    fn step_with_reactive_turn_journal(
        &mut self,
        step_id: u64,
        journal: &mut ProgramReactiveTurnJournal<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        journal.begin_operation("step")?;
        self.capture_reactive_interpreter(self.interpreter.id, journal)?;
        self.interpreter.step_reactive_turn_participating(
            step_id as usize,
            1,
            &mut journal.participant,
            services,
        )?;
        Ok(())
    }

    pub fn ensure_input(
        &mut self,
        interpreter_id: u64,
        symbol_id: u64,
        name: &str,
        initial_value: LegacyValue,
    ) -> MResult<ProgramInputId> {
        let Some(existed) =
            with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
                let symbols = interpreter.symbols();
                let mut symbols_brrw = symbols.borrow_mut();
                let existed = symbols_brrw.contains(symbol_id);
                if !existed {
                    symbols_brrw.insert(symbol_id, initial_value.clone(), true);
                }
                symbols_brrw
                    .dictionary
                    .borrow_mut()
                    .insert(symbol_id, name.to_string());
                interpreter
                    .dictionary()
                    .borrow_mut()
                    .insert(symbol_id, name.to_string());
                existed
            })
        else {
            return Err(MechError::new(
                ProgramInputError {
                    reason: format!("missing interpreter {interpreter_id}"),
                },
                None,
            ));
        };
        let input = ProgramInputId {
            interpreter_id,
            symbol_id,
        };
        if existed {
            self.update_input(input, initial_value)?;
        }
        Ok(input)
    }

    pub fn update_input(&mut self, input: ProgramInputId, value: LegacyValue) -> MResult<usize> {
        self.update_inputs(&[ProgramInputUpdate { input, value }])
    }

    pub fn update_inputs(&mut self, updates: &[ProgramInputUpdate]) -> MResult<usize> {
        Ok(self.update_inputs_with_dirty_cells(updates)?.updated_count)
    }

    pub fn update_inputs_with_dirty_cells(
        &mut self,
        updates: &[ProgramInputUpdate],
    ) -> MResult<ProgramInputUpdateOutcome> {
        let updates = self.resolve_input_updates(updates)?;
        let prepared = self.prepare_cell_updates(&updates)?;
        Self::apply_prepared_cell_updates(&prepared)?;
        Ok(ProgramInputUpdateOutcome {
            updated_count: prepared.assignments.len(),
            dirty_cells: prepared.dirty_cells,
        })
    }

    fn resolve_input_updates(
        &self,
        updates: &[ProgramInputUpdate],
    ) -> MResult<Vec<ProgramCellUpdate>> {
        let mut seen_inputs = BTreeSet::new();
        let mut resolved = Vec::with_capacity(updates.len());
        for update in updates {
            let Some((actual_interpreter_id, sink)) = with_interpreter(
                &self.interpreter,
                update.input.interpreter_id,
                &mut |interpreter| {
                    let sink = interpreter.symbols().borrow().get(update.input.symbol_id);
                    (interpreter.id, sink)
                },
            ) else {
                return Err(MechError::new(
                    ProgramInputError {
                        reason: format!("missing interpreter {}", update.input.interpreter_id),
                    },
                    None,
                ));
            };
            let Some(sink) = sink else {
                return Err(MechError::new(
                    ProgramInputError {
                        reason: format!("missing program input cell {}", update.input.symbol_id),
                    },
                    None,
                ));
            };
            let canonical_input = ProgramInputId {
                interpreter_id: actual_interpreter_id,
                symbol_id: update.input.symbol_id,
            };
            if !seen_inputs.insert(canonical_input) {
                return Err(MechError::new(
                    ProgramInputDuplicateTarget {
                        input: canonical_input,
                    },
                    None,
                ));
            }
            resolved.push(ProgramCellUpdate {
                interpreter_id: actual_interpreter_id,
                target: sink,
                value: update.value.clone(),
            });
        }
        Ok(resolved)
    }

    fn prepare_cell_updates(
        &self,
        updates: &[ProgramCellUpdate],
    ) -> MResult<PreparedProgramCellBatch> {
        let mut seen_targets = BTreeSet::new();
        let mut assignments = Vec::with_capacity(updates.len());
        let mut targets = Vec::with_capacity(updates.len());
        let mut dirty_cells = Vec::new();
        let mut dirty_cells_by_interpreter = BTreeMap::new();
        for update in updates {
            let Some(actual_interpreter_id) = with_interpreter(
                &self.interpreter,
                update.interpreter_id,
                &mut |interpreter| interpreter.id,
            ) else {
                return Err(MechError::new(
                    ProgramInputError {
                        reason: format!("missing interpreter {}", update.interpreter_id),
                    },
                    None,
                ));
            };
            let target_address = update.target.as_ptr() as usize;
            if !seen_targets.insert((actual_interpreter_id, target_address)) {
                return Err(MechError::new(
                    ProgramCellDuplicateTarget {
                        interpreter_id: actual_interpreter_id,
                        target_address,
                    },
                    None,
                ));
            }
            targets.push(update.target.clone());
            assignments.push(compile_stable_value_update(
                update.target.clone(),
                update.value.clone(),
            )?);
            for cell in val_ref_reactive_cell_ids(&update.target) {
                if !dirty_cells.contains(&cell) {
                    dirty_cells.push(cell);
                }
                let cells = dirty_cells_by_interpreter
                    .entry(actual_interpreter_id)
                    .or_insert_with(Vec::new);
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        }
        Ok(PreparedProgramCellBatch {
            assignments,
            targets,
            dirty_cells,
            dirty_cells_by_interpreter,
        })
    }

    fn apply_prepared_cell_updates(prepared: &PreparedProgramCellBatch) -> MResult<()> {
        let mut staged = Vec::with_capacity(prepared.assignments.len());
        for assignment in &prepared.assignments {
            staged.push(assignment.stage_register()?);
        }
        for commit in staged {
            commit.commit();
        }
        Ok(())
    }

    #[cfg(feature = "functions")]
    pub fn advance_reactive_turn(
        &mut self,
        interpreter_id: u64,
        dirty_cells: &[ReactiveCellId],
    ) -> MResult<ReactiveTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.advance_reactive_turn_with_services(interpreter_id, dirty_cells, &mut services)
    }

    #[cfg(feature = "functions")]
    pub fn advance_reactive_turn_with_services(
        &mut self,
        interpreter_id: u64,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        self.advance_reactive_turn_coordinated(interpreter_id, dirty_cells, services, |_| {
            ProgramTurnFinalization::Commit
        })
    }

    #[cfg(feature = "functions")]
    pub fn advance_reactive_turn_coordinated(
        &mut self,
        interpreter_id: u64,
        dirty_cells: &[ReactiveCellId],
        services: &mut dyn MechExecutionServices,
        finalize: impl FnOnce(&ReactiveTurnOutcome) -> ProgramTurnFinalization,
    ) -> MResult<ReactiveTurnOutcome> {
        with_reactive_journal_participant(|participant| {
            let mut journal = ProgramReactiveTurnJournal::new(participant);
            let execution = self.advance_reactive_turn_with_journal(
                interpreter_id,
                dirty_cells,
                &mut journal,
                services,
            );
            match execution {
                Ok(outcome) => {
                    #[cfg(feature = "invariant_define")]
                    if let Err(error) = self.validate_integrity_constraints() {
                        return self.finish_failed_reactive_operation(
                            "advance_reactive_turn",
                            journal,
                            error,
                        );
                    }
                    match finalize(&outcome) {
                        ProgramTurnFinalization::Commit => {
                            journal.commit();
                            Ok(outcome)
                        }
                        ProgramTurnFinalization::Rollback(error) => self
                            .finish_failed_reactive_operation(
                                "advance_reactive_turn",
                                journal,
                                error,
                            ),
                        ProgramTurnFinalization::CommitWithError(error) => {
                            journal.commit();
                            Err(error)
                        }
                    }
                }
                Err(error) => {
                    self.finish_failed_reactive_operation("advance_reactive_turn", journal, error)
                }
            }
        })
    }

    #[cfg(feature = "functions")]
    fn advance_reactive_turn_with_journal(
        &mut self,
        interpreter_id: u64,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ProgramReactiveTurnJournal<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveTurnOutcome> {
        journal.begin_operation("advance_reactive_turn")?;
        self.capture_reactive_interpreter(interpreter_id, journal)?;
        let Some(result) =
            with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
                interpreter.advance_reactive_turn_participating(
                    dirty_cells,
                    &mut journal.participant,
                    services,
                )
            })
        else {
            return Err(MechError::new(
                ProgramInputError {
                    reason: format!("missing interpreter {interpreter_id}"),
                },
                None,
            ));
        };
        result
    }

    /// All input assignments and every affected interpreter turn form one
    /// program-local atomic operation. A failure restores input cells, executed
    /// function state, register state, and affected interpreter scheduler state.
    /// This method does not coordinate external runtime effects.
    #[cfg(feature = "functions")]
    pub fn update_inputs_and_advance_turn(
        &mut self,
        updates: &[ProgramInputUpdate],
    ) -> MResult<ProgramInputTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.update_inputs_and_advance_turn_with_services(updates, &mut services)
    }

    #[cfg(feature = "functions")]
    pub fn update_inputs_and_advance_turn_with_services(
        &mut self,
        updates: &[ProgramInputUpdate],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ProgramInputTurnOutcome> {
        self.update_inputs_and_advance_turn_coordinated(updates, services, |_| {
            ProgramTurnFinalization::Commit
        })
    }

    #[cfg(feature = "functions")]
    pub fn update_inputs_and_advance_turn_coordinated(
        &mut self,
        updates: &[ProgramInputUpdate],
        services: &mut dyn MechExecutionServices,
        finalize: impl FnOnce(&ProgramInputTurnOutcome) -> ProgramTurnFinalization,
    ) -> MResult<ProgramInputTurnOutcome> {
        let updates = self.resolve_input_updates(updates)?;
        self.update_cells_and_advance_turn_coordinated(&updates, services, finalize)
    }

    #[cfg(feature = "functions")]
    pub fn update_cells_and_advance_turn_with_services(
        &mut self,
        updates: &[ProgramCellUpdate],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ProgramInputTurnOutcome> {
        self.update_cells_and_advance_turn_coordinated(updates, services, |_| {
            ProgramTurnFinalization::Commit
        })
    }

    #[cfg(feature = "functions")]
    pub fn update_cells_and_advance_turn_coordinated(
        &mut self,
        updates: &[ProgramCellUpdate],
        services: &mut dyn MechExecutionServices,
        finalize: impl FnOnce(&ProgramInputTurnOutcome) -> ProgramTurnFinalization,
    ) -> MResult<ProgramInputTurnOutcome> {
        with_reactive_journal_participant(|participant| {
            let mut journal = ProgramReactiveTurnJournal::new(participant);
            let execution =
                self.update_cells_and_advance_turn_with_journal(updates, &mut journal, services);
            match execution {
                Ok(outcome) => {
                    #[cfg(feature = "invariant_define")]
                    if let Err(error) = self.validate_integrity_constraints() {
                        return self.finish_failed_reactive_operation(
                            "update_cells_and_advance_turn",
                            journal,
                            error,
                        );
                    }
                    match finalize(&outcome) {
                        ProgramTurnFinalization::Commit => {
                            journal.commit();
                            Ok(outcome)
                        }
                        ProgramTurnFinalization::Rollback(error) => self
                            .finish_failed_reactive_operation(
                                "update_cells_and_advance_turn",
                                journal,
                                error,
                            ),
                        ProgramTurnFinalization::CommitWithError(error) => {
                            journal.commit();
                            Err(error)
                        }
                    }
                }
                Err(error) => self.finish_failed_reactive_operation(
                    "update_cells_and_advance_turn",
                    journal,
                    error,
                ),
            }
        })
    }

    #[cfg(feature = "functions")]
    fn update_cells_and_advance_turn_with_journal(
        &mut self,
        updates: &[ProgramCellUpdate],
        journal: &mut ProgramReactiveTurnJournal<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ProgramInputTurnOutcome> {
        journal.begin_operation("update_cells_and_advance_turn")?;
        let prepared = self.prepare_cell_updates(updates)?;
        for target in &prepared.targets {
            journal.participant.capture_val_ref(target)?;
        }
        for interpreter_id in prepared.dirty_cells_by_interpreter.keys().copied() {
            self.capture_reactive_interpreter(interpreter_id, journal)?;
        }
        Self::apply_prepared_cell_updates(&prepared)?;
        let updated_count = prepared.assignments.len();
        let mut interpreter_turns = Vec::with_capacity(prepared.dirty_cells_by_interpreter.len());
        for (interpreter_id, dirty_cells) in prepared.dirty_cells_by_interpreter {
            let Some(turn) =
                with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
                    interpreter.advance_reactive_turn_participating(
                        &dirty_cells,
                        &mut journal.participant,
                        services,
                    )
                })
            else {
                return Err(MechError::new(
                    ProgramInputError {
                        reason: format!("missing interpreter {interpreter_id}"),
                    },
                    None,
                ));
            };
            let turn = turn?;
            interpreter_turns.push(ProgramInterpreterTurnOutcome {
                interpreter_id,
                dirty_cells,
                turn,
            });
        }
        Ok(ProgramInputTurnOutcome {
            updated_count,
            interpreter_turns,
        })
    }

    #[cfg(all(test, feature = "functions"))]
    fn update_inputs_and_advance_turn_with_journal_for_test(
        &mut self,
        updates: &[ProgramInputUpdate],
        journal: &mut ProgramReactiveTurnJournal<'_>,
    ) -> MResult<ProgramInputTurnOutcome> {
        let mut services = NoMechExecutionServices;
        let updates = self.resolve_input_updates(updates)?;
        self.update_cells_and_advance_turn_with_journal(&updates, journal, &mut services)
    }

    #[cfg(all(test, feature = "functions"))]
    fn advance_reactive_turn_with_journal_for_test(
        &mut self,
        interpreter_id: u64,
        dirty_cells: &[ReactiveCellId],
        journal: &mut ProgramReactiveTurnJournal<'_>,
    ) -> MResult<ReactiveTurnOutcome> {
        let mut services = NoMechExecutionServices;
        self.advance_reactive_turn_with_journal(interpreter_id, dirty_cells, journal, &mut services)
    }

    #[cfg(all(test, feature = "functions"))]
    fn step_with_reactive_turn_journal_for_test(
        &mut self,
        step_id: u64,
        journal: &mut ProgramReactiveTurnJournal<'_>,
    ) -> MResult<()> {
        let mut services = NoMechExecutionServices;
        self.step_with_reactive_turn_journal(step_id, journal, &mut services)
    }

    #[cfg(all(test, feature = "functions"))]
    fn with_program_reactive_turn_journal_for_test<T>(
        &mut self,
        operation: impl FnOnce(&mut Self, ProgramReactiveTurnJournal<'_>) -> MResult<T>,
    ) -> MResult<T> {
        with_reactive_journal_participant(|participant| {
            let journal = ProgramReactiveTurnJournal::new(participant);
            operation(self, journal)
        })
    }

    #[cfg(feature = "functions")]
    pub fn solve_plan(&mut self) -> MResult<ProgramSolveOutcome> {
        let mut services = NoMechExecutionServices;
        self.solve_plan_with_services(&mut services)
    }

    #[cfg(feature = "functions")]
    pub fn solve_plan_with_services(
        &mut self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ProgramSolveOutcome> {
        let plan_len = self.interpreter.plan_len();
        let value = self.interpreter.solve_plan_with_services(services)?;
        Ok(ProgramSolveOutcome { value, plan_len })
    }

    pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_source_with_services(source, &mut services)
    }

    pub fn run_source_with_services(
        &mut self,
        source: &MechSourceCode,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        match source {
            #[cfg(feature = "source")]
            MechSourceCode::String(source) => self.run_string_with_services(source, services),
            #[cfg(feature = "source")]
            MechSourceCode::Tree(tree) => self.run_tree_with_services(tree, services),
            MechSourceCode::ByteCode(bytecode) => {
                self.run_bytecode_with_services(bytecode, services)
            }
            MechSourceCode::Program(sources) => self.run_sources_with_services(sources, services),
            unsupported => Err(MechError::new(
                UnsupportedProgramSourceError {
                    source_kind: format!("{:?}", unsupported),
                },
                None,
            )),
        }
    }

    pub fn run_sources(&mut self, sources: &[MechSourceCode]) -> MResult<LegacyValue> {
        let mut services = NoMechExecutionServices;
        self.run_sources_with_services(sources, &mut services)
    }

    pub fn run_sources_with_services(
        &mut self,
        sources: &[MechSourceCode],
        services: &mut dyn MechExecutionServices,
    ) -> MResult<LegacyValue> {
        let mut value = LegacyValue::Empty;

        for source in sources {
            value = self.run_source_with_services(source, services)?;
        }

        Ok(value)
    }

    #[cfg(feature = "compiler")]
    pub fn compile_bytecode(&mut self) -> MResult<Vec<u8>> {
        Ok(self.compile_program_product()?.into_parts().1)
    }

    #[cfg(feature = "compiler")]
    pub fn compile_program_product(&mut self) -> MResult<ProgramCompilationProduct> {
        self.compile_program_product_with_external_inputs(&BTreeSet::new())
    }

    /// Compiles a program product whose named immutable declarations are
    /// supplied by the embedding host instead of captured as constants.
    #[cfg(feature = "compiler")]
    pub fn compile_program_product_with_external_inputs(
        &mut self,
        external_inputs: &BTreeSet<String>,
    ) -> MResult<ProgramCompilationProduct> {
        let compiled = compile_bytecode(self)?;
        let artifact = compile_executable_program_artifact_with_external_inputs(
            &compiled,
            self.interpreter.function_catalog().as_ref(),
            external_inputs,
        )
        .map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to finalize source ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let sections = encode_program_artifact_sections(&artifact).map_err(|error| {
            MechError::new(
                ProgramArtifactCompilationError {
                    reason: format!("unable to encode source ProgramArtifact: {error:?}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let bytecode = write_bytecode_with_artifact(&compiled.program, &sections)?;
        Ok(ProgramCompilationProduct { artifact, bytecode })
    }
}

/// Builds the executable compiler product before the C3 semantic artifact is
/// finalized. Keeping this boundary separate makes the adapter's legacy input
/// explicit without exposing it through `ProgramArtifact`.
#[cfg(all(feature = "compiler", feature = "invariant_define"))]
struct RetainedIntegrityMarkerMetadata {
    result_register: Register,
    name: LegacyValue,
    expression: LegacyValue,
    operator: LegacyValue,
    lhs: LegacyValue,
    rhs: LegacyValue,
}

#[cfg(feature = "compiler")]
fn compile_bytecode(program: &mut MechProgram) -> MResult<CompiledBytecode> {
    let state = program.interpreter.state.borrow();
    let plan = state.plan.borrow();
    let mut context = CompileCtx::new();

    for step in plan.iter() {
        context.begin_plan_node_with_contract(
            match step.reactive_node_kind() {
                ReactiveNodeKind::Combinational => CompiledNodeKind::Combinational,
                ReactiveNodeKind::Register => CompiledNodeKind::Register,
            },
            step.semantic_operation_contract(),
        )?;
        let compile_result = step.compile(&mut context);
        context.end_plan_node();
        compile_result?;
    }

    #[cfg(feature = "invariant_define")]
    let retained_integrity_metadata = if !state.integrity_constraints.is_empty() {
        let mut metadata = Vec::with_capacity(state.integrity_constraints.len());
        for constraint in state.integrity_constraints.values() {
            let result = LegacyValue::MutableReference(constraint.result.clone());
            let result_register = context.resolve_value_register(&result)?;
            context.record_integrity_constraint(result_register)?;
            let name = LegacyValue::String(Ref::new(constraint.name.clone()));
            let expression = LegacyValue::String(Ref::new(constraint.expression.clone()));
            let operator = match &constraint.operator {
                Some(FormulaOperator::Comparison(ComparisonOp::Equal)) => {
                    LegacyValue::String(Ref::new("eq".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::NotEqual)) => {
                    LegacyValue::String(Ref::new("neq".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::LessThan)) => {
                    LegacyValue::String(Ref::new("lt".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::LessThanEqual)) => {
                    LegacyValue::String(Ref::new("lte".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::GreaterThan)) => {
                    LegacyValue::String(Ref::new("gt".to_owned()))
                }
                Some(FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual)) => {
                    LegacyValue::String(Ref::new("gte".to_owned()))
                }
                Some(other) => {
                    return Err(MechError::new(
                        BytecodeValidationError {
                            reason: format!(
                                "integrity constraint {:?} has unsupported operator {other:?}",
                                constraint.name
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                None => LegacyValue::Empty,
            };
            let lhs = constraint
                .lhs
                .as_ref()
                .map(|value| LegacyValue::MutableReference(value.clone()))
                .unwrap_or(LegacyValue::Empty);
            let rhs = constraint
                .rhs
                .as_ref()
                .map(|value| LegacyValue::MutableReference(value.clone()))
                .unwrap_or(LegacyValue::Empty);
            metadata.push(RetainedIntegrityMarkerMetadata {
                result_register,
                name,
                expression,
                operator,
                lhs,
                rhs,
            });
        }
        metadata
    } else {
        Vec::new()
    };

    #[cfg(feature = "invariant_define")]
    let retained_integrity_marker_output =
        (!retained_integrity_metadata.is_empty()).then(|| LegacyValue::Bool(Ref::new(false)));

    #[cfg(feature = "invariant_define")]
    if let Some(marker_output) = retained_integrity_marker_output.as_ref() {
        let marker_register = context.resolve_value_register(marker_output)?;
        for marker in &retained_integrity_metadata {
            let metadata = vec![
                marker.result_register,
                context.resolve_value_register(&marker.name)?,
                context.resolve_value_register(&marker.expression)?,
                context.resolve_value_register(&marker.operator)?,
                context.resolve_value_register(&marker.lhs)?,
                context.resolve_value_register(&marker.rhs)?,
            ];
            context.emit_integrity_marker(
                hash_str("integrity/constraint"),
                marker_register,
                metadata,
            );
        }
    }

    // A composite return may materialize a CompositePack instruction. Keep it
    // inside the semantic plan so accelerator placement sees the real output
    // boundary instead of an unowned bytecode instruction.
    context.begin_plan_node(CompiledNodeKind::Combinational)?;
    let return_result = context.resolve_value_register(&program.interpreter.out);
    context.end_plan_node();
    let return_register = return_result?;
    let compiled = context.finish_program(return_register)?;

    #[cfg(feature = "invariant_define")]
    {
        drop(retained_integrity_marker_output);
        drop(retained_integrity_metadata);
    }

    Ok(compiled)
}

fn with_interpreter_mut<T>(
    interpreter: &mut Interpreter,
    interpreter_id: u64,
    f: &mut impl FnMut(&mut Interpreter) -> T,
) -> Option<T> {
    if interpreter_id == 0 || interpreter.id == interpreter_id {
        return Some(f(interpreter));
    }
    let child_ids = interpreter
        .sub_interpreters
        .borrow()
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for child_id in child_ids {
        let child = interpreter
            .sub_interpreters
            .borrow()
            .get(&child_id)
            .cloned();
        let Some(child) = child else {
            continue;
        };
        let mut child = child.borrow_mut();
        if let Some(result) = with_interpreter_mut(child.as_mut(), interpreter_id, f) {
            return Some(result);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramOutputNotFound {
    pub name: String,
}
impl MechErrorKind for ProgramOutputNotFound {
    fn name(&self) -> &str {
        "ProgramOutputNotFound"
    }
    fn message(&self) -> String {
        format!("program output symbol `{}` was not found", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct ProgramInputError {
    pub reason: String,
}
impl MechErrorKind for ProgramInputError {
    fn name(&self) -> &str {
        "ProgramInputError"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ProgramInputDuplicateTarget {
    pub input: ProgramInputId,
}

#[derive(Debug, Clone)]
pub struct ProgramCellDuplicateTarget {
    pub interpreter_id: u64,
    pub target_address: usize,
}
impl MechErrorKind for ProgramCellDuplicateTarget {
    fn name(&self) -> &str {
        "ProgramCellDuplicateTarget"
    }
    fn message(&self) -> String {
        format!(
            "duplicate program cell target at address {:#x} for interpreter {}",
            self.target_address, self.interpreter_id,
        )
    }
}
impl MechErrorKind for ProgramInputDuplicateTarget {
    fn name(&self) -> &str {
        "ProgramInputDuplicateTarget"
    }
    fn message(&self) -> String {
        format!("duplicate program input target {:?}", self.input)
    }
}

fn with_interpreter<T>(
    interpreter: &Interpreter,
    interpreter_id: u64,
    f: &mut impl FnMut(&Interpreter) -> T,
) -> Option<T> {
    if interpreter_id == 0 || interpreter.id == interpreter_id {
        return Some(f(interpreter));
    }

    let sub_interpreters = interpreter
        .sub_interpreters
        .borrow()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for sub_interpreter in sub_interpreters {
        let sub_interpreter = sub_interpreter.borrow();
        if let Some(result) = with_interpreter(sub_interpreter.as_ref(), interpreter_id, f) {
            return Some(result);
        }
    }

    None
}

fn bind_ans_recursive(
    interpreter: &mut Interpreter,
    interpreter_id: u64,
    value: &LegacyValue,
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
        let child = interpreter
            .sub_interpreters
            .borrow()
            .get(&child_id)
            .cloned();
        let Some(child) = child else {
            continue;
        };
        let mut child = child.borrow_mut();
        if bind_ans_recursive(child.as_mut(), interpreter_id, value) {
            return true;
        }
    }

    false
}

fn bind_ans_on_interpreter(interpreter: &mut Interpreter, value: &LegacyValue) {
    let resolved_value = match value {
        LegacyValue::MutableReference(reference) => reference.borrow().clone(),
        _ => value.clone(),
    };
    let ans_id = hash_str("ans");
    let symbols = interpreter.symbols();
    let mut symbols_brrw = symbols.borrow_mut();
    symbols_brrw.insert(ans_id, resolved_value, false);
    symbols_brrw
        .dictionary
        .borrow_mut()
        .insert(ans_id, "ans".to_string());
    interpreter
        .dictionary()
        .borrow_mut()
        .insert(ans_id, "ans".to_string());
}

fn symbol_rows(
    symbol_table: &mech_core::SymbolTable,
    names: &[String],
) -> Vec<(String, LegacyValue)> {
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

#[cfg(test)]
fn test_mech_program(config: MechProgramConfig) -> MechProgram {
    MechProgram::with_function_catalog(config, crate::test_support::catalog::function_catalog())
}

#[derive(Debug, Clone)]
pub struct UnsupportedProgramSourceError {
    pub source_kind: String,
}

impl MechErrorKind for UnsupportedProgramSourceError {
    fn name(&self) -> &str {
        "UnsupportedProgramSource"
    }

    fn message(&self) -> String {
        format!("Unsupported program source: {}", self.source_kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::Ref;
    #[cfg(all(
        feature = "compiler",
        feature = "source",
        feature = "invariant_define",
        feature = "compare_default",
        feature = "f64"
    ))]
    use mech_core::{BytecodeInstruction, ParsedProgram, Register};
    #[cfg(feature = "functions")]
    use mech_core::{
        FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionSpecializer, OperationId,
    };

    #[cfg(feature = "functions")]
    struct UnusedTestSpecializer;

    #[cfg(feature = "functions")]
    impl FunctionSpecializer for UnusedTestSpecializer {
        fn specialize(&self, _: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            unreachable!("program checkpoint test never specializes its marker operation")
        }
    }

    #[cfg(feature = "functions")]
    #[test]
    fn program_uses_and_retains_an_explicit_function_catalog() {
        let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
        let mut program =
            MechProgram::with_function_catalog(MechProgramConfig::default(), Arc::clone(&catalog));

        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));

        program.interpreter_mut().clear();

        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));
    }

    #[cfg(feature = "functions")]
    #[test]
    fn program_checkpoint_restore_rolls_back_the_function_environment() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_specializer("test/marker", Arc::new(UnusedTestSpecializer))
            .unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: "test/marker".to_string(),
                module: Some("test".to_string()),
                item: Some("marker".to_string()),
                exposure: FunctionExposure::ModuleOnly,
            })
            .unwrap();
        let catalog = Arc::new(builder.build().unwrap());
        let mut program =
            MechProgram::with_function_catalog(MechProgramConfig::default(), Arc::clone(&catalog));
        let environment_before = program
            .interpreter()
            .state
            .borrow()
            .function_environment
            .clone();
        let checkpoint = program.checkpoint().unwrap();
        let export = catalog.module_export("test", "marker").unwrap().clone();

        program
            .interpreter_mut()
            .state
            .borrow_mut()
            .function_environment
            .bind_catalog_export(&export, "marker")
            .unwrap();
        assert_eq!(
            program
                .interpreter()
                .state
                .borrow()
                .function_environment
                .resolve_name("marker"),
            Some(FunctionBinding::CatalogOperation(OperationId::from_name(
                "test/marker",
            ))),
        );

        program.restore(checkpoint).unwrap();

        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));
        assert_eq!(
            program.interpreter().state.borrow().function_environment,
            environment_before,
        );
        assert_eq!(
            program
                .interpreter()
                .state
                .borrow()
                .function_environment
                .resolve_name("marker"),
            None,
        );
    }

    #[cfg(all(feature = "functions", feature = "f64"))]
    fn extension_f64_value(value: &LegacyValue) -> f64 {
        match value {
            LegacyValue::F64(value) => *value.borrow(),
            LegacyValue::MutableReference(value) => extension_f64_value(&value.borrow()),
            other => panic!("expected f64 value, got {other:?}"),
        }
    }

    #[cfg(all(feature = "functions", feature = "native", feature = "f64"))]
    #[test]
    fn native_closure_registration_replaces_exact_names_and_checkpoint_restores_the_original() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let catalog = Arc::clone(program.function_catalog());

        program
            .register_native_closure("host/checkpoint", |_| Ok(LegacyValue::F64(Ref::new(1.0))))
            .unwrap();
        program.run_string("before := host/checkpoint()").unwrap();
        assert_eq!(
            extension_f64_value(&program.root_symbol_value("before").unwrap()),
            1.0,
        );

        let checkpoint = program.checkpoint().unwrap();
        program
            .register_native_closure("host/checkpoint", |_| Ok(LegacyValue::F64(Ref::new(2.0))))
            .unwrap();
        program
            .run_string("replacement := host/checkpoint()")
            .unwrap();
        assert_eq!(
            extension_f64_value(&program.root_symbol_value("replacement").unwrap()),
            2.0,
        );

        program.restore(checkpoint).unwrap();
        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));
        assert!(program.root_symbol_value("replacement").is_err());
        program.run_string("restored := host/checkpoint()").unwrap();
        assert_eq!(
            extension_f64_value(&program.root_symbol_value("restored").unwrap()),
            1.0,
        );
    }

    #[cfg(all(feature = "functions", feature = "native"))]
    #[test]
    fn native_closure_registration_is_fallible_atomic_and_checkpointed() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let extension_count = program
            .interpreter()
            .state
            .borrow()
            .function_extensions
            .len();

        let error = program
            .register_native_closure("", |_| Ok(LegacyValue::Empty))
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionExtensionInvalidEntry");
        assert_eq!(
            program
                .interpreter()
                .state
                .borrow()
                .function_extensions
                .len(),
            extension_count,
        );

        let checkpoint = program.checkpoint().unwrap();
        program
            .register_native_closure("host/temporary", |_| Ok(LegacyValue::Index(Ref::new(3))))
            .unwrap();
        assert!(matches!(
            program
                .interpreter()
                .state
                .borrow()
                .function_environment
                .resolve_name("host/temporary"),
            Some(FunctionBinding::Extension(_)),
        ));

        program.restore(checkpoint).unwrap();
        assert_eq!(
            program
                .interpreter()
                .state
                .borrow()
                .function_environment
                .resolve_name("host/temporary"),
            None,
        );
        assert_eq!(
            program
                .interpreter()
                .state
                .borrow()
                .function_extensions
                .len(),
            extension_count,
        );
    }

    #[cfg(all(
        feature = "functions",
        feature = "native",
        feature = "compiler",
        feature = "f64"
    ))]
    #[test]
    fn native_closure_bytecode_rejection_remains_structured() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program
            .register_native_closure("host/source-only", |_| Ok(LegacyValue::F64(Ref::new(4.0))))
            .unwrap();
        program
            .run_string("source-only := host/source-only()")
            .unwrap();

        let error = program.compile_bytecode().unwrap_err();
        assert_eq!(
            error.kind_name(),
            "ClosureNativeFunctionNotBytecodeCompilable",
        );
    }

    #[cfg(feature = "invariant_define")]
    fn constraint<'a>(program: &'a MechProgram, name: &str) -> mech_core::IntegrityConstraint {
        program
            .interpreter()
            .state
            .borrow()
            .integrity_constraints
            .get(&hash_str(name))
            .unwrap()
            .clone()
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_declarations_create_live_descriptors_without_enforcement() {
        for (name, expression) in [
            ("true!", "1.0 <= 2.0"),
            ("false!", "2.0 <= 1.0"),
            ("number!", "42.0"),
        ] {
            let mut program = test_mech_program(MechProgramConfig::default());
            program
                .run_string(&format!("{name} := {expression}"))
                .unwrap();
            let descriptor = constraint(&program, name);
            assert_eq!(descriptor.id, hash_str(name));
            assert_eq!(descriptor.name, name);
            assert!(!descriptor.expression.is_empty());
            assert!(!descriptor.tokens.is_empty());
            assert_eq!(
                program
                    .interpreter()
                    .state
                    .borrow()
                    .integrity_constraints
                    .len(),
                1,
            );
        }
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_direct_operands_remain_live() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program
            .run_string("target := 1.0\nmaximum := 2.0\nsafe! := target <= maximum")
            .unwrap();

        let descriptor = constraint(&program, "safe!");
        let target = program
            .interpreter()
            .symbols()
            .borrow()
            .get(hash_str("target"))
            .unwrap();
        assert_eq!(descriptor.lhs.as_ref().unwrap().addr(), target.addr());
        assert!(descriptor.rhs.is_some());
        if let LegacyValue::F64(value) = &*target.borrow() {
            *value.borrow_mut() = 3.0;
        } else {
            panic!("target must be f64");
        }
        if let LegacyValue::F64(value) = &*descriptor.lhs.unwrap().borrow() {
            assert_eq!(*value.borrow(), 3.0);
        } else {
            panic!("captured lhs must remain the target cell");
        }
    }

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_diagnostics_do_not_recompile_complex_operands() {
        let source = "target := 1.0\nmaximum := 3.0";
        let expression = "target + 1.0 <= maximum";
        let mut ordinary = test_mech_program(MechProgramConfig::default());
        ordinary
            .run_string(&format!("{source}\ncandidate := {expression}"))
            .unwrap();
        let ordinary_plan_len = ordinary.interpreter().plan_len();

        let mut constrained = test_mech_program(MechProgramConfig::default());
        constrained
            .run_string(&format!("{source}\nsafe! := {expression}"))
            .unwrap();
        let descriptor = constraint(&constrained, "safe!");

        assert_eq!(constrained.interpreter().plan_len(), ordinary_plan_len);
        assert!(descriptor.lhs.is_none());
        assert!(descriptor.operator.is_some());
        assert!(descriptor.rhs.is_some());
    }

    #[cfg(all(
        feature = "compiler",
        feature = "source",
        feature = "invariant_define",
        feature = "compare_default",
        feature = "f64"
    ))]
    #[test]
    fn integrity_marker_metadata_lives_through_bytecode_finalization() -> MResult<()> {
        let source = concat!(
            "finite-candidate! := 1.0 <= 2.0\n",
            "positive-covariance! := 2.0 <= 3.0\n",
            "symmetric-covariance! := 3.0 <= 4.0",
        );
        let expected_names = BTreeSet::from([
            "finite-candidate!".to_owned(),
            "positive-covariance!".to_owned(),
            "symmetric-covariance!".to_owned(),
        ]);

        for _ in 0..64 {
            let mut program = test_mech_program(MechProgramConfig::default());
            program.run_string(source)?;
            let expected_expressions = program
                .interpreter()
                .state
                .borrow()
                .integrity_constraints
                .values()
                .map(|constraint| (constraint.name.clone(), constraint.expression.clone()))
                .collect::<BTreeMap<_, _>>();
            let parsed = ParsedProgram::from_bytes(&program.compile_bytecode()?)?;
            let constants = parsed.decode_constants()?;
            let constant_registers = parsed
                .instructions
                .iter()
                .filter_map(|instruction| match instruction {
                    BytecodeInstruction::ConstLoad { dst, constant } => Some((*dst, *constant)),
                    _ => None,
                })
                .collect::<BTreeMap<Register, u32>>();
            let string_at = |register: Register| -> String {
                let constant = constant_registers
                    .get(&register)
                    .expect("marker String metadata must be loaded from a constant");
                match &constants[*constant as usize] {
                    LegacyValue::String(value) => value.borrow().clone(),
                    other => {
                        panic!("marker metadata register {register} must be String, got {other:?}")
                    }
                }
            };
            let markers = parsed
                .instructions
                .iter()
                .filter_map(|instruction| match instruction {
                    BytecodeInstruction::RuntimeVariadic {
                        function,
                        arguments,
                        ..
                    } if *function == hash_str("integrity/constraint") => Some(arguments),
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(markers.len(), 3);
            let mut decoded_names = BTreeSet::new();
            let mut distinct_string_registers = BTreeSet::new();
            for arguments in markers {
                assert_eq!(arguments.len(), 6);
                let name = string_at(arguments[1]);
                let expression = string_at(arguments[2]);
                assert_eq!(
                    parsed.symbols.get(&hash_str(&name)),
                    Some(&arguments[0]),
                    "integrity marker must remain associated with its named result register",
                );
                assert_eq!(expected_expressions.get(&name), Some(&expression));
                assert!(decoded_names.insert(name));
                assert!(distinct_string_registers.insert(arguments[1]));
                assert!(distinct_string_registers.insert(arguments[2]));
            }
            assert_eq!(decoded_names, expected_names);
            assert_eq!(distinct_string_registers.len(), 6);
        }

        Ok(())
    }

    fn program_with_nested_interpreter(nested_id: u64, child_id: u64) -> MechProgram {
        let mut program = test_mech_program(MechProgramConfig::default());
        let mut child = program.interpreter().clone();
        child.clear();
        child.id = child_id;
        let mut nested = program.interpreter().clone();
        nested.clear();
        nested.id = nested_id;
        child
            .sub_interpreters
            .borrow_mut()
            .insert(nested_id, Ref::new(Box::new(nested)));
        program
            .interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(child_id, Ref::new(Box::new(child)));
        program
    }

    #[test]
    fn program_has_interpreter_finds_nested_interpreter() {
        let program = program_with_nested_interpreter(4242, 2424);
        assert!(program.has_interpreter(4242));
    }

    #[test]
    fn program_output_value_for_interpreter_finds_nested_interpreter() {
        let nested_id = 4242;
        let child_id = 2424;
        let output_id = 101;
        let mut program = program_with_nested_interpreter(nested_id, child_id);
        {
            let root = program.interpreter_mut();
            let child = root
                .sub_interpreters
                .borrow()
                .get(&child_id)
                .unwrap()
                .clone();
            let child = child.borrow();
            let nested = child
                .sub_interpreters
                .borrow()
                .get(&nested_id)
                .unwrap()
                .clone();
            let nested = nested.borrow();
            nested
                .out_values
                .borrow_mut()
                .insert(output_id, LegacyValue::U64(mech_core::Ref::new(42)));
        }

        assert!(
            program
                .output_value_for_interpreter(nested_id, output_id)
                .is_some()
        );
    }

    #[test]
    fn program_bind_ans_for_interpreter_binds_ans() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let value = LegacyValue::U64(mech_core::Ref::new(42));
        assert!(program.bind_ans_for_interpreter(0, &value));
        let ans_id = hash_str("ans");
        let bound = program
            .interpreter()
            .symbols()
            .borrow()
            .get(ans_id)
            .map(|value| value.borrow().clone());
        assert_eq!(bound, Some(value));
    }
}

#[cfg(test)]
mod live_input_tests {
    use super::*;
    #[cfg(all(feature = "matrix", feature = "f64"))]
    use mech_core::structures::matrix::Matrix as MechMatrix;
    use mech_core::{Ref, hash_str};

    fn f64_value(value: &LegacyValue) -> f64 {
        match value {
            LegacyValue::F64(value) => *value.borrow(),
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_preserves_f64_reference() {
        let sink = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::F64(value) => value.as_ptr(),
            other => panic!("expected f64, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(9.0))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::F64(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 9.0);
            }
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[cfg(feature = "i64")]
    #[test]
    fn stable_value_update_preserves_i64_reference() {
        let sink = Ref::new(LegacyValue::I64(Ref::new(1)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::I64(value) => value.as_ptr(),
            other => panic!("expected i64, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::I64(Ref::new(9))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::I64(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 9);
            }
            other => panic!("expected i64, got {other:?}"),
        }
    }

    #[cfg(feature = "bool")]
    #[test]
    fn stable_value_update_preserves_bool_reference() {
        let sink = Ref::new(LegacyValue::Bool(Ref::new(false)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Bool(value) => value.as_ptr(),
            other => panic!("expected bool, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::Bool(Ref::new(true))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Bool(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert!(*value.borrow());
            }
            other => panic!("expected bool, got {other:?}"),
        }
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    #[test]
    fn stable_value_update_preserves_string_reference() {
        let sink = Ref::new(LegacyValue::String(Ref::new("old".to_string())));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::String(value) => value.as_ptr(),
            other => panic!("expected string, got {other:?}"),
        };
        apply_stable_value_update(
            sink.clone(),
            LegacyValue::String(Ref::new("new".to_string())),
        )
        .unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::String(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(&*value.borrow(), "new");
            }
            other => panic!("expected string, got {other:?}"),
        }
    }

    #[test]
    fn stable_value_update_preserves_index_reference() {
        let sink = Ref::new(LegacyValue::Index(Ref::new(1)));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Index(value) => value.as_ptr(),
            other => panic!("expected index, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::Index(Ref::new(9))).unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Index(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 9);
            }
            other => panic!("expected index, got {other:?}"),
        }
    }

    #[cfg(all(feature = "f64", any(feature = "string", feature = "variable_define")))]
    #[test]
    fn stable_value_update_rejects_incompatible_kind() {
        let sink = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::F64(value) => value.as_ptr(),
            other => panic!("expected f64, got {other:?}"),
        };
        assert!(
            apply_stable_value_update(
                sink.clone(),
                LegacyValue::String(Ref::new("bad".to_string()))
            )
            .is_err()
        );
        match &*sink.borrow() {
            LegacyValue::F64(value) => {
                assert_eq!(inner_pointer, value.as_ptr());
                assert_eq!(*value.borrow(), 1.0);
            }
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[cfg(all(feature = "matrixd", feature = "f64"))]
    #[test]
    fn stable_value_update_preserves_dynamic_matrix_storage() {
        let sink_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            2,
            2,
            vec![1.0, 2.0, 3.0, 4.0],
        )));
        let source_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            2,
            2,
            vec![5.0, 6.0, 7.0, 8.0],
        )));
        let sink = Ref::new(LegacyValue::MatrixF64(sink_matrix));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => value.addr(),
            other => panic!("expected f64 matrix, got {other:?}"),
        };
        apply_stable_value_update(sink.clone(), LegacyValue::MatrixF64(source_matrix.clone()))
            .unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value, &source_matrix);
            }
            other => panic!("expected f64 matrix, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_preserves_typed_scalar_reference() {
        let sink = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            ValueKind::F64,
        ));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => value.as_ptr(),
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        };

        apply_stable_value_update(
            sink.clone(),
            LegacyValue::Typed(Box::new(LegacyValue::F64(Ref::new(9.0))), ValueKind::F64),
        )
        .unwrap();

        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => {
                        assert_eq!(inner_pointer, value.as_ptr());
                        assert_eq!(*value.borrow(), 9.0);
                    }
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_rejects_different_typed_annotation() {
        let sink = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            ValueKind::F64,
        ));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Typed(inner, _) => match inner.as_ref() {
                LegacyValue::F64(value) => value.as_ptr(),
                other => panic!("expected typed f64 inner, got {other:?}"),
            },
            other => panic!("expected typed value, got {other:?}"),
        };

        let result = apply_stable_value_update(
            sink.clone(),
            LegacyValue::Typed(Box::new(LegacyValue::F64(Ref::new(9.0))), ValueKind::String),
        );
        assert!(
            format!("{:?}", result.unwrap_err()).contains("StableValueUpdateContractViolation")
        );

        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => {
                        assert_eq!(inner_pointer, value.as_ptr());
                        assert_eq!(*value.borrow(), 1.0);
                    }
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_rejects_typed_to_untyped() {
        let sink = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(1.0))),
            ValueKind::F64,
        ));
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::Typed(inner, _) => match inner.as_ref() {
                LegacyValue::F64(value) => value.as_ptr(),
                other => panic!("expected typed f64 inner, got {other:?}"),
            },
            other => panic!("expected typed value, got {other:?}"),
        };

        let result = apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(9.0)));
        assert!(
            format!("{:?}", result.unwrap_err()).contains("StableValueUpdateContractViolation")
        );

        match &*sink.borrow() {
            LegacyValue::Typed(inner, annotation) => {
                assert_eq!(annotation, &ValueKind::F64);
                match inner.as_ref() {
                    LegacyValue::F64(value) => {
                        assert_eq!(inner_pointer, value.as_ptr());
                        assert_eq!(*value.borrow(), 1.0);
                    }
                    other => panic!("expected typed f64 inner, got {other:?}"),
                }
            }
            other => panic!("expected typed value, got {other:?}"),
        }
    }

    #[cfg(feature = "compiler")]
    #[test]
    fn empty_stable_assignment_bytecode_compile_returns_error() {
        let assignment =
            compile_stable_value_update(Ref::new(LegacyValue::Empty), LegacyValue::Empty).unwrap();
        let mut ctx = CompileCtx::new();
        let error = assignment.compile(&mut ctx).unwrap_err();
        let rendered = format!("{error:?}");
        assert!(
            rendered.contains("EmptyAssignmentNotBytecodeCompilable"),
            "{rendered}"
        );
    }

    #[test]
    fn stable_value_update_accepts_empty_to_empty() {
        let sink = Ref::new(LegacyValue::Empty);
        compile_stable_value_update(sink.clone(), LegacyValue::Empty).unwrap();
        apply_stable_value_update(sink.clone(), LegacyValue::Empty).unwrap();
        assert_eq!(&*sink.borrow(), &LegacyValue::Empty);
    }

    #[cfg(feature = "f64")]
    #[test]
    fn stable_value_update_rejects_empty_to_value() {
        let sink = Ref::new(LegacyValue::Empty);
        let result = apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(1.0)));
        assert!(
            format!("{:?}", result.unwrap_err()).contains("StableValueUpdateContractViolation")
        );
        assert_eq!(&*sink.borrow(), &LegacyValue::Empty);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_dynamic_matrix_shape_change() {
        let sink_matrix = MechMatrix::from_vec((1..=25).map(|x| x as f64).collect(), 5, 5);
        let source_matrix = MechMatrix::from_vec((1..=36).map(|x| x as f64).collect(), 6, 6);
        let expected = sink_matrix.clone();
        let sink = Ref::new(LegacyValue::MatrixF64(sink_matrix));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => value.addr(),
            other => panic!("expected f64 matrix, got {other:?}"),
        };

        let error = apply_stable_value_update(sink.clone(), LegacyValue::MatrixF64(source_matrix))
            .unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");

        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value.shape(), vec![5, 5]);
                assert_eq!(value, &expected);
            }
            other => panic!("expected f64 matrix, got {other:?}"),
        }
    }

    #[cfg(all(feature = "matrixd", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_equal_length_dynamic_shape_change() {
        let sink_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            2,
            3,
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        )));
        let source_matrix = MechMatrix::DMatrix(Ref::new(crate::na::DMatrix::from_vec(
            3,
            2,
            vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
        )));
        let expected = sink_matrix.clone();
        let sink = Ref::new(LegacyValue::MatrixF64(sink_matrix));
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => value.addr(),
            other => panic!("expected f64 matrix, got {other:?}"),
        };

        let error = apply_stable_value_update(sink.clone(), LegacyValue::MatrixF64(source_matrix))
            .unwrap_err();
        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");

        match &*sink.borrow() {
            LegacyValue::MatrixF64(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value.shape(), vec![2, 3]);
                assert_eq!(value, &expected);
            }
            other => panic!("expected f64 matrix, got {other:?}"),
        }
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_matrix_value_sink() {
        let matrix_value = MechMatrix::from_vec(vec![LegacyValue::F64(Ref::new(1.0))], 1, 1);
        let sink = Ref::new(LegacyValue::MatrixValue(matrix_value));
        let result = apply_stable_value_update(sink.clone(), LegacyValue::F64(Ref::new(9.0)));
        let rendered = format!("{:?}", result.unwrap_err());
        assert!(
            rendered.contains("StableValueUpdateContractViolation"),
            "{rendered}"
        );
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn stable_value_update_rejects_matrix_value_source() {
        let sink = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let matrix_value = MechMatrix::from_vec(vec![LegacyValue::F64(Ref::new(9.0))], 1, 1);
        let result =
            apply_stable_value_update(sink.clone(), LegacyValue::MatrixValue(matrix_value));
        let rendered = format!("{:?}", result.unwrap_err());
        assert!(
            rendered.contains("StableValueUpdateContractViolation"),
            "{rendered}"
        );
    }

    #[cfg(feature = "matrix")]
    #[test]
    fn stable_value_update_preserves_matrix_index_reference() {
        let matrix = MechMatrix::from_vec(vec![1usize, 2, 3, 4], 2, 2);
        let sink = Ref::new(LegacyValue::MatrixIndex(matrix));
        let outer_pointer = sink.as_ptr();
        let inner_pointer = match &*sink.borrow() {
            LegacyValue::MatrixIndex(value) => value.addr(),
            other => panic!("expected index matrix, got {other:?}"),
        };
        apply_stable_value_update(
            sink.clone(),
            LegacyValue::MatrixIndex(MechMatrix::from_vec(vec![5usize, 6, 7, 8], 2, 2)),
        )
        .unwrap();
        assert_eq!(outer_pointer, sink.as_ptr());
        match &*sink.borrow() {
            LegacyValue::MatrixIndex(value) => {
                assert_eq!(inner_pointer, value.addr());
                assert_eq!(value.shape(), vec![2, 2]);
                assert_eq!(value, &MechMatrix::from_vec(vec![5usize, 6, 7, 8], 2, 2));
            }
            other => panic!("expected index matrix, got {other:?}"),
        }
    }

    #[cfg(all(
        feature = "record",
        feature = "map",
        feature = "set",
        feature = "table",
        feature = "tuple",
        feature = "f64",
        any(feature = "string", feature = "variable_define")
    ))]
    #[test]
    fn stable_value_update_publishes_every_composite_without_replacing_its_cell() {
        let record = |value| {
            LegacyValue::Record(Ref::new(mech_core::MechRecord::new(vec![(
                "value",
                LegacyValue::F64(Ref::new(value)),
            )])))
        };
        let map = |value| {
            LegacyValue::Map(Ref::new(mech_core::MechMap::from_typed_vec(
                ValueKind::String,
                ValueKind::F64,
                1,
                vec![(
                    LegacyValue::String(Ref::new("key".to_owned())),
                    LegacyValue::F64(Ref::new(value)),
                )],
            )))
        };
        let set = |value| {
            LegacyValue::Set(Ref::new(mech_core::MechSet::from_vec(vec![
                LegacyValue::F64(Ref::new(value)),
            ])))
        };
        let table = |value| {
            LegacyValue::Table(Ref::new(
                mech_core::MechTable::from_records(vec![mech_core::MechRecord::new(vec![(
                    "value",
                    LegacyValue::F64(Ref::new(value)),
                )])])
                .unwrap(),
            ))
        };
        let tuple = |value| {
            LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
                LegacyValue::F64(Ref::new(value)),
            ])))
        };
        let composite_addr = |value: &LegacyValue| match value {
            LegacyValue::Record(value) => value.addr(),
            LegacyValue::Map(value) => value.addr(),
            LegacyValue::Set(value) => value.addr(),
            LegacyValue::Table(value) => value.addr(),
            LegacyValue::Tuple(value) => value.addr(),
            other => panic!("expected composite, got {other:?}"),
        };
        let nested_f64 = |value: &LegacyValue| -> Option<Ref<f64>> {
            let nested = match value {
                LegacyValue::Record(value) => value.borrow().data.values().next().cloned(),
                LegacyValue::Map(value) => value.borrow().map.values().next().cloned(),
                LegacyValue::Table(value) => value
                    .borrow()
                    .data
                    .values()
                    .next()
                    .and_then(|(_, values)| values.as_vec().into_iter().next()),
                LegacyValue::Tuple(value) => value
                    .borrow()
                    .elements
                    .first()
                    .map(|value| value.as_ref().clone()),
                LegacyValue::Set(_) => None,
                other => panic!("expected composite, got {other:?}"),
            };
            nested.map(|value| match value {
                LegacyValue::F64(value) => value,
                other => panic!("expected nested F64, got {other:?}"),
            })
        };

        for (current, incoming) in [
            (record(1.0), record(2.0)),
            (map(1.0), map(2.0)),
            (set(1.0), set(2.0)),
            (table(1.0), table(2.0)),
            (tuple(1.0), tuple(2.0)),
        ] {
            let expected = incoming.clone();
            let inner_pointer = composite_addr(&current);
            let nested = nested_f64(&current);
            let nested_pointer = nested.as_ref().map(Ref::addr);
            let sink = Ref::new(current);

            compile_stable_value_update(sink.clone(), incoming)
                .unwrap()
                .stage_register()
                .unwrap()
                .commit();

            assert_eq!(composite_addr(&sink.borrow()), inner_pointer);
            assert_eq!(*sink.borrow(), expected);
            if let Some(nested) = nested {
                let updated = nested_f64(&sink.borrow()).expect("nested cell must remain present");
                assert_eq!(Some(updated.addr()), nested_pointer);
                assert_eq!(updated.addr(), nested.addr());
                assert_eq!(*nested.borrow(), 2.0);
            }
        }

        let shared = Ref::new(1.0);
        let aliased = LegacyValue::Record(Ref::new(mech_core::MechRecord::new(vec![
            ("left", LegacyValue::F64(shared.clone())),
            ("right", LegacyValue::F64(shared.clone())),
        ])));
        let distinct = LegacyValue::Record(Ref::new(mech_core::MechRecord::new(vec![
            ("left", LegacyValue::F64(Ref::new(2.0))),
            ("right", LegacyValue::F64(Ref::new(3.0))),
        ])));
        let aliased = Ref::new(aliased);
        let error = match compile_stable_value_update(aliased, distinct)
            .unwrap()
            .stage_register()
        {
            Ok(_) => panic!("aliased structured update unexpectedly staged"),
            Err(error) => error,
        };
        assert!(
            error
                .kind_message()
                .contains("cannot preserve aliased child cell")
        );
        assert_eq!(*shared.borrow(), 1.0);
    }

    #[test]
    fn existing_input_cell_is_mutated_without_replacing_valref() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let input_id = hash_str("input");
        let output_id = hash_str("output");
        program
            .ensure_input(
                program.interpreter().id,
                input_id,
                "input",
                LegacyValue::F64(Ref::new(1.0)),
            )
            .unwrap();
        program.run_string("output := input * 2").unwrap();
        let before = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        let outer_pointer = before.as_ptr();
        let inner_pointer = match &*before.borrow() {
            LegacyValue::F64(value) => value.as_ptr(),
            other => panic!("expected f64 input, got {other:?}"),
        };
        let output = program
            .interpreter()
            .symbols()
            .borrow()
            .get(output_id)
            .unwrap();
        assert_eq!(f64_value(&output.borrow()), 2.0);

        let handle = program
            .ensure_input(
                program.interpreter().id,
                input_id,
                "input",
                LegacyValue::F64(Ref::new(1.0)),
            )
            .unwrap();
        program
            .update_input(handle, LegacyValue::F64(Ref::new(5.0)))
            .unwrap();
        let after = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        assert_eq!(outer_pointer, after.as_ptr());
        match &*after.borrow() {
            LegacyValue::F64(value) => assert_eq!(inner_pointer, value.as_ptr()),
            other => panic!("expected f64 input, got {other:?}"),
        }

        let outcome = program.solve_plan().unwrap();
        assert!(outcome.plan_len > 0);
        let input = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        assert_eq!(f64_value(&input.borrow()), 5.0);
        let output = program
            .interpreter()
            .symbols()
            .borrow()
            .get(output_id)
            .unwrap();
        assert_eq!(f64_value(&output.borrow()), 10.0);
    }

    #[test]
    fn ensure_input_refreshes_existing_cell_without_replacing_valref() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let input_id = hash_str("live-x");
        let interpreter_id = program.interpreter().id;
        program
            .ensure_input(
                interpreter_id,
                input_id,
                "live-x",
                LegacyValue::F64(Ref::new(1.0)),
            )
            .unwrap();
        let before = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        let before_pointer = before.as_ptr();
        assert_eq!(f64_value(&before.borrow()), 1.0);

        program
            .ensure_input(
                interpreter_id,
                input_id,
                "live-x",
                LegacyValue::F64(Ref::new(9.0)),
            )
            .unwrap();
        let after = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        assert_eq!(before_pointer, after.as_ptr());
        assert_eq!(f64_value(&after.borrow()), 9.0);
    }

    #[test]
    fn incompatible_input_type_is_rejected_without_mutation() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let input_id = hash_str("input");
        program
            .ensure_input(
                program.interpreter().id,
                input_id,
                "input",
                LegacyValue::F64(Ref::new(1.0)),
            )
            .unwrap();
        program.run_string("output := input * 2").unwrap();
        let handle = ProgramInputId {
            interpreter_id: program.interpreter().id,
            symbol_id: input_id,
        };
        assert!(
            program
                .update_inputs(&[ProgramInputUpdate {
                    input: handle,
                    value: LegacyValue::String(Ref::new("bad".to_string()))
                }])
                .is_err()
        );
        let input = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        assert_eq!(f64_value(&input.borrow()), 1.0);
        let output = program
            .interpreter()
            .symbols()
            .borrow()
            .get(hash_str("output"))
            .unwrap();
        assert_eq!(f64_value(&output.borrow()), 2.0);
    }

    #[test]
    fn update_inputs_preflight_rejects_before_mutating_any_input() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let a_id = hash_str("a");
        let b_id = hash_str("b");
        let interpreter_id = program.interpreter().id;
        let a = program
            .ensure_input(interpreter_id, a_id, "a", LegacyValue::F64(Ref::new(1.0)))
            .unwrap();
        let b = program
            .ensure_input(interpreter_id, b_id, "b", LegacyValue::F64(Ref::new(2.0)))
            .unwrap();
        let result = program.update_inputs(&[
            ProgramInputUpdate {
                input: a,
                value: LegacyValue::F64(Ref::new(3.0)),
            },
            ProgramInputUpdate {
                input: b,
                value: LegacyValue::String(Ref::new("bad".to_string())),
            },
        ]);
        assert!(result.is_err());
        let a_value = program.interpreter().symbols().borrow().get(a_id).unwrap();
        let b_value = program.interpreter().symbols().borrow().get(b_id).unwrap();
        assert_eq!(f64_value(&a_value.borrow()), 1.0);
        assert_eq!(f64_value(&b_value.borrow()), 2.0);
    }

    #[cfg(feature = "f64")]
    #[test]
    fn program_input_dirty_cells_include_outer_valref() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let input_id = hash_str("input");
        let interpreter_id = program.interpreter().id;
        let input = program
            .ensure_input(
                interpreter_id,
                input_id,
                "input",
                LegacyValue::F64(Ref::new(1.0)),
            )
            .unwrap();
        let input_val_ref = program
            .interpreter()
            .symbols()
            .borrow()
            .get(input_id)
            .unwrap();
        let input_val_ref_id = input_val_ref.id();
        let nested_scalar_id = match &*input_val_ref.borrow() {
            LegacyValue::F64(value) => value.id(),
            other => panic!("expected f64 input, got {other:?}"),
        };

        let outcome = program
            .update_inputs_with_dirty_cells(&[ProgramInputUpdate {
                input,
                value: LegacyValue::F64(Ref::new(2.0)),
            }])
            .unwrap();

        assert!(
            outcome
                .dirty_cells
                .contains(&ReactiveCellId::new(input_val_ref_id))
        );
        assert!(
            outcome
                .dirty_cells
                .contains(&ReactiveCellId::new(nested_scalar_id))
        );
    }

    #[test]
    fn missing_input_returns_error() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let missing = ProgramInputId {
            interpreter_id: program.interpreter().id,
            symbol_id: hash_str("missing"),
        };
        assert!(
            program
                .update_input(missing, LegacyValue::F64(Ref::new(1.0)))
                .is_err()
        );
    }
}

#[cfg(test)]
mod root_symbol_snapshot_tests {
    use super::*;
    use mech_core::Ref;

    fn f64_value(value: &LegacyValue) -> f64 {
        match value {
            LegacyValue::F64(value) => *value.borrow(),
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[test]
    fn root_symbol_value_returns_value() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("answer := 42.0").unwrap();
        assert_eq!(
            f64_value(&program.root_symbol_value("answer").unwrap()),
            42.0
        );
    }

    #[test]
    fn root_symbol_values_preserve_order() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("a := 1.0\nb := 2.0\nc := 3.0").unwrap();
        let rows = program.root_symbol_values(&["c", "a", "b"]).unwrap();
        let names: Vec<_> = rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["c", "a", "b"]);
    }

    #[test]
    fn root_symbol_values_snapshot_multiple_values() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("a := 1.0\nb := 2.0").unwrap();
        let rows = program.root_symbol_values(&["a", "b"]).unwrap();
        assert_eq!(f64_value(&rows[0].1), 1.0);
        assert_eq!(f64_value(&rows[1].1), 2.0);
    }

    #[test]
    fn root_symbol_values_all_are_sorted_by_name() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("c := 3.0\na := 1.0\nb := 2.0").unwrap();
        let rows = program.root_symbol_values_all();
        let names: Vec<_> = rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["a", "ans", "b", "c"]);
    }

    #[test]
    fn missing_root_symbol_returns_structured_error() {
        let program = test_mech_program(MechProgramConfig::default());
        let err = program.root_symbol_value("missing").unwrap_err();
        assert!(format!("{:?}", err).contains("ProgramOutputNotFound"));
    }

    #[test]
    fn snapshot_does_not_hold_symbol_table_borrow() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("answer := 42.0").unwrap();
        let _snapshot = program.root_symbol_value("answer").unwrap();
        let symbols = program.interpreter().symbols();
        let _mutable_borrow = symbols.borrow_mut();
    }
}

#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "f64",
    feature = "variable_define",
    feature = "variable_assign"
))]
mod program_reactive_turn_tests {
    use super::*;
    use mech_core::Ref;
    fn f64_value(v: f64) -> LegacyValue {
        LegacyValue::F64(Ref::new(v))
    }
    fn input(p: &mut MechProgram, n: &str, v: f64) -> ProgramInputId {
        let id = hash_str(n);
        p.ensure_input(p.interpreter().id, id, n, f64_value(v))
            .unwrap()
    }
    fn symbol(p: &MechProgram, id: u64, n: &str) -> LegacyValue {
        with_interpreter(p.interpreter(), id, &mut |i| {
            i.symbols()
                .borrow()
                .get(hash_str(n))
                .expect("symbol")
                .borrow()
                .clone()
        })
        .expect("interpreter")
    }
    fn value(p: &MechProgram, id: u64, n: &str) -> f64 {
        *symbol(p, id, n).as_f64().expect("f64").borrow()
    }
    fn cell(p: &MechProgram, id: u64, n: &str) -> ReactiveCellId {
        let v = symbol(p, id, n).reactive_root_cell_ids();
        assert_eq!(v.len(), 1, "root cell");
        v[0]
    }
    fn register_node_for_output(p: &MechProgram, id: u64, c: ReactiveCellId) -> ReactiveNodeId {
        let v = with_interpreter(p.interpreter(), id, &mut |i| {
            let q = i.plan();
            q.borrow()
                .nodes
                .iter()
                .filter(|n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&c))
                .map(|n| n.id)
                .collect::<Vec<_>>()
        })
        .expect("interpreter");
        assert_eq!(v.len(), 1, "register node");
        v[0]
    }
    fn combinational_node_for_output_and_inputs(
        p: &MechProgram,
        id: u64,
        c: ReactiveCellId,
        required: &[ReactiveCellId],
    ) -> ReactiveNodeId {
        let v = with_interpreter(p.interpreter(), id, &mut |i| {
            let q = i.plan();
            q.borrow()
                .nodes
                .iter()
                .filter(|n| {
                    n.kind == ReactiveNodeKind::Combinational
                        && n.outputs.contains(&c)
                        && required.iter().all(|r| {
                            n.inputs
                                .iter()
                                .any(|d| d.cell == *r && d.kind == ReactiveDependencyKind::Reactive)
                        })
                })
                .map(|n| n.id)
                .collect::<Vec<_>>()
        })
        .expect("interpreter");
        assert_eq!(v.len(), 1, "computation node");
        v[0]
    }
    fn pending(p: &MechProgram, id: u64) -> bool {
        with_interpreter(p.interpreter(), id, &mut |i| {
            i.has_pending_reactive_registers()
        })
        .expect("interpreter")
    }
    fn snapshot(
        p: &MechProgram,
        id: u64,
    ) -> (usize, Vec<ReactiveNodeId>, Vec<Vec<ReactiveCellId>>) {
        with_interpreter(p.interpreter(), id, &mut |i| {
            let q = i.plan();
            let q = q.borrow();
            (
                q.len(),
                q.nodes.iter().map(|n| n.id).collect(),
                q.nodes.iter().map(|n| n.outputs.clone()).collect(),
            )
        })
        .expect("interpreter")
    }
    #[test]
    fn program_reactive_turn_updates_only_reachable_branch() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let (l, r) = (input(&mut p, "left", 1.), input(&mut p, "right", 2.));
        p.run_string("left-output := left + 1.0\nright-output := right + 1.0")
            .unwrap();
        let id = p.interpreter().id;
        let (lc, rc, lo, ro) = (
            cell(&p, id, "left"),
            cell(&p, id, "right"),
            cell(&p, id, "left-output"),
            cell(&p, id, "right-output"),
        );
        let (ln, rn) = (
            combinational_node_for_output_and_inputs(&p, id, lo, &[lc]),
            combinational_node_for_output_and_inputs(&p, id, ro, &[rc]),
        );
        let o = p
            .update_inputs_and_advance_turn(&[ProgramInputUpdate {
                input: l,
                value: f64_value(10.),
            }])
            .unwrap();
        assert_eq!(o.updated_count, 1);
        assert_eq!(o.interpreter_turns.len(), 1);
        let t = &o.interpreter_turns[0];
        assert_eq!(t.interpreter_id, id);
        assert_eq!(
            (value(&p, id, "left-output"), value(&p, id, "right-output")),
            (11., 3.)
        );
        assert!(t.turn.before_commit.executed_nodes.contains(&ln));
        assert!(!t.turn.before_commit.executed_nodes.contains(&rn));
        assert!(t.turn.before_commit.pending_register_nodes.is_empty());
        assert_eq!(
            t.turn.register_commit,
            ReactiveRegisterCommitOutcome::default()
        );
        assert_eq!(t.turn.after_commit, ReactivePlanSolveOutcome::default());
        let _ = r;
    }
    #[test]
    fn program_reactive_turn_batches_inputs_into_one_turn() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let (a, b) = (input(&mut p, "a", 1.), input(&mut p, "b", 2.));
        p.run_string("~total := 0.0\nsum := a + b\ntotal = sum\noutput := total + 1.0")
            .unwrap();
        let id = p.interpreter().id;
        let (ac, bc, sc, tc, oc) = (
            cell(&p, id, "a"),
            cell(&p, id, "b"),
            cell(&p, id, "sum"),
            cell(&p, id, "total"),
            cell(&p, id, "output"),
        );
        let (sum, total, out) = (
            combinational_node_for_output_and_inputs(&p, id, sc, &[ac, bc]),
            register_node_for_output(&p, id, tc),
            combinational_node_for_output_and_inputs(&p, id, oc, &[tc]),
        );
        let o = p
            .update_inputs_and_advance_turn(&[
                ProgramInputUpdate {
                    input: a,
                    value: f64_value(10.),
                },
                ProgramInputUpdate {
                    input: b,
                    value: f64_value(20.),
                },
            ])
            .unwrap();
        let t = &o.interpreter_turns[0].turn;
        assert_eq!(o.updated_count, 2);
        assert_eq!(o.interpreter_turns.len(), 1);
        assert_eq!(
            t.before_commit
                .executed_nodes
                .iter()
                .filter(|node_id| **node_id == sum)
                .count(),
            1
        );
        assert_eq!(t.before_commit.pending_register_nodes, vec![total]);
        assert_eq!(t.register_commit.staged_nodes, vec![total]);
        assert_eq!(t.register_commit.committed_nodes, vec![total]);
        assert!(t.after_commit.executed_nodes.contains(&out));
        assert_eq!(
            (
                value(&p, id, "sum"),
                value(&p, id, "total"),
                value(&p, id, "output")
            ),
            (30., 30., 31.)
        );
        assert!(!pending(&p, id));
    }
    #[test]
    fn program_reactive_turn_preserves_deferred_registers_between_calls() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let x = input(&mut p, "input", 1.);
        p.run_string(
            "~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0",
        )
        .unwrap();
        let id = p.interpreter().id;
        let (a, b) = (
            register_node_for_output(&p, id, cell(&p, id, "a")),
            register_node_for_output(&p, id, cell(&p, id, "b")),
        );
        let o = p
            .update_inputs_and_advance_turn(&[ProgramInputUpdate {
                input: x,
                value: f64_value(10.),
            }])
            .unwrap();
        let t = &o.interpreter_turns[0].turn;
        assert_eq!(t.register_commit.committed_nodes, vec![a]);
        assert_eq!(t.after_commit.pending_register_nodes, vec![b]);
        assert_eq!(
            (
                value(&p, id, "a"),
                value(&p, id, "middle"),
                value(&p, id, "b"),
                value(&p, id, "output")
            ),
            (10., 11., 2., 3.)
        );
        assert!(pending(&p, id));
        let t = p.advance_reactive_turn(id, &[]).unwrap();
        assert_eq!(t.register_commit.committed_nodes, vec![b]);
        assert_eq!(
            (
                value(&p, id, "a"),
                value(&p, id, "middle"),
                value(&p, id, "b"),
                value(&p, id, "output")
            ),
            (10., 11., 11., 12.)
        );
        assert!(!pending(&p, id));
    }
    #[test]
    fn program_reactive_turn_legacy_update_api_does_not_execute_plan() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let x = input(&mut p, "input", 1.);
        p.run_string("output := input * 2.0").unwrap();
        let id = p.interpreter().id;
        let u = p
            .update_inputs_with_dirty_cells(&[ProgramInputUpdate {
                input: x,
                value: f64_value(5.),
            }])
            .unwrap();
        assert_eq!((value(&p, id, "input"), value(&p, id, "output")), (5., 2.));
        assert!(!pending(&p, id));
        assert_eq!(u.updated_count, 1);
        assert!(!u.dirty_cells.is_empty());
        p.advance_reactive_turn(id, &u.dirty_cells).unwrap();
        assert_eq!(value(&p, id, "output"), 10.);
    }
    #[cfg(feature = "string")]
    #[test]
    fn program_reactive_turn_preflight_failure_mutates_nothing() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let (a, b) = (input(&mut p, "a", 1.), input(&mut p, "b", 2.));
        p.run_string("output := a + b").unwrap();
        let id = p.interpreter().id;
        let s = snapshot(&p, id);
        let q = p
            .update_inputs_and_advance_turn(&[
                ProgramInputUpdate {
                    input: a,
                    value: f64_value(10.),
                },
                ProgramInputUpdate {
                    input: b,
                    value: LegacyValue::String(Ref::new("bad".into())),
                },
            ])
            .unwrap_err();
        assert!(format!("{q:?}").contains("StableValueUpdateContractViolation"));
        assert_eq!(
            (
                value(&p, id, "a"),
                value(&p, id, "b"),
                value(&p, id, "output")
            ),
            (1., 2., 3.)
        );
        assert_eq!(snapshot(&p, id), s);
        assert!(!pending(&p, id));
    }
    #[test]
    fn program_reactive_turn_rejects_root_alias_duplicate() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let x = input(&mut p, "input", 1.);
        let id = p.interpreter().id;
        let e = p
            .update_inputs_and_advance_turn(&[
                ProgramInputUpdate {
                    input: ProgramInputId {
                        interpreter_id: 0,
                        symbol_id: x.symbol_id,
                    },
                    value: f64_value(2.),
                },
                ProgramInputUpdate {
                    input: x,
                    value: f64_value(3.),
                },
            ])
            .unwrap_err();
        assert!(format!("{e:?}").contains("ProgramInputDuplicateTarget"));
        assert_eq!(value(&p, id, "input"), 1.);
        assert!(!pending(&p, id));
    }
    #[test]
    fn program_reactive_turn_empty_batch_is_noop() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let id = p.interpreter().id;
        let s = snapshot(&p, id);
        assert_eq!(
            p.update_inputs_and_advance_turn(&[]).unwrap(),
            ProgramInputTurnOutcome::default()
        );
        assert_eq!(snapshot(&p, id), s);
        assert!(!pending(&p, id));
    }
    #[test]
    fn program_reactive_turn_orders_nested_interpreters_deterministically() {
        let mut p = test_mech_program(MechProgramConfig::default());
        let catalog = crate::test_support::catalog::function_catalog();
        let mut c1 = Interpreter::with_function_catalog(101, 10_000, Arc::clone(&catalog));
        let mut c2 = Interpreter::with_function_catalog(202, 10_000, catalog);
        for (i, src) in [
            (
                &mut c1,
                "input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0",
            ),
            (&mut c2, "input := 1.0\noutput := input + 1.0"),
        ] {
            i.interpret(&parser::parse(src).unwrap()).unwrap();
        }
        p.interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(101, Ref::new(Box::new(c1)));
        p.interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(202, Ref::new(Box::new(c2)));
        let (a, b) = (
            ProgramInputId {
                interpreter_id: 101,
                symbol_id: hash_str("input"),
            },
            ProgramInputId {
                interpreter_id: 202,
                symbol_id: hash_str("input"),
            },
        );
        let root = snapshot(&p, p.interpreter().id);
        let o = p
            .update_inputs_and_advance_turn(&[
                ProgramInputUpdate {
                    input: b,
                    value: f64_value(20.),
                },
                ProgramInputUpdate {
                    input: a,
                    value: f64_value(10.),
                },
            ])
            .unwrap();
        assert_eq!(o.updated_count, 2);
        assert_eq!(
            o.interpreter_turns
                .iter()
                .map(|x| x.interpreter_id)
                .collect::<Vec<_>>(),
            vec![101, 202]
        );
        assert!(!o.interpreter_turns[0].dirty_cells.is_empty());
        assert!(!o.interpreter_turns[1].dirty_cells.is_empty());
        assert_ne!(
            o.interpreter_turns[0].dirty_cells,
            o.interpreter_turns[1].dirty_cells
        );
        assert_eq!(
            (
                value(&p, 101, "a"),
                value(&p, 101, "middle"),
                value(&p, 101, "b")
            ),
            (10., 11., 2.)
        );
        assert!(pending(&p, 101));
        assert_eq!(value(&p, 202, "output"), 21.);
        assert!(!pending(&p, 202));
        assert!(!pending(&p, p.interpreter().id));
        assert_eq!(snapshot(&p, p.interpreter().id), root);
        p.advance_reactive_turn(101, &[]).unwrap();
        assert_eq!(
            (
                value(&p, 101, "b"),
                value(&p, 101, "output"),
                value(&p, 202, "output")
            ),
            (11., 12., 21.)
        );
        assert!(!pending(&p, 101));
        assert!(!pending(&p, 202));
    }
    #[cfg(feature = "compiler")]
    #[test]
    fn program_reactive_turn_decoded_plan_reuses_identity() {
        let mut source = test_mech_program(MechProgramConfig::default());
        source
            .run_string("input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nb = a\nb")
            .unwrap();
        let bytes = source.compile_bytecode().unwrap();
        let mut p = test_mech_program(MechProgramConfig::default());
        p.run_bytecode(&bytes).unwrap();
        let id = p.interpreter().id;
        let x = p
            .ensure_input(id, hash_str("input"), "input", f64_value(1.))
            .unwrap();
        let s = snapshot(&p, id);
        let (a, b) = (
            register_node_for_output(&p, id, cell(&p, id, "a")),
            register_node_for_output(&p, id, cell(&p, id, "b")),
        );
        let o = p
            .update_inputs_and_advance_turn(&[ProgramInputUpdate {
                input: x,
                value: f64_value(10.),
            }])
            .unwrap();
        assert_eq!(
            o.interpreter_turns[0].turn.register_commit.committed_nodes,
            vec![a]
        );
        assert_eq!(
            o.interpreter_turns[0]
                .turn
                .after_commit
                .pending_register_nodes,
            vec![b]
        );
        let o = p.advance_reactive_turn(id, &[]).unwrap();
        assert_eq!(o.register_commit.committed_nodes, vec![b]);
        assert_eq!(value(&p, id, "b"), 10.);
        assert!(!pending(&p, id));
        assert_eq!(snapshot(&p, id), s);
    }
}

#[cfg(all(test, feature = "program", feature = "functions"))]
mod compact_program_reactive_turn_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    struct ProgramTestFunction {
        name: &'static str,
        output: Ref<usize>,
        hidden: Vec<Ref<usize>>,
        captures: Rc<RefCell<usize>>,
        solves: Rc<RefCell<usize>>,
        order: Rc<RefCell<Vec<u64>>>,
        interpreter_id: u64,
        fail: bool,
        leak_borrow: bool,
    }

    impl MechFunctionImpl for ProgramTestFunction {
        fn solve_result(&self) -> MResult<()> {
            self.solve_reactive().map(|_| ())
        }

        fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
            *self.solves.borrow_mut() += 1;
            self.order.borrow_mut().push(self.interpreter_id);
            *self.output.borrow_mut() += 1;
            for state in &self.hidden {
                *state.borrow_mut() += 1;
            }
            if self.fail {
                if self.leak_borrow {
                    let borrow = self.output.borrow_mut();
                    std::mem::forget(borrow);
                }
                return Err(MechError::new(
                    GenericError {
                        msg: format!("deliberate {} failure", self.name),
                    },
                    None,
                ));
            }
            Ok(ReactiveSolveStatus::Changed)
        }

        fn out(&self) -> LegacyValue {
            LegacyValue::Index(self.output.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            *self.captures.borrow_mut() += 1;
            let mut values = vec![LegacyValue::Index(self.output.clone())];
            values.extend(self.hidden.iter().cloned().map(LegacyValue::Index));
            Ok(values)
        }

        fn to_string(&self) -> String {
            self.name.into()
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for ProgramTestFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<mech_core::Register> {
            Ok(0)
        }
    }

    fn test_function(
        name: &'static str,
        output: Ref<usize>,
        interpreter_id: u64,
        order: Rc<RefCell<Vec<u64>>>,
    ) -> (ProgramTestFunction, Rc<RefCell<usize>>, Rc<RefCell<usize>>) {
        let captures = Rc::new(RefCell::new(0));
        let solves = Rc::new(RefCell::new(0));
        (
            ProgramTestFunction {
                name,
                output,
                hidden: Vec::new(),
                captures: captures.clone(),
                solves: solves.clone(),
                order,
                interpreter_id,
                fail: false,
                leak_borrow: false,
            },
            captures,
            solves,
        )
    }

    fn step_fixture(
        failing_step: Option<usize>,
    ) -> (
        MechProgram,
        Vec<Ref<usize>>,
        Vec<Rc<RefCell<usize>>>,
        Vec<Rc<RefCell<usize>>>,
    ) {
        let program = test_mech_program(MechProgramConfig::default());
        let interpreter_id = program.interpreter().id;
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut outputs = Vec::new();
        let mut captures = Vec::new();
        let mut solves = Vec::new();
        for (step_id, name) in ["first", "second", "third"].into_iter().enumerate() {
            let output = Ref::new(0usize);
            let (mut function, capture_count, solve_count) =
                test_function(name, output.clone(), interpreter_id, order.clone());
            function.fail = failing_step == Some(step_id + 1);
            program
                .interpreter()
                .plan()
                .add_function(Box::new(function));
            outputs.push(output);
            captures.push(capture_count);
            solves.push(solve_count);
        }
        (program, outputs, captures, solves)
    }

    fn ref_values(values: &[Ref<usize>]) -> Vec<usize> {
        values.iter().map(|value| *value.borrow()).collect()
    }

    fn counter_values(values: &[Rc<RefCell<usize>>]) -> Vec<usize> {
        values.iter().map(|value| *value.borrow()).collect()
    }

    fn index_input(
        program: &mut MechProgram,
        interpreter_id: u64,
        name: &str,
        value: usize,
    ) -> (ProgramInputId, ValRef, Ref<usize>) {
        let symbol_id = hash_str(name);
        let input = program
            .ensure_input(
                interpreter_id,
                symbol_id,
                name,
                LegacyValue::Index(Ref::new(value)),
            )
            .unwrap();
        let outer = with_interpreter(program.interpreter(), interpreter_id, &mut |interpreter| {
            interpreter.symbols().borrow().get(symbol_id).unwrap()
        })
        .unwrap();
        let inner = match &*outer.borrow() {
            LegacyValue::Index(inner) => inner.clone(),
            other => panic!("expected index input, got {other:?}"),
        };
        (input, outer, inner)
    }

    fn add_reactive(
        program: &MechProgram,
        interpreter_id: u64,
        function: ProgramTestFunction,
        input: &Ref<usize>,
    ) {
        let mut function = Some(function);
        with_interpreter(program.interpreter(), interpreter_id, &mut |interpreter| {
            interpreter
                .plan()
                .0
                .borrow_mut()
                .register(
                    Box::new(function.take().expect("function registered once")),
                    &[LegacyValue::Index(input.clone())],
                )
                .unwrap();
        })
        .unwrap();
    }

    fn update(input: ProgramInputId, value: usize) -> ProgramInputUpdate {
        ProgramInputUpdate {
            input,
            value: LegacyValue::Index(Ref::new(value)),
        }
    }

    #[test]
    fn direct_cell_update_preserves_target_and_triggers_reactive_node() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (_, target, inner) = index_input(&mut program, id, "input", 1);
        let target_address = target.addr();
        let inner_address = inner.addr();
        let output = Ref::new(10usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (function, _, solves) = test_function("direct", output.clone(), id, order);
        add_reactive(&program, id, function, &inner);

        let mut services = NoMechExecutionServices;
        let outcome = program
            .update_cells_and_advance_turn_with_services(
                &[ProgramCellUpdate {
                    interpreter_id: id,
                    target: target.clone(),
                    value: LegacyValue::Index(Ref::new(9)),
                }],
                &mut services,
            )
            .unwrap();

        assert_eq!(outcome.updated_count, 1);
        assert_eq!(outcome.interpreter_turns.len(), 1);
        assert_eq!(
            (*inner.borrow(), *output.borrow(), *solves.borrow()),
            (9, 11, 1)
        );
        assert_eq!(
            (target.addr(), inner.addr()),
            (target_address, inner_address)
        );
    }

    #[test]
    fn direct_cell_update_rolls_back_target_and_output_after_downstream_failure() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (_, target, inner) = index_input(&mut program, id, "input", 1);
        let target_address = target.addr();
        let inner_address = inner.addr();
        let output = Ref::new(10usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("direct", output.clone(), id, order);
        function.fail = true;
        add_reactive(&program, id, function, &inner);

        let mut services = NoMechExecutionServices;
        let error = program
            .update_cells_and_advance_turn_with_services(
                &[ProgramCellUpdate {
                    interpreter_id: id,
                    target: target.clone(),
                    value: LegacyValue::Index(Ref::new(9)),
                }],
                &mut services,
            )
            .unwrap_err();

        assert!(error.kind_message().contains("deliberate direct failure"));
        assert_eq!((*inner.borrow(), *output.borrow()), (1, 10));
        assert_eq!(
            (target.addr(), inner.addr()),
            (target_address, inner_address)
        );
    }

    #[test]
    fn later_combinational_failure_restores_input_and_earlier_output() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, outer, inner) = index_input(&mut program, id, "input", 1);
        let outer_address = outer.addr();
        let inner_address = inner.addr();
        let first_output = Ref::new(10usize);
        let second_output = Ref::new(20usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (first, _, _) = test_function("first", first_output.clone(), id, order.clone());
        let (mut second, _, _) = test_function("second", second_output.clone(), id, order);
        second.fail = true;
        add_reactive(&program, id, first, &inner);
        add_reactive(&program, id, second, &inner);

        let error = program
            .update_inputs_and_advance_turn(&[update(input, 9)])
            .unwrap_err();

        assert!(error.kind_message().contains("deliberate second failure"));
        assert_eq!(
            (
                *inner.borrow(),
                *first_output.borrow(),
                *second_output.borrow()
            ),
            (1, 10, 20)
        );
        assert_eq!((outer.addr(), inner.addr()), (outer_address, inner_address));
    }

    struct ProgramRegister {
        source: Ref<usize>,
        sink: Ref<usize>,
    }

    impl MechFunctionImpl for ProgramRegister {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }
        fn out(&self) -> LegacyValue {
            LegacyValue::Index(self.sink.clone())
        }
        fn reactive_node_kind(&self) -> ReactiveNodeKind {
            ReactiveNodeKind::Register
        }
        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
            Ok(Box::new(ReactiveRegisterWrite::new(
                self.sink.clone(),
                *self.source.borrow(),
                vec![ReactiveCellId::new(self.sink.id())],
            )))
        }
        fn to_string(&self) -> String {
            "program-register".into()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(self.reactive_output_values())
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for ProgramRegister {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<mech_core::Register> {
            Ok(0)
        }
    }

    #[test]
    fn post_register_failure_restores_commit_and_pending_state() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, source) = index_input(&mut program, id, "input", 1);
        let sink = Ref::new(2usize);
        let plan = program.interpreter().plan();
        plan.0
            .borrow_mut()
            .register(
                Box::new(ProgramRegister {
                    source: source.clone(),
                    sink: sink.clone(),
                }),
                &[LegacyValue::Index(source.clone())],
            )
            .unwrap();
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut failure, _, _) = test_function("post-register", Ref::new(0), id, order);
        failure.fail = true;
        plan.0
            .borrow_mut()
            .register(Box::new(failure), &[LegacyValue::Index(sink.clone())])
            .unwrap();

        program
            .update_inputs_and_advance_turn(&[update(input, 8)])
            .unwrap_err();

        assert_eq!((*source.borrow(), *sink.borrow()), (1, 2));
        assert!(!program.interpreter().has_pending_reactive_registers());
    }

    fn add_child(program: &mut MechProgram, id: u64) {
        program
            .interpreter_mut()
            .sub_interpreters
            .borrow_mut()
            .insert(id, Ref::new(Box::new(Interpreter::new(id, 100))));
    }

    #[test]
    fn all_prior_interpreters_restore_and_later_interpreters_do_not_execute() {
        let mut program = test_mech_program(MechProgramConfig::default());
        for id in [10, 20, 30, 40] {
            add_child(&mut program, id);
        }
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut updates = Vec::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for id in [10, 20, 30, 40] {
            let (input, _, inner) = index_input(&mut program, id, "input", 1);
            let output = Ref::new(id as usize);
            let (mut function, _, _) = test_function("child", output.clone(), id, order.clone());
            function.fail = id == 30;
            add_reactive(&program, id, function, &inner);
            updates.push(update(input, 9));
            inputs.push(inner);
            outputs.push(output);
        }

        program
            .update_inputs_and_advance_turn(&updates)
            .unwrap_err();

        assert_eq!(order.borrow().as_slice(), &[10, 20, 30]);
        assert!(inputs.iter().all(|input| *input.borrow() == 1));
        assert_eq!(
            outputs
                .iter()
                .map(|output| *output.borrow())
                .collect::<Vec<_>>(),
            vec![10, 20, 30, 40],
        );
    }

    #[test]
    fn successful_interpreter_order_is_ascending_actual_id() {
        let mut program = test_mech_program(MechProgramConfig::default());
        for id in [30, 10, 20] {
            add_child(&mut program, id);
        }
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut updates = Vec::new();
        for id in [30, 10, 20] {
            let (input, _, inner) = index_input(&mut program, id, "input", 1);
            let (function, _, _) = test_function("child", Ref::new(0), id, order.clone());
            add_reactive(&program, id, function, &inner);
            updates.push(update(input, 2));
        }

        let outcome = program.update_inputs_and_advance_turn(&updates).unwrap();

        assert_eq!(order.borrow().as_slice(), &[10, 20, 30]);
        assert_eq!(
            outcome
                .interpreter_turns
                .iter()
                .map(|turn| turn.interpreter_id)
                .collect::<Vec<_>>(),
            vec![10, 20, 30],
        );
    }

    #[test]
    fn shared_cells_restore_once_and_retain_identity() {
        let mut program = test_mech_program(MechProgramConfig::default());
        for id in [10, 20] {
            add_child(&mut program, id);
        }
        let shared = Ref::new(5usize);
        let address = shared.addr();
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut updates = Vec::new();
        for id in [10, 20] {
            let (input, _, inner) = index_input(&mut program, id, "input", 1);
            let (function, _, _) = test_function("shared", shared.clone(), id, order.clone());
            add_reactive(&program, id, function, &inner);
            updates.push(update(input, 2));
        }
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program
                    .update_inputs_and_advance_turn_with_journal_for_test(&updates, &mut journal)?;
                assert_eq!(*shared.borrow(), 7);
                program.rollback_reactive_turn(journal)
            })
            .unwrap();

        assert_eq!(*shared.borrow(), 5);
        assert_eq!(shared.addr(), address);
    }

    #[test]
    fn hidden_function_owned_state_is_restored() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let hidden = Ref::new(40usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("hidden", Ref::new(0), id, order);
        function.hidden.push(hidden.clone());
        function.fail = true;
        add_reactive(&program, id, function, &inner);

        program
            .update_inputs_and_advance_turn(&[update(input, 2)])
            .unwrap_err();

        assert_eq!(*hidden.borrow(), 40);
    }

    #[test]
    fn fsm_transition_like_retained_state_is_restored() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "event", 0);
        let transition_state = Ref::new(3usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("fsm transition", Ref::new(0), id, order);
        function.hidden.push(transition_state.clone());
        function.fail = true;
        add_reactive(&program, id, function, &inner);
        program
            .update_inputs_and_advance_turn(&[update(input, 1)])
            .unwrap_err();
        assert_eq!(*transition_state.borrow(), 3);
    }

    #[test]
    fn pattern_matcher_guard_selector_gate_and_capture_state_restore() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "pattern", 0);
        let states = (0..5).map(Ref::new).collect::<Vec<_>>();
        let addresses = states.iter().map(Ref::addr).collect::<Vec<_>>();
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("pattern pipeline", Ref::new(0), id, order);
        function.hidden = states.clone();
        function.fail = true;
        add_reactive(&program, id, function, &inner);
        program
            .update_inputs_and_advance_turn(&[update(input, 1)])
            .unwrap_err();
        assert_eq!(
            states
                .iter()
                .map(|state| *state.borrow())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(states.iter().map(Ref::addr).collect::<Vec<_>>(), addresses);
    }

    #[cfg(any())]
    #[test]
    fn rollback_removes_trace_suffixes_from_every_affected_interpreter() {
        let mut program = test_mech_program(MechProgramConfig::default());
        for id in [10, 20] {
            add_child(&mut program, id);
        }
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut updates = Vec::new();
        for id in [10, 20] {
            let (input, _, inner) = index_input(&mut program, id, "input", 1);
            let (function, _, _) = test_function("trace", Ref::new(0), id, order.clone());
            add_reactive(&program, id, function, &inner);
            updates.push(update(input, 2));
        }
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program
                    .update_inputs_and_advance_turn_with_journal_for_test(&updates, &mut journal)?;
                for id in [10, 20] {
                    with_interpreter(program.interpreter(), id, &mut |interpreter| {
                        interpreter.push_trace_line("suffix".into());
                    })
                    .unwrap();
                }
                program.rollback_reactive_turn(journal)
            })
            .unwrap();
        for id in [10, 20] {
            assert!(
                with_interpreter(program.interpreter(), id, &mut |interpreter| {
                    interpreter.trace_events().is_empty()
                })
                .unwrap()
            );
        }
    }

    #[test]
    fn single_interpreter_advance_is_atomic() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let input = Ref::new(1usize);
        let output = Ref::new(7usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("advance", output.clone(), id, order);
        function.fail = true;
        add_reactive(&program, id, function, &input);
        program
            .advance_reactive_turn(id, &LegacyValue::Index(input).reactive_root_cell_ids())
            .unwrap_err();
        assert_eq!(*output.borrow(), 7);
    }

    #[test]
    fn input_turn_api_is_atomic() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("input-turn", Ref::new(0), id, order);
        function.fail = true;
        add_reactive(&program, id, function, &inner);
        program
            .update_inputs_and_advance_turn(&[update(input, 2)])
            .unwrap_err();
        assert_eq!(*inner.borrow(), 1);
    }

    #[test]
    fn step_zero_executes_the_whole_plan_once() {
        let (mut program, outputs, captures, solves) = step_fixture(None);

        program.step(0).unwrap();

        assert_eq!(ref_values(&outputs), vec![1, 1, 1]);
        assert_eq!(counter_values(&captures), vec![1, 1, 1]);
        assert_eq!(counter_values(&solves), vec![1, 1, 1]);
    }

    #[test]
    fn coordinated_step_commit_retains_state() {
        let (mut program, outputs, _, _) = step_fixture(None);
        let mut services = NoMechExecutionServices;

        program
            .step_coordinated(1, &mut services, || ProgramTurnFinalization::Commit)
            .unwrap();

        assert_eq!(ref_values(&outputs), vec![1, 0, 0]);
    }

    #[test]
    fn coordinated_step_rollback_restores_state() {
        let (mut program, outputs, _, _) = step_fixture(None);
        let mut services = NoMechExecutionServices;

        let error = program
            .step_coordinated(1, &mut services, || {
                ProgramTurnFinalization::Rollback(MechError::new(
                    GenericError {
                        msg: "deliberate coordinated rollback".to_string(),
                    },
                    None,
                ))
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert_eq!(ref_values(&outputs), vec![0, 0, 0]);
    }

    #[test]
    fn coordinated_step_commit_with_error_retains_state() {
        let (mut program, outputs, _, _) = step_fixture(None);
        let mut services = NoMechExecutionServices;

        let error = program
            .step_coordinated(1, &mut services, || {
                ProgramTurnFinalization::CommitWithError(MechError::new(
                    GenericError {
                        msg: "deliberate post-commit error".to_string(),
                    },
                    None,
                ))
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert_eq!(ref_values(&outputs), vec![1, 0, 0]);
    }

    #[test]
    fn positive_step_id_executes_only_that_function_once() {
        let (mut program, outputs, captures, solves) = step_fixture(None);

        program.step(3).unwrap();

        assert_eq!(ref_values(&outputs), vec![0, 0, 1]);
        assert_eq!(counter_values(&captures), vec![0, 0, 1]);
        assert_eq!(counter_values(&solves), vec![0, 0, 1]);
    }

    #[test]
    fn selected_step_failure_rolls_back_only_the_operation() {
        let (mut program, outputs, captures, solves) = step_fixture(Some(3));
        let selected_output_address = outputs[2].addr();
        let plan_address = program.interpreter().plan().0.addr();

        let error = program.step(3).unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert!(error.kind_message().contains("deliberate third failure"));
        assert_eq!(ref_values(&outputs), vec![0, 0, 0]);
        assert_eq!(counter_values(&captures), vec![0, 0, 1]);
        assert_eq!(counter_values(&solves), vec![0, 0, 1]);
        assert_eq!(outputs[2].addr(), selected_output_address);
        assert_eq!(program.interpreter().plan().0.addr(), plan_address);
    }

    #[test]
    fn whole_plan_failure_still_rolls_back_earlier_functions() {
        let (mut program, outputs, captures, solves) = step_fixture(Some(3));

        let error = program.step(0).unwrap_err();

        assert_eq!(error.kind_name(), "GenericError");
        assert!(error.kind_message().contains("deliberate third failure"));
        assert_eq!(ref_values(&outputs), vec![0, 0, 0]);
        assert_eq!(counter_values(&captures), vec![1, 1, 1]);
        assert_eq!(counter_values(&solves), vec![1, 1, 1]);
    }

    #[test]
    fn journal_aware_failure_leaves_changes_until_explicit_rollback() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let output = Ref::new(3usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("journal", output.clone(), id, order);
        function.fail = true;
        add_reactive(&program, id, function, &inner);
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program
                    .update_inputs_and_advance_turn_with_journal_for_test(
                        &[update(input, 2)],
                        &mut journal,
                    )
                    .unwrap_err();
                assert_eq!((*inner.borrow(), *output.borrow()), (2, 4));
                program.rollback_reactive_turn(journal)
            })
            .unwrap();
        assert_eq!((*inner.borrow(), *output.borrow()), (1, 3));
    }

    #[test]
    fn rollback_after_journal_aware_success_restores_operation() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let output = Ref::new(3usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (function, _, _) = test_function("success", output.clone(), id, order);
        add_reactive(&program, id, function, &inner);
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(input, 2)],
                    &mut journal,
                )?;
                program.rollback_reactive_turn(journal)
            })
            .unwrap();
        assert_eq!((*inner.borrow(), *output.borrow()), (1, 3));
    }

    #[test]
    fn unfinalized_journal_is_automatically_rolled_back() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let error = program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(input, 2)],
                    &mut journal,
                )?;
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.kind_name(), "ReactiveJournalFinalizationMissing");
        assert_eq!(*inner.borrow(), 1);
    }

    #[test]
    fn journal_reuse_fails_before_second_mutation() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(input, 2)],
                    &mut journal,
                )?;
                let error = program
                    .update_inputs_and_advance_turn_with_journal_for_test(
                        &[update(input, 3)],
                        &mut journal,
                    )
                    .unwrap_err();
                assert_eq!(error.kind_name(), "ProgramReactiveTurnJournalAlreadyUsed",);
                journal.commit();
                Ok(())
            })
            .unwrap();
        assert_eq!(*inner.borrow(), 2);
    }

    #[test]
    fn rollback_preflight_failure_uses_automatic_value_restore() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let error = program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(input, 2)],
                    &mut journal,
                )?;
                let (tail, _, _) = test_function(
                    "topology-tail",
                    Ref::new(0),
                    id,
                    Rc::new(RefCell::new(Vec::new())),
                );
                program.interpreter().plan().add_function(Box::new(tail));
                program.rollback_reactive_turn(journal)
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert_eq!(*inner.borrow(), 1);
    }

    #[test]
    fn program_rollback_failure_preserves_original_and_rollback_errors() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        let output = Ref::new(0usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (mut function, _, _) = test_function("borrow leak", output, id, order);
        function.fail = true;
        function.leak_borrow = true;
        add_reactive(&program, id, function, &inner);

        let error = program
            .update_inputs_and_advance_turn(&[update(input, 2)])
            .unwrap_err();

        assert_eq!(error.kind_name(), "ReactiveJournalAutomaticRollbackFailed",);
        let rollback = error
            .kind_as::<ReactiveJournalAutomaticRollbackFailed>()
            .unwrap();
        let original_error = rollback.original_error.as_ref().unwrap();
        assert!(original_error.contains("ProgramReactiveTurnRollbackFailed"));
        assert!(original_error.contains("deliberate borrow leak failure"));
        assert!(original_error.contains("ValueStateBorrowConflict"));
        assert!(rollback.rollback_error.contains("ValueStateBorrowConflict"));
    }

    #[test]
    fn compact_operations_preserve_plan_identity_without_program_checkpoint() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let plan_address = program.interpreter().plan().0.addr();
        let (input, _, _) = index_input(&mut program, id, "input", 1);
        program
            .update_inputs_and_advance_turn(&[update(input, 2)])
            .unwrap();
        assert_eq!(program.interpreter().plan().0.addr(), plan_address);
    }

    #[test]
    fn one_of_one_thousand_independent_nodes_has_compact_structural_count() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let order = Rc::new(RefCell::new(Vec::new()));
        let mut selected = None;
        let mut selected_capture = None;
        let mut all_captures = Vec::new();
        for index in 0..1000 {
            let (input, _, inner) = index_input(&mut program, id, &format!("input-{index}"), index);
            let (function, captures, _) =
                test_function("independent", Ref::new(index), id, order.clone());
            add_reactive(&program, id, function, &inner);
            if index == 777 {
                selected = Some(input);
                selected_capture = Some(captures.clone());
            }
            all_captures.push(captures);
        }
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(selected.unwrap(), 9000)],
                    &mut journal,
                )?;
                assert_eq!(*selected_capture.unwrap().borrow(), 1);
                assert_eq!(
                    all_captures
                        .iter()
                        .map(|count| *count.borrow())
                        .sum::<usize>(),
                    1,
                );
                assert_eq!(journal.interpreter_count(), 1);
                assert_eq!(journal.cell_count(), 3);
                journal.commit();
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn reactive_turn_journal_reports_empty_and_operation_counts() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, _) = index_input(&mut program, id, "input", 1);
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                assert!(journal.is_empty());
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(input, 2)],
                    &mut journal,
                )?;
                assert_eq!(journal.interpreter_count(), 1);
                assert!(journal.cell_count() > 0);
                assert!(!journal.is_empty());
                journal.commit();
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn journal_aware_single_turn_can_be_rolled_back_after_success() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let input = Ref::new(1usize);
        let output = Ref::new(5usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (function, _, _) = test_function("single success", output.clone(), id, order);
        add_reactive(&program, id, function, &input);
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.advance_reactive_turn_with_journal_for_test(
                    id,
                    &LegacyValue::Index(input).reactive_root_cell_ids(),
                    &mut journal,
                )?;
                assert_eq!(*output.borrow(), 6);
                program.rollback_reactive_turn(journal)
            })
            .unwrap();
        assert_eq!(*output.borrow(), 5);
    }

    #[test]
    fn journal_aware_step_leaves_state_until_rollback() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let output = Ref::new(5usize);
        let order = Rc::new(RefCell::new(Vec::new()));
        let (function, _, _) = test_function("step success", output.clone(), id, order);
        program
            .interpreter()
            .plan()
            .add_function(Box::new(function));
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.step_with_reactive_turn_journal_for_test(1, &mut journal)?;
                assert_eq!(*output.borrow(), 6);
                program.rollback_reactive_turn(journal)
            })
            .unwrap();
        assert_eq!(*output.borrow(), 5);
    }

    #[test]
    fn rollback_missing_interpreter_uses_automatic_value_restore() {
        let mut program = test_mech_program(MechProgramConfig::default());
        add_child(&mut program, 77);
        let (input, _, inner) = index_input(&mut program, 77, "input", 1);
        let error = program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                program.update_inputs_and_advance_turn_with_journal_for_test(
                    &[update(input, 2)],
                    &mut journal,
                )?;
                program
                    .interpreter_mut()
                    .sub_interpreters
                    .borrow_mut()
                    .remove(&77);
                program.rollback_reactive_turn(journal)
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "ProgramInputError");
        assert_eq!(*inner.borrow(), 1);
    }

    #[test]
    fn duplicate_input_rejection_happens_before_any_journal_capture() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let id = program.interpreter().id;
        let (input, _, inner) = index_input(&mut program, id, "input", 1);
        program
            .with_program_reactive_turn_journal_for_test(|program, mut journal| {
                let error = program
                    .update_inputs_and_advance_turn_with_journal_for_test(
                        &[update(input, 2), update(input, 3)],
                        &mut journal,
                    )
                    .unwrap_err();
                assert_eq!(error.kind_name(), "ProgramInputDuplicateTarget");
                assert_eq!(journal.cell_count(), 0);
                assert_eq!(journal.interpreter_count(), 0);
                journal.commit();
                Ok(())
            })
            .unwrap();
        assert_eq!(*inner.borrow(), 1);
    }
}

#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "f64",
    feature = "variable_define",
))]
mod retained_checkpoint_tests {
    use super::*;
    use mech_core::{MechRecord, MechTuple, Ref, ToMatrix};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct RestoreProbe {
        output: Ref<f64>,
        solve_count: Rc<RefCell<usize>>,
    }

    impl MechFunctionImpl for RestoreProbe {
        fn solve_result(&self) -> MResult<()> {
            *self.solve_count.borrow_mut() += 1;
            Ok(())
        }
        fn out(&self) -> LegacyValue {
            LegacyValue::F64(self.output.clone())
        }
        fn to_string(&self) -> String {
            "RestoreProbe".into()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(self.reactive_output_values())
        }
    }

    #[cfg(feature = "compiler")]
    impl mech_core::MechFunctionCompiler for RestoreProbe {
        fn compile(
            &self,
            _context: &mut dyn BytecodeCompilerContext,
        ) -> MResult<mech_core::Register> {
            Ok(0)
        }
    }

    struct UnsupportedCheckpointFunction;

    impl MechFunctionImpl for UnsupportedCheckpointFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }
        fn out(&self) -> LegacyValue {
            LegacyValue::Empty
        }
        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Err(MechError::new(
                mech_core::TransactionStateUnsupportedError {
                    function: self.to_string(),
                    reason: "opaque test state".into(),
                },
                None,
            ))
        }
        fn to_string(&self) -> String {
            "UnsupportedCheckpointFunction".into()
        }
    }

    #[cfg(feature = "compiler")]
    impl mech_core::MechFunctionCompiler for UnsupportedCheckpointFunction {
        fn compile(
            &self,
            _context: &mut dyn BytecodeCompilerContext,
        ) -> MResult<mech_core::Register> {
            Ok(0)
        }
    }

    fn scalar_cell(program: &MechProgram, name: &str) -> (ValRef, Ref<f64>) {
        let outer = program
            .interpreter()
            .symbols()
            .borrow()
            .get(hash_str(name))
            .expect("symbol");
        let inner = match &*outer.borrow() {
            LegacyValue::F64(value) => value.clone(),
            other => panic!("expected f64 symbol, got {other:?}"),
        };
        (outer, inner)
    }

    fn activation_capture(
        program: &MechProgram,
        arm_index: usize,
        capture_index: usize,
    ) -> (ReactiveCellId, LegacyValue) {
        let plan = program.interpreter().plan();
        let registration = {
            let registrations = plan.pattern_activation_registrations();
            assert_eq!(registrations.len(), 1);
            registrations[0].clone()
        };
        let arm = registration.arms.get(arm_index).expect("activation arm");
        let capture = arm.captures.get(capture_index).expect("activation capture");
        let capture_cell = capture.cell;
        let capture_value = plan
            .borrow()
            .node(arm.gate_node)
            .expect("activation gate")
            .function
            .reactive_output_values()
            .into_iter()
            .find(|value| value.reactive_root_cell_ids().contains(&capture_cell))
            .expect("committed activation capture");
        (capture_cell, capture_value)
    }

    fn tuple_f64_backing(tuple: &Ref<MechTuple>, index: usize) -> Ref<f64> {
        let tuple = tuple.borrow();
        match tuple.elements.get(index).map(|element| element.as_ref()) {
            Some(LegacyValue::F64(backing)) => backing.clone(),
            other => panic!("expected tuple f64 backing, got {other:?}"),
        }
    }

    fn insert_interpreter_scalar(
        interpreter: &Interpreter,
        name: &str,
        value: f64,
    ) -> (ValRef, Ref<f64>) {
        let id = hash_str(name);
        let inner = Ref::new(value);
        let outer =
            interpreter
                .symbols()
                .borrow_mut()
                .insert(id, LegacyValue::F64(inner.clone()), true);
        interpreter
            .symbols()
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name.to_string());
        interpreter
            .dictionary()
            .borrow_mut()
            .insert(id, name.to_string());
        (outer, inner)
    }

    #[test]
    fn program_checkpoint_restores_config_structure_values_and_identity() {
        let mut program = test_mech_program(MechProgramConfig {
            name: "retained".into(),
            environment: MechProgramEnvironment::default(),
        });
        program.run_string("x := 1.0\ny := x + 1.0").unwrap();

        let catalog = Arc::clone(program.function_catalog());
        let symbols_addr = program.interpreter().symbols().addr();
        let original_plan = program.interpreter().plan();
        let plan_addr = original_plan.0.addr();
        let original_plan_len = original_plan.borrow().len();
        let (x_outer, x_inner) = scalar_cell(&program, "x");
        let x_outer_addr = x_outer.addr();
        let x_inner_addr = x_inner.addr();
        let checkpoint = program.checkpoint().unwrap();

        program.config.name = "changed".into();
        program.config.environment.rounds_per_step = 17;
        *x_inner.borrow_mut() = 99.0;
        program.run_string("temporary := x + 10.0").unwrap();
        assert!(program.interpreter().plan_len() > original_plan_len);

        program.restore(checkpoint).unwrap();

        assert_eq!(program.config.name, "retained");
        assert_eq!(program.config.environment.rounds_per_step, 10_000);
        assert!(Arc::ptr_eq(program.function_catalog(), &catalog));
        assert_eq!(program.interpreter().symbols().addr(), symbols_addr);
        assert_eq!(program.interpreter().plan().0.addr(), plan_addr);
        assert_eq!(program.interpreter().plan_len(), original_plan_len);
        assert!(
            program
                .interpreter()
                .symbols()
                .borrow()
                .get(hash_str("temporary"))
                .is_none()
        );
        let (restored_outer, restored_inner) = scalar_cell(&program, "x");
        assert_eq!(restored_outer.addr(), x_outer_addr);
        assert_eq!(restored_inner.addr(), x_inner_addr);
        assert_eq!(*restored_inner.borrow(), 1.0);
    }

    #[cfg(all(
        feature = "compiler",
        feature = "functions",
        feature = "symbol_table",
        feature = "f64",
    ))]
    #[test]
    fn bytecode_compilation_preserves_checkpointability_and_execution_state() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("x := 1.0\ny := x + 2.0").unwrap();

        let checkpoint_before_compilation = program.checkpoint().unwrap();
        let plan = program.interpreter().plan();
        let plan_address = plan.0.addr();
        let plan_length = plan.borrow().len();
        let symbols_address = program.interpreter().symbols().addr();
        let output_before = program.interpreter().out.to_string();
        let (_, x_before) = scalar_cell(&program, "x");
        let (_, y_before) = scalar_cell(&program, "y");

        let bytecode = program.compile_bytecode().unwrap();
        assert!(!bytecode.is_empty());
        let _checkpoint_after_compilation = program.checkpoint().unwrap();

        assert_eq!(program.interpreter().plan().0.addr(), plan_address);
        assert_eq!(program.interpreter().plan_len(), plan_length);
        assert_eq!(program.interpreter().symbols().addr(), symbols_address);
        assert_eq!(program.interpreter().out.to_string(), output_before);
        assert_eq!(*x_before.borrow(), 1.0);
        assert_eq!(*y_before.borrow(), 3.0);

        program.restore(checkpoint_before_compilation).unwrap();
        assert_eq!(program.interpreter().plan().0.addr(), plan_address);
        assert_eq!(program.interpreter().plan_len(), plan_length);
        let (_, restored_x) = scalar_cell(&program, "x");
        let (_, restored_y) = scalar_cell(&program, "y");
        assert_eq!(*restored_x.borrow(), 1.0);
        assert_eq!(*restored_y.borrow(), 3.0);
    }

    #[test]
    fn program_checkpoint_restores_activation_capture_identity_and_payload() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program
            .run_string(
                r#"
event := (1.0, 2.0)
~> event
  | whole => {
      selected := whole
    }
  | * => {
      selected := (-1.0, -1.0)
    }
"#,
            )
            .unwrap();

        let (capture_cell, capture_value) = activation_capture(&program, 0, 0);
        let LegacyValue::Tuple(capture_outer) = capture_value else {
            panic!("expected tuple activation capture")
        };
        let first_backing = tuple_f64_backing(&capture_outer, 0);
        let second_backing = tuple_f64_backing(&capture_outer, 1);
        let outer_addr = capture_outer.addr();
        let first_backing_addr = first_backing.addr();
        let second_backing_addr = second_backing.addr();
        assert_eq!(capture_cell, ReactiveCellId::new(capture_outer.id()));
        assert_eq!(*first_backing.borrow(), 1.0);
        assert_eq!(*second_backing.borrow(), 2.0);
        let checkpoint = program.checkpoint().unwrap();

        *first_backing.borrow_mut() = 99.0;
        *second_backing.borrow_mut() = 100.0;
        *capture_outer.borrow_mut() = MechTuple::from_vec(vec![
            LegacyValue::F64(Ref::new(101.0)),
            LegacyValue::F64(Ref::new(102.0)),
        ]);

        program.restore(checkpoint).unwrap();

        let (restored_cell, restored_value) = activation_capture(&program, 0, 0);
        let LegacyValue::Tuple(restored_outer) = restored_value else {
            panic!("expected restored tuple activation capture")
        };
        let restored_first = tuple_f64_backing(&restored_outer, 0);
        let restored_second = tuple_f64_backing(&restored_outer, 1);
        assert_eq!(restored_cell, capture_cell);
        assert_eq!(restored_outer.addr(), outer_addr);
        assert_eq!(restored_first.addr(), first_backing_addr);
        assert_eq!(restored_second.addr(), second_backing_addr);
        assert_eq!(*restored_first.borrow(), 1.0);
        assert_eq!(*restored_second.borrow(), 2.0);
    }

    #[test]
    fn program_checkpoint_restore_preflights_everything_before_mutation() {
        let mut program = test_mech_program(MechProgramConfig::default());
        program.run_string("x := 1.0").unwrap();
        let (x_outer, x_inner) = scalar_cell(&program, "x");
        let checkpoint = program.checkpoint().unwrap();

        *x_inner.borrow_mut() = 42.0;
        program.config.name = "still-mutated-on-error".into();
        program.run_string("temporary := 2.0").unwrap();
        let held_late_borrow = x_outer.borrow();

        let error = program.restore(checkpoint.clone()).unwrap_err();
        assert_eq!(error.kind_name(), "ValueStateBorrowConflict");
        assert_eq!(program.config.name, "still-mutated-on-error");
        assert_eq!(*x_inner.borrow(), 42.0);
        assert!(
            program
                .interpreter()
                .symbols()
                .borrow()
                .get(hash_str("temporary"))
                .is_some()
        );

        drop(held_late_borrow);
        program.restore(checkpoint).unwrap();
        assert_eq!(program.config.name, "program");
        let (_, restored_inner) = scalar_cell(&program, "x");
        assert_eq!(*restored_inner.borrow(), 1.0);
        assert!(
            program
                .interpreter()
                .symbols()
                .borrow()
                .get(hash_str("temporary"))
                .is_none()
        );
    }

    #[test]
    fn program_checkpoint_restores_removed_recursive_interpreters_with_identity() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let child_id = 101;
        let grandchild_id = 202;

        let child = Interpreter::new(child_id, 10_000);
        let (child_outer, child_inner) = insert_interpreter_scalar(&child, "child-value", 1.0);
        let grandchild = Interpreter::new(grandchild_id, 10_000);
        let (grandchild_outer, grandchild_inner) =
            insert_interpreter_scalar(&grandchild, "grandchild-value", 2.0);
        let grandchild_ref = Ref::new(Box::new(grandchild));
        let grandchild_handle_addr = grandchild_ref.addr();
        child
            .sub_interpreters
            .borrow_mut()
            .insert(grandchild_id, grandchild_ref.clone());
        let child_ref = Ref::new(Box::new(child));
        let child_handle_addr = child_ref.addr();
        program
            .interpreter()
            .sub_interpreters
            .borrow_mut()
            .insert(child_id, child_ref.clone());

        let checkpoint = program.checkpoint().unwrap();

        let removed_child = program
            .interpreter()
            .sub_interpreters
            .borrow_mut()
            .remove(&child_id)
            .unwrap();
        removed_child
            .borrow()
            .sub_interpreters
            .borrow_mut()
            .remove(&grandchild_id);
        *child_inner.borrow_mut() = 10.0;
        *grandchild_inner.borrow_mut() = 20.0;
        program.interpreter().sub_interpreters.borrow_mut().insert(
            child_id,
            Ref::new(Box::new(Interpreter::new(child_id, 10_000))),
        );
        program
            .interpreter()
            .sub_interpreters
            .borrow_mut()
            .insert(303, Ref::new(Box::new(Interpreter::new(303, 10_000))));

        program.restore(checkpoint).unwrap();

        let restored_child = program
            .interpreter()
            .sub_interpreters
            .borrow()
            .get(&child_id)
            .unwrap()
            .clone();
        assert_eq!(program.interpreter().sub_interpreters.borrow().len(), 1);
        assert_eq!(restored_child.addr(), child_handle_addr);
        assert_eq!(
            child_outer.addr(),
            restored_child
                .borrow()
                .symbols()
                .borrow()
                .get(hash_str("child-value"))
                .unwrap()
                .addr()
        );
        assert_eq!(*child_inner.borrow(), 1.0);

        let restored_grandchild = restored_child
            .borrow()
            .sub_interpreters
            .borrow()
            .get(&grandchild_id)
            .unwrap()
            .clone();
        assert_eq!(restored_grandchild.addr(), grandchild_handle_addr);
        assert_eq!(
            grandchild_outer.addr(),
            restored_grandchild
                .borrow()
                .symbols()
                .borrow()
                .get(hash_str("grandchild-value"))
                .unwrap()
                .addr(),
        );
        assert_eq!(*grandchild_inner.borrow(), 2.0);
    }

    #[test]
    fn program_restore_does_not_execute_functions_and_preserves_output_identity() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let output = Ref::new(1.0);
        let solve_count = Rc::new(RefCell::new(0));
        program
            .interpreter()
            .plan()
            .add_function(Box::new(RestoreProbe {
                output: output.clone(),
                solve_count: solve_count.clone(),
            }));
        let checkpoint = program.checkpoint().unwrap();
        *output.borrow_mut() = 99.0;

        program.restore(checkpoint).unwrap();

        assert_eq!(*solve_count.borrow(), 0);
        assert_eq!(*output.borrow(), 1.0);
    }

    #[test]
    fn unsupported_function_state_fails_checkpoint_creation_without_changes() {
        let program = test_mech_program(MechProgramConfig::default());
        let plan = program.interpreter().plan();
        plan.add_function(Box::new(UnsupportedCheckpointFunction));
        let plan_len = plan.len();

        let error = match program.checkpoint() {
            Ok(_) => panic!("unsupported checkpoint unexpectedly succeeded"),
            Err(error) => error,
        };

        assert_eq!(error.kind_name(), "TransactionStateUnsupported");
        assert_eq!(program.interpreter().plan_len(), plan_len);
    }

    #[test]
    fn program_checkpoint_restores_nested_container_and_matrix_topology() {
        let mut program = test_mech_program(MechProgramConfig::default());
        let shared = Ref::new(1.0);
        let matrix = <f64 as ToMatrix>::to_matrixd(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let record = Ref::new(MechRecord::new(vec![
            ("shared", LegacyValue::F64(shared.clone())),
            ("matrix", LegacyValue::MatrixF64(matrix.clone())),
        ]));
        let record_addr = record.addr();
        let matrix_cells = LegacyValue::MatrixF64(matrix.clone()).reactive_cell_ids();
        let record_outer = program.interpreter().symbols().borrow_mut().insert(
            hash_str("nested"),
            LegacyValue::Record(record.clone()),
            true,
        );
        let checkpoint = program.checkpoint().unwrap();

        *shared.borrow_mut() = 10.0;
        matrix.set(vec![10.0, 20.0, 30.0, 40.0]);
        *record.borrow_mut() =
            MechRecord::new(vec![("replacement", LegacyValue::F64(Ref::new(99.0)))]);

        program.restore(checkpoint).unwrap();

        assert_eq!(record.addr(), record_addr);
        assert_eq!(*shared.borrow(), 1.0);
        assert_eq!(matrix.as_vec(), vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(
            LegacyValue::MatrixF64(matrix.clone()).reactive_cell_ids(),
            matrix_cells,
        );
        assert_eq!(record.borrow().cols, 2);
        assert_eq!(
            record_outer.borrow().reactive_cell_ids(),
            LegacyValue::Record(record).reactive_cell_ids(),
        );
    }

    #[test]
    fn program_checkpoint_rejects_cross_program_restore_before_mutation() {
        let source = test_mech_program(MechProgramConfig {
            name: "source".into(),
            environment: MechProgramEnvironment::default(),
        });
        let checkpoint = source.checkpoint().unwrap();
        let mut target = test_mech_program(MechProgramConfig {
            name: "target".into(),
            environment: MechProgramEnvironment::default(),
        });
        let target_state_addr = target.interpreter().state.addr();

        let error = target.restore(checkpoint).unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterCheckpointOwnerMismatch");
        assert_eq!(target.config.name, "target");
        assert_eq!(target.interpreter().state.addr(), target_state_addr);
    }
}
