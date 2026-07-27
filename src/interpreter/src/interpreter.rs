use crate::*;
use std::cell::RefCell as StdRefCell;
use std::ops::Deref;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::time::Duration;
#[cfg(all(
  target_arch = "wasm32",
  target_os = "unknown",
))]
use web_time::Instant;

#[cfg(not(all(
  target_arch = "wasm32",
  target_os = "unknown",
)))]
use std::time::Instant;

// Interpreter
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeContextBinding {
  pub name: String,
  pub base_uri: String,
}

pub type InterpreterRef = Ref<Box<Interpreter>>;

pub struct Interpreter {
  pub id: u64,
  checkpoint_owner: Rc<()>,
  pub profile: bool,
  pub max_steps: usize,
  #[cfg(feature = "trace")]
  pub trace: bool,
  #[cfg(feature = "trace")]
  pub trace_to_stdout: bool,
  #[cfg(feature = "trace")]
  pub trace_events: Ref<Vec<TraceEvent>>,
  ip: usize, // instruction pointer
  pub state: Ref<ProgramState>,
  #[cfg(feature = "functions")]
  reactive_turn_state: ReactiveTurnState,
  #[cfg(feature = "functions")]
  pub(crate) persistent_user_function_plan_depth: Ref<usize>,
  pub(crate) deferred_expression_solve_depth: Ref<usize>,
  #[cfg(feature = "functions")]
  pub stack: Vec<Frame>,
  registers: Vec<Value>,
  constants: Vec<Value>,
  #[cfg(feature = "compiler")]
  pub context: Option<CompileCtx>,
  pub code: Vec<MechSourceCode>,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  #[cfg(feature = "subscript_formula")]
  pub string_access_live_values: Ref<std::collections::BTreeSet<usize>>,
  #[cfg(feature = "subscript_formula")]
  pub current_string_access_expression_live: Ref<bool>,
  pub inline_eval_counter: Ref<u64>,
  pub context_bindings: Ref<HashMap<u64, RuntimeContextBinding>>,
  pub module_manifests: Ref<ModuleManifestCatalog>,
  #[cfg(feature = "state_machines")]
  pub user_state_machines: Ref<HashMap<u64, FsmImplementation>>,
  #[cfg(feature = "state_machines")]
  pub user_state_machine_specs: Ref<HashMap<u64, FsmSpecification>>,
  pub sub_interpreters: Ref<HashMap<u64, InterpreterRef>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointUnsupportedCompilerContext {
  pub interpreter_id: u64,
}

impl MechErrorKind for InterpreterCheckpointUnsupportedCompilerContext {
  fn name(&self) -> &str {
    "InterpreterCheckpointUnsupportedCompilerContext"
  }

  fn message(&self) -> String {
    format!(
      "Interpreter {} has a retained compiler context that cannot be checkpointed safely.",
      self.interpreter_id,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointBorrowConflict {
  pub phase: &'static str,
  pub interpreter_id: u64,
  pub type_name: &'static str,
}

impl MechErrorKind for InterpreterCheckpointBorrowConflict {
  fn name(&self) -> &str {
    "InterpreterCheckpointBorrowConflict"
  }

  fn message(&self) -> String {
    format!(
      "Cannot borrow {} checkpoint target for interpreter {} during {}.",
      self.type_name,
      self.interpreter_id,
      self.phase,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointHierarchyAlias {
  pub interpreter_id: u64,
  pub child_id: u64,
}

impl MechErrorKind for InterpreterCheckpointHierarchyAlias {
  fn name(&self) -> &str {
    "InterpreterCheckpointHierarchyAlias"
  }

  fn message(&self) -> String {
    format!(
      "Interpreter {} contains a repeated or cyclic handle for child {}.",
      self.interpreter_id,
      self.child_id,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointOwnerMismatch {
  pub interpreter_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterCheckpointChildKeyMismatch {
  pub parent_id: u64,
  pub key: u64,
  pub child_id: u64,
}

impl MechErrorKind for InterpreterCheckpointChildKeyMismatch {
  fn name(&self) -> &str {
    "InterpreterCheckpointChildKeyMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Interpreter {} stores child {} under mismatched key {}.",
      self.parent_id,
      self.child_id,
      self.key,
    )
  }
}

impl MechErrorKind for InterpreterCheckpointOwnerMismatch {
  fn name(&self) -> &str {
    "InterpreterCheckpointOwnerMismatch"
  }

  fn message(&self) -> String {
    format!(
      "The checkpoint belongs to a different interpreter than interpreter {}.",
      self.interpreter_id,
    )
  }
}

fn checkpoint_borrow_error<T: 'static>(
  phase: &'static str,
  interpreter_id: u64,
  _target: &Ref<T>,
) -> MechError {
  MechError::new(
    InterpreterCheckpointBorrowConflict {
      phase,
      interpreter_id,
      type_name: core::any::type_name::<T>(),
    },
    None,
  )
  .with_compiler_loc()
}

fn checkpoint_clone_ref<T: Clone + 'static>(
  target: &Ref<T>,
  interpreter_id: u64,
) -> MResult<T> {
  target
    .try_borrow()
    .map(|value| value.clone())
    .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, target))
}

fn checkpoint_preflight_ref<T: 'static>(
  target: &Ref<T>,
  interpreter_id: u64,
) -> MResult<()> {
  target
    .try_borrow_mut()
    .map(|_| ())
    .map_err(|_| checkpoint_borrow_error("checkpoint-restore", interpreter_id, target))
}

#[cfg(feature = "functions")]
fn checkpoint_preflight_plan_capture(
  plan: &Plan,
  interpreter_id: u64,
) -> MResult<()> {
  plan
    .0
    .try_borrow()
    .map(|_| ())
    .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, &plan.0))?;
  plan
    .1
    .try_borrow()
    .map(|_| ())
    .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, &plan.1))?;
  plan.validate_checkpoint_invariants()
}

struct RefPayloadCheckpoint<T: Clone + 'static> {
  target: Ref<T>,
  payload: T,
}

impl<T: Clone + 'static> RefPayloadCheckpoint<T> {
  fn capture(target: &Ref<T>, interpreter_id: u64) -> MResult<Self> {
    Ok(Self {
      target: target.clone(),
      payload: checkpoint_clone_ref(target, interpreter_id)?,
    })
  }

  fn preflight(&self, interpreter_id: u64) -> MResult<()> {
    checkpoint_preflight_ref(&self.target, interpreter_id)
  }

  fn apply(&self) {
    *self.target.borrow_mut() = self.payload.clone();
  }
}

#[cfg(feature = "symbol_table")]
struct SymbolTableCellCheckpoint {
  target: SymbolTableRef,
  snapshot: SymbolTableSnapshot,
}

#[cfg(feature = "symbol_table")]
impl SymbolTableCellCheckpoint {
  fn capture(
    target: &SymbolTableRef,
    interpreter_id: u64,
    journal: &mut ValueStateJournal,
  ) -> MResult<Self> {
    let snapshot = {
      let table = target
        .try_borrow()
        .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, target))?;
      table
        .dictionary
        .try_borrow()
        .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, &table.dictionary))?;
      let snapshot = table.snapshot();
      for value in table.symbols.values() {
        journal.capture_val_ref(value)?;
      }
      for value in table.mutable_variables.values() {
        journal.capture_val_ref(value)?;
      }
      snapshot
    };
    Ok(Self {
      target: target.clone(),
      snapshot,
    })
  }

  fn preflight(&self, interpreter_id: u64) -> MResult<()> {
    checkpoint_preflight_ref(&self.target, interpreter_id)?;
    let table = self
      .target
      .try_borrow()
      .map_err(|_| checkpoint_borrow_error("checkpoint-restore", interpreter_id, &self.target))?;
    table.preflight_restore(&self.snapshot)
  }

  fn apply(&self) {
    self.target.borrow_mut().apply_restore(&self.snapshot);
  }
}

#[cfg(feature = "functions")]
struct FrameCellCheckpoint {
  locals: SymbolTableCellCheckpoint,
  plan: Plan,
  plan_checkpoint: PlanCheckpoint,
}

#[cfg(feature = "functions")]
impl FrameCellCheckpoint {
  fn capture(
    frame: &Frame,
    interpreter_id: u64,
    journal: &mut ValueStateJournal,
  ) -> MResult<Self> {
    let plan = frame.checkpoint_plan();
    checkpoint_preflight_plan_capture(&plan, interpreter_id)?;
    for value in plan.transaction_state_values()? {
      journal.capture_value(&value)?;
    }
    if let Some(value) = frame.checkpoint_out() {
      journal.capture_value(&value)?;
    }
    Ok(Self {
      locals: SymbolTableCellCheckpoint::capture(
        &frame.checkpoint_locals(),
        interpreter_id,
        journal,
      )?,
      plan_checkpoint: plan.checkpoint(),
      plan,
    })
  }

  fn preflight(&self, interpreter_id: u64) -> MResult<()> {
    self.locals.preflight(interpreter_id)?;
    self.plan.preflight_rollback(&self.plan_checkpoint)
  }

  fn apply_structure(&self) {
    self.locals.apply();
    self.plan.apply_rollback_structure(&self.plan_checkpoint);
  }

  fn rebuild_checkpoint_indexes(&self) {
    self.plan.rebuild_checkpoint_indexes();
    self
      .plan
      .validate_checkpoint_invariants()
      .expect("frame checkpoint preflight guarantees restored plan invariants");
  }
}

struct ProgramStateCheckpoint {
  target: Ref<ProgramState>,
  #[cfg(feature = "symbol_table")]
  symbol_table: SymbolTableCellCheckpoint,
  #[cfg(feature = "symbol_table")]
  environment: Option<SymbolTableCellCheckpoint>,
  #[cfg(feature = "functions")]
  functions: FunctionsRef,
  #[cfg(feature = "functions")]
  functions_snapshot: FunctionsSnapshot,
  #[cfg(feature = "functions")]
  plan: Plan,
  #[cfg(feature = "functions")]
  plan_checkpoint: PlanCheckpoint,
  kinds: KindTable,
  #[cfg(feature = "enum")]
  enums: EnumTable,
  #[cfg(feature = "enum")]
  enum_dictionaries: Vec<RefPayloadCheckpoint<Dictionary>>,
  #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
  invariants: InvariantTable,
  #[cfg(feature = "invariant_define")]
  invariant_violations: Vec<InvariantViolation>,
  #[cfg(feature = "invariant_define")]
  invariant_expressions: InvariantExpressionTable,
  #[cfg(feature = "invariant_define")]
  invariant_evaluations: InvariantEvaluationTable,
  dictionary: RefPayloadCheckpoint<Dictionary>,
}

