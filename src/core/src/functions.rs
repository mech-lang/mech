use crate::types::*;
use crate::value::*;
use crate::nodes::*;
use crate::*;

use hashbrown::{HashMap, HashSet};
use indexmap::map::IndexMap;
use std::rc::Rc;
use std::cell::RefCell;
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};
use std::fmt;

// Functions ------------------------------------------------------------------

pub trait MechFunction {
  fn solve(&self);
  fn out(&self) -> Value;
  fn to_string(&self) -> String;
}

pub trait NativeFunctionCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>>;
}

pub struct Functions {
  pub functions: HashMap<u64,FunctionDefinition>,
  pub function_compilers: HashMap<u64, Box<dyn NativeFunctionCompiler>>,
  pub kinds: HashMap<u64,ValueKind>,
  pub enums: HashMap<u64,MechEnum>,
}
  
impl Functions {
  pub fn new() -> Self {
      Self {
        functions: HashMap::new(), 
        function_compilers: HashMap::new(), 
        kinds: HashMap::new(),
        enums: HashMap::new(),
      }
  }
}

#[derive(Clone)]
pub struct FunctionDefinition {
  pub code: FunctionDefine,
  pub id: u64,
  pub name: String,
  pub input: IndexMap<u64, KindAnnotation>,
  pub output: IndexMap<u64, KindAnnotation>,
  pub symbols: SymbolTableRef,
  pub out: Ref<Value>,
  pub plan: Plan,
}

impl fmt::Debug for FunctionDefinition {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let input_str = format!("{:#?}", self.input);
    let output_str = format!("{:#?}", self.output);
    let symbols_str = format!("{:#?}", self.symbols);
    let mut plan_str = "".to_string();
    for step in self.plan.borrow().iter() {
      plan_str = format!("{}  - {}\n",plan_str,step.to_string());
    }
    let data = vec!["📥 Input", &input_str, 
                    "📤 Output", &output_str, 
                    "🔣 Symbols",   &symbols_str,
                    "📋 Plan", &plan_str];
    let mut table = tabled::Table::new(data);
    table
        .with(Style::modern())
        .with(Panel::header(format!("📈 UserFxn::{}\n({})", self.name, humanize(&self.id))))
        .with(Alignment::left());
    println!("{table}");
    Ok(())
  }
}

impl FunctionDefinition {

  pub fn new(id: u64, name: String, code: FunctionDefine) -> Self {
    Self {
      id,
      name,
      code,
      input: IndexMap::new(),
      output: IndexMap::new(),
      out: new_ref(Value::Empty),
      symbols: new_ref(SymbolTable::new()),
      plan: new_ref(Vec::new()),
    }
  }

  pub fn solve(&self) -> ValRef {
    let plan_brrw = self.plan.borrow();
    for step in plan_brrw.iter() {
      let result = step.solve();
    }
    self.out.clone()
  }

  pub fn out(&self) -> ValRef {
    self.out.clone()
  }
}

// User Function --------------------------------------------------------------

#[derive(Debug)]
pub struct UserFunction {
  pub fxn: FunctionDefinition,
}

impl MechFunction for UserFunction {
  fn solve(&self) {
    self.fxn.solve();
  }
  fn out(&self) -> Value {
    Value::MutableReference(self.fxn.out.clone())
  }
  fn to_string(&self) -> String { format!("UserFxn::{:?}", self.fxn.name)}
}

// Symbol Table ---------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SymbolTable {
  pub symbols: HashMap<u64,ValRef>,
  pub reverse_lookup: HashMap<*const RefCell<Value>, u64>,
}

impl SymbolTable {

  pub fn new() -> SymbolTable {
    Self {
      symbols: HashMap::new(),
      reverse_lookup: HashMap::new(),
    }
  }

  pub fn get(&self, key: u64) -> Option<ValRef> {
    self.symbols.get(&key).cloned()
  }

  pub fn insert(&mut self, key: u64, value: Value) -> ValRef {
    let cell = new_ref(value);
    self.reverse_lookup.insert(Rc::as_ptr(&cell), key);
    self.symbols.insert(key,cell.clone());
    cell.clone()
  }
}