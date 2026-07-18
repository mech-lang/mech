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
use std::sync::Arc;

// Functions ------------------------------------------------------------------

pub type FunctionsRef = Ref<Functions>;
pub type FunctionTable = HashMap<u64, fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>>;
pub type FunctionCompilerTable = HashMap<u64, Arc<dyn NativeFunctionCompiler>>;
pub type UserFunctionTable = HashMap<u64, FunctionDefinition>;

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

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ModuleItemDescriptor {
  pub module: &'static str,
  pub item: &'static str,
}

unsafe impl Sync for ModuleItemDescriptor {}

pub trait MechFunctionFactory {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>>;
}

pub trait MechFunctionImpl {
  fn solve(&self);
  fn solve_result(&self) -> MResult<()> {
    self.solve();
    Ok(())
  }
  fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
    self.solve_result()?;
    Ok(ReactiveSolveStatus::Changed)
  }
  fn out(&self) -> Value;
  fn reactive_dependency_kinds(
    &self,
    _argument_count: usize,
  ) -> Option<Vec<ReactiveDependencyKind>> {
    None
  }
  fn reactive_output_values(&self) -> Vec<Value> {
    vec![self.out()]
  }
  fn reactive_node_kind(&self) -> ReactiveNodeKind {
    ReactiveNodeKind::Combinational
  }
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


pub struct StaticNativeFunctionCompiler {
  inner: &'static dyn NativeFunctionCompiler,
}

impl StaticNativeFunctionCompiler {
  pub fn new(inner: &'static dyn NativeFunctionCompiler) -> Self { Self { inner } }
}

impl NativeFunctionCompiler for StaticNativeFunctionCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    self.inner.compile(arguments)
  }
}

#[derive(Clone)]
pub struct Functions {
  pub functions: FunctionTable,
  pub function_compilers: FunctionCompilerTable,
  pub user_functions: UserFunctionTable,
  pub dictionary: Ref<Dictionary>,
}

impl Functions {
  pub fn new() -> Self {
    Self {
      functions: HashMap::new(),
      function_compilers: HashMap::new(),
      user_functions: HashMap::new(),
      dictionary: Ref::new(Dictionary::new()),
    }
  }

  pub fn insert_function(&mut self, fxn: FunctionDescriptor) {
    let id = hash_str(&fxn.name);
    self.functions.insert(id.clone(), fxn.ptr);
    self.dictionary.borrow_mut().insert(id, fxn.name.to_string());
  }

  pub fn insert_function_compiler(&mut self, name: impl Into<String>, compiler: Arc<dyn NativeFunctionCompiler>) {
    let name = name.into();
    let id = hash_str(&name);
    self.function_compilers.insert(id, compiler);
    self.dictionary.borrow_mut().insert(id, name);
  }

