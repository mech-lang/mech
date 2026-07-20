use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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

use mech_core::{
  hash_str, CompileCtx, MResult, MechError, MechErrorKind, MechSourceCode,
  MechFunction, NativeFunctionCompiler, ParsedProgram, ValRef, Value, ValueKind,
  val_ref_reactive_cell_ids, ReactiveCellId, ReactiveTurnOutcome,
};

use mech_interpreter::Interpreter;
use mech_syntax::parser;

use crate::ClosureNativeFunctionCompiler;

#[derive(Debug, Clone)]
pub struct StableValueUpdateKindMismatch {
  pub expected: ValueKind,
  pub actual: ValueKind,
}
impl MechErrorKind for StableValueUpdateKindMismatch {
  fn name(&self) -> &str {
    "StableValueUpdateKindMismatch"
  }

  fn message(&self) -> String {
    format!(
      "stable value update requires the same value kind and shape; expected {:?}, found {:?}",
      self.expected,
      self.actual,
    )
  }
}

#[derive(Debug, Clone)]
pub struct StableValueUpdateUnsupported {
  pub kind: ValueKind,
}
impl MechErrorKind for StableValueUpdateUnsupported {
  fn name(&self) -> &str {
    "StableValueUpdateUnsupported"
  }

  fn message(&self) -> String {
    format!(
      "stable value update does not support preserving values of kind {:?}",
      self.kind,
    )
  }
}

fn stable_value_update_kind_mismatch(expected: ValueKind, actual: ValueKind) -> MechError {
  MechError::new(StableValueUpdateKindMismatch { expected, actual }, None)
}

fn is_stable_value_update_supported_value(value: &Value) -> bool {
  match value {
    #[cfg(feature = "u8")]
    Value::U8(_) => true,
    #[cfg(feature = "u16")]
    Value::U16(_) => true,
    #[cfg(feature = "u32")]
    Value::U32(_) => true,
    #[cfg(feature = "u64")]
    Value::U64(_) => true,
    #[cfg(feature = "u128")]
    Value::U128(_) => true,
    #[cfg(feature = "i8")]
    Value::I8(_) => true,
    #[cfg(feature = "i16")]
    Value::I16(_) => true,
    #[cfg(feature = "i32")]
    Value::I32(_) => true,
    #[cfg(feature = "i64")]
    Value::I64(_) => true,
    #[cfg(feature = "i128")]
    Value::I128(_) => true,
    #[cfg(feature = "f32")]
    Value::F32(_) => true,
    #[cfg(feature = "f64")]
    Value::F64(_) => true,
    #[cfg(feature = "complex")]
    Value::C64(_) => true,
    #[cfg(feature = "rational")]
    Value::R64(_) => true,
    #[cfg(any(feature = "string", feature = "variable_define"))]
    Value::String(_) => true,
    #[cfg(any(feature = "bool", feature = "variable_define"))]
    Value::Bool(_) => true,
    Value::Index(_) => true,

    #[cfg(feature = "matrix")]
    Value::MatrixIndex(_) => false,
    #[cfg(feature = "matrix")]
    Value::MatrixValue(_) => false,
    #[cfg(all(feature = "matrix", feature = "bool"))]
    Value::MatrixBool(_) => true,
    #[cfg(all(feature = "matrix", feature = "u8"))]
    Value::MatrixU8(_) => true,
    #[cfg(all(feature = "matrix", feature = "u16"))]
    Value::MatrixU16(_) => true,
    #[cfg(all(feature = "matrix", feature = "u32"))]
    Value::MatrixU32(_) => true,
    #[cfg(all(feature = "matrix", feature = "u64"))]
    Value::MatrixU64(_) => true,
    #[cfg(all(feature = "matrix", feature = "u128"))]
    Value::MatrixU128(_) => true,
    #[cfg(all(feature = "matrix", feature = "i8"))]
    Value::MatrixI8(_) => true,
    #[cfg(all(feature = "matrix", feature = "i16"))]
    Value::MatrixI16(_) => true,
    #[cfg(all(feature = "matrix", feature = "i32"))]
    Value::MatrixI32(_) => true,
    #[cfg(all(feature = "matrix", feature = "i64"))]
    Value::MatrixI64(_) => true,
    #[cfg(all(feature = "matrix", feature = "i128"))]
    Value::MatrixI128(_) => true,
    #[cfg(all(feature = "matrix", feature = "f32"))]
    Value::MatrixF32(_) => true,
    #[cfg(all(feature = "matrix", feature = "f64"))]
    Value::MatrixF64(_) => true,
    #[cfg(all(feature = "matrix", feature = "string"))]
    Value::MatrixString(_) => true,
    #[cfg(all(feature = "matrix", feature = "rational"))]
    Value::MatrixR64(_) => true,
    #[cfg(all(feature = "matrix", feature = "complex"))]
    Value::MatrixC64(_) => true,

    _ => false,
  }
}

fn validate_stable_value_update(current: &Value, next: &Value) -> MResult<()> {
  match (current, next) {
    (
      Value::Typed(current_inner, current_annotation),
      Value::Typed(next_inner, next_annotation),
    ) => {
      if current_annotation != next_annotation {
        return Err(stable_value_update_kind_mismatch(
          current_annotation.clone(),
          next_annotation.clone(),
        ));
      }
      validate_stable_value_update(current_inner.as_ref(), next_inner.as_ref())
    }
    (Value::Typed(_, _), _) | (_, Value::Typed(_, _)) => {
      Err(stable_value_update_kind_mismatch(current.kind(), next.kind()))
    }
    (Value::Empty, Value::Empty) => Ok(()),
    #[cfg(feature = "matrix")]
    (Value::MatrixValue(_), _) => Err(MechError::new(
      StableValueUpdateUnsupported { kind: current.kind() },
      None,
    )),
    #[cfg(feature = "matrix")]
    (_, Value::MatrixValue(_)) => Err(MechError::new(
      StableValueUpdateUnsupported { kind: next.kind() },
      None,
    )),
    _ => {
      let expected = current.kind();
      let actual = next.kind();
      if expected != actual {
        return Err(stable_value_update_kind_mismatch(expected, actual));
      }
      if !is_stable_value_update_supported_value(current) {
        return Err(MechError::new(
          StableValueUpdateUnsupported { kind: expected },
          None,
        ));
      }
      if !is_stable_value_update_supported_value(next) {
        return Err(MechError::new(
          StableValueUpdateUnsupported { kind: actual },
          None,
        ));
      }
      Ok(())
    }
  }
}

