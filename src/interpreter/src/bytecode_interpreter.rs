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
  symbols: SymbolTableRef,
  pub code: ParsedProgram,
  plan: Plan,
  functions: FunctionsRef,
  pub out: Value,
  pub out_values: Ref<HashMap<u64, Value>>,
  pub sub_interpreters: Ref<HashMap<u64, Box<BytecodeInterpreter>>>,
}

impl BytecodeInterpreter {
  pub fn new(id: u64, code: ParsedProgram) -> BytecodeInterpreter {
    let mut intrp = BytecodeInterpreter {
      id,
      symbols: Ref::new(SymbolTable::new()),
      plan: Plan::new(),
      functions: Ref::new(Functions::new()),
      out: Value::Empty,
      sub_interpreters: Ref::new(HashMap::new()),
      out_values: Ref::new(HashMap::new()),
      code,
    };
    let fxns = &intrp.functions;
    load_stdkinds(fxns);
    load_stdlib(fxns);
    intrp
  }

  pub fn plan(&self) -> Plan {
    self.plan.clone()
  }

  pub fn clear(&mut self) {
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

  pub fn load_program(&mut self, code: ParsedProgram) {
    self.code = code;
    

    
  }

}