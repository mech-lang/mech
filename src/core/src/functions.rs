use crate::types::*;
use crate::value::*;
use crate::nodes::*;
use crate::*;

use std::collections::HashMap;
#[cfg(feature = "functions")]
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

pub type FunctionsRef = Ref<Functions>;
pub type FunctionTable = HashMap<u64, fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>>;
pub type FunctionCompilerTable = HashMap<u64, &'static dyn NativeFunctionCompiler>;

#[derive(Clone,Debug)]
pub enum FunctionArgs {
  Nullary(Value),
  Unary(Value, Value),
  Binary(Value, Value, Value),
  Ternary(Value, Value, Value, Value),
  Quaternary(Value, Value, Value, Value, Value),
  Variadic(Value, Vec<Value>),
}

#[repr(C)]
#[derive(Clone)]
pub struct FunctionDescriptor {
  pub name: &'static str,
  pub ptr: fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>,
}

impl Debug for FunctionDescriptor {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{{ name: {:?}, ptr: {:?} }}", self.name, self.ptr)
  }
}

unsafe impl Sync for FunctionDescriptor {}

#[repr(C)]
pub struct FunctionCompilerDescriptor {
  pub name: &'static str,
  pub ptr: &'static dyn NativeFunctionCompiler,
}

impl Debug for FunctionCompilerDescriptor {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:?}", self.name)
  }
}

unsafe impl Sync for FunctionCompilerDescriptor {}

pub trait MechFunctionFactory {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>>;
}

pub trait MechFunctionImpl {
  fn solve(&self);
  fn out(&self) -> Value;
  fn to_string(&self) -> String;
}

#[cfg(feature = "compiler")]
pub trait MechFunctionCompilerDescriptor {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register>;
}

#[cfg(feature = "compiler")]
pub trait MechFunction: MechFunctionImpl + MechFunctionCompilerDescriptor {}
#[cfg(feature = "compiler")]
impl<T> MechFunction for T where T: MechFunctionImpl + MechFunctionCompilerDescriptor {}

#[cfg(not(feature = "compiler"))]
pub trait MechFunction: MechFunctionImpl {}
#[cfg(not(feature = "compiler"))]
impl<T> MechFunction for T where T: MechFunctionImpl {}

pub trait NativeFunctionCompilerDescriptor {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>>;
}

pub struct Functions {
  pub functions: FunctionTable,
  pub function_compilers: FunctionCompilerTable,
  pub dictionary: Ref<Dictionary>,
}

impl Functions {
  pub fn new() -> Self {
    Self {
      functions: HashMap::new(), 
      function_compilers: HashMap::new(), 
      dictionary: Ref::new(Dictionary::new()),
    }
  }

  pub fn insert_function(&mut self, fxn: FunctionDescriptor) {
    let id = hash_str(&fxn.name);
    self.functions.insert(id.clone(), fxn.ptr);
    self.dictionary.borrow_mut().insert(id, fxn.name.to_string());
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
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if cfg!(feature = "pretty_print") {
      #[cfg(feature = "pretty_print")]
      return fmt::Display::fmt(&self.pretty_print(), f);
      fmt::Display::fmt(&"".to_string(), f)
    } else {
      write!(f, "FunctionDefinition {{ id: {}, name: {}, input: {:?}, output: {:?}, symbols: {:?} }}", 
      self.id, self.name, self.input, self.output, self.symbols.borrow())
    }
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for FunctionDefinition {
  fn pretty_print(&self) -> String {
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
    format!("{table}")
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
      out: Ref::new(Value::Empty),
      symbols: Ref::new(SymbolTable::new()),
      plan: Plan::new(),
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

impl MechFunctionImpl for UserFunction {
  fn solve(&self) {
    self.fxn.solve();
  }
  fn out(&self) -> Value {
    Value::MutableReference(self.fxn.out.clone())
  }
  fn to_string(&self) -> String { format!("UserFxn::{:?}", self.fxn.name) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for UserFunction {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

// Plan
// ----------------------------------------------------------------------------

pub struct Plan(pub Ref<Vec<Box<dyn MechFunction>>>);

impl Clone for Plan {
  fn clone(&self) -> Self { Plan(self.0.clone()) }
}

impl fmt::Debug for Plan {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for p in &(*self.0.borrow()) {
      writeln!(f, "{}", p.to_string())?;
    }
    Ok(())
  }
}

impl Plan {
  pub fn new() -> Self { Plan(Ref::new(vec![])) }
  pub fn borrow(&self) -> std::cell::Ref<'_, Vec<Box<dyn MechFunction>>> { self.0.borrow() }
  pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Vec<Box<dyn MechFunction>>> { self.0.borrow_mut() }
  pub fn add_function(&self, func: Box<dyn MechFunction>) { self.0.borrow_mut().push(func); }
  pub fn get_functions(&self) -> std::cell::Ref<'_, Vec<Box<dyn MechFunction>>> { self.0.borrow() }
  pub fn len(&self) -> usize { self.0.borrow().len() }
  pub fn is_empty(&self) -> bool { self.0.borrow().is_empty() }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Plan {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();

    let mut row = vec![];
    let plan_brrw = self.0.borrow();
    if self.is_empty() {
      builder.push_record(vec!["".to_string()]);
    } else {
      for (ix, fxn) in plan_brrw.iter().enumerate() {
        let plan_str = format!("{}. {}\n", ix + 1, fxn.to_string());
        row.push(plan_str.clone());
        if row.len() == 4 {
          builder.push_record(row.clone());
          row.clear();
        }
      }
    }
    if row.is_empty() == false {
      builder.push_record(row.clone());
    }
    let mut table = builder.build();
    table.with(Style::modern_rounded())
        .with(Panel::header("📋 Plan"));
    format!("{table}")
  }
}

// Function Registry
// ----------------------------------------------------------------------------

// Function registry is a mapping from function IDs to the actual fucntion implementaionts

/*lazy_static! {
  pub static ref FUNCTION_REGISTRY: RefCell<HashMap<u64, Box<dyn NativeFunctionCompiler>>> = RefCell::new(HashMap::new());
}*/

pub struct FunctionRegistry {
  pub registry: RefCell<HashMap<u64, Box<dyn MechFunctionImpl>>>,
}