impl ProgramStateCheckpoint {
  fn capture(
    target: &Ref<ProgramState>,
    interpreter_id: u64,
    journal: &mut ValueStateJournal,
  ) -> MResult<Self> {
    let state = target
      .try_borrow()
      .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter_id, target))?;

    #[cfg(feature = "symbol_table")]
    let symbol_table = SymbolTableCellCheckpoint::capture(
      &state.symbol_table,
      interpreter_id,
      journal,
    )?;
    #[cfg(feature = "symbol_table")]
    let environment = match &state.environment {
      Some(environment) => Some(SymbolTableCellCheckpoint::capture(
        environment,
        interpreter_id,
        journal,
      )?),
      None => None,
    };

    #[cfg(feature = "functions")]
    let functions = state.functions.clone();
    #[cfg(feature = "functions")]
    let functions_snapshot = FunctionsSnapshot::capture(&functions)?;
    #[cfg(feature = "functions")]
    for value in functions_snapshot.transaction_state_values()? {
      journal.capture_value(&value)?;
    }

    #[cfg(feature = "functions")]
    let plan = state.plan.clone();
    #[cfg(feature = "functions")]
    checkpoint_preflight_plan_capture(&plan, interpreter_id)?;
    #[cfg(feature = "functions")]
    for value in plan.transaction_state_values()? {
      journal.capture_value(&value)?;
    }
    #[cfg(feature = "functions")]
    let plan_checkpoint = plan.checkpoint();

    #[cfg(feature = "enum")]
    let enums = state.enums.clone();
    #[cfg(feature = "enum")]
    let mut enum_dictionaries = Vec::new();
    #[cfg(feature = "enum")]
    for enum_value in enums.values() {
      enum_dictionaries.push(RefPayloadCheckpoint::capture(
        &enum_value.names,
        interpreter_id,
      )?);
      for (_, payload) in &enum_value.variants {
        if let Some(payload) = payload {
          journal.capture_value(payload)?;
        }
      }
    }

    #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
    for (_, value) in state.invariants.values() {
      journal.capture_val_ref(value)?;
    }

    Ok(Self {
      target: target.clone(),
      #[cfg(feature = "symbol_table")]
      symbol_table,
      #[cfg(feature = "symbol_table")]
      environment,
      #[cfg(feature = "functions")]
      functions,
      #[cfg(feature = "functions")]
      functions_snapshot,
      #[cfg(feature = "functions")]
      plan,
      #[cfg(feature = "functions")]
      plan_checkpoint,
      kinds: state.kinds.clone(),
      #[cfg(feature = "enum")]
      enums,
      #[cfg(feature = "enum")]
      enum_dictionaries,
      #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
      invariants: state.invariants.clone(),
      #[cfg(feature = "invariant_define")]
      invariant_violations: state.invariant_violations.clone(),
      #[cfg(feature = "invariant_define")]
      invariant_expressions: state.invariant_expressions.clone(),
      #[cfg(feature = "invariant_define")]
      invariant_evaluations: state.invariant_evaluations.clone(),
      dictionary: RefPayloadCheckpoint::capture(&state.dictionary, interpreter_id)?,
    })
  }

  fn preflight(&self, interpreter_id: u64) -> MResult<()> {
    checkpoint_preflight_ref(&self.target, interpreter_id)?;
    #[cfg(feature = "symbol_table")]
    self.symbol_table.preflight(interpreter_id)?;
    #[cfg(feature = "symbol_table")]
    if let Some(environment) = &self.environment {
      environment.preflight(interpreter_id)?;
    }
    #[cfg(feature = "functions")]
    self.functions_snapshot.preflight_restore()?;
    #[cfg(feature = "functions")]
    self.plan.preflight_rollback(&self.plan_checkpoint)?;
    #[cfg(feature = "enum")]
    for dictionary in &self.enum_dictionaries {
      dictionary.preflight(interpreter_id)?;
    }
    self.dictionary.preflight(interpreter_id)
  }

  fn apply_structure(&self) {
    {
      let mut state = self.target.borrow_mut();
      #[cfg(feature = "symbol_table")]
      {
        state.symbol_table = self.symbol_table.target.clone();
        state.environment = self
          .environment
          .as_ref()
          .map(|environment| environment.target.clone());
      }
      #[cfg(feature = "functions")]
      {
        state.functions = self.functions.clone();
        state.plan = self.plan.clone();
      }
      state.kinds = self.kinds.clone();
      #[cfg(feature = "enum")]
      {
        state.enums = self.enums.clone();
      }
      #[cfg(all(feature = "invariant_define", feature = "symbol_table"))]
      {
        state.invariants = self.invariants.clone();
      }
      #[cfg(feature = "invariant_define")]
      {
        state.invariant_violations = self.invariant_violations.clone();
        state.invariant_expressions = self.invariant_expressions.clone();
        state.invariant_evaluations = self.invariant_evaluations.clone();
      }
      state.dictionary = self.dictionary.target.clone();
    }

    #[cfg(feature = "symbol_table")]
    self.symbol_table.apply();
    #[cfg(feature = "symbol_table")]
    if let Some(environment) = &self.environment {
      environment.apply();
    }
    #[cfg(feature = "functions")]
    self.functions_snapshot.apply_restore_structure();
    #[cfg(feature = "functions")]
    self.plan.apply_rollback_structure(&self.plan_checkpoint);
    #[cfg(feature = "enum")]
    for dictionary in &self.enum_dictionaries {
      dictionary.apply();
    }
    self.dictionary.apply();
  }

  #[cfg(feature = "functions")]
  fn rebuild_checkpoint_indexes(&self, turn_state: &ReactiveTurnState) {
    self.functions_snapshot.rebuild_checkpoint_indexes();
    self.plan.rebuild_checkpoint_indexes();
    self
      .functions_snapshot
      .validate_checkpoint_invariants()
      .expect("function checkpoint preflight guarantees restored plan invariants");
    self
      .plan
      .validate_checkpoint_invariants()
      .expect("program checkpoint preflight guarantees restored plan invariants");
    self
      .plan
      .validate_checkpoint_turn_state(turn_state)
      .expect("program checkpoint preflight guarantees restored turn invariants");
  }
}

struct ChildInterpreterCheckpoint {
  target: InterpreterRef,
  checkpoint: Box<InterpreterStructureCheckpoint>,
}

struct InterpreterStructureCheckpoint {
  owner: Rc<()>,
  id: u64,
  profile: bool,
  max_steps: usize,
  ip: usize,
  #[cfg(feature = "trace")]
  trace: bool,
  #[cfg(feature = "trace")]
  trace_to_stdout: bool,
  #[cfg(feature = "trace")]
  trace_events: RefPayloadCheckpoint<Vec<TraceEvent>>,
  state: ProgramStateCheckpoint,
  #[cfg(feature = "functions")]
  reactive_turn_state: ReactiveTurnState,
  #[cfg(feature = "functions")]
  persistent_user_function_plan_depth: RefPayloadCheckpoint<usize>,
  deferred_expression_solve_depth: RefPayloadCheckpoint<usize>,
  #[cfg(feature = "functions")]
  stack: Vec<Frame>,
  #[cfg(feature = "functions")]
  frame_checkpoints: Vec<FrameCellCheckpoint>,
  registers: Vec<Value>,
  constants: Vec<Value>,
  code: Vec<MechSourceCode>,
  out: Value,
  out_values: RefPayloadCheckpoint<HashMap<u64, Value>>,
  #[cfg(feature = "subscript_formula")]
  string_access_live_values: RefPayloadCheckpoint<std::collections::BTreeSet<usize>>,
  #[cfg(feature = "subscript_formula")]
  current_string_access_expression_live: RefPayloadCheckpoint<bool>,
  inline_eval_counter: RefPayloadCheckpoint<u64>,
  context_bindings: RefPayloadCheckpoint<HashMap<u64, RuntimeContextBinding>>,
  module_manifests: RefPayloadCheckpoint<ModuleManifestCatalog>,
  #[cfg(feature = "state_machines")]
  user_state_machines: RefPayloadCheckpoint<HashMap<u64, FsmImplementation>>,
  #[cfg(feature = "state_machines")]
  user_state_machine_specs: RefPayloadCheckpoint<HashMap<u64, FsmSpecification>>,
  sub_interpreters: Ref<HashMap<u64, InterpreterRef>>,
  sub_interpreter_entries: HashMap<u64, InterpreterRef>,
  children: Vec<ChildInterpreterCheckpoint>,
}

impl InterpreterStructureCheckpoint {
  fn capture(
    interpreter: &Interpreter,
    journal: &mut ValueStateJournal,
    seen_children: &mut Vec<InterpreterRef>,
  ) -> MResult<Self> {
    #[cfg(feature = "compiler")]
    if interpreter.context.is_some() {
      return Err(MechError::new(
        InterpreterCheckpointUnsupportedCompilerContext {
          interpreter_id: interpreter.id,
        },
        None,
      )
      .with_compiler_loc());
    }

    let state = ProgramStateCheckpoint::capture(
      &interpreter.state,
      interpreter.id,
      journal,
    )?;
    #[cfg(feature = "functions")]
    state
      .plan
      .validate_checkpoint_turn_state(&interpreter.reactive_turn_state)?;

    for value in &interpreter.registers {
      journal.capture_value(value)?;
    }
    for value in &interpreter.constants {
      journal.capture_value(value)?;
    }
    journal.capture_value(&interpreter.out)?;

    let out_values = RefPayloadCheckpoint::capture(
      &interpreter.out_values,
      interpreter.id,
    )?;
    for value in out_values.payload.values() {
      journal.capture_value(value)?;
    }

    #[cfg(feature = "functions")]
    let mut frame_checkpoints = Vec::with_capacity(interpreter.stack.len());
    #[cfg(feature = "functions")]
    for frame in &interpreter.stack {
      frame_checkpoints.push(FrameCellCheckpoint::capture(
        frame,
        interpreter.id,
        journal,
      )?);
    }

    let sub_interpreter_entries = checkpoint_clone_ref(
      &interpreter.sub_interpreters,
      interpreter.id,
    )?;
    let mut children = Vec::with_capacity(sub_interpreter_entries.len());
    for (child_id, child) in &sub_interpreter_entries {
      if seen_children.iter().any(|seen| seen.same_handle(child)) {
        return Err(MechError::new(
          InterpreterCheckpointHierarchyAlias {
            interpreter_id: interpreter.id,
            child_id: *child_id,
          },
          None,
        )
        .with_compiler_loc());
      }
      seen_children.push(child.clone());
      let child_borrow = child
        .try_borrow()
        .map_err(|_| checkpoint_borrow_error("checkpoint-capture", interpreter.id, child))?;
      if child_borrow.id != *child_id {
        return Err(MechError::new(
          InterpreterCheckpointChildKeyMismatch {
            parent_id: interpreter.id,
            key: *child_id,
            child_id: child_borrow.id,
          },
          None,
        )
        .with_compiler_loc());
      }
      let checkpoint = Self::capture(
        child_borrow.as_ref(),
        journal,
        seen_children,
      )?;
      children.push(ChildInterpreterCheckpoint {
        target: child.clone(),
        checkpoint: Box::new(checkpoint),
      });
    }

    Ok(Self {
      owner: interpreter.checkpoint_owner.clone(),
      id: interpreter.id,
      profile: interpreter.profile,
      max_steps: interpreter.max_steps,
      ip: interpreter.ip,
      #[cfg(feature = "trace")]
      trace: interpreter.trace,
      #[cfg(feature = "trace")]
      trace_to_stdout: interpreter.trace_to_stdout,
      #[cfg(feature = "trace")]
      trace_events: RefPayloadCheckpoint::capture(
        &interpreter.trace_events,
        interpreter.id,
      )?,
      state,
      #[cfg(feature = "functions")]
      reactive_turn_state: interpreter.reactive_turn_state.clone(),
      #[cfg(feature = "functions")]
      persistent_user_function_plan_depth: RefPayloadCheckpoint::capture(
        &interpreter.persistent_user_function_plan_depth,
        interpreter.id,
      )?,
      deferred_expression_solve_depth: RefPayloadCheckpoint::capture(
        &interpreter.deferred_expression_solve_depth,
        interpreter.id,
      )?,
      #[cfg(feature = "functions")]
      stack: interpreter.stack.clone(),
      #[cfg(feature = "functions")]
      frame_checkpoints,
      registers: interpreter.registers.clone(),
      constants: interpreter.constants.clone(),
      code: interpreter.code.clone(),
      out: interpreter.out.clone(),
      out_values,
      #[cfg(feature = "subscript_formula")]
      string_access_live_values: RefPayloadCheckpoint::capture(
        &interpreter.string_access_live_values,
        interpreter.id,
      )?,
      #[cfg(feature = "subscript_formula")]
      current_string_access_expression_live: RefPayloadCheckpoint::capture(
        &interpreter.current_string_access_expression_live,
        interpreter.id,
      )?,
      inline_eval_counter: RefPayloadCheckpoint::capture(
        &interpreter.inline_eval_counter,
        interpreter.id,
      )?,
      context_bindings: RefPayloadCheckpoint::capture(
        &interpreter.context_bindings,
        interpreter.id,
      )?,
      module_manifests: RefPayloadCheckpoint::capture(
        &interpreter.module_manifests,
        interpreter.id,
      )?,
      #[cfg(feature = "state_machines")]
      user_state_machines: RefPayloadCheckpoint::capture(
        &interpreter.user_state_machines,
        interpreter.id,
      )?,
      #[cfg(feature = "state_machines")]
      user_state_machine_specs: RefPayloadCheckpoint::capture(
        &interpreter.user_state_machine_specs,
        interpreter.id,
      )?,
      sub_interpreters: interpreter.sub_interpreters.clone(),
      sub_interpreter_entries,
      children,
    })
  }

  fn preflight(&self) -> MResult<()> {
    self.state.preflight(self.id)?;
    #[cfg(feature = "trace")]
    self.trace_events.preflight(self.id)?;
    #[cfg(feature = "functions")]
    self.persistent_user_function_plan_depth.preflight(self.id)?;
    self.deferred_expression_solve_depth.preflight(self.id)?;
    self.out_values.preflight(self.id)?;
    #[cfg(feature = "subscript_formula")]
    self.string_access_live_values.preflight(self.id)?;
    #[cfg(feature = "subscript_formula")]
    self.current_string_access_expression_live.preflight(self.id)?;
    self.inline_eval_counter.preflight(self.id)?;
    self.context_bindings.preflight(self.id)?;
    self.module_manifests.preflight(self.id)?;
    #[cfg(feature = "state_machines")]
    self.user_state_machines.preflight(self.id)?;
    #[cfg(feature = "state_machines")]
    self.user_state_machine_specs.preflight(self.id)?;
    checkpoint_preflight_ref(&self.sub_interpreters, self.id)?;
    #[cfg(feature = "functions")]
    for frame in &self.frame_checkpoints {
      frame.preflight(self.id)?;
    }
    for child in &self.children {
      checkpoint_preflight_ref(&child.target, self.id)?;
      let child_borrow = child
        .target
        .try_borrow()
        .map_err(|_| checkpoint_borrow_error("checkpoint-restore", self.id, &child.target))?;
      child.checkpoint.preflight_owner(child_borrow.as_ref())?;
      child.checkpoint.preflight()?;
    }
    Ok(())
  }

