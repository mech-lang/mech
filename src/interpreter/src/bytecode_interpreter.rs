use crate::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::io::{Cursor, Read, Write};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

// Bytecode Interpreter 
// ----------------------------------------------------------------------------

pub struct BytecodeInterpreter {
  pub id: u64,
  ip: usize,  // instruction pointer
  regs: Vec<Value>,
  const_cache: Vec<Value>,
  symbols: SymbolTableRef,
  plan: Plan,
  functions: FunctionsRef,
  pub code: ParsedProgram,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<BytecodeInterpreter>>>,
}

impl BytecodeInterpreter {
  
  pub fn new(id: u64, code: ParsedProgram) -> MResult<BytecodeInterpreter> {
    code.validate()?;
    let const_n = code.header.const_count as usize;
    let fxns = Ref::new(Functions::new());
    load_stdkinds(&fxns);
    load_stdlib(&fxns);
    let intrp = BytecodeInterpreter {
      id,
      ip: 0,
      regs: vec![Value::Empty; const_n],
      const_cache: vec![Value::Empty; const_n],
      symbols: Ref::new(SymbolTable::new()),
      plan: Plan::new(),
      functions: fxns,
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      code,
    };
    Ok(intrp)
  }

  pub fn plan(&self) -> Plan {
    self.plan.clone()
  }

  pub fn clear(&mut self) {
    self.ip = 0;
    self.regs = Vec::new();
    self.const_cache = Vec::new();
    self.symbols = Ref::new(SymbolTable::new());
    self.plan = Plan::new();
    self.functions = Ref::new(Functions::new());
    self.out = Value::Empty;
    self.out_values = Ref::new(HashMap::new());
    self.sub_interpreters = Ref::new(HashMap::new());
    let fxns = &self.functions;
    load_stdkinds(fxns);
    load_stdlib(fxns);
  }

  pub fn run_program(&mut self) -> MResult<Value> {
    self.ip = 0;
    while self.ip < self.code.instrs.len() {
      use DecodedInstr::*;
      let instr = &self.code.instrs[self.ip];
      println!("IP {}: {:?}", self.ip, instr);
      match instr {
        DecodedInstr::ConstLoad { dst, const_id } => {
          //let value = self.code.const_entries[*const_id as usize].value.clone();
          //self.regs[*reg] = value;
        },
        _ => todo!(),
      }
      self.ip += 1;
    }
    Ok(Value::Empty)
  }
}