  #[cfg(feature = "pretty_print")]
  pub fn pretty_print(&self) -> String {
    let mut output = String::new();
    output.push_str("\nFunctions:\n");
    // print number of functions loaded:
    output.push_str(&format!("Total Functions: {}\n", self.functions.len()));
    output.push_str(&format!("User Functions: {}\n", self.user_functions.len()));
    //for (id, fxn_ptr) in &self.functions {
    //  let dict_brrw = self.dictionary.borrow();
    //  let name = dict_brrw.get(id).unwrap();
    //  output.push_str(&format!("  {}: {:?}\n", name, fxn_ptr));
    //}
    output
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

  pub fn solve_result(&self) -> MResult<ValRef> {
    let plan_brrw = self.plan.borrow();
    for step in plan_brrw.iter() {
      step.solve_result()?;
    }
    Ok(self.out.clone())
  }

  pub fn solve(&self) -> ValRef {
    let _ = self.solve_result();
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
    let _ = self.solve_result();
  }
  fn solve_result(&self) -> MResult<()> {
    self.fxn.solve_result()?;
    Ok(())
  }
  fn out(&self) -> Value {
    self.fxn.out.borrow().clone()
  }
  fn to_string(&self) -> String { format!("UserFxn::{:?}", self.fxn.name) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for UserFunction {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

// Reactive Plan
// ----------------------------------------------------------------------------

pub type ReactiveNodeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveDependencyKind {
  Reactive,
  Sampled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReactiveNodeKind {
  Combinational,
  Register,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveSolveStatus {
  Changed,
  Unchanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactiveDependency {
  pub cell: ReactiveCellId,
  pub kind: ReactiveDependencyKind,
}

pub struct ReactivePlanNode {
  pub id: ReactiveNodeId,
  pub plan_index: usize,
  pub function: Box<dyn MechFunction>,
  pub inputs: Vec<ReactiveDependency>,
  pub outputs: Vec<ReactiveCellId>,
  pub kind: ReactiveNodeKind,
}

pub struct ReactivePlan {
  pub nodes: Vec<ReactivePlanNode>,
  pub reactive_consumers: HashMap<ReactiveCellId, Vec<ReactiveNodeId>>,
  pub sampled_consumers: HashMap<ReactiveCellId, Vec<ReactiveNodeId>>,
}

impl ReactivePlan {
  pub fn new() -> Self {
    Self {
      nodes: Vec::new(),
      reactive_consumers: HashMap::new(),
      sampled_consumers: HashMap::new(),
    }
  }

  pub fn len(&self) -> usize {
    self.nodes.len()
  }

  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }

  pub fn clear(&mut self) {
    self.nodes.clear();
    self.reactive_consumers.clear();
    self.sampled_consumers.clear();
  }

  pub fn iter(&self) -> impl Iterator<Item = &Box<dyn MechFunction>> {
    self.nodes.iter().map(|node| &node.function)
  }

  pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn MechFunction>> {
    self.nodes.iter_mut().map(|node| &mut node.function)
  }

  pub fn last(&self) -> Option<&Box<dyn MechFunction>> {
    self.nodes.last().map(|node| &node.function)
  }

  pub fn append(&mut self, functions: &mut Vec<Box<dyn MechFunction>>) {
    for function in functions.drain(..) {
      self.push(function);
    }
  }

  pub fn push(&mut self, function: Box<dyn MechFunction>) -> ReactiveNodeId {
    let node_id = self.nodes.len();
    let mut outputs = Vec::<ReactiveCellId>::new();

    for output in function.reactive_output_values() {
      for cell in output.reactive_cell_ids() {
        if !outputs.contains(&cell) {
          outputs.push(cell);
        }
      }
    }

    let node = ReactivePlanNode {
      id: node_id,
      plan_index: node_id,
      inputs: Vec::new(),
      outputs,
      kind: function.reactive_node_kind(),
      function,
    };

    self.nodes.push(node);
    node_id
  }
}

impl core::ops::Index<usize> for ReactivePlan {
  type Output = Box<dyn MechFunction>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.nodes[index].function
  }
}

impl core::ops::IndexMut<usize> for ReactivePlan {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.nodes[index].function
  }
}

pub struct Plan(pub Ref<ReactivePlan>);

impl Clone for Plan {
  fn clone(&self) -> Self { Plan(self.0.clone()) }
}

impl fmt::Debug for Plan {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for p in self.0.borrow().iter() {
      writeln!(f, "{}", p.to_string())?;
    }
    Ok(())
  }
}

impl Plan {
  pub fn new() -> Self {
    Self(Ref::new(ReactivePlan::new()))
  }

  pub fn borrow(&self) -> std::cell::Ref<'_, ReactivePlan> {
    self.0.borrow()
  }

  pub fn borrow_mut(&self) -> std::cell::RefMut<'_, ReactivePlan> {
    self.0.borrow_mut()
  }

  pub fn add_function(&self, function: Box<dyn MechFunction>) -> ReactiveNodeId {
    self.0.borrow_mut().push(function)
  }

  pub fn get_functions(&self) -> std::cell::Ref<'_, ReactivePlan> {
    self.0.borrow()
  }

  pub fn len(&self) -> usize {
    self.0.borrow().len()
  }