  fn preflight_owner(&self, interpreter: &Interpreter) -> MResult<()> {
    if Rc::ptr_eq(&self.owner, &interpreter.checkpoint_owner) {
      Ok(())
    } else {
      Err(MechError::new(
        InterpreterCheckpointOwnerMismatch {
          interpreter_id: interpreter.id,
        },
        None,
      )
      .with_compiler_loc())
    }
  }

  fn apply_structure(&self, interpreter: &mut Interpreter) {
    interpreter.id = self.id;
    interpreter.profile = self.profile;
    interpreter.max_steps = self.max_steps;
    interpreter.ip = self.ip;
    #[cfg(feature = "trace")]
    {
      interpreter.trace = self.trace;
      interpreter.trace_to_stdout = self.trace_to_stdout;
      interpreter.trace_events = self.trace_events.target.clone();
    }
    interpreter.state = self.state.target.clone();
    #[cfg(feature = "functions")]
    {
      interpreter.reactive_turn_state = self.reactive_turn_state.clone();
      interpreter.persistent_user_function_plan_depth =
        self.persistent_user_function_plan_depth.target.clone();
      interpreter.stack = self.stack.clone();
    }
    interpreter.deferred_expression_solve_depth =
      self.deferred_expression_solve_depth.target.clone();
    interpreter.registers = self.registers.clone();
    interpreter.constants = self.constants.clone();
    #[cfg(feature = "compiler")]
    {
      interpreter.context = None;
    }
    interpreter.code = self.code.clone();
    interpreter.out = self.out.clone();
    interpreter.out_values = self.out_values.target.clone();
    #[cfg(feature = "subscript_formula")]
    {
      interpreter.string_access_live_values = self.string_access_live_values.target.clone();
      interpreter.current_string_access_expression_live =
        self.current_string_access_expression_live.target.clone();
    }
    interpreter.inline_eval_counter = self.inline_eval_counter.target.clone();
    interpreter.context_bindings = self.context_bindings.target.clone();
    interpreter.module_manifests = self.module_manifests.target.clone();
    #[cfg(feature = "state_machines")]
    {
      interpreter.user_state_machines = self.user_state_machines.target.clone();
      interpreter.user_state_machine_specs = self.user_state_machine_specs.target.clone();
    }
    interpreter.sub_interpreters = self.sub_interpreters.clone();

    self.state.apply_structure();
    #[cfg(feature = "trace")]
    self.trace_events.apply();
    #[cfg(feature = "functions")]
    self.persistent_user_function_plan_depth.apply();
    self.deferred_expression_solve_depth.apply();
    self.out_values.apply();
    #[cfg(feature = "subscript_formula")]
    self.string_access_live_values.apply();
    #[cfg(feature = "subscript_formula")]
    self.current_string_access_expression_live.apply();
    self.inline_eval_counter.apply();
    self.context_bindings.apply();
    self.module_manifests.apply();
    #[cfg(feature = "state_machines")]
    self.user_state_machines.apply();
    #[cfg(feature = "state_machines")]
    self.user_state_machine_specs.apply();
    #[cfg(feature = "functions")]
    for frame in &self.frame_checkpoints {
      frame.apply_structure();
    }

    *self.sub_interpreters.borrow_mut() = self.sub_interpreter_entries.clone();
    for child in &self.children {
      let mut child_borrow = child.target.borrow_mut();
      child.checkpoint.apply_structure(child_borrow.as_mut());
    }
  }

  fn rebuild_checkpoint_indexes(&self) {
    #[cfg(feature = "functions")]
    {
      self
        .state
        .rebuild_checkpoint_indexes(&self.reactive_turn_state);
      for frame in &self.frame_checkpoints {
        frame.rebuild_checkpoint_indexes();
      }
    }
    for child in &self.children {
      child.checkpoint.rebuild_checkpoint_indexes();
    }
  }
}

struct InterpreterCheckpointData {
  structure: InterpreterStructureCheckpoint,
  journal: ValueStateJournal,
}

/// A process-local explicit savepoint for retained interpreter state.
///
/// This type is intentionally opaque and is not a serializable history record
/// or a source of durable transaction identities.
#[derive(Clone)]
pub struct InterpreterCheckpoint {
  data: Rc<InterpreterCheckpointData>,
}