pub fn compile_stable_value_update(
  sink: ValRef,
  source: Value,
) -> MResult<Box<dyn MechFunction>> {
  {
    let current = sink.borrow();
    validate_stable_value_update(&current, &source)?;
  }

  let compiler = mech_interpreter::AssignValue {};
  compiler.compile(&vec![
    Value::MutableReference(sink),
    source,
  ])
}

pub fn apply_stable_value_update(
  sink: ValRef,
  source: Value,
) -> MResult<Value> {
  let update = compile_stable_value_update(sink.clone(), source)?;
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
  pub value: Value,
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

struct PreparedProgramInputBatch {
  assignments: Vec<Box<dyn MechFunction>>,
  dirty_cells: Vec<ReactiveCellId>,
  dirty_cells_by_interpreter: BTreeMap<u64, Vec<ReactiveCellId>>,
}

#[derive(Clone, Debug)]
pub struct ProgramSolveOutcome {
  pub value: Value,
  pub plan_len: usize,
}

pub struct MechProgram {
  pub config: MechProgramConfig,
  interpreter: Interpreter,
}

impl MechProgram {
  pub fn new(config: MechProgramConfig) -> Self {
    let id = hash_str(&format!("program/{}", config.name));
    let mut interpreter = Interpreter::new(id, config.environment.rounds_per_step);

    interpreter.set_trace_enabled(config.environment.trace_enabled);

    Self {
      config,
      interpreter,
    }
  }

  #[cfg(feature = "functions")]
  pub fn load_full_stdlib(&mut self) {
    mech_interpreter::load_stdlib(&mut self.interpreter.functions().borrow_mut());
  }

  pub fn register_native_function_compiler(
    &mut self,
    name: impl Into<String>,
    compiler: Arc<dyn NativeFunctionCompiler>,
  ) {
    self
      .interpreter
      .functions()
      .borrow_mut()
      .insert_function_compiler(name, compiler);
  }

  pub fn register_native_closure(
    &mut self,
    name: impl Into<String>,
    function: impl Fn(Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
  ) {
    let name = name.into();

    self.register_native_function_compiler(
      name.clone(),
      Arc::new(ClosureNativeFunctionCompiler::new(name, function)),
    );
  }

  pub fn from_environment(
    name: impl Into<String>,
    environment: MechProgramEnvironment,
  ) -> Self {
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
    self
      .interpreter
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

  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let tree = parser::parse(source.trim())?;
    self.run_tree(&tree)
  }

  pub fn run_tree(&mut self, tree: &mech_core::Program) -> MResult<Value> {
    self.interpreter.interpret(tree)
  }

  pub fn run_bytecode(&mut self, bytecode: &[u8]) -> MResult<Value> {
    let parsed = ParsedProgram::from_bytes(&bytecode.to_vec())?;
    self.run_bytecode_program(&parsed)
  }

  pub fn run_bytecode_program(&mut self, program: &ParsedProgram) -> MResult<Value> {
    self.interpreter.run_program(program)
  }

  pub fn run_program(&mut self, source: &str) -> MResult<Value> {
    self.run_profiled_string(source)
  }

  pub fn run_profiled_string(&mut self, source: &str) -> MResult<Value> {
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
  ) -> Option<Value> {
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
      interpreter.symbols().borrow().get_symbol_name_by_id(output_id)
    })
    .flatten()
  }

  pub fn symbol_values_for_interpreter(
    &self,
    interpreter_id: u64,
    names: &[String],
  ) -> Option<Vec<(String, Value)>> {
    with_interpreter(&self.interpreter, interpreter_id, &mut |interpreter| {
      let symbols = interpreter.symbols();
      let symbols_brrw = symbols.borrow();
      symbol_rows(&symbols_brrw, names)
    })
  }

  pub fn root_symbol_value(&self, name: &str) -> MResult<Value> {
    let mut values = self.root_symbol_values(&[name])?;
    Ok(values.remove(0).1)
  }

  pub fn root_symbol_values(&self, names: &[&str]) -> MResult<Vec<(String, Value)>> {
    let symbols = self.interpreter.symbols();
    let symbols_brrw = symbols.borrow();
    let mut values = Vec::with_capacity(names.len());
    for name in names {
      let symbol_id = hash_str(name);
      let Some(value_ref) = symbols_brrw.get(symbol_id) else {
        return Err(MechError::new(ProgramOutputNotFound { name: (*name).to_string() }, None));
      };
      values.push(((*name).to_string(), value_ref.borrow().clone()));
    }
    Ok(values)
  }

  pub fn bind_ans_for_interpreter(
    &mut self,
    interpreter_id: u64,
    value: &Value,
  ) -> bool {
    bind_ans_recursive(&mut self.interpreter, interpreter_id, value)
  }

  #[cfg(feature = "functions")]
  pub fn step(&mut self, count: u64) -> MResult<()> {
    self.interpreter.step(count as usize, 1)?;
    Ok(())
  }


  pub fn ensure_input(
    &mut self,
    interpreter_id: u64,
    symbol_id: u64,
    name: &str,
    initial_value: Value,
  ) -> MResult<ProgramInputId> {
    let Some(existed) = with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
      let symbols = interpreter.symbols();
      let mut symbols_brrw = symbols.borrow_mut();
      let existed = symbols_brrw.contains(symbol_id);
      if !existed {
        symbols_brrw.insert(symbol_id, initial_value.clone(), true);
      }
      symbols_brrw.dictionary.borrow_mut().insert(symbol_id, name.to_string());
      interpreter.dictionary().borrow_mut().insert(symbol_id, name.to_string());
      existed
    }) else {
      return Err(MechError::new(ProgramInputError { reason: format!("missing interpreter {interpreter_id}") }, None));
    };
    let input = ProgramInputId { interpreter_id, symbol_id };
    if existed {
      self.update_input(input, initial_value)?;
    }
    Ok(input)
  }

  pub fn update_input(&mut self, input: ProgramInputId, value: Value) -> MResult<usize> {
    self.update_inputs(&[ProgramInputUpdate { input, value }])
  }

  pub fn update_inputs(&mut self, updates: &[ProgramInputUpdate]) -> MResult<usize> {
    Ok(self.update_inputs_with_dirty_cells(updates)?.updated_count)
  }

  pub fn update_inputs_with_dirty_cells(
    &mut self,
    updates: &[ProgramInputUpdate],
  ) -> MResult<ProgramInputUpdateOutcome> {
    let prepared = self.prepare_input_updates(updates)?;
    Self::apply_prepared_input_updates(&prepared)?;
    Ok(ProgramInputUpdateOutcome {
      updated_count: prepared.assignments.len(),
      dirty_cells: prepared.dirty_cells,
    })
  }

  fn prepare_input_updates(
    &self,
    updates: &[ProgramInputUpdate],
  ) -> MResult<PreparedProgramInputBatch> {
    let mut seen_targets = BTreeSet::new();
    let mut assignments = Vec::with_capacity(updates.len());
    let mut dirty_cells = Vec::new();
    let mut dirty_cells_by_interpreter = BTreeMap::new();
    for update in updates {
      let Some((actual_interpreter_id, sink)) = with_interpreter(&self.interpreter, update.input.interpreter_id, &mut |interpreter| {
        let sink = interpreter.symbols().borrow().get(update.input.symbol_id);
        (interpreter.id, sink)
      }) else {
        return Err(MechError::new(ProgramInputError { reason: format!("missing interpreter {}", update.input.interpreter_id) }, None));
      };
      let Some(sink) = sink else {
        return Err(MechError::new(ProgramInputError { reason: format!("missing program input cell {}", update.input.symbol_id) }, None));
      };
      let canonical_input = ProgramInputId {
        interpreter_id: actual_interpreter_id,
        symbol_id: update.input.symbol_id,
      };
      if !seen_targets.insert(canonical_input) {
        return Err(MechError::new(ProgramInputDuplicateTarget { input: canonical_input }, None));
      }
      assignments.push(compile_stable_value_update(sink.clone(), update.value.clone())?);
      for cell in val_ref_reactive_cell_ids(&sink) {
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
    Ok(PreparedProgramInputBatch {
      assignments,
      dirty_cells,
      dirty_cells_by_interpreter,
    })
  }

  fn apply_prepared_input_updates(prepared: &PreparedProgramInputBatch) -> MResult<()> {
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
    let Some(result) = with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
      interpreter.advance_reactive_turn(dirty_cells)
    }) else {
      return Err(MechError::new(ProgramInputError { reason: format!("missing interpreter {interpreter_id}") }, None));
    };
    result
  }

  /// Applies all input writes before advancing affected interpreters in ascending
  /// actual interpreter-ID order. If a turn fails, prior input writes and prior
  /// interpreter commits remain, the failing turn retains its reactive-plan error
  /// state, later interpreters are not executed, and no rollback is attempted.
  #[cfg(feature = "functions")]
  pub fn update_inputs_and_advance_turn(
    &mut self,
    updates: &[ProgramInputUpdate],
  ) -> MResult<ProgramInputTurnOutcome> {
    let prepared = self.prepare_input_updates(updates)?;
    Self::apply_prepared_input_updates(&prepared)?;
    let updated_count = prepared.assignments.len();
    let mut interpreter_turns = Vec::with_capacity(prepared.dirty_cells_by_interpreter.len());
    for (interpreter_id, dirty_cells) in prepared.dirty_cells_by_interpreter {
      let turn = self.advance_reactive_turn(interpreter_id, &dirty_cells)?;
      interpreter_turns.push(ProgramInterpreterTurnOutcome {
        interpreter_id,
        dirty_cells,
        turn,
      });
    }
    Ok(ProgramInputTurnOutcome { updated_count, interpreter_turns })
  }

  #[cfg(feature = "functions")]
  pub fn solve_plan(&mut self) -> MResult<ProgramSolveOutcome> {
    let plan_len = self.interpreter.plan_len();
    let value = self.interpreter.solve_plan()?;
    Ok(ProgramSolveOutcome { value, plan_len })
  }

  pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<Value> {
    match source {
      MechSourceCode::String(source) => self.run_string(source),
      MechSourceCode::Tree(tree) => self.run_tree(tree),
      MechSourceCode::ByteCode(bytecode) => self.run_bytecode(bytecode),
      MechSourceCode::Program(sources) => self.run_sources(sources),
      unsupported => Err(MechError::new(
        UnsupportedProgramSourceError {
          source_kind: format!("{:?}", unsupported),
        },
        None,
      )),
    }
  }

  pub fn run_sources(&mut self, sources: &[MechSourceCode]) -> MResult<Value> {
    let mut value = Value::Empty;

    for source in sources {
      value = self.run_source(source)?;
    }

    Ok(value)
  }

  #[cfg(feature = "compiler")]
  pub fn compile_bytecode(&mut self) -> MResult<Vec<u8>> {
    let state_brrw = self.interpreter.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut();

    let mut ctx = CompileCtx::new();

    for step in plan_brrw.iter() {
      step.compile(&mut ctx)?;
    }

    let bytes = ctx.compile()?;
    self.interpreter.context = Some(ctx);

    Ok(bytes)
  }
}


