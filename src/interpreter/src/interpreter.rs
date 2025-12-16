use crate::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::io::{Cursor, Read, Write};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

// Interpreter 
// ----------------------------------------------------------------------------

pub struct Interpreter {
  pub id: u64,
  ip: usize,  // instruction pointer
  pub state: Ref<ProgramState>,
  pub stack: Vec<Frame>,
  registers: Vec<Value>,
  constants: Vec<Value>,
  #[cfg(feature = "compiler")]
  pub context: Option<CompileCtx>,
  pub code: Vec<MechSourceCode>,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<Interpreter>>>,
}

impl Clone for Interpreter {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      ip: self.ip,
      state: Ref::new(self.state.borrow().clone()),
      stack: self.stack.clone(),
      registers: self.registers.clone(),
      constants: self.constants.clone(),
      #[cfg(feature = "compiler")]
      context: None,
      code: self.code.clone(),
      out: self.out.clone(),
      out_values: self.out_values.clone(),
      sub_interpreters: self.sub_interpreters.clone(),
    }
  }
}

impl Interpreter {
  pub fn new(id: u64) -> Self {
    let mut state = ProgramState::new();
    load_stdkinds(&mut state.kinds);
    #[cfg(feature = "functions")]
    load_stdlib(&mut state.functions.borrow_mut());
    Self {
      id,
      ip: 0,
      state: Ref::new(state),
      stack: Vec::new(),
      registers: Vec::new(),
      constants: Vec::new(),
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      code: Vec::new(),
      #[cfg(feature = "compiler")]
      context: None,
    }
  }

  #[cfg(feature = "symbol_table")]
  pub fn set_environment(&mut self, env: SymbolTableRef) {
    self.state.borrow_mut().environment = Some(env);
  }

  pub fn clear_plan(&mut self) {
    self.state.borrow_mut().plan.borrow_mut().clear();
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
    *self = Interpreter::new(id);
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
  pub fn step(&mut self, step_id: usize, step_count: u64) -> MResult<Value> {
    let state_brrw = self.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut(); // RefMut<Vec<Box<dyn MechFunction>>>

    if plan_brrw.is_empty() {
      return Err(MechError2::new(
          NoStepsInPlanError,
          None
        ).with_compiler_loc()
      );
    }

    let len = plan_brrw.len();

    // Case 1: step_id == 0, run entire plan step_count times
    if step_id == 0 {
      for _ in 0..step_count {
        for fxn in plan_brrw.iter_mut() {
          fxn.solve();
        }
      }
      return Ok(plan_brrw[len - 1].out().clone());
    }

    // Case 2: step a single function by index
    let idx = step_id as usize;
    if idx > len {
      return Err(MechError2::new(
        StepIndexOutOfBoundsError {
          step_id,
          plan_length: len,
        },
        None
      ).with_compiler_loc());
    }

    let fxn = &mut plan_brrw[idx - 1];

    let fxn_str = fxn.to_string();
    if fxn_str.lines().count() > 30 {
      let lines: Vec<&str> = fxn_str.lines().collect();
      println!("Stepping function:");
      for line in &lines[0..10] {
        println!("{}", line);
      }
      println!("...");
      for line in &lines[lines.len() - 10..] {
        println!("{}", line);
      }
    } else {
      println!("Stepping function:\n{}", fxn_str);
    }

    for _ in 0..step_count {
      fxn.solve();
    }

    Ok(fxn.out().clone())
  }



  #[cfg(feature = "functions")]
  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    self.code.push(MechSourceCode::Tree(tree.clone()));
    catch_unwind(AssertUnwindSafe(|| {
      let result = program(tree, &self);
      if let Some(last_step) = self.state.borrow().plan.borrow().last() {
        self.out = last_step.out().clone();
      } else {
        self.out = Value::Empty;
      }
      result
    }))
    .map_err(|err| {
      if let Some(raw_msg) = err.downcast_ref::<&'static str>() {
        if raw_msg.contains("Index out of bounds") {
          MechError2::new(
            IndexOutOfBoundsError,
            None,
          ).with_compiler_loc()
        } else if raw_msg.contains("attempt to subtract with overflow") {
          MechError2::new(
            OverflowSubtractionError,
            None,
          ).with_compiler_loc()
        } else {
          MechError2::new(
            UnknownPanicError {
              details: raw_msg.to_string(),
            },
            None,
          ).with_compiler_loc()
        }
      } else {
        MechError2::new(
          UnknownPanicError {
            details: "Non-string panic".to_string(),
          },
          None,
        ).with_compiler_loc()
      }
    })?
}

  
  #[cfg(feature = "program")]
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
          },
          DecodedInstr::NullOp{ fxn_id, dst } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let out = &self.registers[*dst as usize];
                let fxn = fxn_factory(FunctionArgs::Nullary(out.clone()))?;
                self.out = fxn.out().clone();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError2::new(
                  UnknownNullaryFunctionError { fxn_id: *fxn_id },
                  None
                ).with_compiler_loc());
              }
            }
          },
          DecodedInstr::UnOp{ fxn_id, dst, src } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let src = &self.registers[*src as usize];
                let out = &self.registers[*dst as usize];
                let fxn = fxn_factory(FunctionArgs::Unary(out.clone(), src.clone()))?;
                self.out = fxn.out().clone();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError2::new(
                  UnknownUnaryFunctionError { fxn_id: *fxn_id },
                  None
                ).with_compiler_loc());
              }
            }
          },
          DecodedInstr::BinOp{ fxn_id, dst, lhs, rhs } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let lhs = &self.registers[*lhs as usize];
                let rhs = &self.registers[*rhs as usize];
                let out = &self.registers[*dst as usize];
                let fxn = fxn_factory(FunctionArgs::Binary(out.clone(), lhs.clone(), rhs.clone()))?;
                self.out = fxn.out().clone();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError2::new(
                  UnknownBinaryFunctionError { fxn_id: *fxn_id },
                  None
                ).with_compiler_loc());
              }
            }
          },
          DecodedInstr::TernOp{ fxn_id, dst, a, b, c } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let arg1 = &self.registers[*a as usize];
                let arg2 = &self.registers[*b as usize];
                let arg3 = &self.registers[*c as usize];
                let out = &self.registers[*dst as usize];
                let fxn = fxn_factory(FunctionArgs::Ternary(out.clone(), arg1.clone(), arg2.clone(), arg3.clone()))?;
                self.out = fxn.out().clone();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError2::new(
                  UnknownTernaryFunctionError { fxn_id: *fxn_id },
                  None
                ).with_compiler_loc());
              }
            }
          },
          DecodedInstr::QuadOp{ fxn_id, dst, a, b, c, d } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let arg1 = &self.registers[*a as usize];
                let arg2 = &self.registers[*b as usize];
                let arg3 = &self.registers[*c as usize];
                let arg4 = &self.registers[*d as usize];
                let out = &self.registers[*dst as usize];
                let fxn = fxn_factory(FunctionArgs::Quaternary(out.clone(), arg1.clone(), arg2.clone(), arg3.clone(), arg4.clone()))?;
                self.out = fxn.out().clone();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError2::new(
                  UnknownQuadFunctionError { fxn_id: *fxn_id },
                  None
                ).with_compiler_loc());
              }
            }
          },
          DecodedInstr::VarArg{ fxn_id, dst, args } => {
            match functions_table.functions.get(fxn_id) {
              Some(fxn_factory) => {
                let arg_values: Vec<Value> = args.iter().map(|r| self.registers[*r as usize].clone()).collect();
                let out = &self.registers[*dst as usize];
                let fxn = fxn_factory(FunctionArgs::Variadic(out.clone(), arg_values))?;
                self.out = fxn.out().clone();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError2::new(
                  UnknownVariadicFunctionError { fxn_id: *fxn_id },
                  None
                ).with_compiler_loc());
              }
            }
          },
          DecodedInstr::Ret{ src } => {
            todo!();
          },
          x => {
            return Err(MechError2::new(
              UnknownInstructionError { instr: format!("{:?}", x) },
              None
            ).with_compiler_loc());
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
        symbol_table.dictionary.borrow_mut().insert(*id, name.clone());
        state_brrw.dictionary.borrow_mut().insert(*id, name.clone());
      } 
    }
    Ok(self.out.clone())
  }

  #[cfg(feature = "compiler")]
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