/// An opaque checkpoint of scheduler metadata for one reactive operation.
///
/// Function values are intentionally excluded and are captured lazily through
/// a reactive journal participant only when their functions execute.
#[cfg(feature = "functions")]
pub struct InterpreterReactiveTurnCheckpoint {
  owner: Rc<()>,
  interpreter_id: u64,
  plan: Plan,
  plan_node_len: usize,
  activation_registration_depth: usize,
  reactive_turn_state: ReactiveTurnState,
  #[cfg(feature = "trace")]
  trace_events: Ref<Vec<TraceEvent>>,
  #[cfg(feature = "trace")]
  trace_event_len: usize,
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnOwnerMismatch {
  pub interpreter_id: u64,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnOwnerMismatch {
  fn name(&self) -> &str {
    "InterpreterReactiveTurnOwnerMismatch"
  }

  fn message(&self) -> String {
    format!(
      "The reactive-turn checkpoint belongs to a different interpreter than interpreter {}.",
      self.interpreter_id,
    )
  }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnInvariant {
  pub interpreter_id: u64,
  pub reason: String,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnInvariant {
  fn name(&self) -> &str {
    "InterpreterReactiveTurnInvariant"
  }

  fn message(&self) -> String {
    format!(
      "Interpreter {} cannot restore its reactive-turn scheduler state: {}.",
      self.interpreter_id,
      self.reason,
    )
  }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnBorrowConflict {
  pub phase: &'static str,
  pub interpreter_id: u64,
  pub component: &'static str,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnBorrowConflict {
  fn name(&self) -> &str {
    "InterpreterReactiveTurnBorrowConflict"
  }

  fn message(&self) -> String {
    format!(
      "Cannot borrow {} for interpreter {} during reactive-turn {}.",
      self.component,
      self.interpreter_id,
      self.phase,
    )
  }
}

#[cfg(feature = "functions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpreterReactiveTurnRollbackFailed {
  pub interpreter_id: u64,
  pub original_error: String,
  pub rollback_error: String,
}

#[cfg(feature = "functions")]
impl MechErrorKind for InterpreterReactiveTurnRollbackFailed {
  fn name(&self) -> &str {
    "InterpreterReactiveTurnRollbackFailed"
  }

  fn message(&self) -> String {
    format!(
      "Interpreter {} reactive-turn rollback failed after {}: {}.",
      self.interpreter_id,
      self.original_error,
      self.rollback_error,
    )
  }
}

impl Clone for Interpreter {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      checkpoint_owner: Rc::new(()),
      ip: self.ip,
      profile: false,
      max_steps: self.max_steps,
      #[cfg(feature = "trace")]
      trace: self.trace,
      #[cfg(feature = "trace")]
      trace_to_stdout: self.trace_to_stdout,
      #[cfg(feature = "trace")]
      trace_events: self.trace_events.clone(),
      state: Ref::new(self.state.borrow().clone()),
      #[cfg(feature = "functions")]
      reactive_turn_state: self.reactive_turn_state.clone(),
      #[cfg(feature = "functions")]
      persistent_user_function_plan_depth: self.persistent_user_function_plan_depth.clone(),
      deferred_expression_solve_depth: self.deferred_expression_solve_depth.clone(),
      #[cfg(feature = "functions")]
      stack: self.stack.clone(),
      registers: self.registers.clone(),
      constants: self.constants.clone(),
      #[cfg(feature = "compiler")]
      context: None,
      code: self.code.clone(),
      out: self.out.clone(),
      out_values: self.out_values.clone(),
      #[cfg(feature = "subscript_formula")]
      string_access_live_values: Ref::new(self.string_access_live_values.borrow().clone()),
      #[cfg(feature = "subscript_formula")]
      current_string_access_expression_live: Ref::new(*self.current_string_access_expression_live.borrow()),
      inline_eval_counter: self.inline_eval_counter.clone(),
      context_bindings: self.context_bindings.clone(),
      module_manifests: self.module_manifests.clone(),
      #[cfg(feature = "state_machines")]
      user_state_machines: self.user_state_machines.clone(),
      #[cfg(feature = "state_machines")]
      user_state_machine_specs: self.user_state_machine_specs.clone(),
      sub_interpreters: self.sub_interpreters.clone(),
    }
  }
}

impl Interpreter {
  #[cfg(feature = "functions")]
  fn reactive_turn_invariant(&self, reason: impl Into<String>) -> MechError {
    MechError::new(
      InterpreterReactiveTurnInvariant {
        interpreter_id: self.id,
        reason: reason.into(),
      },
      None,
    )
    .with_compiler_loc()
  }

  #[cfg(feature = "functions")]
  fn reactive_turn_borrow_conflict(
    &self,
    phase: &'static str,
    component: &'static str,
  ) -> MechError {
    MechError::new(
      InterpreterReactiveTurnBorrowConflict {
        phase,
        interpreter_id: self.id,
        component,
      },
      None,
    )
    .with_compiler_loc()
  }

  #[cfg(feature = "functions")]
  pub fn reactive_turn_checkpoint(&self) -> MResult<InterpreterReactiveTurnCheckpoint> {
    let plan = self.plan();
    plan.validate_checkpoint_invariants()
      .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
    plan.validate_checkpoint_turn_state(&self.reactive_turn_state)
      .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
    #[cfg(feature = "trace")]
    let trace_event_len = self.trace_events.try_borrow()
      .map_err(|_| self.reactive_turn_borrow_conflict(
        "checkpoint",
        "trace events",
      ))?
      .len();
    Ok(InterpreterReactiveTurnCheckpoint {
      owner: self.checkpoint_owner.clone(),
      interpreter_id: self.id,
      plan_node_len: plan.len(),
      activation_registration_depth: plan.activation_registration_depth(),
      plan,
      reactive_turn_state: self.reactive_turn_state.clone(),
      #[cfg(feature = "trace")]
      trace_events: self.trace_events.clone(),
      #[cfg(feature = "trace")]
      trace_event_len,
    })
  }

  #[cfg(feature = "functions")]
  pub fn preflight_restore_reactive_turn(
    &self,
    checkpoint: &InterpreterReactiveTurnCheckpoint,
  ) -> MResult<()> {
    if !Rc::ptr_eq(&checkpoint.owner, &self.checkpoint_owner) {
      return Err(MechError::new(
        InterpreterReactiveTurnOwnerMismatch {
          interpreter_id: self.id,
        },
        None,
      ).with_compiler_loc());
    }
    if checkpoint.interpreter_id != self.id {
      return Err(self.reactive_turn_invariant(format!(
        "checkpoint interpreter ID {} does not match {}",
        checkpoint.interpreter_id,
        self.id,
      )));
    }
    let plan = self.plan();
    if plan.0.addr() != checkpoint.plan.0.addr()
      || plan.1.addr() != checkpoint.plan.1.addr()
    {
      return Err(self.reactive_turn_invariant(
        "the current plan handles do not match the checkpoint",
      ));
    }
    if plan.len() != checkpoint.plan_node_len {
      return Err(self.reactive_turn_invariant(format!(
        "plan length {} does not match checkpoint length {}",
        plan.len(),
        checkpoint.plan_node_len,
      )));
    }
    if plan.activation_registration_depth() != checkpoint.activation_registration_depth {
      return Err(self.reactive_turn_invariant(format!(
        "activation registration depth {} does not match checkpoint depth {}",
        plan.activation_registration_depth(),
        checkpoint.activation_registration_depth,
      )));
    }
    plan.validate_checkpoint_invariants()
      .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
    plan.validate_checkpoint_turn_state(&checkpoint.reactive_turn_state)
      .map_err(|error| self.reactive_turn_invariant(format!("{:?}", error)))?;
    #[cfg(feature = "trace")]
    {
      if self.trace_events.addr() != checkpoint.trace_events.addr() {
        return Err(self.reactive_turn_invariant(
          "the current trace target does not match the checkpoint",
        ));
      }
      let events = self.trace_events.try_borrow_mut()
        .map_err(|_| self.reactive_turn_borrow_conflict(
          "restore",
          "trace events",
        ))?;
      if events.len() < checkpoint.trace_event_len {
        return Err(self.reactive_turn_invariant(format!(
          "trace length {} is shorter than checkpoint length {}",
          events.len(),
          checkpoint.trace_event_len,
        )));
      }
    }
    Ok(())
  }

  #[cfg(feature = "functions")]
  pub fn apply_restore_reactive_turn(
    &mut self,
    checkpoint: &InterpreterReactiveTurnCheckpoint,
  ) {
    self.reactive_turn_state = checkpoint.reactive_turn_state.clone();
    #[cfg(feature = "trace")]
    self.trace_events.borrow_mut().truncate(checkpoint.trace_event_len);
  }

  #[cfg(feature = "functions")]
  fn finish_failed_reactive_operation<T>(
    &mut self,
    checkpoint: &InterpreterReactiveTurnCheckpoint,
    participant: &mut ReactiveJournalParticipant<'_>,
    original_error: MechError,
  ) -> MResult<T> {
    let rollback_result = self.preflight_restore_reactive_turn(checkpoint)
      .and_then(|_| participant.preflight_restore_before());
    if let Err(rollback_error) = rollback_result {
      participant.commit();
      return Err(MechError::new(
        InterpreterReactiveTurnRollbackFailed {
          interpreter_id: self.id,
          original_error: format!("{:?}", original_error),
          rollback_error: format!("{:?}", rollback_error),
        },
        None,
      ).with_compiler_loc());
    }
    participant.apply_restore_before();
    self.apply_restore_reactive_turn(checkpoint);
    Err(original_error)
  }

  pub fn checkpoint(&self) -> MResult<InterpreterCheckpoint> {
    let mut journal = ValueStateJournal::new();
    let mut seen_children = Vec::new();
    let structure = InterpreterStructureCheckpoint::capture(
      self,
      &mut journal,
      &mut seen_children,
    )?;
    Ok(InterpreterCheckpoint {
      data: Rc::new(InterpreterCheckpointData {
        structure,
        journal,
      }),
    })
  }

  pub fn restore(&mut self, checkpoint: InterpreterCheckpoint) -> MResult<()> {
    checkpoint.data.structure.preflight_owner(self)?;
    checkpoint.data.structure.preflight()?;
    checkpoint.data.journal.preflight_restore_before()?;
    checkpoint.data.structure.apply_structure(self);
    checkpoint.data.journal.apply_restore_before();
    checkpoint.data.structure.rebuild_checkpoint_indexes();
    Ok(())
  }

  pub fn new(id: u64, max_steps: usize) -> Self {
    let mut state = ProgramState::new();
    load_stdkinds(&mut state.kinds);
    #[cfg(feature = "symbol_table")]
    {
      let ans_id = hash_str("ans");
      state
        .symbol_table
        .borrow_mut()
        .insert(ans_id, Value::Empty, false);
      state
        .symbol_table
        .borrow_mut()
        .dictionary
        .borrow_mut()
        .insert(ans_id, "ans".to_string());
      state.dictionary.borrow_mut().insert(ans_id, "ans".to_string());
    }
    #[cfg(feature = "functions")]
    load_prelude(&mut state.functions.borrow_mut());
    Self {
      id,
      checkpoint_owner: Rc::new(()),
      ip: 0,
      profile: false,
      max_steps, // Default maximum steps
      #[cfg(feature = "trace")]
      trace: false,
      #[cfg(feature = "trace")]
      trace_to_stdout: true,
      #[cfg(feature = "trace")]
      trace_events: Ref::new(Vec::new()),
      state: Ref::new(state),
      #[cfg(feature = "functions")]
      reactive_turn_state: ReactiveTurnState::default(),
      #[cfg(feature = "functions")]
      persistent_user_function_plan_depth: Ref::new(0),
      deferred_expression_solve_depth: Ref::new(0),
      #[cfg(feature = "functions")]
      stack: Vec::new(),
      registers: Vec::new(),
      constants: Vec::new(),
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      #[cfg(feature = "subscript_formula")]
      string_access_live_values: Ref::new(std::collections::BTreeSet::new()),
      #[cfg(feature = "subscript_formula")]
      current_string_access_expression_live: Ref::new(false),
      inline_eval_counter: Ref::new(0),
      context_bindings: Ref::new(HashMap::new()),
      module_manifests: Ref::new(ModuleManifestCatalog::with_builtin_hosts()),
      #[cfg(feature = "state_machines")]
      user_state_machines: Ref::new(HashMap::new()),
      #[cfg(feature = "state_machines")]
      user_state_machine_specs: Ref::new(HashMap::new()),
      code: Vec::new(),
      #[cfg(feature = "compiler")]
      context: None,
    }
  }

  pub fn default() -> Self {
    Self::new(0, 10_000)
  }

  pub fn bind_context(&self, name: &Identifier, base_uri: impl Into<String>) {
    self.context_bindings.borrow_mut().insert(name.hash(), RuntimeContextBinding {
      name: name.to_string(),
      base_uri: base_uri.into(),
    });
  }

  pub fn context_binding(&self, name: &Identifier) -> Option<RuntimeContextBinding> {
    self.context_bindings.borrow().get(&name.hash()).cloned()
  }

  pub fn bind_context_export(
    &self,
    alias: &Identifier,
    module: &str,
    item: &str,
  ) -> MResult<()> {
    let base_uri = {
      let manifests = self.module_manifests.borrow();
      manifests.context_export(module, item)?.base_uri.clone()
    };
    self.bind_context(alias, base_uri);
    Ok(())
  }

  #[cfg(feature = "functions")]
  pub fn new_with_full_stdlib(id: u64) -> Self {
    Self::new_with_full_stdlib_steps(id, 10_000)
  }

  #[cfg(feature = "functions")]
  pub fn new_with_full_stdlib_steps(id: u64, max_steps: usize) -> Self {
    let intrp = Self::new(id, max_steps);
    load_stdlib(&mut intrp.functions().borrow_mut());
    intrp
  }

  #[cfg(feature = "symbol_table")]
  pub fn set_environment(&mut self, env: SymbolTableRef) {
    self.state.borrow_mut().environment = Some(env);
  }

  #[cfg(feature = "functions")]
  pub fn clear_plan(&mut self) {
    self.state.borrow_mut().plan.borrow_mut().clear();
    self.reactive_turn_state = ReactiveTurnState::default();
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print(&self) -> String {
    let mut output = String::new();
    output.push_str(&format!("Interpreter ID: {}\n", self.id));
    // print state
    output.push_str(&self.state.borrow().pretty_print());

    output.push_str("Registers:\n");
    for (i, reg) in self.registers.iter().enumerate() {
      output.push_str(&format!("  R{}: {}\n", i, reg));
    }
    output.push_str("Constants:\n");
    for (i, constant) in self.constants.iter().enumerate() {
      output.push_str(&format!("  C{}: {}\n", i, constant));
    }
    output.push_str(&format!("Output Value: {}\n", self.out));
    output.push_str(&format!(
      "Number of Sub-Interpreters: {}\n",
      self.sub_interpreters.borrow().len()
    ));
    output.push_str("Output Values:\n");
    for (key, value) in self.out_values.borrow().iter() {
      output.push_str(&format!("  {}: {}\n", key, value));
    }
    output.push_str(&format!("Code Length: {}\n", self.code.len()));
    #[cfg(feature = "compiler")]
    if let Some(context) = &self.context {
      output.push_str("Context: Exists\n");
    } else {
      output.push_str("Context: None\n");
    }
    output
  }

  pub fn clear(&mut self) {
    let id = self.id;
    let checkpoint_owner = self.checkpoint_owner.clone();
    *self = Interpreter::new(id, self.max_steps);
    self.checkpoint_owner = checkpoint_owner;
  }

  pub fn set_trace_enabled(&mut self, enabled: bool) {
    #[cfg(feature = "trace")]
    {
      self.trace = enabled;
    }
    #[cfg(not(feature = "trace"))]
    {
      let _ = enabled;
    }
  }

  #[cfg(feature = "trace")]
  pub fn set_trace_to_stdout(&mut self, enabled: bool) {
    self.trace_to_stdout = enabled;
  }

  #[cfg(feature = "trace")]
  pub fn clear_trace_events(&self) {
    self.trace_events.borrow_mut().clear();
  }

  #[cfg(feature = "trace")]
  pub fn trace_events(&self) -> Vec<TraceEvent> {
    self.trace_events.borrow().clone()
  }

  #[cfg(feature = "trace")]
  pub fn trace_events_to_json(&self) -> String {
    let trace_events = self.trace_events.borrow();
    trace_events_to_json(trace_events.as_slice())
  }

  #[cfg(feature = "trace")]
  pub fn push_trace_line(&self, rendered: String) {
    let (channel, label, message) = parse_trace_line(&rendered);
    let mut trace_events = self.trace_events.borrow_mut();
    let index = trace_events.len();
    trace_events.push(TraceEvent {
        index,
        channel,
        label,
        message,
        rendered,
    });
  }

  #[cfg(all(feature = "trace", feature = "state_machines"))]
  pub fn formatted_fsm_trace(&self) -> String {
    format_fsm_trace_report(&self.trace_events())
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print_symbols(&self) -> String {
    let state_brrw = self.state.borrow();
    let syms = state_brrw.symbol_table.borrow();
    syms.pretty_print()
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print_plan(&self) -> String {
    let state_brrw = self.state.borrow();
    let plan = state_brrw.plan.borrow();
    let mut result = String::new();
    for (i, step) in plan.iter().enumerate() {
      result.push_str(&format!("Step {}:\n", i));
      result.push_str(&format!("{}\n", step.to_string()));
    }
    result
  }

  #[cfg(feature = "functions")]
  pub fn plan(&self) -> Plan {
    self.state.borrow().plan.clone()
  }

  #[cfg(feature = "functions")]
  pub fn advance_reactive_turn(
    &mut self,
    dirty_cells: &[ReactiveCellId],
  ) -> MResult<ReactiveTurnOutcome> {
    let mut services = NoMechExecutionServices;
    self.advance_reactive_turn_with_services(
      dirty_cells,
      &mut services,
    )
  }

  #[cfg(feature = "functions")]
  pub fn advance_reactive_turn_with_services(
    &mut self,
    dirty_cells: &[ReactiveCellId],
    services: &mut dyn MechExecutionServices,
  ) -> MResult<ReactiveTurnOutcome> {
    let checkpoint = self.reactive_turn_checkpoint()?;
    with_reactive_journal_participant(|participant| {
      let execution = self.advance_reactive_turn_participating(
        dirty_cells,
        participant,
        services,
      );
      match execution {
        Ok(outcome) => {
          participant.commit();
          Ok(outcome)
        }
        Err(error) => self.finish_failed_reactive_operation(
          &checkpoint,
          participant,
          error,
        ),
      }
    })
  }

  /// Participates in a program-owned reactive operation.
  ///
  /// The capability can only be received inside the sealed journal callback;
  /// callers cannot construct or retain it.
  #[cfg(feature = "functions")]
  pub fn advance_reactive_turn_participating(
    &mut self,
    dirty_cells: &[ReactiveCellId],
    participant: &mut ReactiveJournalParticipant<'_>,
    services: &mut dyn MechExecutionServices,
  ) -> MResult<ReactiveTurnOutcome> {
    let plan = self.plan();
    plan.advance_reactive_turn_participating(
      &mut self.reactive_turn_state,
      dirty_cells,
      participant,
      services,
    )
  }

  #[cfg(feature = "functions")]
  pub fn has_pending_reactive_registers(&self) -> bool {
    self.reactive_turn_state.has_pending_registers()
  }

  #[cfg(feature = "symbol_table")]
  pub fn symbols(&self) -> SymbolTableRef {
    self.state.borrow().symbol_table.clone()
  }

  pub fn dictionary(&self) -> Ref<Dictionary> {
    self.state.borrow().dictionary.clone()
  }

  #[cfg(feature = "functions")]
  pub fn functions(&self) -> FunctionsRef {
    self.state.borrow().functions.clone()
  }

  #[cfg(feature = "functions")]
  pub fn set_functions(&mut self, functions: FunctionsRef) {
    self.state.borrow_mut().functions = functions;
  }


  #[cfg(feature = "functions")]
  pub fn plan_len(&self) -> usize {
    self.state.borrow().plan.len()
  }

  #[cfg(feature = "functions")]
  pub fn solve_plan(&mut self) -> MResult<Value> {
    let mut services = NoMechExecutionServices;
    self.solve_plan_with_services(&mut services)
  }

  #[cfg(feature = "functions")]
  pub fn solve_plan_with_services(
    &mut self,
    services: &mut dyn MechExecutionServices,
  ) -> MResult<Value> {
    self.step_with_services(0, 1, services)
  }

  #[cfg(feature = "functions")]
  pub fn step(&mut self, step_id: usize, step_count: u64) -> MResult<Value> {
    let mut services = NoMechExecutionServices;
    self.step_with_services(step_id, step_count, &mut services)
  }

  #[cfg(feature = "functions")]
  pub fn step_with_services(
    &mut self,
    step_id: usize,
    step_count: u64,
    services: &mut dyn MechExecutionServices,
  ) -> MResult<Value> {
    let checkpoint = self.reactive_turn_checkpoint()?;
    with_reactive_journal_participant(|participant| {
      let execution = self.step_reactive_turn_participating(
        step_id,
        step_count,
        participant,
        services,
      );
      match execution {
        Ok(value) => {
          participant.commit();
          Ok(value)
        }
        Err(error) => self.finish_failed_reactive_operation(
          &checkpoint,
          participant,
          error,
        ),
      }
    })
  }

  /// Participates in a program-owned stepping operation.
  ///
  /// The capability can only be received inside the sealed journal callback;
  /// callers cannot construct or retain it.
  #[cfg(feature = "functions")]
  pub fn step_reactive_turn_participating(
    &mut self,
    step_id: usize,
    step_count: u64,
    participant: &mut ReactiveJournalParticipant<'_>,
    services: &mut dyn MechExecutionServices,
  ) -> MResult<Value> {
    let state_brrw = self.state.borrow();
    let mut plan_brrw = state_brrw
      .plan
      .0
      .try_borrow_mut()
      .map_err(|_| {
        self.reactive_turn_borrow_conflict(
          "execute",
          "plan",
        )
      })?; // RefMut<Vec<Box<dyn MechFunction>>>

    if plan_brrw.is_empty() {
      return Err(MechError::new(NoStepsInPlanError, None).with_compiler_loc());
    }

    let len = plan_brrw.len();

    // Case 1: step_id == 0, run entire plan step_count times
    if step_id == 0 {
      if self.profile {
        // Initialize total durations per step
        let mut total_durations = vec![Duration::ZERO; len];
        for _ in 0..step_count {
          for (idx, fxn) in plan_brrw.iter_mut().enumerate() {
            let start = Instant::now();
            participant.capture_function_state(fxn.as_ref())?;
            fxn.solve_result_with(services)?;
            total_durations[idx] += start.elapsed();
          }
        }
        // Print histogram if profiling is enabled
        if self.profile {
          println!("\nStep timing summary and histogram:");
          print_histogram(&total_durations);
        }
        return Ok(plan_brrw[len - 1].out().clone());
      } else {
        for _ in 0..step_count {
          for (idx, fxn) in plan_brrw.iter_mut().enumerate() {
            trace_println!(self, "{}", {
              let fxn_header = fxn
                .to_string()
                .lines()
                .next()
                .unwrap_or("<unknown-step>")
                .to_string();
              format!("[trace][plan] step[{idx}] {fxn_header}")
            });
            participant.capture_function_state(fxn.as_ref())?;
            fxn.solve_result_with(services)?;
            trace_println!(self, "{}", {
              let output = fxn.out().to_string();
              let output = if output.chars().count() > 96 {
                  format!("{}…", output.chars().take(96).collect::<String>())
              } else {
                  output
              };
              format!("[trace][plan] step[{idx}] out={output}")
            });
          }
        }
        return Ok(plan_brrw[len - 1].out().clone());
      }
    }

    // Case 2: step a single function by index
    let idx = step_id as usize;
    if idx > len {
      return Err(MechError::new(
        StepIndexOutOfBoundsError {
            step_id,
            plan_length: len,
        },
        None,
      )
      .with_compiler_loc());
    }

    let fxn = &mut plan_brrw[idx - 1];

    let fxn_str = fxn.to_string();
    if fxn_str.lines().count() > 30 {
      let lines: Vec<&str> = fxn_str.lines().collect();
      println!("Stepping function:");
      for line in &lines[0..10] {
        println!("{}", line);
      }
      println!("…");
      for line in &lines[lines.len() - 10..] {
        println!("{}", line);
      }
    } else {
      println!("Stepping function:\n{}", fxn_str);
    }

    for _ in 0..step_count {
      participant.capture_function_state(fxn.as_ref())?;
      fxn.solve_result_with(services)?;
    }

    Ok(fxn.out().clone())
  }

  #[cfg(feature = "functions")]
  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    let mut services = NoMechExecutionServices;
    self.interpret_with_services(tree, &mut services)
  }

  #[cfg(feature = "functions")]
  pub fn interpret_with_services(
    &mut self,
    tree: &Program,
    services: &mut dyn MechExecutionServices,
  ) -> MResult<Value> {
    self.code.push(MechSourceCode::Tree(tree.clone()));
    let result = catch_unwind(AssertUnwindSafe(|| {
      let execution = InterpreterExecution::new(
        self,
        services,
      );
      program(tree, &execution)
    }))
    .map_err(|err| {
      match err.downcast_ref::<&'static str>() {
         Some(raw_msg) => {
          if raw_msg.contains("Index out of bounds") {
              MechError::new(IndexOutOfBoundsError, None).with_compiler_loc()
          } else if raw_msg.contains("attempt to subtract with overflow") {
              MechError::new(OverflowSubtractionError, None).with_compiler_loc()
          } else {
            MechError::new(
              UnknownPanicError {
                details: raw_msg.to_string(),
              },
              None,
            )
            .with_compiler_loc()
          }
        }
        None => {
          MechError::new(
            UnknownPanicError {
              details: "Non-string panic".to_string(),
            },
            None,
          )
          .with_compiler_loc()
        }
      }
    })??;
    match self.state.borrow().plan.borrow().last() {
      Some(last_step) => self.out = last_step.out().clone(),
      None => self.out = Value::Empty,
    }
    Ok(result)
  }


  #[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
  pub fn run_program(&mut self, program: &ParsedProgram) -> MResult<Value> {
    // Reset the instruction pointer
    self.ip = 0;
    // Resize the registers and constant table
    self.registers = vec![Value::Empty; program.header.reg_count as usize];
    self.constants = vec![Value::Empty; program.const_entries.len()];
    // Load the constants
    self.constants = program.decode_const_entries()?;
    // Load the symbol table
    {
      let mut state_brrw = self.state.borrow_mut();
      let mut symbol_table = state_brrw.symbol_table.borrow_mut();
      for (id, reg) in program.symbols.iter() {
        let constant = self.constants[*reg as usize].clone();
        self.out = constant.clone();
        let mutable = program.mutable_symbols.contains(id);
        symbol_table.insert(*id, constant, mutable);
      }
    }
    // Load the instructions
    {
      let state_brrw = self.state.borrow();
      let functions_table = state_brrw.functions.borrow();
      while self.ip < program.instrs.len() {
        let instr = &program.instrs[self.ip];
        match instr {
          DecodedInstr::ConstLoad { dst, const_id } => {
            let value = self.constants[*const_id as usize].clone();
            self.registers[*dst as usize] = value;
          }
          DecodedInstr::NullOp { fxn_id, dst } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let out = self.registers[*dst as usize].clone();
                let function_args = FunctionArgs::Nullary(out);
                self.out = register_bytecode_function(
                  &state_brrw,
                  *fxn_factory,
                  function_args,
                )?;
              }
              None => {
                return Err(MechError::new(
                  UnknownNullaryFunctionError { fxn_id: *fxn_id },
                  None,
                )
                .with_compiler_loc());
              }
            }
          }
          DecodedInstr::UnOp { fxn_id, dst, src } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let out = self.registers[*dst as usize].clone();
                let input = self.registers[*src as usize].clone();
                let function_args = FunctionArgs::Unary(out, input);
                self.out = register_bytecode_function(
                  &state_brrw,
                  *fxn_factory,
                  function_args,
                )?;
              }
              None => {
                return Err(MechError::new(
                  UnknownUnaryFunctionError { fxn_id: *fxn_id },
                  None,
                )
                .with_compiler_loc());
              }
            }
          }
          DecodedInstr::BinOp { fxn_id, dst, lhs, rhs } =>
          match functions_table.functions.get(fxn_id) {
            Some(fxn_factory) => {
              let out = self.registers[*dst as usize].clone();
              let lhs = self.registers[*lhs as usize].clone();
              let rhs = self.registers[*rhs as usize].clone();
              let function_args = FunctionArgs::Binary(out, lhs, rhs);
              self.out = register_bytecode_function(
                &state_brrw,
                *fxn_factory,
                function_args,
              )?;
            }
            None => {
              return Err(MechError::new(
                UnknownBinaryFunctionError { fxn_id: *fxn_id },
                None,
              )
              .with_compiler_loc());
            }
          },
          DecodedInstr::TernOp {fxn_id,dst,a,b,c} =>
          match functions_table.functions.get(fxn_id) {
            Some(fxn_factory) => {
              let out = self.registers[*dst as usize].clone();
              let arg_a = self.registers[*a as usize].clone();
              let arg_b = self.registers[*b as usize].clone();
              let arg_c = self.registers[*c as usize].clone();
              let function_args = FunctionArgs::Ternary(
                out,
                arg_a,
                arg_b,
                arg_c,
              );
              self.out = register_bytecode_function(
                &state_brrw,
                *fxn_factory,
                function_args,
              )?;
            }
            None => {
              return Err(MechError::new(
                UnknownTernaryFunctionError { fxn_id: *fxn_id },
                None,
              )
              .with_compiler_loc());
            }
          },
          DecodedInstr::QuadOp {fxn_id,dst,a,b,c,d } =>
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let out = self.registers[*dst as usize].clone();
                let arg_a = self.registers[*a as usize].clone();
                let arg_b = self.registers[*b as usize].clone();
                let arg_c = self.registers[*c as usize].clone();
                let arg_d = self.registers[*d as usize].clone();
                let function_args = FunctionArgs::Quaternary(
                  out,
                  arg_a,
                  arg_b,
                  arg_c,
                  arg_d,
                );
                self.out = register_bytecode_function(
                  &state_brrw,
                  *fxn_factory,
                  function_args,
                )?;
              }
              None => {
                return Err(MechError::new(
                  UnknownQuadFunctionError { fxn_id: *fxn_id },
                  None,
                )
                .with_compiler_loc());
              }
          },
          DecodedInstr::VarArg { fxn_id, dst, args } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let out = self.registers[*dst as usize].clone();
                let argument_values = args
                  .iter()
                  .map(|register| self.registers[*register as usize].clone())
                  .collect::<Vec<Value>>();
                let function_args = FunctionArgs::Variadic(out, argument_values);
                self.out = register_bytecode_function(
                  &state_brrw,
                  *fxn_factory,
                  function_args,
                )?;
              }
              None => {
                return Err(MechError::new(
                  UnknownVariadicFunctionError { fxn_id: *fxn_id },
                  None,
                )
                .with_compiler_loc());
              }
            }
          }
          DecodedInstr::Ret { src } => {
            todo!();
          }
          x => {
            return Err(MechError::new(
              UnknownInstructionError {
                instr: format!("{:?}", x),
              },
              None,
            )
            .with_compiler_loc());
          }
        }
        self.ip += 1;
      }
    }
    // Load the dictionary
    {
      let mut state_brrw = self.state.borrow_mut();
      let mut symbol_table = state_brrw.symbol_table.borrow_mut();
      for (id, name) in &program.dictionary {
        symbol_table
            .dictionary
            .borrow_mut()
            .insert(*id, name.clone());
        state_brrw.dictionary.borrow_mut().insert(*id, name.clone());
      }
    }
    Ok(self.out.clone())
  }

  #[cfg(all(feature = "compiler", feature = "functions"))]
  pub fn compile(&mut self) -> MResult<Vec<u8>> {
    let state_brrw = self.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut();
    let mut ctx = CompileCtx::new();
    for step in plan_brrw.iter() {
      step.compile(&mut ctx)?;
    }
    let bytes = ctx.compile()?;
    self.context = Some(ctx);
    Ok(bytes)
  }
}

