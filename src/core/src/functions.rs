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
    for step in self.plan.descriptions() {
      plan_str = format!("{}  - {}\n",plan_str,step);
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
    self.plan.solve_all()?;
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

// Plan
// ----------------------------------------------------------------------------

struct ExecutablePlanState {
  functions: Vec<Box<dyn MechFunction>>,
  graph: PlanGraph,
}

pub struct Plan(Ref<ExecutablePlanState>);

impl Clone for Plan {
  fn clone(&self) -> Self { Plan(self.0.clone()) }
}

impl fmt::Debug for Plan {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for description in self.descriptions() {
      writeln!(f, "{}", description)?;
    }
    Ok(())
  }
}

impl Plan {
  pub fn new() -> Self {
    Self(Ref::new(ExecutablePlanState {
      functions: Vec::new(),
      graph: PlanGraph::new(),
    }))
  }

  pub fn add_node(&self, function: Box<dyn MechFunction>, spec: PlanNodeSpec) -> PlanNodeId {
    let mut state = self.0.borrow_mut();
    let expected_index = state.functions.len();
    let node = state.graph.add_node(spec);
    debug_assert_eq!(node.as_usize(), expected_index);
    state.functions.push(function);
    node
  }

  pub fn len(&self) -> usize { self.0.borrow().functions.len() }
  pub fn is_empty(&self) -> bool { self.0.borrow().functions.is_empty() }

  pub fn clear(&self) {
    let mut state = self.0.borrow_mut();
    state.functions.clear();
    state.graph.clear();
  }

  pub fn descriptions(&self) -> Vec<String> {
    self.0.borrow().functions.iter().map(|fxn| fxn.to_string()).collect()
  }

  pub fn node_description(&self, index: usize) -> Option<String> {
    self.0.borrow().functions.get(index).map(|fxn| fxn.to_string())
  }

  pub fn node_output(&self, index: usize) -> Option<Value> {
    self.0.borrow().functions.get(index).map(|fxn| fxn.out())
  }

  pub fn last_output(&self) -> Option<Value> {
    self.0.borrow().functions.last().map(|fxn| fxn.out())
  }

  pub fn solve_node(&self, node: PlanNodeId) -> MResult<Value> {
    self.solve_index(node.as_usize())
  }

  pub fn solve_index(&self, index: usize) -> MResult<Value> {
    let length = self.len();
    if index >= length {
      return Err(MechError::new(
        PlanNodeOutOfBounds {
          node: index,
          plan_length: length,
        },
        None,
      ));
    }
    let state = self.0.borrow();
    let function = &state.functions[index];
    function.solve_result()?;
    Ok(function.out())
  }

  pub fn solve_from(&self, invalidations: &[PlanInvalidation]) -> MResult<PlanSolveOutcome> {
    let schedule = {
      let state = self.0.borrow();
      state.graph.schedule_from(invalidations)?
    };

    let mut executed_nodes = 0usize;
    let mut value = Value::Empty;

    for node in &schedule.ordered_nodes {
      value = self.solve_node(*node)?;
      executed_nodes += 1;
    }

    Ok(PlanSolveOutcome {
      invalidated_cells: schedule.invalidated_cells,
      scheduled_nodes: schedule.scheduled_nodes,
      executed_nodes,
      value,
    })
  }

  pub fn solve_all(&self) -> MResult<PlanSolveOutcome> {
    let scheduled_nodes = self.len();
    let mut executed_nodes = 0usize;
    let mut value = Value::Empty;

    for index in 0..scheduled_nodes {
      value = self.solve_index(index)?;
      executed_nodes += 1;
    }

    Ok(PlanSolveOutcome {
      invalidated_cells: 0,
      scheduled_nodes,
      executed_nodes,
      value,
    })
  }

