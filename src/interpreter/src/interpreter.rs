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
  registers: Vec<Value>,
  constants: Vec<Value>,
  symbols: SymbolTableRef,
  pub code: Vec<MechSourceCode>,
  plan: Plan,
  functions: FunctionsRef,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<Interpreter>>>,
}

impl Interpreter {
  pub fn new(id: u64) -> Self {
    let mut state = ProgramState::new();
    load_stdkinds(&mut state.kinds);
    let mut fxns = Functions::new();
    load_stdlib(&mut fxns);
    Self {
      id,
      ip: 0,
      state: Ref::new(state),
      registers: Vec::new(),
      constants: Vec::new(),
      symbols: Ref::new(SymbolTable::new()),
      plan: Plan::new(),
      functions: Ref::new(fxns),
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      code: Vec::new(),
    }
  }

  pub fn plan(&self) -> Plan {
    self.plan.clone()
  }

  pub fn clear(&mut self) {
    self.ip = 0;
    let mut state = ProgramState::new();
    self.symbols = Ref::new(SymbolTable::new());
    self.registers.clear();
    self.constants.clear();
    self.plan = Plan::new();
    self.out = Value::Empty;
    self.out_values = Ref::new(HashMap::new());
    self.code = Vec::new();
    self.sub_interpreters = Ref::new(HashMap::new());
    let mut fxns = Functions::new();
    load_stdkinds(&mut state.kinds);
    load_stdlib(&mut fxns);
    self.functions = Ref::new(fxns);
    self.state = Ref::new(ProgramState::new());
  }

  #[cfg(feature = "symbol_table")]
  pub fn get_symbol(&self, id: u64) -> Option<Ref<Value>> {
    let symbols_brrw = self.symbols.borrow();
    symbols_brrw.get(id)
  }

  #[cfg(feature = "symbol_table")]
  pub fn symbols(&self) -> SymbolTableRef {
    self.symbols.clone()
  }

  #[cfg(feature = "functions")]
  pub fn functions(&self) -> FunctionsRef {
    self.functions.clone()
  }
  
  #[cfg(feature = "functions")]
  pub fn add_plan_step(&self, step: Box<dyn MechFunction>) {
    let mut plan_brrw = self.plan.borrow_mut();
    plan_brrw.push(step);
  }

  #[cfg(feature = "functions")]
  pub fn insert_function(&self, fxn: FunctionDefinition) {
    let mut fxns_brrw = self.functions.borrow_mut();
    fxns_brrw.functions.insert(fxn.id, fxn);
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print_symbols(&self) -> String {
    let symbol_table = self.symbols.borrow();
    symbol_table.pretty_print()
  }

  pub fn dictionary(&self) -> Ref<Dictionary> {
    let symbols_ref = self.symbols.borrow();
    symbols_ref.dictionary.clone()
  }

  #[cfg(feature = "functions")]
  pub fn step(&mut self, steps: u64) -> &Value {
    let plan_brrw = self.plan.borrow();
    let mut result = Value::Empty;
    for i in 0..steps {
      for fxn in plan_brrw.iter() {
        fxn.solve();
      }
    }
    self.out = plan_brrw.last().unwrap().out().clone();
    &self.out
  }

  pub fn interpret(&mut self, tree: &Program) -> MResult<Value> {
    self.code.push(MechSourceCode::Tree(tree.clone()));
    catch_unwind(AssertUnwindSafe(|| {
      let result = program(tree, &self);
      if let Some(last_step) = self.plan.borrow().last() {
        self.out = last_step.out().clone();
      } else {
        self.out = Value::Empty;
      }
      result
    }))
    .map_err(|err| {
      let kind = {
        if let Some(raw_msg) = err.downcast_ref::<&'static str>() {
          if raw_msg.contains("Index out of bounds") {
            MechErrorKind::IndexOutOfBounds
          } else if raw_msg.contains("attempt to subtract with overflow") {
            MechErrorKind::IndexOutOfBounds
          } else {
            MechErrorKind::GenericError(raw_msg.to_string())
          }
        } else {
          MechErrorKind::GenericError("Unknown panic".to_string())
        }
      };
      MechError {
        file: file!().to_string(),
        tokens: vec![],
        msg: "Interpreter panicked".to_string(),
        id: line!(),
        kind
      }
    })?
  }

  #[cfg(feature = "compiler")]
  pub fn run_program(&mut self, program: &ParsedProgram) -> MResult<Value> {
    println!("Program! {:#?}", program);
    // Reset the instruction pointer
    self.ip = 0;
    // Resize the registers and constant table
    self.registers = vec![Value::Empty; program.header.reg_count as usize];
    self.constants = vec![Value::Empty; program.const_entries.len()];
    // Load the constants
    self.constants = program.decode_const_entries()?;
    while self.ip < program.instrs.len() {
      let instr = &program.instrs[self.ip];
      match instr {
        DecodedInstr::ConstLoad { dst, const_id } => {
          let value = self.constants[*const_id as usize].clone();
          self.registers[*dst as usize] = value;
        },
        x => todo!("Implement instruction: {x:?}"),
      }
      self.ip += 1;
    }
    // Load the symbol table
    for (id, reg) in program.symbols.iter() {
      self.symbols.borrow_mut().insert(*id, self.constants[*reg as usize].clone(), false); // the false indicates it's not mutable.
    }
    // Load the dictionary
    for (id, name) in &program.dictionary {
      self.symbols.borrow().dictionary.borrow_mut().insert(*id, name.clone());
    } 
    Ok(Value::Empty)
  }

  #[cfg(feature = "compiler")]
  pub fn compile(&self) -> MResult<Vec<u8>> {
    let plan_brrw = self.plan.borrow();
    let mut ctx = CompileCtx::new();
    for step in plan_brrw.iter() {
      step.compile(&mut ctx)?;
    }
    ctx.compile()
  }

}