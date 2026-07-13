use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use mech_core::{
  hash_str, value_cell_id, CompileCtx, MResult, MechError, MechErrorKind, MechSourceCode,
  NativeFunctionCompiler, ParsedProgram, PlanInvalidation, PlanInvalidationKind, PlanSolveOutcome,
  Value, ValueCellId,
};

use mech_interpreter::Interpreter;
use mech_syntax::parser;

use crate::ClosureNativeFunctionCompiler;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramInvalidation {
  pub interpreter_id: u64,
  pub invalidation: PlanInvalidation,
}

#[derive(Clone, Debug)]
pub struct ProgramInputUpdateOutcome {
  pub updated_inputs: usize,
  pub invalidations: Vec<ProgramInvalidation>,
}

#[derive(Clone, Debug)]
pub struct ProgramInterpreterSolveOutcome {
  pub interpreter_id: u64,
  pub solve: PlanSolveOutcome,
}

#[derive(Clone, Debug)]
pub struct ProgramSolveOutcome {
  pub invalidated_cells: usize,
  pub scheduled_nodes: usize,
  pub executed_nodes: usize,
  pub value: Value,
  pub interpreters: Vec<ProgramInterpreterSolveOutcome>,
}

struct PreparedProgramInputUpdate {
  interpreter_id: u64,
  assignment: Box<dyn mech_core::MechFunction>,
  cell: ValueCellId,
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
    let now = std::time::Instant::now();
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
    let Some(()) = with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
      let symbols = interpreter.symbols();
      let mut symbols_brrw = symbols.borrow_mut();
      if !symbols_brrw.contains(symbol_id) {
        symbols_brrw.insert(symbol_id, initial_value.clone(), true);
      }
      symbols_brrw.dictionary.borrow_mut().insert(symbol_id, name.to_string());
      interpreter.dictionary().borrow_mut().insert(symbol_id, name.to_string());
    }) else {
      return Err(MechError::new(ProgramInputError { reason: format!("missing interpreter {interpreter_id}") }, None));
    };
    Ok(ProgramInputId { interpreter_id, symbol_id })
  }

  pub fn update_input(&mut self, input: ProgramInputId, value: Value) -> MResult<ProgramInputUpdateOutcome> {
    self.update_inputs(&[ProgramInputUpdate { input, value }])
  }

  pub fn update_inputs(&mut self, updates: &[ProgramInputUpdate]) -> MResult<ProgramInputUpdateOutcome> {
    let mut seen_targets = BTreeSet::new();
    for update in updates {
      if !seen_targets.insert(update.input) {
        return Err(MechError::new(ProgramInputDuplicateTarget { input: update.input }, None));
      }
    }

    let mut prepared = Vec::with_capacity(updates.len());
    for update in updates {
      let Some(sink) = with_interpreter_mut(&mut self.interpreter, update.input.interpreter_id, &mut |interpreter| {
        interpreter.symbols().borrow().get(update.input.symbol_id)
      }) else {
        return Err(MechError::new(ProgramInputError { reason: format!("missing interpreter {}", update.input.interpreter_id) }, None));
      };
      let Some(sink) = sink else {
        return Err(MechError::new(ProgramInputError { reason: format!("missing program input cell {}", update.input.symbol_id) }, None));
      };
      let sink_value = Value::MutableReference(sink.clone());
      let Some(cell) = value_cell_id(&sink_value) else {
        return Err(MechError::new(ProgramInputError { reason: format!("program input cell {} has no stable identity", update.input.symbol_id) }, None));
      };
      let compiler = mech_interpreter::AssignValue {};
      let assignment = compiler.compile(&vec![sink_value, update.value.clone()])?;
      prepared.push(PreparedProgramInputUpdate {
        interpreter_id: update.input.interpreter_id,
        assignment,
        cell,
      });
    }

    let mut invalidations = BTreeSet::new();
    for update in &prepared {
      update.assignment.solve_result()?;
      invalidations.insert(ProgramInvalidation {
        interpreter_id: update.interpreter_id,
        invalidation: PlanInvalidation {
          cell: update.cell,
          kind: PlanInvalidationKind::Triggered,
        },
      });
    }

    Ok(ProgramInputUpdateOutcome {
      updated_inputs: prepared.len(),
      invalidations: invalidations.into_iter().collect(),
    })
  }

  #[cfg(feature = "functions")]
  pub fn solve_invalidated(&mut self, invalidations: &[ProgramInvalidation]) -> MResult<ProgramSolveOutcome> {
    let mut grouped: BTreeMap<u64, BTreeSet<PlanInvalidation>> = BTreeMap::new();
    for invalidation in invalidations {
      grouped.entry(invalidation.interpreter_id).or_default().insert(invalidation.invalidation);
    }

    let mut interpreters = Vec::new();
    let mut invalidated_cells = 0usize;
    let mut scheduled_nodes = 0usize;
    let mut executed_nodes = 0usize;
    let mut value = Value::Empty;

    for (interpreter_id, set) in grouped {
      let interpreter_invalidations = set.into_iter().collect::<Vec<_>>();
      let Some(solve) = with_interpreter_mut(&mut self.interpreter, interpreter_id, &mut |interpreter| {
        interpreter.solve_invalidated(&interpreter_invalidations)
      }) else {
        return Err(MechError::new(ProgramInputError { reason: format!("missing interpreter {interpreter_id}") }, None));
      };
      let solve = solve?;
      invalidated_cells += solve.invalidated_cells;
      scheduled_nodes += solve.scheduled_nodes;
      executed_nodes += solve.executed_nodes;
      value = solve.value.clone();
      interpreters.push(ProgramInterpreterSolveOutcome { interpreter_id, solve });
    }

    Ok(ProgramSolveOutcome { invalidated_cells, scheduled_nodes, executed_nodes, value, interpreters })
  }

  #[cfg(feature = "functions")]
  pub fn solve_all(&mut self) -> MResult<ProgramSolveOutcome> {
    let interpreter_id = self.interpreter.id;
    let solve = self.interpreter.solve_all()?;
    let value = solve.value.clone();
    let scheduled_nodes = solve.scheduled_nodes;
    let executed_nodes = solve.executed_nodes;
    Ok(ProgramSolveOutcome {
      invalidated_cells: 0,
      scheduled_nodes,
      executed_nodes,
      value,
      interpreters: vec![ProgramInterpreterSolveOutcome { interpreter_id, solve }],
    })
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
    let plan = state_brrw.plan.clone();

    let mut ctx = CompileCtx::new();

    plan.compile_into(&mut ctx)?;

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

  fn f64_value(value: &Value) -> f64 {
    match value {
      Value::F64(value) => *value.borrow(),
      other => panic!("expected f64, got {other:?}"),
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

    let outcome = program.solve_all().unwrap();
    assert!(outcome.scheduled_nodes > 0);
    let input = program.interpreter().symbols().borrow().get(input_id).unwrap();
    assert_eq!(f64_value(&input.borrow()), 5.0);
    let output = program.interpreter().symbols().borrow().get(output_id).unwrap();
    assert_eq!(f64_value(&output.borrow()), 10.0);
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
  fn multi_update_preflight_is_atomic() {
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

  #[test]
  fn missing_input_returns_error() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let missing = ProgramInputId { interpreter_id: program.interpreter().id, symbol_id: hash_str("missing") };
    assert!(program.update_input(missing, Value::F64(Ref::new(1.0))).is_err());
  }
}