  #[cfg(feature = "compiler")]
  pub fn compile_into(&self, ctx: &mut CompileCtx) -> MResult<()> {
    let state = self.0.borrow();
    for step in &state.functions {
      step.compile(ctx)?;
    }
    Ok(())
  }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Plan {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    let descriptions = self.descriptions();

    if self.is_empty() {
      builder.push_record(vec!["".to_string()]);
    } else {
      let total = descriptions.len();
      let mut display_fxns: Vec<String> = Vec::new();

      // Determine which functions to display
      let indices: Vec<usize> = if total > 30 {
          (0..10).chain((total - 10)..total).collect()
      } else {
          (0..total).collect()
      };

      for &ix in &indices {
        let fxn_str = descriptions[ix].clone();
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

#[cfg(test)]
mod plan_execution_tests {
  use super::*;

  struct LoggedFunction {
    id: usize,
    log: Ref<Vec<usize>>,
    output: Value,
  }

  impl MechFunctionImpl for LoggedFunction {
    fn solve(&self) {
      self.log.borrow_mut().push(self.id);
    }

    fn out(&self) -> Value {
      self.output.clone()
    }

    fn to_string(&self) -> String {
      format!("logged-function-{}", self.id)
    }
  }

  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for LoggedFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
      unreachable!("test logged functions are never bytecode compiled")
    }
  }

  fn value(index: usize) -> Value {
    Value::Index(Ref::new(index))
  }

  fn cell(value: &Value) -> ValueCellId {
    value_cell_id(value).unwrap()
  }

  fn input(value: &Value) -> PlanInput {
    PlanInput {
      cell: cell(value),
      mode: PlanInputMode::Reactive,
    }
  }

  fn invalidation(value: &Value) -> PlanInvalidation {
    PlanInvalidation {
      cell: cell(value),
      kind: PlanInvalidationKind::Changed,
    }
  }

  fn function(id: usize, log: &Ref<Vec<usize>>, output: &Value) -> Box<dyn MechFunction> {
    Box::new(LoggedFunction {
      id,
      log: log.clone(),
      output: output.clone(),
    })
  }

  #[test]
  fn add_node_keeps_function_and_graph_indexes_aligned() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    let c = value(3);

    let node_0 = plan.add_node(function(0, &log, &a), PlanNodeSpec::default());
    let node_1 = plan.add_node(function(1, &log, &b), PlanNodeSpec::default());
    let node_2 = plan.add_node(function(2, &log, &c), PlanNodeSpec::default());

    assert_eq!(node_0.as_usize(), 0);
    assert_eq!(node_1.as_usize(), 1);
    assert_eq!(node_2.as_usize(), 2);
    assert_eq!(plan.len(), 3);
  }

  #[test]
  fn solve_from_executes_scheduled_nodes_in_dependency_order() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    let c = value(3);
    plan.add_node(
      function(0, &log, &c),
      PlanNodeSpec::explicit(vec![input(&b)], vec![cell(&c)]),
    );
    plan.add_node(
      function(1, &log, &b),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&b)]),
    );

    let outcome = plan.solve_from(&[invalidation(&a)]).unwrap();

    assert_eq!(*log.borrow(), vec![1, 0]);
    assert_eq!(outcome.invalidated_cells, 1);
    assert_eq!(outcome.scheduled_nodes, 2);
    assert_eq!(outcome.executed_nodes, 2);
  }

  #[test]
  fn solve_from_skips_unrelated_functions() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    let x = value(3);
    let y = value(4);
    plan.add_node(
      function(0, &log, &b),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&b)]),
    );
    plan.add_node(
      function(1, &log, &y),
      PlanNodeSpec::explicit(vec![input(&x)], vec![cell(&y)]),
    );

    plan.solve_from(&[invalidation(&a)]).unwrap();

    assert_eq!(*log.borrow(), vec![0]);
  }

  #[test]
  fn solve_from_executes_diamond_join_once() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    let c = value(3);
    let d = value(4);
    plan.add_node(
      function(0, &log, &b),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&b)]),
    );
    plan.add_node(
      function(1, &log, &c),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&c)]),
    );
    plan.add_node(
      function(2, &log, &d),
      PlanNodeSpec::explicit(vec![input(&b), input(&c)], vec![cell(&d)]),
    );

    plan.solve_from(&[invalidation(&a)]).unwrap();

    assert_eq!(*log.borrow(), vec![0, 1, 2]);
    assert_eq!(log.borrow().iter().filter(|id| **id == 2).count(), 1);
  }

  #[test]
  fn solve_from_rejects_cycle_before_execution() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    plan.add_node(
      function(0, &log, &b),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&b)]),
    );
    plan.add_node(
      function(1, &log, &a),
      PlanNodeSpec::explicit(vec![input(&b)], vec![cell(&a)]),
    );

    let error = plan.solve_from(&[invalidation(&a)]).unwrap_err();

    assert_eq!(error.kind_name(), "PlanDependencyCycle");
    assert!(log.borrow().is_empty());
  }

  #[test]
  fn solve_all_executes_every_function_in_registration_order() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    let c = value(3);
    plan.add_node(function(0, &log, &a), PlanNodeSpec::default());
    plan.add_node(
      function(1, &log, &b),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&b)]),
    );
    plan.add_node(function(2, &log, &c), PlanNodeSpec::default());

    let outcome = plan.solve_all().unwrap();

    assert_eq!(*log.borrow(), vec![0, 1, 2]);
    assert_eq!(outcome.invalidated_cells, 0);
    assert_eq!(outcome.scheduled_nodes, 3);
    assert_eq!(outcome.executed_nodes, 3);
  }

  #[test]
  fn default_spec_nodes_are_full_solve_only() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    plan.add_node(function(0, &log, &a), PlanNodeSpec::default());

    let incremental = plan.solve_from(&[]).unwrap();
    assert_eq!(incremental.executed_nodes, 0);
    assert!(log.borrow().is_empty());

    let full = plan.solve_all().unwrap();
    assert_eq!(full.executed_nodes, 1);
    assert_eq!(*log.borrow(), vec![0]);
  }

  #[test]
  fn solve_index_executes_only_selected_function() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    let c = value(3);
    plan.add_node(function(0, &log, &a), PlanNodeSpec::default());
    plan.add_node(function(1, &log, &b), PlanNodeSpec::default());
    plan.add_node(function(2, &log, &c), PlanNodeSpec::default());

    plan.solve_index(1).unwrap();

    assert_eq!(*log.borrow(), vec![1]);
  }

  #[test]
  fn solve_index_rejects_out_of_bounds() {
    let plan = Plan::new();
    let error = plan.solve_index(0).unwrap_err();

    assert_eq!(error.kind_name(), "PlanNodeOutOfBounds");
  }

  #[test]
  fn clear_removes_functions_and_dependency_indexes() {
    let plan = Plan::new();
    let log = Ref::new(Vec::new());
    let a = value(1);
    let b = value(2);
    plan.add_node(
      function(0, &log, &b),
      PlanNodeSpec::explicit(vec![input(&a)], vec![cell(&b)]),
    );
    plan.clear();

    assert_eq!(plan.len(), 0);
    let full = plan.solve_all().unwrap();
    let incremental = plan.solve_from(&[invalidation(&a)]).unwrap();
    assert_eq!(full.scheduled_nodes, 0);
    assert_eq!(full.executed_nodes, 0);
    assert_eq!(incremental.scheduled_nodes, 0);
    assert_eq!(incremental.executed_nodes, 0);
  }

  #[test]
  fn empty_plan_returns_empty_outcomes() {
    let plan = Plan::new();

    let full = plan.solve_all().unwrap();
    let incremental = plan.solve_from(&[]).unwrap();

    assert_eq!(full.scheduled_nodes, 0);
    assert_eq!(full.executed_nodes, 0);
    assert_eq!(full.value, Value::Empty);
    assert_eq!(incremental.scheduled_nodes, 0);
    assert_eq!(incremental.executed_nodes, 0);
    assert_eq!(incremental.value, Value::Empty);
  }
}