#[cfg(all(feature = "program", feature = "functions", feature = "symbol_table"))]
fn register_bytecode_function(
  state: &ProgramState,
  factory: fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>,
  function_args: FunctionArgs,
) -> MResult<Value> {
  let input_values = function_args.input_values();
  let function = factory(function_args)?;
  let output = function.out();

  state.plan.register_function(function, &input_values)?;

  Ok(output)
}

#[cfg(all(
  test,
  feature = "functions",
  feature = "symbol_table",
  feature = "f64"
))]
mod checkpoint_tests {
  use super::*;

  fn f64_value(value: &Ref<f64>) -> Value {
    Value::F64(value.clone())
  }

  fn install_scalar(
    interpreter: &Interpreter,
    name: &str,
    value: f64,
  ) -> (ValRef, Ref<f64>) {
    let backing = Ref::new(value);
    let id = hash_str(name);
    let symbols = interpreter.symbols();
    let cell = symbols
      .borrow_mut()
      .insert(id, f64_value(&backing), true);
    symbols
      .borrow()
      .dictionary
      .borrow_mut()
      .insert(id, name.to_string());
    (cell, backing)
  }

  #[test]
  fn interpreter_checkpoint_restores_private_state_and_recursive_child_identity() {
    let mut root = Interpreter::new(1, 100);
    root.ip = 7;
    root.profile = true;
    let (symbol_cell, symbol_backing) = install_scalar(&root, "kept", 1.0);
    let symbol_cell_address = symbol_cell.addr();
    let symbol_backing_address = symbol_backing.addr();
    let symbol_backing_identity = ReactiveCellId::new(symbol_backing.id());
    root.registers = vec![f64_value(&symbol_backing)];
    root.constants = vec![f64_value(&Ref::new(2.0))];
    root.out = f64_value(&symbol_backing);
    root.code.push(MechSourceCode::String("before".to_string()));
    root
      .out_values
      .borrow_mut()
      .insert(hash_str("out"), f64_value(&symbol_backing));
    *root.inline_eval_counter.borrow_mut() = 4;
    *root.persistent_user_function_plan_depth.borrow_mut() = 2;
    *root.deferred_expression_solve_depth.borrow_mut() = 3;
    let context_binding_id = hash_str("checkpoint-context");
    root.context_bindings.borrow_mut().insert(
      context_binding_id,
      RuntimeContextBinding {
        name: "checkpoint-context".to_string(),
        base_uri: "test://checkpoint".to_string(),
      },
    );
    let original_manifests = root.module_manifests.borrow().clone();
    #[cfg(feature = "trace")]
    {
      root.trace = true;
      root.trace_to_stdout = false;
    }
    #[cfg(feature = "state_machines")]
    {
      let name = internal_pattern_value_identifier("checkpoint-fsm");
      root.user_state_machines.borrow_mut().insert(
        hash_str("checkpoint-fsm"),
        FsmImplementation {
          name: name.clone(),
          input: Vec::new(),
          start: Pattern::Wildcard,
          arms: Vec::new(),
        },
      );
      root.user_state_machine_specs.borrow_mut().insert(
        hash_str("checkpoint-fsm"),
        FsmSpecification {
          name,
          input: Vec::new(),
          output: None,
          states: Vec::new(),
        },
      );
    }
    #[cfg(feature = "invariant_define")]
    {
      let invariant_id = hash_str("checkpoint-invariant");
      let invariant_value = Ref::new(Value::Bool(Ref::new(true)));
      let mut state = root.state.borrow_mut();
      state
        .invariants
        .insert(invariant_id, ("checkpoint invariant".to_string(), invariant_value));
      state
        .invariant_expressions
        .insert(invariant_id, "value == true".to_string());
      state.invariant_evaluations.insert(
        invariant_id,
        InvariantEvaluation {
          reason: "checkpoint reason".to_string(),
          evaluated_kind: "bool".to_string(),
          actual: "true".to_string(),
          expected: "true".to_string(),
        },
      );
      state.invariant_violations.push(InvariantViolation {
        id: invariant_id,
        error: MechError::new(
          GenericError {
            msg: "checkpoint violation".to_string(),
          },
          None,
        ),
      });
    }

    let state_address = root.state.addr();
    let symbols_address = root.symbols().addr();
    let symbol_dictionary_address = root.symbols().borrow().dictionary.addr();
    let out_values_address = root.out_values.addr();
    let inline_counter_address = root.inline_eval_counter.addr();
    let context_bindings_address = root.context_bindings.addr();
    let module_manifests_address = root.module_manifests.addr();
    #[cfg(feature = "trace")]
    let trace_events_address = root.trace_events.addr();
    #[cfg(feature = "state_machines")]
    let user_state_machines_address = root.user_state_machines.addr();
    #[cfg(feature = "state_machines")]
    let user_state_machine_specs_address = root.user_state_machine_specs.addr();
    let sub_interpreters_address = root.sub_interpreters.addr();

    let child_id = 2;
    let grandchild_id = 3;
    let mut child = Interpreter::new(child_id, 200);
    child.ip = 8;
    let (_child_cell, child_backing) = install_scalar(&child, "child", 20.0);
    let mut grandchild = Interpreter::new(grandchild_id, 300);
    grandchild.ip = 9;
    let (_grandchild_cell, grandchild_backing) =
      install_scalar(&grandchild, "grandchild", 30.0);
    let grandchild_ref = Ref::new(Box::new(grandchild));
    let grandchild_handle_address = grandchild_ref.addr();
    child
      .sub_interpreters
      .borrow_mut()
      .insert(grandchild_id, grandchild_ref);
    let child_ref = Ref::new(Box::new(child));
    let child_handle_address = child_ref.addr();
    root
      .sub_interpreters
      .borrow_mut()
      .insert(child_id, child_ref);

    let checkpoint = root.checkpoint().unwrap();

    root.id = 99;
    root.ip = 99;
    root.profile = false;
    root.max_steps = 999;
    root.registers.clear();
    root.constants.clear();
    root.code.push(MechSourceCode::String("after".to_string()));
    root.out = Value::Empty;
    root.state = Ref::new(ProgramState::new());
    root.out_values = Ref::new(HashMap::new());
    root.inline_eval_counter = Ref::new(99);
    root.persistent_user_function_plan_depth = Ref::new(99);
    root.deferred_expression_solve_depth = Ref::new(99);
    root.context_bindings = Ref::new(HashMap::new());
    root.module_manifests = Ref::new(ModuleManifestCatalog::new());
    #[cfg(feature = "trace")]
    {
      root.trace = false;
      root.trace_to_stdout = true;
      root.trace_events = Ref::new(Vec::new());
    }
    #[cfg(feature = "state_machines")]
    {
      root.user_state_machines = Ref::new(HashMap::new());
      root.user_state_machine_specs = Ref::new(HashMap::new());
    }
    *symbol_backing.borrow_mut() = 11.0;
    *child_backing.borrow_mut() = 21.0;
    *grandchild_backing.borrow_mut() = 31.0;

    let removed_child = root
      .sub_interpreters
      .borrow_mut()
      .remove(&child_id)
      .unwrap();
    {
      let mut child = removed_child.borrow_mut();
      child.id = 22;
      child.ip = 88;
      let removed_grandchild = child
        .sub_interpreters
        .borrow_mut()
        .remove(&grandchild_id)
        .unwrap();
      drop(removed_grandchild);
    }
    drop(removed_child);
    root
      .sub_interpreters
      .borrow_mut()
      .insert(999, Ref::new(Box::new(Interpreter::new(999, 10))));

    let held = symbol_backing.borrow();
    let error = root.restore(checkpoint.clone()).unwrap_err();
    assert_eq!(
      error.kind_as::<ValueStateBorrowConflict>().unwrap().phase,
      "restore-before"
    );
    assert_eq!(root.id, 99);
    assert_eq!(root.ip, 99);
    assert_ne!(root.state.addr(), state_address);
    assert!(root.sub_interpreters.borrow().contains_key(&999));
    assert!(!root.sub_interpreters.borrow().contains_key(&child_id));
    assert_eq!(*held, 11.0);
    drop(held);

    root.restore(checkpoint).unwrap();

    assert_eq!(root.id, 1);
    assert_eq!(root.ip, 7);
    assert!(root.profile);
    assert_eq!(root.max_steps, 100);
    assert_eq!(root.state.addr(), state_address);
    assert_eq!(root.symbols().addr(), symbols_address);
    assert_eq!(
      root.symbols().borrow().dictionary.addr(),
      symbol_dictionary_address
    );
    assert_eq!(root.out_values.addr(), out_values_address);
    assert_eq!(root.inline_eval_counter.addr(), inline_counter_address);
    assert_eq!(root.context_bindings.addr(), context_bindings_address);
    assert_eq!(root.module_manifests.addr(), module_manifests_address);
    assert_eq!(*root.module_manifests.borrow(), original_manifests);
    assert_eq!(
      root.context_bindings
        .borrow()
        .get(&context_binding_id)
        .unwrap()
        .base_uri,
      "test://checkpoint",
    );
    #[cfg(feature = "trace")]
    {
      assert!(root.trace);
      assert!(!root.trace_to_stdout);
      assert_eq!(root.trace_events.addr(), trace_events_address);
    }
    #[cfg(feature = "state_machines")]
    {
      assert_eq!(root.user_state_machines.addr(), user_state_machines_address);
      assert_eq!(
        root.user_state_machine_specs.addr(),
        user_state_machine_specs_address,
      );
      assert!(root
        .user_state_machines
        .borrow()
        .contains_key(&hash_str("checkpoint-fsm")));
      assert!(root
        .user_state_machine_specs
        .borrow()
        .contains_key(&hash_str("checkpoint-fsm")));
    }
    #[cfg(feature = "invariant_define")]
    {
      let state = root.state.borrow();
      let invariant_id = hash_str("checkpoint-invariant");
      assert!(state.invariants.contains_key(&invariant_id));
      assert_eq!(
        state.invariant_expressions.get(&invariant_id).unwrap(),
        "value == true",
      );
      assert_eq!(
        state
          .invariant_evaluations
          .get(&invariant_id)
          .unwrap()
          .reason,
        "checkpoint reason",
      );
      assert_eq!(state.invariant_violations.len(), 1);
      assert_eq!(state.invariant_violations[0].id, invariant_id);
    }
    assert_eq!(root.sub_interpreters.addr(), sub_interpreters_address);
    assert_eq!(root.registers.len(), 1);
    assert_eq!(root.constants.len(), 1);
    assert_eq!(root.code, vec![MechSourceCode::String("before".to_string())]);
    assert_eq!(*root.inline_eval_counter.borrow(), 4);
    assert_eq!(*root.persistent_user_function_plan_depth.borrow(), 2);
    assert_eq!(*root.deferred_expression_solve_depth.borrow(), 3);
    assert_eq!(symbol_cell.addr(), symbol_cell_address);
    assert_eq!(symbol_backing.addr(), symbol_backing_address);
    assert_eq!(
      ReactiveCellId::new(symbol_backing.id()),
      symbol_backing_identity
    );
    assert_eq!(*symbol_backing.borrow(), 1.0);
    assert!(root.sub_interpreters.borrow().get(&999).is_none());

    let restored_child = root
      .sub_interpreters
      .borrow()
      .get(&child_id)
      .cloned()
      .unwrap();
    assert_eq!(restored_child.addr(), child_handle_address);
    let restored_grandchild = {
      let child = restored_child.borrow();
      assert_eq!(child.id, child_id);
      assert_eq!(child.ip, 8);
      assert_eq!(*child_backing.borrow(), 20.0);
      child
        .sub_interpreters
        .borrow()
        .get(&grandchild_id)
        .cloned()
        .unwrap()
    };
    assert_eq!(restored_grandchild.addr(), grandchild_handle_address);
    let grandchild = restored_grandchild.borrow();
    assert_eq!(grandchild.id, grandchild_id);
    assert_eq!(grandchild.ip, 9);
    assert_eq!(*grandchild_backing.borrow(), 30.0);
  }