fn with_interpreter_mut<T>(
  interpreter: &mut Interpreter,
  interpreter_id: u64,
  f: &mut impl FnMut(&mut Interpreter) -> T,
) -> Option<T> {
  if interpreter_id == 0 || interpreter.id == interpreter_id {
    return Some(f(interpreter));
  }
  let child_ids = interpreter.sub_interpreters.borrow().keys().copied().collect::<Vec<_>>();
  for child_id in child_ids {
    let mut sub_interpreters = interpreter.sub_interpreters.borrow_mut();
    let Some(child) = sub_interpreters.get_mut(&child_id) else { continue; };
    if let Some(result) = with_interpreter_mut(child.as_mut(), interpreter_id, f) { return Some(result); }
  }
  None
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramOutputNotFound { pub name: String }
impl MechErrorKind for ProgramOutputNotFound {
  fn name(&self) -> &str { "ProgramOutputNotFound" }
  fn message(&self) -> String { format!("program output symbol `{}` was not found", self.name) }
}

#[derive(Debug, Clone)]
pub struct ProgramInputError { pub reason: String }
impl MechErrorKind for ProgramInputError {
  fn name(&self) -> &str { "ProgramInputError" }
  fn message(&self) -> String { self.reason.clone() }
}

#[derive(Debug, Clone)]
pub struct ProgramInputDuplicateTarget { pub input: ProgramInputId }
impl MechErrorKind for ProgramInputDuplicateTarget {
  fn name(&self) -> &str { "ProgramInputDuplicateTarget" }
  fn message(&self) -> String { format!("duplicate program input target {:?}", self.input) }
}

fn with_interpreter<T>(
  interpreter: &Interpreter,
  interpreter_id: u64,
  f: &mut impl FnMut(&Interpreter) -> T,
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
  interpreter: &mut Interpreter,
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
  interpreter: &mut Interpreter,
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

  fn program_with_nested_interpreter(nested_id: u64, child_id: u64) -> MechProgram {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let mut child = program.interpreter().clone();
    child.clear();
    child.id = child_id;
    let mut nested = program.interpreter().clone();
    nested.clear();
    nested.id = nested_id;
    child
      .sub_interpreters
      .borrow_mut()
      .insert(nested_id, Box::new(nested));
    program
      .interpreter_mut()
      .sub_interpreters
      .borrow_mut()
      .insert(child_id, Box::new(child));
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
      let mut sub_interpreters = root.sub_interpreters.borrow_mut();
      let child = sub_interpreters.get_mut(&child_id).unwrap();
      let mut child_sub_interpreters = child.sub_interpreters.borrow_mut();
      let nested = child_sub_interpreters.get_mut(&nested_id).unwrap();
      nested
        .out_values
        .borrow_mut()
        .insert(output_id, Value::U64(mech_core::Ref::new(42)));
    }

    assert!(program.output_value_for_interpreter(nested_id, output_id).is_some());
  }

  #[test]
  fn program_bind_ans_for_interpreter_binds_ans() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let value = Value::U64(mech_core::Ref::new(42));
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
  use mech_core::{hash_str, Ref};
  #[cfg(all(feature = "matrix", feature = "f64"))]
  use mech_core::structures::matrix::Matrix as MechMatrix;

  fn f64_value(value: &Value) -> f64 {
    match value {
      Value::F64(value) => *value.borrow(),
      other => panic!("expected f64, got {other:?}"),
    }
  }

  #[cfg(feature = "f64")]
  #[test]
  fn stable_value_update_preserves_f64_reference() {
    let sink = Ref::new(Value::F64(Ref::new(1.0)));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::F64(value) => value.as_ptr(),
      other => panic!("expected f64, got {other:?}"),
    };
    apply_stable_value_update(sink.clone(), Value::F64(Ref::new(9.0))).unwrap();
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::F64(value) => {
        assert_eq!(inner_pointer, value.as_ptr());
        assert_eq!(*value.borrow(), 9.0);
      }
      other => panic!("expected f64, got {other:?}"),
    }
  }

  #[cfg(feature = "i64")]
  #[test]
  fn stable_value_update_preserves_i64_reference() {
    let sink = Ref::new(Value::I64(Ref::new(1)));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::I64(value) => value.as_ptr(),
      other => panic!("expected i64, got {other:?}"),
    };
    apply_stable_value_update(sink.clone(), Value::I64(Ref::new(9))).unwrap();
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::I64(value) => {
        assert_eq!(inner_pointer, value.as_ptr());
        assert_eq!(*value.borrow(), 9);
      }
      other => panic!("expected i64, got {other:?}"),
    }
  }

  #[cfg(feature = "bool")]
  #[test]
  fn stable_value_update_preserves_bool_reference() {
    let sink = Ref::new(Value::Bool(Ref::new(false)));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::Bool(value) => value.as_ptr(),
      other => panic!("expected bool, got {other:?}"),
    };
    apply_stable_value_update(sink.clone(), Value::Bool(Ref::new(true))).unwrap();
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::Bool(value) => {
        assert_eq!(inner_pointer, value.as_ptr());
        assert!(*value.borrow());
      }
      other => panic!("expected bool, got {other:?}"),
    }
  }

  #[cfg(any(feature = "string", feature = "variable_define"))]
  #[test]
  fn stable_value_update_preserves_string_reference() {
    let sink = Ref::new(Value::String(Ref::new("old".to_string())));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::String(value) => value.as_ptr(),
      other => panic!("expected string, got {other:?}"),
    };
    apply_stable_value_update(sink.clone(), Value::String(Ref::new("new".to_string()))).unwrap();
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::String(value) => {
        assert_eq!(inner_pointer, value.as_ptr());
        assert_eq!(&*value.borrow(), "new");
      }
      other => panic!("expected string, got {other:?}"),
    }
  }

  #[test]
  fn stable_value_update_preserves_index_reference() {
    let sink = Ref::new(Value::Index(Ref::new(1)));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::Index(value) => value.as_ptr(),
      other => panic!("expected index, got {other:?}"),
    };
    apply_stable_value_update(sink.clone(), Value::Index(Ref::new(9))).unwrap();
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::Index(value) => {
        assert_eq!(inner_pointer, value.as_ptr());
        assert_eq!(*value.borrow(), 9);
      }
      other => panic!("expected index, got {other:?}"),
    }
  }

  #[cfg(all(feature = "f64", any(feature = "string", feature = "variable_define")))]
  #[test]
  fn stable_value_update_rejects_incompatible_kind() {
    let sink = Ref::new(Value::F64(Ref::new(1.0)));
    let inner_pointer = match &*sink.borrow() {
      Value::F64(value) => value.as_ptr(),
      other => panic!("expected f64, got {other:?}"),
    };
    assert!(apply_stable_value_update(sink.clone(), Value::String(Ref::new("bad".to_string()))).is_err());
    match &*sink.borrow() {
      Value::F64(value) => {
        assert_eq!(inner_pointer, value.as_ptr());
        assert_eq!(*value.borrow(), 1.0);
      }
      other => panic!("expected f64, got {other:?}"),
    }
  }

  #[cfg(all(feature = "matrix", feature = "f64"))]
  #[test]
  fn stable_value_update_preserves_matrix_storage() {
    let sink_matrix = MechMatrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let source_matrix = MechMatrix::from_vec(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let sink = Ref::new(Value::MatrixF64(sink_matrix));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::MatrixF64(value) => value.addr(),
      other => panic!("expected f64 matrix, got {other:?}"),
    };
    apply_stable_value_update(sink.clone(), Value::MatrixF64(source_matrix.clone())).unwrap();
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::MatrixF64(value) => {
        assert_eq!(inner_pointer, value.addr());
        assert_eq!(value, &source_matrix);
      }
      other => panic!("expected f64 matrix, got {other:?}"),
    }
  }


  #[cfg(feature = "f64")]
  #[test]
  fn stable_value_update_preserves_typed_scalar_reference() {
    let sink = Ref::new(Value::Typed(
      Box::new(Value::F64(Ref::new(1.0))),
      ValueKind::F64,
    ));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::Typed(inner, annotation) => {
        assert_eq!(annotation, &ValueKind::F64);
        match inner.as_ref() {
          Value::F64(value) => value.as_ptr(),
          other => panic!("expected typed f64 inner, got {other:?}"),
        }
      }
      other => panic!("expected typed value, got {other:?}"),
    };

    apply_stable_value_update(
      sink.clone(),
      Value::Typed(Box::new(Value::F64(Ref::new(9.0))), ValueKind::F64),
    ).unwrap();

    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::Typed(inner, annotation) => {
        assert_eq!(annotation, &ValueKind::F64);
        match inner.as_ref() {
          Value::F64(value) => {
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
    let sink = Ref::new(Value::Typed(
      Box::new(Value::F64(Ref::new(1.0))),
      ValueKind::F64,
    ));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::Typed(inner, _) => match inner.as_ref() {
        Value::F64(value) => value.as_ptr(),
        other => panic!("expected typed f64 inner, got {other:?}"),
      },
      other => panic!("expected typed value, got {other:?}"),
    };

    let result = apply_stable_value_update(
      sink.clone(),
      Value::Typed(Box::new(Value::F64(Ref::new(9.0))), ValueKind::String),
    );
    assert!(format!("{:?}", result.unwrap_err()).contains("StableValueUpdateKindMismatch"));

    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::Typed(inner, annotation) => {
        assert_eq!(annotation, &ValueKind::F64);
        match inner.as_ref() {
          Value::F64(value) => {
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
    let sink = Ref::new(Value::Typed(
      Box::new(Value::F64(Ref::new(1.0))),
      ValueKind::F64,
    ));
    let inner_pointer = match &*sink.borrow() {
      Value::Typed(inner, _) => match inner.as_ref() {
        Value::F64(value) => value.as_ptr(),
        other => panic!("expected typed f64 inner, got {other:?}"),
      },
      other => panic!("expected typed value, got {other:?}"),
    };

    let result = apply_stable_value_update(sink.clone(), Value::F64(Ref::new(9.0)));
    assert!(format!("{:?}", result.unwrap_err()).contains("StableValueUpdateKindMismatch"));

    match &*sink.borrow() {
      Value::Typed(inner, annotation) => {
        assert_eq!(annotation, &ValueKind::F64);
        match inner.as_ref() {
          Value::F64(value) => {
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
    let assignment = compile_stable_value_update(Ref::new(Value::Empty), Value::Empty).unwrap();
    let mut ctx = CompileCtx::new();
    let error = assignment.compile(&mut ctx).unwrap_err();
    let rendered = format!("{error:?}");
    assert!(rendered.contains("EmptyAssignmentNotBytecodeCompilable"), "{rendered}");
  }

  #[test]
  fn stable_value_update_accepts_empty_to_empty() {
    let sink = Ref::new(Value::Empty);
    compile_stable_value_update(sink.clone(), Value::Empty).unwrap();
    apply_stable_value_update(sink.clone(), Value::Empty).unwrap();
    assert_eq!(&*sink.borrow(), &Value::Empty);
  }

  #[cfg(feature = "f64")]
  #[test]
  fn stable_value_update_rejects_empty_to_value() {
    let sink = Ref::new(Value::Empty);
    let result = apply_stable_value_update(sink.clone(), Value::F64(Ref::new(1.0)));
    assert!(format!("{:?}", result.unwrap_err()).contains("StableValueUpdateKindMismatch"));
    assert_eq!(&*sink.borrow(), &Value::Empty);
  }

  #[cfg(all(feature = "matrix", feature = "f64"))]
  #[test]
  fn stable_value_update_rejects_dynamic_matrix_shape_change() {
    let sink_matrix = MechMatrix::from_vec((1..=25).map(|x| x as f64).collect(), 5, 5);
    let original = sink_matrix.clone();
    let source_matrix = MechMatrix::from_vec((1..=36).map(|x| x as f64).collect(), 6, 6);
    let sink = Ref::new(Value::MatrixF64(sink_matrix));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::MatrixF64(value) => value.addr(),
      other => panic!("expected f64 matrix, got {other:?}"),
    };

    let result = apply_stable_value_update(sink.clone(), Value::MatrixF64(source_matrix));
    assert!(format!("{:?}", result.unwrap_err()).contains("StableValueUpdateKindMismatch"));

    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::MatrixF64(value) => {
        assert_eq!(inner_pointer, value.addr());
        assert_eq!(value.shape(), vec![5, 5]);
        assert_eq!(value, &original);
      }
      other => panic!("expected f64 matrix, got {other:?}"),
    }
  }

  #[cfg(all(feature = "matrix", feature = "f64"))]
  #[test]
  fn stable_value_update_rejects_equal_length_different_shape() {
    let sink_matrix = MechMatrix::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let original = sink_matrix.clone();
    let source_matrix = MechMatrix::from_vec(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], 3, 2);
    let sink = Ref::new(Value::MatrixF64(sink_matrix));
    let inner_pointer = match &*sink.borrow() {
      Value::MatrixF64(value) => value.addr(),
      other => panic!("expected f64 matrix, got {other:?}"),
    };

    let result = apply_stable_value_update(sink.clone(), Value::MatrixF64(source_matrix));
    assert!(format!("{:?}", result.unwrap_err()).contains("StableValueUpdateKindMismatch"));

    match &*sink.borrow() {
      Value::MatrixF64(value) => {
        assert_eq!(inner_pointer, value.addr());
        assert_eq!(value.shape(), vec![2, 3]);
        assert_eq!(value, &original);
      }
      other => panic!("expected f64 matrix, got {other:?}"),
    }
  }


  #[cfg(all(feature = "matrix", feature = "f64"))]
  #[test]
  fn stable_value_update_rejects_matrix_value_sink() {
    let matrix_value = MechMatrix::from_vec(vec![Value::F64(Ref::new(1.0))], 1, 1);
    let sink = Ref::new(Value::MatrixValue(matrix_value));
    let result = apply_stable_value_update(sink.clone(), Value::F64(Ref::new(9.0)));
    let rendered = format!("{:?}", result.unwrap_err());
    assert!(rendered.contains("StableValueUpdateUnsupported"), "{rendered}");
    assert!(rendered.contains("Matrix(F64"), "{rendered}");
  }

  #[cfg(all(feature = "matrix", feature = "f64"))]
  #[test]
  fn stable_value_update_rejects_matrix_value_source() {
    let sink = Ref::new(Value::F64(Ref::new(1.0)));
    let matrix_value = MechMatrix::from_vec(vec![Value::F64(Ref::new(9.0))], 1, 1);
    let result = apply_stable_value_update(sink.clone(), Value::MatrixValue(matrix_value));
    let rendered = format!("{:?}", result.unwrap_err());
    assert!(rendered.contains("StableValueUpdateUnsupported"), "{rendered}");
    assert!(rendered.contains("Matrix(F64"), "{rendered}");
  }

  #[cfg(feature = "matrix")]
  #[test]
  fn stable_value_update_rejects_matrix_index() {
    let matrix = MechMatrix::from_vec(vec![1usize, 2, 3, 4], 2, 2);
    let original = matrix.clone();
    let sink = Ref::new(Value::MatrixIndex(matrix));
    let outer_pointer = sink.as_ptr();
    let inner_pointer = match &*sink.borrow() {
      Value::MatrixIndex(value) => value.addr(),
      other => panic!("expected index matrix, got {other:?}"),
    };
    let result = apply_stable_value_update(
      sink.clone(),
      Value::MatrixIndex(MechMatrix::from_vec(vec![5usize, 6, 7, 8], 2, 2)),
    );
    let rendered = format!("{:?}", result.unwrap_err());
    assert!(rendered.contains("StableValueUpdateUnsupported"), "{rendered}");
    assert_eq!(outer_pointer, sink.as_ptr());
    match &*sink.borrow() {
      Value::MatrixIndex(value) => {
        assert_eq!(inner_pointer, value.addr());
        assert_eq!(value.shape(), vec![2, 2]);
        assert_eq!(value, &original);
      }
      other => panic!("expected index matrix, got {other:?}"),
    }
  }

  #[test]
  fn existing_input_cell_is_mutated_without_replacing_valref() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input_id = hash_str("input");
    let output_id = hash_str("output");
    program.ensure_input(program.interpreter().id, input_id, "input", Value::F64(Ref::new(1.0))).unwrap();
    program.run_string("output := input * 2").unwrap();
    let before = program.interpreter().symbols().borrow().get(input_id).unwrap();
    let outer_pointer = before.as_ptr();
    let inner_pointer = match &*before.borrow() {
      Value::F64(value) => value.as_ptr(),
      other => panic!("expected f64 input, got {other:?}"),
    };
    let output = program.interpreter().symbols().borrow().get(output_id).unwrap();
    assert_eq!(f64_value(&output.borrow()), 2.0);

    let handle = program.ensure_input(program.interpreter().id, input_id, "input", Value::F64(Ref::new(1.0))).unwrap();
    program.update_input(handle, Value::F64(Ref::new(5.0))).unwrap();
    let after = program.interpreter().symbols().borrow().get(input_id).unwrap();
    assert_eq!(outer_pointer, after.as_ptr());
    match &*after.borrow() {
      Value::F64(value) => assert_eq!(inner_pointer, value.as_ptr()),
      other => panic!("expected f64 input, got {other:?}"),
    }

    let outcome = program.solve_plan().unwrap();
    assert!(outcome.plan_len > 0);
    let input = program.interpreter().symbols().borrow().get(input_id).unwrap();
    assert_eq!(f64_value(&input.borrow()), 5.0);
    let output = program.interpreter().symbols().borrow().get(output_id).unwrap();
    assert_eq!(f64_value(&output.borrow()), 10.0);
  }



  #[test]
  fn ensure_input_refreshes_existing_cell_without_replacing_valref() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input_id = hash_str("live-x");
    let interpreter_id = program.interpreter().id;
    program.ensure_input(interpreter_id, input_id, "live-x", Value::F64(Ref::new(1.0))).unwrap();
    let before = program.interpreter().symbols().borrow().get(input_id).unwrap();
    let before_pointer = before.as_ptr();
    assert_eq!(f64_value(&before.borrow()), 1.0);

    program.ensure_input(interpreter_id, input_id, "live-x", Value::F64(Ref::new(9.0))).unwrap();
    let after = program.interpreter().symbols().borrow().get(input_id).unwrap();
    assert_eq!(before_pointer, after.as_ptr());
    assert_eq!(f64_value(&after.borrow()), 9.0);
  }

  #[test]
  fn incompatible_input_type_is_rejected_without_mutation() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input_id = hash_str("input");
    program.ensure_input(program.interpreter().id, input_id, "input", Value::F64(Ref::new(1.0))).unwrap();
    program.run_string("output := input * 2").unwrap();
    let handle = ProgramInputId { interpreter_id: program.interpreter().id, symbol_id: input_id };
    assert!(program.update_inputs(&[ProgramInputUpdate { input: handle, value: Value::String(Ref::new("bad".to_string())) }]).is_err());
    let input = program.interpreter().symbols().borrow().get(input_id).unwrap();
    assert_eq!(f64_value(&input.borrow()), 1.0);
    let output = program.interpreter().symbols().borrow().get(hash_str("output")).unwrap();
    assert_eq!(f64_value(&output.borrow()), 2.0);
  }

  #[test]
  fn update_inputs_preflight_rejects_before_mutating_any_input() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let a_id = hash_str("a");
    let b_id = hash_str("b");
    let interpreter_id = program.interpreter().id;
    let a = program.ensure_input(interpreter_id, a_id, "a", Value::F64(Ref::new(1.0))).unwrap();
    let b = program.ensure_input(interpreter_id, b_id, "b", Value::F64(Ref::new(2.0))).unwrap();
    let result = program.update_inputs(&[
      ProgramInputUpdate { input: a, value: Value::F64(Ref::new(3.0)) },
      ProgramInputUpdate { input: b, value: Value::String(Ref::new("bad".to_string())) },
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
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input_id = hash_str("input");
    let interpreter_id = program.interpreter().id;
    let input = program
      .ensure_input(interpreter_id, input_id, "input", Value::F64(Ref::new(1.0)))
      .unwrap();
    let input_val_ref = program.interpreter().symbols().borrow().get(input_id).unwrap();
    let input_val_ref_id = input_val_ref.id();
    let nested_scalar_id = match &*input_val_ref.borrow() {
      Value::F64(value) => value.id(),
      other => panic!("expected f64 input, got {other:?}"),
    };

    let outcome = program
      .update_inputs_with_dirty_cells(&[ProgramInputUpdate {
        input,
        value: Value::F64(Ref::new(2.0)),
      }])
      .unwrap();

    assert!(outcome.dirty_cells.contains(&ReactiveCellId::new(input_val_ref_id)));
    assert!(outcome.dirty_cells.contains(&ReactiveCellId::new(nested_scalar_id)));
  }

  #[test]
  fn missing_input_returns_error() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let missing = ProgramInputId { interpreter_id: program.interpreter().id, symbol_id: hash_str("missing") };
    assert!(program.update_input(missing, Value::F64(Ref::new(1.0))).is_err());
  }
}

#[cfg(test)]
mod root_symbol_snapshot_tests {
  use super::*;
  use mech_core::Ref;

  fn f64_value(value: &Value) -> f64 {
    match value {
      Value::F64(value) => *value.borrow(),
      other => panic!("expected f64, got {other:?}"),
    }
  }

  #[test]
  fn root_symbol_value_returns_value() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string("answer := 42.0").unwrap();
    assert_eq!(f64_value(&program.root_symbol_value("answer").unwrap()), 42.0);
  }

  #[test]
  fn root_symbol_values_preserve_order() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string("a := 1.0\nb := 2.0\nc := 3.0").unwrap();
    let rows = program.root_symbol_values(&["c", "a", "b"]).unwrap();
    let names: Vec<_> = rows.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["c", "a", "b"]);
  }

  #[test]
  fn root_symbol_values_snapshot_multiple_values() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string("a := 1.0\nb := 2.0").unwrap();
    let rows = program.root_symbol_values(&["a", "b"]).unwrap();
    assert_eq!(f64_value(&rows[0].1), 1.0);
    assert_eq!(f64_value(&rows[1].1), 2.0);
  }

  #[test]
  fn missing_root_symbol_returns_structured_error() {
    let program = MechProgram::new(MechProgramConfig::default());
    let err = program.root_symbol_value("missing").unwrap_err();
    assert!(format!("{:?}", err).contains("ProgramOutputNotFound"));
  }

  #[test]
  fn snapshot_does_not_hold_symbol_table_borrow() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string("answer := 42.0").unwrap();
    let _snapshot = program.root_symbol_value("answer").unwrap();
    let symbols = program.interpreter().symbols();
    let _mutable_borrow = symbols.borrow_mut();
  }
}

