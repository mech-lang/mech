use crate::types::*;
use crate::value::*;
use crate::nodes::*;
use crate::*;

use hashbrown::{HashMap, HashSet};
use indexmap::map::IndexMap;
use std::rc::Rc;
use std::cell::RefCell;
#[cfg(feature = "pretty_print")]
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

#[cfg(feature = "pretty_print")]
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
    table.with(Style::modern_rounded())
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

pub type Dictionary = IndexMap<u64,String>;

#[derive(Clone, Debug)]
pub struct SymbolTable {
  pub symbols: HashMap<u64,ValRef>,
  pub mutable_variables: HashMap<u64,ValRef>,
  pub dictionary: Ref<Dictionary>,
  pub reverse_lookup: HashMap<*const RefCell<Value>, u64>,
}

impl SymbolTable {

  pub fn new() -> SymbolTable {
    Self {
      symbols: HashMap::new(),
      mutable_variables: HashMap::new(),
      dictionary: new_ref(IndexMap::new()),
      reverse_lookup: HashMap::new(),
    }
  }

  pub fn get_symbol_name_by_id(&self, id: u64) -> Option<String> {
    self.dictionary.borrow().get(&id).cloned()
  }

  pub fn get_mutable(&self, key: u64) -> Option<ValRef> {
    self.mutable_variables.get(&key).cloned()
  }

  pub fn get(&self, key: u64) -> Option<ValRef> {
    self.symbols.get(&key).cloned()
  }

  pub fn contains(&self, key: u64) -> bool {
    self.symbols.contains_key(&key)
  }

  pub fn insert(&mut self, key: u64, value: Value, mutable: bool) -> ValRef {
    let cell = new_ref(value);
    self.reverse_lookup.insert(Rc::as_ptr(&cell), key);
    let old = self.symbols.insert(key,cell.clone());
    if mutable {
      self.mutable_variables.insert(key,cell.clone());
    }
    cell.clone()
  }

}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for SymbolTable {
  pub fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let dict_brrw = self.dictionary.borrow();
    for (k,v) in &self.symbols {
      let name = dict_brrw.get(k).unwrap_or(&"??".to_string()).clone();
      let v_brrw = v.borrow();
      builder.push_record(vec![format!("\n{} : {}\n{}\n",name, v_brrw.kind(), v_brrw.pretty_print())])
    }
    if self.symbols.is_empty() {
      builder.push_record(vec!["".to_string()]);
    }
    let mut table = builder.build();
    let table_style = Style::empty()
    .top(' ')
    .left(' ')
    .right(' ')
    .bottom(' ')
    .vertical(' ')
    .horizontal('·')
    .intersection_bottom(' ')
    .corner_top_left(' ')
    .corner_top_right(' ')
    .corner_bottom_left(' ')
    .corner_bottom_right(' ');
    table.with(table_style);
    format!("{table}")
  }
}