  #[cfg(feature = "compiler")]
  #[test]
  fn interpreter_checkpoint_rejects_retained_compiler_context() {
    let mut interpreter = Interpreter::new(41, 100);
    interpreter.context = Some(CompileCtx::new());
    let error = match interpreter.checkpoint() {
      Ok(_) => panic!("retained compiler context checkpoint unexpectedly succeeded"),
      Err(error) => error,
    };
    let unsupported = error
      .kind_as::<InterpreterCheckpointUnsupportedCompilerContext>()
      .unwrap();
    assert_eq!(unsupported.interpreter_id, 41);
  }
}

#[cfg(all(test, feature = "program", feature = "functions", feature = "symbol_table", feature = "f64"))]
mod bytecode_dependency_tests {
  use super::*;

  struct BytecodeDependencyTestFunction {
    output: Value,
  }

  impl MechFunctionImpl for BytecodeDependencyTestFunction {
    fn solve(&self) {}

    fn out(&self) -> Value {
      self.output.clone()
    }

    fn to_string(&self) -> String {
      "bytecode-dependency-test".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
      Ok(self.reactive_output_values())
    }
  }

  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for BytecodeDependencyTestFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
      Ok(0)
    }
  }

  fn bytecode_dependency_test_factory(
    args: FunctionArgs,
  ) -> MResult<Box<dyn MechFunction>> {
    let output = match args {
      FunctionArgs::Nullary(output)
      | FunctionArgs::Unary(output, _)
      | FunctionArgs::Binary(output, _, _)
      | FunctionArgs::Ternary(output, _, _, _)
      | FunctionArgs::Quaternary(output, _, _, _, _)
      | FunctionArgs::Variadic(output, _) => output,
    };

    Ok(Box::new(BytecodeDependencyTestFunction { output }))
  }

  fn scalar(value: f64) -> (Value, ReactiveCellId) {
    let cell = Ref::new(value);
    let id = ReactiveCellId::new(cell.id());
    (Value::F64(cell), id)
  }

  #[test]
  fn bytecode_nullary_registration_has_no_inputs() {
    let state = ProgramState::new();
    let (output, output_cell) = scalar(1.0);

    let result = register_bytecode_function(
      &state,
      bytecode_dependency_test_factory,
      FunctionArgs::Nullary(output.clone()),
    )
    .unwrap();

    let plan = state.plan.borrow();
    let node = plan.node(0).unwrap();
    assert_eq!(plan.len(), 1);
    assert!(node.inputs.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
    assert!(node.outputs.contains(&output_cell));
    assert_eq!(result.reactive_cell_ids(), output.reactive_cell_ids());
  }

  #[test]
  fn bytecode_unary_registration_indexes_operand() {
    let state = ProgramState::new();
    let (output, output_cell) = scalar(1.0);
    let (input, input_cell) = scalar(2.0);

    register_bytecode_function(
      &state,
      bytecode_dependency_test_factory,
      FunctionArgs::Unary(output, input),
    )
    .unwrap();

    let plan = state.plan.borrow();
    let node = plan.node(0).unwrap();
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.inputs[0].cell, input_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Reactive);
    assert_eq!(plan.reactive_consumers_for(input_cell), &[0]);
    assert!(!node.inputs.iter().any(|dependency| dependency.cell == output_cell));
    assert!(node.outputs.contains(&output_cell));
  }

  #[test]
  fn bytecode_binary_registration_preserves_operand_order() {
    let state = ProgramState::new();
    let (output, _) = scalar(1.0);
    let (lhs, lhs_cell) = scalar(2.0);
    let (rhs, rhs_cell) = scalar(3.0);

    register_bytecode_function(
      &state,
      bytecode_dependency_test_factory,
      FunctionArgs::Binary(output, lhs, rhs),
    )
    .unwrap();

    let plan = state.plan.borrow();
    let node = plan.node(0).unwrap();
    assert_eq!(
      node.inputs.iter().map(|dependency| dependency.cell).collect::<Vec<_>>(),
      vec![lhs_cell, rhs_cell],
    );
    assert!(node.inputs.iter().all(|dependency| {
      dependency.kind == ReactiveDependencyKind::Reactive
    }));
    assert_eq!(plan.reactive_consumers_for(lhs_cell), &[0]);
    assert_eq!(plan.reactive_consumers_for(rhs_cell), &[0]);
  }

  #[test]
  fn bytecode_variadic_registration_preserves_all_inputs() {
    let state = ProgramState::new();
    let (output, _) = scalar(1.0);
    let (first, first_cell) = scalar(2.0);
    let (second, second_cell) = scalar(3.0);
    let (third, third_cell) = scalar(4.0);

    register_bytecode_function(
      &state,
      bytecode_dependency_test_factory,
      FunctionArgs::Variadic(output, vec![first, second, third]),
    )
    .unwrap();

    let plan = state.plan.borrow();
    let node = plan.node(0).unwrap();
    assert_eq!(
      node.inputs.iter().map(|dependency| dependency.cell).collect::<Vec<_>>(),
      vec![first_cell, second_cell, third_cell],
    );
  }

  #[test]
  fn bytecode_registration_deduplicates_alias_operands() {
    let state = ProgramState::new();
    let (output, _) = scalar(1.0);
    let (input, input_cell) = scalar(2.0);

    register_bytecode_function(
      &state,
      bytecode_dependency_test_factory,
      FunctionArgs::Binary(output, input.clone(), input),
    )
    .unwrap();

    let plan = state.plan.borrow();
    let node = plan.node(0).unwrap();
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.inputs[0].cell, input_cell);
    assert_eq!(plan.reactive_consumers_for(input_cell), &[0]);
  }
}

