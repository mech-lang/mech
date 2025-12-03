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

impl FunctionArgs {
  pub fn len(&self) -> usize {
    match self {
      FunctionArgs::Nullary(_) => 0,
      FunctionArgs::Unary(_, _) => 1,
      FunctionArgs::Binary(_, _, _) => 2,
      FunctionArgs::Ternary(_, _, _, _) => 3,
      FunctionArgs::Quaternary(_, _, _, _, _) => 4,
      FunctionArgs::Variadic(_, args) => args.len(),
    }
  }
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
pub trait MechFunctionCompiler {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register>;
}

#[cfg(feature = "compiler")]
pub trait MechFunction: MechFunctionImpl + MechFunctionCompiler {}
#[cfg(feature = "compiler")]
impl<T> MechFunction for T where T: MechFunctionImpl + MechFunctionCompiler {}

#[cfg(not(feature = "compiler"))]
pub trait MechFunction: MechFunctionImpl {}
#[cfg(not(feature = "compiler"))]
impl<T> MechFunction for T where T: MechFunctionImpl {}

pub trait NativeFunctionCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>>;
}

#[derive(Clone)]
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
    let plan_brrw = self.0.borrow();

    if self.is_empty() {
      builder.push_record(vec!["".to_string()]);
    } else {
      let total = plan_brrw.len();
      let mut display_fxns: Vec<String> = Vec::new();

      // Determine which functions to display
      let indices: Vec<usize> = if total > 30 {
          (0..10).chain((total - 10)..total).collect()
      } else {
          (0..total).collect()
      };

      for &ix in &indices {
        let fxn_str = plan_brrw[ix].to_string();
        let lines: Vec<&str> = fxn_str.lines().collect();

        let truncated = if lines.len() > 20 {
          let mut t = Vec::new();
          t.extend_from_slice(&lines[..10]);           // first 10
          t.push("…");                                 // ellipsis
          t.extend_from_slice(&lines[lines.len()-10..]); // last 10
          t.join("\n")
        } else {
          lines.join("\n")
        };

        display_fxns.push(format!("{}. {}", ix + 1, truncated));
      }

      // Insert ellipsis for skipped functions
      if total > 30 {
        display_fxns.insert(10, "…".to_string());
      }

      // Push rows in chunks of 4 (like before)
      let mut row: Vec<String> = Vec::new();
      for plan_str in display_fxns {
        row.push(plan_str);
        if row.len() == 4 {
          builder.push_record(row.clone());
          row.clear();
        }
      }
      if !row.is_empty() {
        builder.push_record(row);
      }
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

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind1 {
  pub arg: ValueKind,
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentKind1 {
  fn name(&self) -> &str { "UnhandledFunctionArgumentKind1" }
  fn message(&self) -> String {
    format!("Unhandled function argument kind for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind2 {
  pub arg: (ValueKind, ValueKind),
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentKind2 {
  fn name(&self) -> &str { "UnhandledFunctionArgumentKind2" }
  fn message(&self) -> String {
    format!("Unhandled function argument kinds for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind3 {
  pub arg: (ValueKind, ValueKind, ValueKind),
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentKind3 {
  fn name(&self) -> &str { "UnhandledFunctionArgumentKind3" }
  fn message(&self) -> String {
    format!("Unhandled function argument kinds for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind4 {
  pub arg: (ValueKind, ValueKind, ValueKind, ValueKind),
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentKind4 {
  fn name(&self) -> &str { "UnhandledFunctionArgumentKind4" }
  fn message(&self) -> String {
    format!("Unhandled function argument kinds for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKindVarg {
  pub arg: Vec<ValueKind>,
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentKindVarg {
  fn name(&self) -> &str { "UnhandledFunctionArgumentKindVarg" }
  fn message(&self) -> String {
    format!("Unhandled function argument kinds for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentIxes {
  pub arg: (ValueKind, Vec<ValueKind>, ValueKind),
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentIxes {
  fn name(&self) -> &str { "UnhandledFunctionArgumentIxes" }
  fn message(&self) -> String {
    format!("Unhandled function argument kinds for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentIxesMono {
  pub arg: (ValueKind, Vec<ValueKind>),
  pub fxn_name: String,
}
impl MechErrorKind2 for UnhandledFunctionArgumentIxesMono {
  fn name(&self) -> &str { "UnhandledFunctionArgumentIxesMono" }
  fn message(&self) -> String {
    format!("Unhandled function argument kinds for function '{}': arg = {:?}", self.fxn_name, self.arg)
  }
}

#[derive(Debug, Clone)]
pub struct IncorrectNumberOfArguments {
  pub expected: usize,
  pub found: usize,
}
impl MechErrorKind2 for IncorrectNumberOfArguments {
  fn name(&self) -> &str {
    "IncorrectNumberOfArguments"
  }

  fn message(&self) -> String {
    format!("Expected {} arguments, but found {}", self.expected, self.found)
  }
}