#[cfg(all(test, feature = "program", feature = "functions", feature = "f64", feature = "variable_define", feature = "variable_assign"))]
mod program_reactive_turn_tests {
  use super::*;
  use mech_core::Ref;
  fn f64_value(v:f64)->Value {Value::F64(Ref::new(v))}
  fn input(p:&mut MechProgram,n:&str,v:f64)->ProgramInputId {let id=hash_str(n);p.ensure_input(p.interpreter().id,id,n,f64_value(v)).unwrap()}
  fn symbol(p:&MechProgram,id:u64,n:&str)->Value {with_interpreter(p.interpreter(),id,&mut |i|i.symbols().borrow().get(hash_str(n)).expect("symbol").borrow().clone()).expect("interpreter")}
  fn value(p:&MechProgram,id:u64,n:&str)->f64 {*symbol(p,id,n).as_f64().expect("f64").borrow()}
  fn cell(p:&MechProgram,id:u64,n:&str)->ReactiveCellId {let v=symbol(p,id,n).reactive_root_cell_ids();assert_eq!(v.len(),1,"root cell");v[0]}
  fn node(p:&MechProgram,id:u64,c:ReactiveCellId,k:ReactiveNodeKind)->ReactiveNodeId {let v=with_interpreter(p.interpreter(),id,&mut |i|{let q=i.plan();q.borrow().nodes.iter().filter(|n|n.kind==k&&n.outputs.contains(&c)).map(|n|n.id).collect::<Vec<_>>()}).expect("interpreter");assert_eq!(v.len(),1,"node");v[0]}
  fn pending(p:&MechProgram,id:u64)->bool {with_interpreter(p.interpreter(),id,&mut |i|i.has_pending_reactive_registers()).expect("interpreter")}
  fn snapshot(p:&MechProgram,id:u64)->(usize,Vec<ReactiveNodeId>,Vec<Vec<ReactiveCellId>>){with_interpreter(p.interpreter(),id,&mut |i|{let q=i.plan();let q=q.borrow();(q.len(),q.nodes.iter().map(|n|n.id).collect(),q.nodes.iter().map(|n|n.outputs.clone()).collect())}).expect("interpreter")}
  #[test] fn program_reactive_turn_updates_only_reachable_branch(){let mut p=MechProgram::new(MechProgramConfig::default());let(l,r)=(input(&mut p,"left",1.),input(&mut p,"right",2.));p.run_string("left_output := left + 1.0\nright_output := right + 1.0").unwrap();let id=p.interpreter().id;let(ln,rn)=(node(&p,id,cell(&p,id,"left_output"),ReactiveNodeKind::Combinational),node(&p,id,cell(&p,id,"right_output"),ReactiveNodeKind::Combinational));let o=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:l,value:f64_value(10.)}]).unwrap();assert_eq!(o.updated_count,1);assert_eq!(o.interpreter_turns.len(),1);let t=&o.interpreter_turns[0];assert_eq!(t.interpreter_id,id);assert_eq!((value(&p,id,"left_output"),value(&p,id,"right_output")),(11.,3.));assert!(t.turn.before_commit.executed_nodes.contains(&ln));assert!(!t.turn.before_commit.executed_nodes.contains(&rn));assert!(t.turn.before_commit.pending_register_nodes.is_empty());assert_eq!(t.turn.register_commit,ReactiveRegisterCommitOutcome::default());assert_eq!(t.turn.after_commit,ReactivePlanSolveOutcome::default());let _=r;}
  #[test] fn program_reactive_turn_batches_inputs_into_one_turn(){let mut p=MechProgram::new(MechProgramConfig::default());let(a,b)=(input(&mut p,"a",1.),input(&mut p,"b",2.));p.run_string("~total := 0.0\nsum := a + b\ntotal = sum\noutput := total + 1.0").unwrap();let id=p.interpreter().id;let(sum,total,out)=(node(&p,id,cell(&p,id,"sum"),ReactiveNodeKind::Combinational),node(&p,id,cell(&p,id,"total"),ReactiveNodeKind::Register),node(&p,id,cell(&p,id,"output"),ReactiveNodeKind::Combinational));let o=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:a,value:f64_value(10.)},ProgramInputUpdate{input:b,value:f64_value(20.)}]).unwrap();let t=&o.interpreter_turns[0].turn;assert_eq!(o.updated_count,2);assert_eq!(o.interpreter_turns.len(),1);assert_eq!(t.before_commit.executed_nodes.iter().filter(|&&x|x==sum).count(),1);assert_eq!(t.before_commit.pending_register_nodes,vec![total]);assert_eq!(t.register_commit.staged_nodes,vec![total]);assert_eq!(t.register_commit.committed_nodes,vec![total]);assert!(t.after_commit.executed_nodes.contains(&out));assert_eq!((value(&p,id,"sum"),value(&p,id,"total"),value(&p,id,"output")),(30.,30.,31.));assert!(!pending(&p,id));}
  #[test] fn program_reactive_turn_preserves_deferred_registers_between_calls(){let mut p=MechProgram::new(MechProgramConfig::default());let x=input(&mut p,"input",1.);p.run_string("~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0").unwrap();let id=p.interpreter().id;let(a,b)=(node(&p,id,cell(&p,id,"a"),ReactiveNodeKind::Register),node(&p,id,cell(&p,id,"b"),ReactiveNodeKind::Register));let o=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:x,value:f64_value(10.)}]).unwrap();let t=&o.interpreter_turns[0].turn;assert_eq!(t.register_commit.committed_nodes,vec![a]);assert_eq!(t.after_commit.pending_register_nodes,vec![b]);assert_eq!((value(&p,id,"a"),value(&p,id,"middle"),value(&p,id,"b"),value(&p,id,"output")),(10.,11.,2.,3.));assert!(pending(&p,id));let t=p.advance_reactive_turn(id,&[]).unwrap();assert_eq!(t.register_commit.committed_nodes,vec![b]);assert_eq!((value(&p,id,"a"),value(&p,id,"middle"),value(&p,id,"b"),value(&p,id,"output")),(10.,11.,11.,12.));assert!(!pending(&p,id));}
  #[test] fn program_reactive_turn_legacy_update_api_does_not_execute_plan(){let mut p=MechProgram::new(MechProgramConfig::default());let x=input(&mut p,"input",1.);p.run_string("output := input * 2.0").unwrap();let id=p.interpreter().id;let u=p.update_inputs_with_dirty_cells(&[ProgramInputUpdate{input:x,value:f64_value(5.)}]).unwrap();assert_eq!((value(&p,id,"input"),value(&p,id,"output")),(5.,2.));assert!(!pending(&p,id));assert_eq!(u.updated_count,1);assert!(!u.dirty_cells.is_empty());p.advance_reactive_turn(id,&u.dirty_cells).unwrap();assert_eq!(value(&p,id,"output"),10.);}
  #[cfg(feature="string")] #[test] fn program_reactive_turn_preflight_failure_mutates_nothing(){let mut p=MechProgram::new(MechProgramConfig::default());let(a,b)=(input(&mut p,"a",1.),input(&mut p,"b",2.));p.run_string("output := a + b").unwrap();let id=p.interpreter().id;let s=snapshot(&p,id);let q=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:a,value:f64_value(10.)},ProgramInputUpdate{input:b,value:Value::String(Ref::new("bad".into()))}]).unwrap_err();assert!(format!("{q:?}").contains("StableValueUpdateKindMismatch"));assert_eq!((value(&p,id,"a"),value(&p,id,"b"),value(&p,id,"output")),(1.,2.,3.));assert_eq!(snapshot(&p,id),s);assert!(!pending(&p,id));}
  #[test] fn program_reactive_turn_rejects_root_alias_duplicate(){let mut p=MechProgram::new(MechProgramConfig::default());let x=input(&mut p,"input",1.);let id=p.interpreter().id;let e=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:ProgramInputId{interpreter_id:0,symbol_id:x.symbol_id},value:f64_value(2.)},ProgramInputUpdate{input:x,value:f64_value(3.)}]).unwrap_err();assert!(format!("{e:?}").contains("ProgramInputDuplicateTarget"));assert_eq!(value(&p,id,"input"),1.);assert!(!pending(&p,id));}
  #[test] fn program_reactive_turn_empty_batch_is_noop(){let mut p=MechProgram::new(MechProgramConfig::default());let id=p.interpreter().id;let s=snapshot(&p,id);assert_eq!(p.update_inputs_and_advance_turn(&[]).unwrap(),ProgramInputTurnOutcome::default());assert_eq!(snapshot(&p,id),s);assert!(!pending(&p,id));}
  #[test] fn program_reactive_turn_orders_nested_interpreters_deterministically(){let mut p=MechProgram::new(MechProgramConfig::default());let mut c1=Interpreter::new_with_full_stdlib(101);let mut c2=Interpreter::new_with_full_stdlib(202);for(i,src) in [(&mut c1,"input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0"),(&mut c2,"input := 1.0\noutput := input + 1.0")]{i.interpret(&parser::parse(src).unwrap()).unwrap();}p.interpreter_mut().sub_interpreters.borrow_mut().insert(101,Box::new(c1));p.interpreter_mut().sub_interpreters.borrow_mut().insert(202,Box::new(c2));let(a,b)=(ProgramInputId{interpreter_id:101,symbol_id:hash_str("input")},ProgramInputId{interpreter_id:202,symbol_id:hash_str("input")});let root=snapshot(&p,p.interpreter().id);let o=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:b,value:f64_value(20.)},ProgramInputUpdate{input:a,value:f64_value(10.)}]).unwrap();assert_eq!(o.updated_count,2);assert_eq!(o.interpreter_turns.iter().map(|x|x.interpreter_id).collect::<Vec<_>>(),vec![101,202]);assert!(!o.interpreter_turns[0].dirty_cells.is_empty());assert!(!o.interpreter_turns[1].dirty_cells.is_empty());assert_ne!(o.interpreter_turns[0].dirty_cells,o.interpreter_turns[1].dirty_cells);assert_eq!((value(&p,101,"a"),value(&p,101,"middle"),value(&p,101,"b")),(10.,11.,2.));assert!(pending(&p,101));assert_eq!(value(&p,202,"output"),21.);assert!(!pending(&p,202));assert!(!pending(&p,p.interpreter().id));assert_eq!(snapshot(&p,p.interpreter().id),root);p.advance_reactive_turn(101,&[]).unwrap();assert_eq!((value(&p,101,"b"),value(&p,101,"output"),value(&p,202,"output")),(11.,12.,21.));assert!(!pending(&p,101));assert!(!pending(&p,202));}
  #[cfg(feature="compiler")] #[test] fn program_reactive_turn_decoded_plan_reuses_identity(){let mut source=MechProgram::new(MechProgramConfig::default());source.load_full_stdlib();source.run_string("input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0\noutput").unwrap();let bytes=source.compile_bytecode().unwrap();let mut p=MechProgram::new(MechProgramConfig::default());p.load_full_stdlib();p.run_bytecode(&bytes).unwrap();let id=p.interpreter().id;let x=p.ensure_input(id,hash_str("input"),"input",f64_value(1.)).unwrap();let s=snapshot(&p,id);let(a,b)=(node(&p,id,cell(&p,id,"a"),ReactiveNodeKind::Register),node(&p,id,cell(&p,id,"b"),ReactiveNodeKind::Register));let o=p.update_inputs_and_advance_turn(&[ProgramInputUpdate{input:x,value:f64_value(10.)}]).unwrap();assert_eq!(o.interpreter_turns[0].turn.register_commit.committed_nodes,vec![a]);assert_eq!(o.interpreter_turns[0].turn.after_commit.pending_register_nodes,vec![b]);let o=p.advance_reactive_turn(id,&[]).unwrap();assert_eq!(o.register_commit.committed_nodes,vec![b]);assert_eq!(value(&p,id,"output"),12.);assert!(!pending(&p,id));assert_eq!(snapshot(&p,id),s);}
}