  pub fn is_empty(&self) -> bool {
    self.0.borrow().is_empty()
  }
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
          t.extend_from_slice(&lines[..10]);
          t.push("…");
          t.extend_from_slice(&lines[lines.len()-10..]);
          t.join("\n")
        } else {
          lines.join("\n")
        };

        display_fxns.push(format!("{}. {}", ix + 1, truncated));
      }

      if total > 30 {
        display_fxns.insert(10, "…".to_string());
      }

      let mut row: Vec<String> = Vec::new();
      for plan_str in display_fxns {
        row.push(plan_str);
        if row.len() == 4 {
          builder.push_record(row.clone());
          row.clear();
        }
      }
      if !row.is_empty() {
        while row.len() < 4 {
          row.push("".to_string());
        }
        builder.push_record(row);
      }
    }

    let mut table = builder.build();
    table.with(Style::modern_rounded())
          .with(Panel::header("📋 Plan"));

    format!("{table}")
  }
}

#[cfg(test)]
mod reactive_plan_tests {
  use super::*;

  struct TestFunction {
    name: &'static str,
    output: Value,
  }

  impl TestFunction {
    fn new(name: &'static str) -> Self {
      Self { name, output: Value::Empty }
    }

    #[cfg(feature = "f64")]
    fn with_output(name: &'static str, output: Value) -> Self {
      Self { name, output }
    }
  }

  impl MechFunctionImpl for TestFunction {
    fn solve(&self) {}

    fn out(&self) -> Value {
      self.output.clone()
    }

    fn to_string(&self) -> String {
      self.name.to_string()
    }
  }

  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for TestFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
      Ok(0)
    }
  }

  #[test]
  fn reactive_plan_push_creates_one_node() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].id, 0);
    assert_eq!(plan.nodes[0].plan_index, 0);
  }

  #[test]
  fn reactive_plan_preserves_insertion_order() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.push(Box::new(TestFunction::new("second")));

    let names = plan.iter().map(|function| function.to_string()).collect::<Vec<_>>();
    assert_eq!(names, vec!["first".to_string(), "second".to_string()]);
    assert_eq!(plan[0].to_string(), "first");
    assert_eq!(plan[1].to_string(), "second");
  }

  #[test]
  fn reactive_plan_node_is_only_function_owner() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.push(Box::new(TestFunction::new("second")));

    assert_eq!(plan.len(), plan.nodes.len());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn reactive_plan_records_output_cells() {
    let output = Ref::new(42.0);
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::with_output("output", Value::F64(output.clone()))));

    assert!(plan.nodes[0].outputs.contains(&ReactiveCellId::new(output.id())));
  }

  #[test]
  fn reactive_plan_clone_shares_storage() {
    let plan = Plan::new();
    let clone = plan.clone();

    plan.add_function(Box::new(TestFunction::new("shared")));

    assert_eq!(plan.len(), 1);
    assert_eq!(clone.len(), 1);
  }

  #[test]
  fn reactive_plan_clear_removes_nodes_and_indexes() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.reactive_consumers.insert(ReactiveCellId::new(1), vec![0]);
    plan.sampled_consumers.insert(ReactiveCellId::new(2), vec![0]);

    plan.clear();

    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
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
impl MechErrorKind for UnhandledFunctionArgumentKind1 {
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
impl MechErrorKind for UnhandledFunctionArgumentKind2 {
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
impl MechErrorKind for UnhandledFunctionArgumentKind3 {
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
impl MechErrorKind for UnhandledFunctionArgumentKind4 {
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
impl MechErrorKind for UnhandledFunctionArgumentKindVarg {
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
impl MechErrorKind for UnhandledFunctionArgumentIxes {
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
impl MechErrorKind for UnhandledFunctionArgumentIxesMono {
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
impl MechErrorKind for IncorrectNumberOfArguments {
  fn name(&self) -> &str {
    "IncorrectNumberOfArguments"
  }

  fn message(&self) -> String {
    format!("Expected {} arguments, but found {}", self.expected, self.found)
  }
}
