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
  pub code: Vec<MechSourceCode>,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<Interpreter>>>,
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
      registers: Vec::new(),
      constants: Vec::new(),
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      code: Vec::new(),
    }
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
  pub fn step(&mut self, steps: u64) -> &Value {
    let state_brrw = self.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut();
    let mut result = Value::Empty;
    for i in 0..steps {
      for fxn in plan_brrw.iter() {
        fxn.solve();
      }
    }
    self.out = plan_brrw.last().unwrap().out().clone();
    &self.out
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
    let mut state_brrw = self.state.borrow_mut();
    let mut symbol_table = state_brrw.symbol_table.borrow_mut();
    for (id, reg) in program.symbols.iter() {
      let constant = self.constants[*reg as usize].clone();
      self.out = constant.clone();`
      let mutable = program.mutable_symbols.contains(id);
      symbol_table.insert(*id, constant, mutable);
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
                let mut state_brrw = self.state.borrow_mut();
                state_brrw.add_plan_step(fxn);
              },
              None => {
                return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Unknown nullary function ID: {}", fxn_id), id: line!(), kind: MechErrorKind::GenericError("Unknown nullary function".to_string())});
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
                return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Unknown unary function ID: {}", fxn_id), id: line!(), kind: MechErrorKind::GenericError("Unknown unary function".to_string())});
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
                return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Unknown binary function ID: {}", fxn_id), id: line!(), kind: MechErrorKind::GenericError("Unknown binary function".to_string())});
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
                return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Unknown ternary function ID: {}", fxn_id), id: line!(), kind: MechErrorKind::GenericError("Unknown ternary function".to_string())});
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
                return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Unknown quaternary function ID: {}", fxn_id), id: line!(), kind: MechErrorKind::GenericError("Unknown quaternary function".to_string())});
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
                return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Unknown variadic function ID: {}", fxn_id), id: line!(), kind: MechErrorKind::GenericError("Unknown variadic function".to_string())});
              }
            }
          },
          DecodedInstr::Ret{ src } => {
            todo!();
          },
          x => {
            return Err(MechError {
              file: file!().to_string(),
              tokens: vec![],
              msg: format!("Unknown instruction: {:?}", x),
              id: line!(),
              kind: MechErrorKind::GenericError("Unknown instruction".to_string()),
            });
          }
        }
        self.ip += 1;
      }
    }
    // Load the dictionary
    for (id, name) in &program.dictionary {
      symbol_table.dictionary.borrow_mut().insert(*id, name.clone());
      state_brrw.dictionary.borrow_mut().insert(*id, name.clone());
    } 
    Ok(self.out.clone())
  }

  #[cfg(feature = "compiler")]
  pub fn compile(&self) -> MResult<Vec<u8>> {
    let state_brrw = self.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut();
    let mut ctx = CompileCtx::new();
    for step in plan_brrw.iter() {
      step.compile(&mut ctx)?;
    }
    ctx.compile()
  }

}