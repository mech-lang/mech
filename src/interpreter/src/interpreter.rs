use crate::*;
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

pub struct Interpreter {
  pub id: u64,
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
  pub sub_interpreters: Ref<HashMap<u64, Box<Interpreter>>>,
}

impl Clone for Interpreter {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
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
    *self = Interpreter::new(id, self.max_steps);
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
    let plan = self.plan();
    plan.advance_reactive_turn(&mut self.reactive_turn_state, dirty_cells)
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
    self.step(0, 1)
  }

  #[cfg(feature = "functions")]
  pub fn step(&mut self, step_id: usize, step_count: u64) -> MResult<Value> {
    let state_brrw = self.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut(); // RefMut<Vec<Box<dyn MechFunction>>>

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
            fxn.solve_result()?;
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
            fxn.solve_result()?;
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
      fxn.solve_result()?;
    }

    Ok(fxn.out().clone())
  }

  #[cfg(feature = "functions")]
  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    self.code.push(MechSourceCode::Tree(tree.clone()));
    catch_unwind(AssertUnwindSafe(|| {
      let result = program(tree, &self);
      match self.state.borrow().plan.borrow().last() {
        Some(last_step) => self.out = last_step.out().clone(),
        None => self.out = Value::Empty,
      }
      result
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
    })?
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