// Interpreter Errors
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UnknownInstructionError {
  pub instr: String,
}
impl MechErrorKind for UnknownInstructionError {
  fn name(&self) -> &str {
    "UnknownInstruction"
  }

  fn message(&self) -> String {
    format!("Unknown instruction: {}", self.instr)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownVariadicFunctionError {
  pub fxn_id: u64,
}

impl MechErrorKind for UnknownVariadicFunctionError {
  fn name(&self) -> &str {
    "UnknownVariadicFunction"
  }
  fn message(&self) -> String {
    format!("Unknown variadic function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownQuadFunctionError {
  pub fxn_id: u64,
}
impl MechErrorKind for UnknownQuadFunctionError {
  fn name(&self) -> &str {
    "UnknownQuadFunction"
  }
  fn message(&self) -> String {
    format!("Unknown quad function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownTernaryFunctionError {
  pub fxn_id: u64,
}
impl MechErrorKind for UnknownTernaryFunctionError {
  fn name(&self) -> &str {
    "UnknownTernaryFunction"
  }
  fn message(&self) -> String {
    format!("Unknown ternary function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownBinaryFunctionError {
  pub fxn_id: u64,
}
impl MechErrorKind for UnknownBinaryFunctionError {
  fn name(&self) -> &str {
    "UnknownBinaryFunction"
  }
  fn message(&self) -> String {
    format!("Unknown binary function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownUnaryFunctionError {
  pub fxn_id: u64,
}
impl MechErrorKind for UnknownUnaryFunctionError {
  fn name(&self) -> &str {
    "UnknownUnaryFunction"
  }
  fn message(&self) -> String {
    format!("Unknown unary function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownNullaryFunctionError {
  pub fxn_id: u64,
}
impl MechErrorKind for UnknownNullaryFunctionError {
  fn name(&self) -> &str {
    "UnknownNullaryFunction"
  }
  fn message(&self) -> String {
    format!("Unknown nullary function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct IndexOutOfBoundsError;
impl MechErrorKind for IndexOutOfBoundsError {
  fn name(&self) -> &str {
    "IndexOutOfBounds"
  }
  fn message(&self) -> String {
    "Index out of bounds".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct OverflowSubtractionError;
impl MechErrorKind for OverflowSubtractionError {
  fn name(&self) -> &str {
    "OverflowSubtraction"
  }
  fn message(&self) -> String {
    "Attempted subtraction overflow".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct UnknownPanicError {
  pub details: String,
}
impl MechErrorKind for UnknownPanicError {
  fn name(&self) -> &str {
    "UnknownPanic"
  }
  fn message(&self) -> String {
    self.details.clone()
  }
}

#[derive(Debug, Clone)]
struct StepIndexOutOfBoundsError {
  pub step_id: usize,
  pub plan_length: usize,
}
impl MechErrorKind for StepIndexOutOfBoundsError {
  fn name(&self) -> &str {
    "StepIndexOutOfBounds"
  }
  fn message(&self) -> String {
    format!(
      "Step id {} out of range (plan has {} steps)",
      self.step_id, self.plan_length
    )
  }
}

#[derive(Debug, Clone)]
struct NoStepsInPlanError;
impl MechErrorKind for NoStepsInPlanError {
  fn name(&self) -> &str {
    "NoStepsInPlan"
  }
  fn message(&self) -> String {
    "Plan contains no steps. This program doesn't do anything.".to_string()
  }
}

#[cfg(all(test, feature = "program", feature = "compiler", feature = "functions", feature = "variables", feature = "variable_define", feature = "variable_assign", feature = "assign", feature = "f64", feature = "math"))]
mod reactive_turn_interpreter_state_tests {
  use super::*;
  const SOURCE: &str = "input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0";
  fn interpreter() -> Interpreter { let mut i=Interpreter::new_with_full_stdlib(1); let t=mech_syntax::parser::parse(SOURCE).unwrap(); i.interpret(&t).unwrap(); i }
  fn value(i:&Interpreter,n:&str)->f64 {let value=i.symbols().borrow().get(hash_str(n)).expect("symbol").borrow().clone();*value.as_f64().expect("f64").borrow()}
  fn cell(i:&Interpreter,n:&str)->ReactiveCellId {let v=i.symbols().borrow().get(hash_str(n)).expect("symbol").borrow().reactive_root_cell_ids(); assert_eq!(v.len(),1,"root cell");v[0]}
  fn register(i:&Interpreter,c:ReactiveCellId)->ReactiveNodeId {let p=i.plan();let v=p.borrow().nodes.iter().filter(|n|n.kind==ReactiveNodeKind::Register&&n.outputs.contains(&c)).map(|n|n.id).collect::<Vec<_>>();assert_eq!(v.len(),1,"register");v[0]}
  fn first(i:&mut Interpreter)->(ReactiveNodeId,ReactiveNodeId){assert_eq!((value(i,"input"),value(i,"a"),value(i,"middle"),value(i,"b"),value(i,"output")),(1.,1.,2.,2.,3.));let(input,a,b)=(cell(i,"input"),register(i,cell(i,"a")),register(i,cell(i,"b")));let input_value=i.symbols().borrow().get(hash_str("input")).unwrap().borrow().clone();*input_value.as_f64().unwrap().borrow_mut()=10.;let o=i.advance_reactive_turn(&[input]).unwrap();assert_eq!(o.register_commit.committed_nodes,vec![a]);assert_eq!(o.after_commit.pending_register_nodes,vec![b]);(a,b)}
  #[test] fn reactive_turn_interpreter_state_persists_between_calls(){let mut i=interpreter();let(a,b)=first(&mut i);assert_eq!((value(&i,"a"),value(&i,"middle"),value(&i,"b"),value(&i,"output")),(10.,11.,2.,3.));assert!(i.has_pending_reactive_registers());let o=i.advance_reactive_turn(&[]).unwrap();assert_eq!(o.register_commit.committed_nodes,vec![b]);assert!(!o.register_commit.committed_nodes.contains(&a));assert_eq!((value(&i,"a"),value(&i,"middle"),value(&i,"b"),value(&i,"output")),(10.,11.,11.,12.));assert!(o.after_commit.pending_register_nodes.is_empty());assert!(!i.has_pending_reactive_registers());}
  #[test] fn reactive_turn_interpreter_state_clear_plan_resets_pending(){let mut i=interpreter();first(&mut i);assert!(i.has_pending_reactive_registers());assert!(i.plan_len()>0);i.clear_plan();assert_eq!(i.plan_len(),0);assert!(!i.has_pending_reactive_registers());}
  #[test] fn reactive_turn_interpreter_state_clone_preserves_pending(){let mut i=interpreter();first(&mut i);let c=i.clone();assert!(i.has_pending_reactive_registers());assert!(c.has_pending_reactive_registers());assert_eq!(i.plan_len(),c.plan_len());}
}

#[cfg(all(test, feature = "functions"))]
mod compact_reactive_turn_checkpoint_tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};

  struct CompactTestFunction {
    name: &'static str,
    output: Ref<usize>,
    captures: Rc<RefCell<usize>>,
    solves: Rc<RefCell<usize>>,
    fail_on_solve: Option<usize>,
    leak_borrow_on_failure: bool,
  }

  impl CompactTestFunction {
    fn new(
      name: &'static str,
      output: Ref<usize>,
      captures: Rc<RefCell<usize>>,
      solves: Rc<RefCell<usize>>,
    ) -> Self {
      Self {
        name,
        output,
        captures,
        solves,
        fail_on_solve: None,
        leak_borrow_on_failure: false,
      }
    }

    fn execute(&self) -> MResult<()> {
      let solve = {
        let mut solves = self.solves.borrow_mut();
        *solves += 1;
        *solves
      };
      *self.output.borrow_mut() += 1;
      if self.fail_on_solve == Some(solve) {
        if self.leak_borrow_on_failure {
          let borrow = self.output.borrow_mut();
          std::mem::forget(borrow);
        }
        return Err(MechError::new(
          GenericError { msg: format!("deliberate {} execution failure", self.name) },
          None,
        ));
      }
      Ok(())
    }
  }

  impl MechFunctionImpl for CompactTestFunction {
    fn solve(&self) {}

    fn solve_result(&self) -> MResult<()> {
      self.execute()
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
      self.execute()?;
      Ok(ReactiveSolveStatus::Changed)
    }

    fn out(&self) -> Value {
      Value::Index(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
      *self.captures.borrow_mut() += 1;
      Ok(vec![Value::Index(self.output.clone())])
    }

    fn to_string(&self) -> String {
      self.name.into()
    }
  }

  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for CompactTestFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
      Ok(0)
    }
  }

  fn function(
    name: &'static str,
    output: Ref<usize>,
  ) -> (CompactTestFunction, Rc<RefCell<usize>>, Rc<RefCell<usize>>) {
    let captures = Rc::new(RefCell::new(0));
    let solves = Rc::new(RefCell::new(0));
    (
      CompactTestFunction::new(name, output, captures.clone(), solves.clone()),
      captures,
      solves,
    )
  }

  fn add_reactive(
    interpreter: &Interpreter,
    function: CompactTestFunction,
    input: &Ref<usize>,
  ) -> ReactiveNodeId {
    interpreter.plan().0.borrow_mut()
      .register(Box::new(function), &[Value::Index(input.clone())])
      .unwrap()
  }

  #[test]
  fn reactive_turn_checkpoint_contains_only_scheduler_metadata_and_plan_handles() {
    let interpreter = Interpreter::new(7, 100);
    let plan = interpreter.plan();
    let (function, _, _) = function("node", Ref::new(0));
    plan.add_function(Box::new(function));

    let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();

    assert_eq!(checkpoint.interpreter_id, 7);
    assert_eq!(checkpoint.plan_node_len, 1);
    assert_eq!(checkpoint.activation_registration_depth, 0);
    assert_eq!(checkpoint.plan.0.addr(), plan.0.addr());
    assert_eq!(checkpoint.plan.1.addr(), plan.1.addr());
  }

  #[test]
  fn reactive_turn_checkpoint_does_not_capture_function_values() {
    let interpreter = Interpreter::new(8, 100);
    let (function, captures, _) = function("node", Ref::new(0));
    interpreter.plan().add_function(Box::new(function));

    interpreter.reactive_turn_checkpoint().unwrap();

    assert_eq!(*captures.borrow(), 0);
  }

  #[test]
  fn reactive_turn_checkpoint_restores_pending_registers_exactly() {
    let mut interpreter = Interpreter::new(9, 100);
    interpreter.reactive_turn_state.pending_register_nodes = vec![3, 5, 3];
    let checkpoint = match interpreter.reactive_turn_checkpoint() {
      Ok(_) => panic!("invalid pending registers unexpectedly checkpointed"),
      Err(error) => error,
    };
    assert_eq!(checkpoint.kind_name(), "InterpreterReactiveTurnInvariant");

    let (function, _, _) = function("register-shaped", Ref::new(0));
    let node = interpreter.plan().add_function(Box::new(function));
    interpreter.plan().0.borrow_mut().nodes[node].kind = ReactiveNodeKind::Register;
    interpreter.reactive_turn_state.pending_register_nodes = vec![node, node];
    let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    interpreter.reactive_turn_state.pending_register_nodes.clear();
    interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap();
    interpreter.apply_restore_reactive_turn(&checkpoint);
    assert_eq!(interpreter.reactive_turn_state.pending_register_nodes, vec![node, node]);
  }

  #[test]
  fn reactive_turn_failure_truncates_appended_trace_events() {
    let mut interpreter = Interpreter::new(10, 100);
    #[cfg(feature = "trace")]
    {
    interpreter.trace = true;
    interpreter.trace_to_stdout = false;
    }
    let output = Ref::new(0);
    let (mut function, _, _) = function("trace-fail", output);
    function.fail_on_solve = Some(1);
    interpreter.plan().add_function(Box::new(function));
    #[cfg(feature = "trace")]
    let original_len = interpreter.trace_events.borrow().len();

    interpreter.step(0, 1).unwrap_err();

    #[cfg(feature = "trace")]
    assert_eq!(interpreter.trace_events.borrow().len(), original_len);
  }

  #[test]
  fn reactive_turn_owner_mismatch_fails_before_mutation() {
    let first = Interpreter::new(11, 100);
    let second = Interpreter::new(11, 100);
    let checkpoint = first.reactive_turn_checkpoint().unwrap();

    let error = second.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnOwnerMismatch");
  }

  #[test]
  fn reactive_turn_interpreter_id_mismatch_fails_before_mutation() {
    let mut interpreter = Interpreter::new(12, 100);
    let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    interpreter.id = 13;

    let error = interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
    assert!(error.kind_message().contains("interpreter ID"));
  }

  #[test]
  fn reactive_turn_plan_handle_mismatch_fails_before_mutation() {
    let mut interpreter = Interpreter::new(14, 100);
    let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    interpreter.state.borrow_mut().plan = Plan::new();

    let error = interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
    assert!(error.kind_message().contains("plan handles"));
  }

  #[test]
  fn reactive_turn_plan_length_mismatch_fails_before_mutation() {
    let interpreter = Interpreter::new(15, 100);
    let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    let (function, _, _) = function("tail", Ref::new(0));
    interpreter.plan().add_function(Box::new(function));

    let error = interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
    assert!(error.kind_message().contains("plan length"));
  }

  #[test]
  fn reactive_turn_activation_depth_mismatch_fails_before_mutation() {
    let interpreter = Interpreter::new(16, 100);
    let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    interpreter.plan().push_activation_registration_scope(Vec::new());

    let error = interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
    assert!(error.kind_message().contains("activation registration depth"));
  }

  #[test]
  fn reactive_turn_saved_missing_pending_node_is_rejected() {
    let interpreter = Interpreter::new(17, 100);
    let mut checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    checkpoint.reactive_turn_state.pending_register_nodes = vec![99];

    let error = interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
    assert!(error.kind_message().contains("does not exist"));
  }

  #[test]
  fn reactive_turn_saved_non_register_pending_node_is_rejected() {
    let interpreter = Interpreter::new(18, 100);
    let (function, _, _) = function("comb", Ref::new(0));
    let node = interpreter.plan().add_function(Box::new(function));
    let mut checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
    checkpoint.reactive_turn_state.pending_register_nodes = vec![node];

    let error = interpreter.preflight_restore_reactive_turn(&checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
    assert!(error.kind_message().contains("not a register"));
  }

  #[test]
  fn participating_interpreter_turn_leaves_rollback_to_coordinator() {
    let mut interpreter = Interpreter::new(19, 100);
    let input = Ref::new(1usize);
    let output = Ref::new(4usize);
    let (mut function, _, _) = function("failure", output.clone());
    function.fail_on_solve = Some(1);
    add_reactive(&interpreter, function, &input);
    with_reactive_journal_participant(|participant| {
      let mut services = NoMechExecutionServices;
      interpreter
        .advance_reactive_turn_participating(
          &Value::Index(input).reactive_root_cell_ids(),
          participant,
          &mut services,
        )
        .unwrap_err();
      assert_eq!(*output.borrow(), 5);
      participant.preflight_restore_before()?;
      participant.apply_restore_before();
      Ok(())
    })
    .unwrap();
    assert_eq!(*output.borrow(), 4);
  }

  #[test]
  fn ordinary_interpreter_reactive_turn_rolls_back_on_failure() {
    let mut interpreter = Interpreter::new(20, 100);
    let input = Ref::new(1usize);
    let output = Ref::new(4usize);
    let (mut function, _, _) = function("failure", output.clone());
    function.fail_on_solve = Some(1);
    add_reactive(&interpreter, function, &input);

    interpreter.advance_reactive_turn(
      &Value::Index(input).reactive_root_cell_ids(),
    ).unwrap_err();

    assert_eq!(*output.borrow(), 4);
  }

  #[test]
  fn ordinary_interpreter_step_rolls_back_earlier_function_mutation() {
    let mut interpreter = Interpreter::new(21, 100);
    let first_output = Ref::new(1usize);
    let second_output = Ref::new(2usize);
    let (first, _, _) = function("first", first_output.clone());
    let (mut second, _, _) = function("second", second_output.clone());
    second.fail_on_solve = Some(1);
    interpreter.plan().add_function(Box::new(first));
    interpreter.plan().add_function(Box::new(second));

    interpreter.step(0, 1).unwrap_err();

    assert_eq!((*first_output.borrow(), *second_output.borrow()), (1, 2));
  }

  #[test]
  fn repeated_whole_plan_step_restores_before_first_repetition() {
    let mut interpreter = Interpreter::new(22, 100);
    let output = Ref::new(10usize);
    let (mut function, _, _) = function("repeat", output.clone());
    function.fail_on_solve = Some(3);
    interpreter.plan().add_function(Box::new(function));

    interpreter.step(0, 4).unwrap_err();

    assert_eq!(*output.borrow(), 10);
  }

  #[test]
  fn indexed_step_captures_only_selected_function() {
    let mut interpreter = Interpreter::new(23, 100);
    let (first, first_captures, _) = function("first", Ref::new(1));
    let (second, second_captures, _) = function("second", Ref::new(2));
    interpreter.plan().add_function(Box::new(first));
    interpreter.plan().add_function(Box::new(second));
    with_reactive_journal_participant(|participant| {
      let mut services = NoMechExecutionServices;
      interpreter.step_reactive_turn_participating(
        2,
        1,
        participant,
        &mut services,
      )?;
      assert_eq!(
        (*first_captures.borrow(), *second_captures.borrow()),
        (0, 1),
      );
      participant.commit();
      Ok(())
    })
    .unwrap();
  }

  #[test]
  fn reactive_turn_rollback_failure_retains_original_error() {
    let mut interpreter = Interpreter::new(24, 100);
    let output = Ref::new(0usize);
    let (mut function, _, _) = function("borrow-leak", output);
    function.fail_on_solve = Some(1);
    function.leak_borrow_on_failure = true;
    interpreter.plan().add_function(Box::new(function));

    let error = interpreter.step(0, 1).unwrap_err();

    assert_eq!(error.kind_name(), "InterpreterReactiveTurnRollbackFailed");
    let rollback = error.kind_as::<InterpreterReactiveTurnRollbackFailed>().unwrap();
    assert!(rollback.original_error.contains("deliberate borrow-leak execution failure"));
    assert!(rollback.rollback_error.contains("ValueStateBorrowConflict"));
  }

  #[test]
  fn reactive_turn_paths_preserve_plan_identity_without_full_checkpoints() {
    let mut interpreter = Interpreter::new(25, 100);
    let plan = interpreter.plan();
    let plan_address = plan.0.addr();
    let (function, _, _) = function("success", Ref::new(0));
    let node = plan.add_function(Box::new(function));

    interpreter.step(0, 2).unwrap();

    assert_eq!(interpreter.plan().0.addr(), plan_address);
    assert_eq!(interpreter.plan().borrow().nodes[node].id, node);
  }
}

#[cfg(all(test, feature = "program", feature = "compiler", feature = "functions", feature = "symbol_table", feature = "variable_define", feature = "f64"))]
mod decoded_variable_definition_symbol_metadata_tests {
  use super::*;

  #[test]
  fn decoded_variable_definition_symbol_metadata_round_trips() {
    let tree = mech_syntax::parser::parse("input := 1.0\n~state := 2.0").unwrap();
    let mut source = Interpreter::new_with_full_stdlib(1);
    source.interpret(&tree).unwrap();
    let bytes = source.compile().unwrap();
    let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
    let input_id = hash_str("input");
    let state_id = hash_str("state");
    assert!(parsed.symbols.contains_key(&input_id));
    assert!(parsed.symbols.contains_key(&state_id));
    assert_eq!(parsed.dictionary.get(&input_id).unwrap(), "input");
    assert_eq!(parsed.dictionary.get(&state_id).unwrap(), "state");
    assert!(!parsed.mutable_symbols.contains(&input_id));
    assert!(parsed.mutable_symbols.contains(&state_id));
    let mut decoded = Interpreter::new_with_full_stdlib(2);
    decoded.run_program(&parsed).unwrap();
    for (name, expected) in [("input", 1.0), ("state", 2.0)] {
      let value = decoded.symbols().borrow().get(hash_str(name)).unwrap().borrow().clone();
      assert_eq!(*value.as_f64().unwrap().borrow(), expected);
    }
    let state = decoded.state.borrow();
    assert!(state.get_mutable_symbol(input_id).is_none());
    assert!(state.get_mutable_symbol(state_id).is_some());
  }
}
pub struct InterpreterExecution<'a> {
  interpreter: &'a Interpreter,
  services: StdRefCell<&'a mut dyn MechExecutionServices>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionServicesBorrowConflict {
  pub operation: &'static str,
}

impl MechErrorKind for ExecutionServicesBorrowConflict {
  fn name(&self) -> &str {
    "ExecutionServicesBorrowConflict"
  }

  fn message(&self) -> String {
    format!(
      "Execution services are already borrowed while attempting `{}`.",
      self.operation,
    )
  }
}

impl<'a> InterpreterExecution<'a> {
  pub fn new(
    interpreter: &'a Interpreter,
    services: &'a mut dyn MechExecutionServices,
  ) -> Self {
    Self {
      interpreter,
      services: StdRefCell::new(services),
    }
  }

  pub fn with_services<T>(
    &self,
    operation: impl FnOnce(
      &mut dyn MechExecutionServices,
    ) -> MResult<T>,
  ) -> MResult<T> {
    let mut services = self.services.try_borrow_mut().map_err(|_| {
      MechError::new(
        ExecutionServicesBorrowConflict {
          operation: "with_services",
        },
        None,
      )
      .with_compiler_loc()
    })?;
    operation(&mut **services)
  }

  pub fn with_interpreter<T>(
    &self,
    interpreter: &Interpreter,
    operation: impl FnOnce(&InterpreterExecution<'_>) -> MResult<T>,
  ) -> MResult<T> {
    let mut services = self.services.try_borrow_mut().map_err(|_| {
      MechError::new(
        ExecutionServicesBorrowConflict {
          operation: "with_interpreter",
        },
        None,
      )
      .with_compiler_loc()
    })?;
    let execution = InterpreterExecution::new(
      interpreter,
      &mut **services,
    );
    operation(&execution)
  }
}

impl Deref for InterpreterExecution<'_> {
  type Target = Interpreter;

  fn deref(&self) -> &Self::Target {
    self.interpreter
  }
}

#[cfg(test)]
mod execution_services_borrow_tests {
  use super::*;

  #[test]
  fn nested_execution_service_access_returns_a_structured_error() {
    let interpreter = Interpreter::new(7001, 100);
    let mut services = NoMechExecutionServices;
    let execution = InterpreterExecution::new(
      &interpreter,
      &mut services,
    );

    let error = execution
      .with_services(|_| {
        execution.with_services(|_| Ok(()))
      })
      .unwrap_err();

    assert_eq!(error.kind_name(), "ExecutionServicesBorrowConflict");
    assert!(error.kind_message().contains("with_services"));
  }

  #[test]
  fn reentrant_interpreter_execution_returns_an_error_without_panicking() {
    let interpreter = Interpreter::new(7002, 100);
    let nested = Interpreter::new(7003, 100);
    let mut services = NoMechExecutionServices;
    let execution = InterpreterExecution::new(
      &interpreter,
      &mut services,
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
      || {
        execution.with_services(|_| {
          execution.with_interpreter(
            &nested,
            |_| Ok(()),
          )
        })
      },
    ));

    let error = result
      .expect("reentrant service access must not panic")
      .unwrap_err();
    assert_eq!(error.kind_name(), "ExecutionServicesBorrowConflict");
    assert!(error.kind_message().contains("with_interpreter"));
  }
}