#[derive(Debug, Clone)]
pub struct UnknownInstructionError {
  pub instr: String,
}
impl MechErrorKind2 for UnknownInstructionError {
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

impl MechErrorKind2 for UnknownVariadicFunctionError {
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
impl MechErrorKind2 for UnknownQuadFunctionError {
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
impl MechErrorKind2 for UnknownTernaryFunctionError {
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
impl MechErrorKind2 for UnknownBinaryFunctionError {
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
impl MechErrorKind2 for UnknownUnaryFunctionError {
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
impl MechErrorKind2 for UnknownNullaryFunctionError {
  fn name(&self) -> &str {
    "UnknownNullaryFunction"
  }
  fn message(&self) -> String {
    format!("Unknown nullary function ID: {}", self.fxn_id)
  }
}

#[derive(Debug, Clone)]
pub struct IndexOutOfBoundsError;
impl MechErrorKind2 for IndexOutOfBoundsError {
  fn name(&self) -> &str { "IndexOutOfBounds" }
  fn message(&self) -> String { "Index out of bounds".to_string() }
}

#[derive(Debug, Clone)]
pub struct OverflowSubtractionError;
impl MechErrorKind2 for OverflowSubtractionError {
  fn name(&self) -> &str { "OverflowSubtraction" }
  fn message(&self) -> String { "Attempted subtraction overflow".to_string() }
}

#[derive(Debug, Clone)]
pub struct UnknownPanicError {
  pub details: String
}
impl MechErrorKind2 for UnknownPanicError {
  fn name(&self) -> &str { "UnknownPanic" }
  fn message(&self) -> String { self.details.clone() }
}

#[derive(Debug, Clone)]
struct StepIndexOutOfBoundsError{
  pub step_id: usize,
  pub plan_length: usize,
}
impl MechErrorKind2 for StepIndexOutOfBoundsError {
  fn name(&self) -> &str { "StepIndexOutOfBounds" }
  fn message(&self) -> String {
    format!("Step id {} out of range (plan has {} steps)", self.step_id, self.plan_length)
  }
}

#[derive(Debug, Clone)]
struct NoStepsInPlanError;
impl MechErrorKind2 for NoStepsInPlanError {
  fn name(&self) -> &str { "NoStepsInPlan" }
  fn message(&self) -> String {
    "Plan contains no steps. This program doesn't do anything.".to_string()
  }
}