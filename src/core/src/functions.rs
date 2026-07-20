use crate::types::*;
use crate::value::*;
use crate::nodes::*;
use crate::*;

use std::collections::{BTreeSet, HashMap, HashSet};
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

  pub fn input_values(&self) -> Vec<Value> {
    match self {
      FunctionArgs::Nullary(_) =>
        Vec::new(),

      FunctionArgs::Unary(_, a) =>
        vec![a.clone()],

      FunctionArgs::Binary(_, a, b) =>
        vec![
          a.clone(),
          b.clone(),
        ],

      FunctionArgs::Ternary(_, a, b, c) =>
        vec![
          a.clone(),
          b.clone(),
          c.clone(),
        ],

      FunctionArgs::Quaternary(
        _,
        a,
        b,
        c,
        d,
      ) =>
        vec![
          a.clone(),
          b.clone(),
          c.clone(),
          d.clone(),
        ],

      FunctionArgs::Variadic(_, arguments) =>
        arguments.clone(),
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
  fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
    Err(MechError::new(
      ReactiveRegisterStagingUnsupportedError { function: self.to_string() },
      None,
    ).with_compiler_loc())
  }
  fn out(&self) -> Value;
  fn reactive_dependency_kinds(
    &self,
    _argument_count: usize,
  ) -> Option<Vec<ReactiveDependencyKind>> {
    None
  }
  fn reactive_dependency_scopes(
    &self,
    _argument_count: usize,
  ) -> Option<Vec<ReactiveDependencyScope>> {
    None
  }
  fn reactive_output_values(&self) -> Vec<Value> {
    vec![self.out()]
  }
  fn reactive_output_cell_ids(&self) -> Vec<ReactiveCellId> {
    let mut cells = Vec::new();

    for output in self.reactive_output_values() {
      for cell in output.reactive_root_cell_ids() {
        if !cells.contains(&cell) {
          cells.push(cell);
        }
      }
    }

    cells
  }
  fn reactive_node_kind(&self) -> ReactiveNodeKind {
    ReactiveNodeKind::Combinational
  }
  fn to_string(&self) -> String;
}

/// An already validated register write. Implementations must not fail or run
/// arbitrary reactive work when they are committed.
pub trait ReactiveRegisterCommit {
  fn output_cells(&self) -> &[ReactiveCellId];
  fn commit(self: Box<Self>);
}

pub struct ReactiveRegisterWrite<T> {
  sink: Ref<T>,
  next: T,
  output_cells: Vec<ReactiveCellId>,
}

impl<T> ReactiveRegisterWrite<T> {
  pub fn new(sink: Ref<T>, next: T, output_cells: Vec<ReactiveCellId>) -> Self {
    Self { sink, next, output_cells }
  }
}

impl<T: 'static> ReactiveRegisterCommit for ReactiveRegisterWrite<T> {
  fn output_cells(&self) -> &[ReactiveCellId] { self.output_cells.as_slice() }
  fn commit(self: Box<Self>) {
    let ReactiveRegisterWrite { sink, next, output_cells: _ } = *self;
    *sink.borrow_mut() = next;
  }
}

pub struct ReactiveRegisterNoopCommit { output_cells: Vec<ReactiveCellId> }
impl ReactiveRegisterNoopCommit {
  pub fn new(output_cells: Vec<ReactiveCellId>) -> Self { Self { output_cells } }
}
impl ReactiveRegisterCommit for ReactiveRegisterNoopCommit {
  fn output_cells(&self) -> &[ReactiveCellId] { self.output_cells.as_slice() }
  fn commit(self: Box<Self>) {}
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveDependencyScope {
  Recursive,
  Root,
  None,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactivePlanSolveOutcome {
  pub executed_nodes: Vec<ReactiveNodeId>,
  pub changed_nodes: Vec<ReactiveNodeId>,
  pub unchanged_nodes: Vec<ReactiveNodeId>,
  pub pending_register_nodes: Vec<ReactiveNodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveRegisterCommitOutcome {
  pub staged_nodes: Vec<ReactiveNodeId>,
  pub committed_nodes: Vec<ReactiveNodeId>,
  pub dirty_cells: Vec<ReactiveCellId>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveTurnState {
  pub pending_register_nodes: Vec<ReactiveNodeId>,
}

impl ReactiveTurnState {
  pub fn has_pending_registers(&self) -> bool {
    !self.pending_register_nodes.is_empty()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactiveTurnOutcome {
  pub before_commit: ReactivePlanSolveOutcome,
  pub register_commit: ReactiveRegisterCommitOutcome,
  pub after_commit: ReactivePlanSolveOutcome,
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

#[derive(Debug, Clone)]
pub struct ReactiveDependencyArityMismatchError {
  pub function: String,
  pub expected: usize,
  pub found: usize,
}

impl MechErrorKind for ReactiveDependencyArityMismatchError {
  fn name(&self) -> &str {
    "ReactiveDependencyArityMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Reactive dependency arity mismatch for function '{}': expected {} dependency kinds, found {}.",
      self.function,
      self.expected,
      self.found,
    )
  }
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyScopeArityMismatchError {
  pub function: String,
  pub expected: usize,
  pub found: usize,
}

impl MechErrorKind for ReactiveDependencyScopeArityMismatchError {
  fn name(&self) -> &str {
    "ReactiveDependencyScopeArityMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Reactive dependency scope arity mismatch for function '{}': expected argument count {}, provided scope count {}.",
      self.function,
      self.expected,
      self.found,
    )
  }
}

#[derive(Debug, Clone)]
pub struct ReactiveDependencyKindConflictError {
  pub function: String,
  pub cell: ReactiveCellId,
}

#[derive(Debug, Clone)]
pub struct ReactiveRegisterStagingUnsupportedError { pub function: String }
impl MechErrorKind for ReactiveRegisterStagingUnsupportedError {
  fn name(&self) -> &str { "ReactiveRegisterStagingUnsupported" }
  fn message(&self) -> String { format!("Reactive register staging is not implemented for function '{}'.", self.function) }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterNodeNotFoundError { pub node_id: ReactiveNodeId }
impl MechErrorKind for ReactiveRegisterNodeNotFoundError {
  fn name(&self) -> &str { "ReactiveRegisterNodeNotFound" }
  fn message(&self) -> String { format!("Reactive register node {} does not exist.", self.node_id) }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterNodeKindError { pub node_id: ReactiveNodeId, pub actual: ReactiveNodeKind }
impl MechErrorKind for ReactiveRegisterNodeKindError {
  fn name(&self) -> &str { "ReactiveRegisterNodeKind" }
  fn message(&self) -> String { format!("Reactive node {} must be a register for commit, but its kind is {:?}.", self.node_id, self.actual) }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterOutputConflictError { pub cell: ReactiveCellId, pub first_node: ReactiveNodeId, pub second_node: ReactiveNodeId }
impl MechErrorKind for ReactiveRegisterOutputConflictError {
  fn name(&self) -> &str { "ReactiveRegisterOutputConflict" }
  fn message(&self) -> String { format!("Reactive register nodes {} and {} both write output cell {:?}.", self.first_node, self.second_node, self.cell) }
}
#[derive(Debug, Clone)]
pub struct ReactiveRegisterStagedOutputMismatchError { pub node_id: ReactiveNodeId, pub expected: Vec<ReactiveCellId>, pub found: Vec<ReactiveCellId> }
impl MechErrorKind for ReactiveRegisterStagedOutputMismatchError {
  fn name(&self) -> &str { "ReactiveRegisterStagedOutputMismatch" }
  fn message(&self) -> String { format!("Reactive register node {} staged outputs {:?}, but its registered outputs are {:?}.", self.node_id, self.found, self.expected) }
}

impl MechErrorKind for ReactiveDependencyKindConflictError {
  fn name(&self) -> &str {
    "ReactiveDependencyKindConflict"
  }

  fn message(&self) -> String {
    format!(
      "Reactive dependency kind conflict for function '{}': one node classified cell {:?} as both reactive and sampled.",
      self.function,
      self.cell,
    )
  }
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
    let outputs = function.reactive_output_cell_ids();
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

  pub fn register(
    &mut self,
    function: Box<dyn MechFunction>,
    arguments: &[Value],
  ) -> MResult<ReactiveNodeId> {
    let node_id = self.nodes.len();
    let plan_index = node_id;
    let function_description = function.to_string();

    let dependency_kinds = match function.reactive_dependency_kinds(arguments.len()) {
      Some(kinds) => {
        if kinds.len() != arguments.len() {
          return Err(MechError::new(
            ReactiveDependencyArityMismatchError {
              function: function_description,
              expected: arguments.len(),
              found: kinds.len(),
            },
            None,
          ));
        }
        kinds
      }
      None => vec![
        ReactiveDependencyKind::Reactive;
        arguments.len()
      ],
    };

    let dependency_scopes = match function.reactive_dependency_scopes(arguments.len()) {
      Some(scopes) => {
        if scopes.len() != arguments.len() {
          return Err(MechError::new(
            ReactiveDependencyScopeArityMismatchError {
              function: function_description,
              expected: arguments.len(),
              found: scopes.len(),
            },
            None,
          ));
        }
        scopes
      }
      None => vec![ReactiveDependencyScope::Recursive; arguments.len()],
    };

    let node_kind = function.reactive_node_kind();
    let outputs = function.reactive_output_cell_ids();
    let mut inputs = Vec::<ReactiveDependency>::new();

    if node_kind == ReactiveNodeKind::Register {
      for cell in &outputs {
        inputs.push(ReactiveDependency {
          cell: *cell,
          kind: ReactiveDependencyKind::Sampled,
        });
      }
    }

    for ((argument, kind), scope) in arguments
      .iter()
      .zip(dependency_kinds.iter())
      .zip(dependency_scopes.iter())
    {
      let cells = match scope {
        ReactiveDependencyScope::Recursive => argument.reactive_cell_ids(),
        ReactiveDependencyScope::Root => argument.reactive_root_cell_ids(),
        ReactiveDependencyScope::None => Vec::new(),
      };

      for cell in cells {
        match inputs.iter().find(|dependency| dependency.cell == cell) {
          Some(dependency) if dependency.kind == *kind => {}
          Some(dependency)
            if node_kind == ReactiveNodeKind::Register
              && outputs.contains(&cell)
              && (dependency.kind == ReactiveDependencyKind::Sampled
                || *kind == ReactiveDependencyKind::Sampled) => {}
          Some(_) => {
            return Err(MechError::new(
              ReactiveDependencyKindConflictError {
                function: function_description,
                cell,
              },
              None,
            ));
          }
          None => inputs.push(ReactiveDependency {
            cell,
            kind: *kind,
          }),
        }
      }
    }

    let node = ReactivePlanNode {
      id: node_id,
      plan_index,
      inputs,
      outputs,
      kind: node_kind,
      function,
    };

    self.nodes.push(node);

    for dependency in &self.nodes[node_id].inputs {
      let consumers = match dependency.kind {
        ReactiveDependencyKind::Reactive =>
          self.reactive_consumers
            .entry(dependency.cell)
            .or_default(),
        ReactiveDependencyKind::Sampled =>
          self.sampled_consumers
            .entry(dependency.cell)
            .or_default(),
      };

      if !consumers.contains(&node_id) {
        consumers.push(node_id);
      }
    }

    Ok(node_id)
  }

  pub fn node(&self, node_id: ReactiveNodeId) -> Option<&ReactivePlanNode> {
    self.nodes.get(node_id)
  }

  pub fn reactive_consumers_for(&self, cell: ReactiveCellId) -> &[ReactiveNodeId] {
    self.reactive_consumers
      .get(&cell)
      .map(Vec::as_slice)
      .unwrap_or(&[])
  }

  pub fn sampled_consumers_for(&self, cell: ReactiveCellId) -> &[ReactiveNodeId] {
    self.sampled_consumers
      .get(&cell)
      .map(Vec::as_slice)
      .unwrap_or(&[])
  }

  pub fn solve_dirty_cells(
    &mut self,
    dirty_cells: &[ReactiveCellId],
  ) -> MResult<ReactivePlanSolveOutcome> {
    let mut outcome = ReactivePlanSolveOutcome::default();
    self.solve_dirty_cells_into(dirty_cells, &mut outcome)?;
    Ok(outcome)
  }

  fn solve_dirty_cells_into(
    &mut self,
    dirty_cells: &[ReactiveCellId],
    outcome: &mut ReactivePlanSolveOutcome,
  ) -> MResult<()> {
    let dirty_cells = dirty_cells.iter().copied().collect::<HashSet<_>>();
    let mut work = BTreeSet::new();
    let mut processed = BTreeSet::new();

    for cell in dirty_cells.iter().copied() {
      for node_id in self.reactive_consumers_for(cell) {
        let node = &self.nodes[*node_id];
        work.insert((node.plan_index, node.id));
      }
    }

    while let Some((_, node_id)) = work.pop_first() {
      if !processed.insert(node_id) {
        continue;
      }

      let node = &self.nodes[node_id];
      if node.kind == ReactiveNodeKind::Register {
        outcome.pending_register_nodes.push(node.id);
        continue;
      }

      let status = node.function.solve_reactive()?;
      outcome.executed_nodes.push(node.id);
      match status {
        ReactiveSolveStatus::Changed => {
          outcome.changed_nodes.push(node.id);
          let outputs = node.outputs.clone();
          for cell in outputs {
            for consumer_id in self.reactive_consumers_for(cell) {
              let consumer = &self.nodes[*consumer_id];
              work.insert((consumer.plan_index, consumer.id));
            }
          }
        }
        ReactiveSolveStatus::Unchanged => outcome.unchanged_nodes.push(node.id),
      }
    }

    Ok(())
  }

  pub fn commit_pending_registers(&mut self, pending_nodes: &[ReactiveNodeId]) -> MResult<ReactiveRegisterCommitOutcome> {
    let mut unique = HashSet::new();
    let mut ordered = BTreeSet::new();
    for node_id in pending_nodes.iter().copied() {
      if !unique.insert(node_id) { continue; }
      let node = self.nodes.get(node_id).ok_or_else(|| MechError::new(
        ReactiveRegisterNodeNotFoundError { node_id }, None))?;
      if node.kind != ReactiveNodeKind::Register {
        return Err(MechError::new(ReactiveRegisterNodeKindError { node_id, actual: node.kind }, None));
      }
      ordered.insert((node.plan_index, node.id));
    }

    let mut owners = HashMap::new();
    for (_, node_id) in &ordered {
      let node = &self.nodes[*node_id];
      for cell in &node.outputs {
        if let Some(first_node) = owners.insert(*cell, node.id) {
          return Err(MechError::new(ReactiveRegisterOutputConflictError {
            cell: *cell, first_node, second_node: node.id,
          }, None));
        }
      }
    }

    let mut staged: Vec<(ReactiveNodeId, Box<dyn ReactiveRegisterCommit>)> = Vec::new();
    for (_, node_id) in &ordered {
      let node = &self.nodes[*node_id];
      let commit = node.function.stage_register()?;
      let found = commit.output_cells().to_vec();
      if found != node.outputs {
        return Err(MechError::new(ReactiveRegisterStagedOutputMismatchError {
          node_id: node.id, expected: node.outputs.clone(), found,
        }, None));
      }
      staged.push((node.id, commit));
    }

    let staged_nodes = staged.iter().map(|(id, _)| *id).collect();
    let mut outcome = ReactiveRegisterCommitOutcome { staged_nodes, ..Default::default() };
    for (node_id, commit) in staged {
      let outputs = commit.output_cells().to_vec();
      commit.commit();
      outcome.committed_nodes.push(node_id);
      for cell in outputs { if !outcome.dirty_cells.contains(&cell) { outcome.dirty_cells.push(cell); } }
    }
    Ok(outcome)
  }
  /// Advances one synchronous reactive turn using this existing plan.
  ///
  /// Pre-commit and staging failures occur before register mutation. Post-commit
  /// propagation failures occur after the atomic register batch has been committed
  /// and are therefore not rolled back.
  pub fn advance_reactive_turn(
    &mut self,
    state: &mut ReactiveTurnState,
    dirty_cells: &[ReactiveCellId],
  ) -> MResult<ReactiveTurnOutcome> {
    let before_commit = self.solve_dirty_cells(dirty_cells)?;
    let mut pending_register_nodes = std::mem::take(&mut state.pending_register_nodes);
    pending_register_nodes.extend(before_commit.pending_register_nodes.iter().copied());
    let register_commit = match self.commit_pending_registers(&pending_register_nodes) {
      Ok(outcome) => outcome,
      Err(error) => {
        state.pending_register_nodes = pending_register_nodes;
        return Err(error);
      }
    };
    state.pending_register_nodes.clear();
    let mut after_commit = ReactivePlanSolveOutcome::default();
    if let Err(error) = self.solve_dirty_cells_into(&register_commit.dirty_cells, &mut after_commit) {
      state.pending_register_nodes = after_commit.pending_register_nodes;
      return Err(error);
    }
    state.pending_register_nodes = after_commit.pending_register_nodes.clone();
    Ok(ReactiveTurnOutcome {
      before_commit,
      register_commit,
      after_commit,
    })
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

  pub fn register_function(
    &self,
    function: Box<dyn MechFunction>,
    arguments: &[Value],
  ) -> MResult<ReactiveNodeId> {
    self.0
      .borrow_mut()
      .register(
        function,
        arguments,
      )
  }

  pub fn solve_dirty_cells(
    &self,
    dirty_cells: &[ReactiveCellId],
  ) -> MResult<ReactivePlanSolveOutcome> {
    self.0.borrow_mut().solve_dirty_cells(dirty_cells)
  }

  pub fn commit_pending_registers(&self, pending_nodes: &[ReactiveNodeId]) -> MResult<ReactiveRegisterCommitOutcome> {
    self.0.borrow_mut().commit_pending_registers(pending_nodes)
  }
  pub fn advance_reactive_turn(
    &self,
    state: &mut ReactiveTurnState,
    dirty_cells: &[ReactiveCellId],
  ) -> MResult<ReactiveTurnOutcome> {
    self.0
      .borrow_mut()
      .advance_reactive_turn(state, dirty_cells)
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
    dependency_kinds: Option<Vec<ReactiveDependencyKind>>,
    dependency_scopes: Option<Vec<ReactiveDependencyScope>>,
    node_kind: ReactiveNodeKind,
  }

  impl TestFunction {
    fn new(name: &'static str) -> Self {
      Self {
        name,
        output: Value::Empty,
        dependency_kinds: None,
        dependency_scopes: None,
        node_kind: ReactiveNodeKind::Combinational,
      }
    }

    #[cfg(feature = "f64")]
    fn with_output(name: &'static str, output: Value) -> Self {
      Self {
        name,
        output,
        dependency_kinds: None,
        dependency_scopes: None,
        node_kind: ReactiveNodeKind::Combinational,
      }
    }

    fn with_dependency_kinds(
      mut self,
      dependency_kinds: Option<Vec<ReactiveDependencyKind>>,
    ) -> Self {
      self.dependency_kinds = dependency_kinds;
      self
    }

    fn with_dependency_scopes(
      mut self,
      scopes: Option<Vec<ReactiveDependencyScope>>,
    ) -> Self {
      self.dependency_scopes = scopes;
      self
    }

    fn with_node_kind(mut self, node_kind: ReactiveNodeKind) -> Self {
      self.node_kind = node_kind;
      self
    }
  }

  impl MechFunctionImpl for TestFunction {
    fn solve(&self) {}

    fn out(&self) -> Value {
      self.output.clone()
    }

    fn reactive_dependency_kinds(
      &self,
      _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyKind>> {
      self.dependency_kinds.clone()
    }

    fn reactive_dependency_scopes(
      &self,
      _argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
      self.dependency_scopes.clone()
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
      self.node_kind
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

  #[cfg(all(feature = "set", feature = "f64"))]
  fn set_output() -> (Value, ReactiveCellId, ReactiveCellId, ReactiveCellId) {
    let first = Ref::new(1.0);
    let second = Ref::new(2.0);
    let mut members = indexmap::IndexSet::new();
    members.insert(Value::F64(first.clone()));
    members.insert(Value::F64(second.clone()));
    let set = Ref::new(MechSet { kind: ValueKind::F64, num_elements: 2, set: members });

    (
      Value::Set(set.clone()),
      ReactiveCellId::new(set.id()),
      ReactiveCellId::new(first.id()),
      ReactiveCellId::new(second.id()),
    )
  }

  #[cfg(all(feature = "set", feature = "f64"))]
  #[test]
  fn reactive_plan_push_records_root_output_cells() {
    let (output, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::with_output("set", output)));

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].outputs, vec![outer]);
    assert!(!plan.nodes[0].outputs.contains(&first));
    assert!(!plan.nodes[0].outputs.contains(&second));
  }

  #[cfg(all(feature = "set", feature = "f64"))]
  #[test]
  fn reactive_plan_register_records_root_output_cells() {
    let (output, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();
    let node_id = plan.register(Box::new(TestFunction::with_output("set", output)), &[]).unwrap();
    let node = plan.node(node_id).unwrap();

    assert_eq!(node.outputs, vec![outer]);
    assert!(!node.outputs.contains(&first));
    assert!(!node.outputs.contains(&second));
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

  #[cfg(feature = "f64")]
  fn scalar(value: f64) -> (Value, ReactiveCellId) {
    let reference = Ref::new(value);
    let cell = ReactiveCellId::new(reference.id());
    (Value::F64(reference), cell)
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_node_indexes_output_as_sampled_state() {
    let (output, output_cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan.register(Box::new(TestFunction::with_output("register", output).with_node_kind(ReactiveNodeKind::Register)), &[]).unwrap();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![output_cell]);
    assert_eq!(node.inputs, vec![ReactiveDependency { cell: output_cell, kind: ReactiveDependencyKind::Sampled }]);
    assert_eq!(plan.sampled_consumers_for(output_cell), &[node_id]);
    assert!(plan.reactive_consumers_for(output_cell).is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_node_keeps_source_dependency_reactive() {
    let (output, output_cell) = scalar(1.0);
    let (source, source_cell) = scalar(2.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan.register(Box::new(TestFunction::with_output("register", output).with_node_kind(ReactiveNodeKind::Register)), &[source]).unwrap();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.inputs, vec![
      ReactiveDependency { cell: output_cell, kind: ReactiveDependencyKind::Sampled },
      ReactiveDependency { cell: source_cell, kind: ReactiveDependencyKind::Reactive },
    ]);
    assert_eq!(plan.sampled_consumers_for(output_cell), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(source_cell), &[node_id]);
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_node_coalesces_output_operand_alias_to_sampled() {
    let (output, output_cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan.register(Box::new(TestFunction::with_output("register", output.clone()).with_node_kind(ReactiveNodeKind::Register)), &[output]).unwrap();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.inputs, vec![ReactiveDependency { cell: output_cell, kind: ReactiveDependencyKind::Sampled }]);
    assert!(plan.reactive_consumers_for(output_cell).is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_node_has_no_reactive_self_consumer() {
    let (output, output_cell) = scalar(1.0);
    let (source, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan.register(Box::new(TestFunction::with_output("register", output).with_node_kind(ReactiveNodeKind::Register)), &[source]).unwrap();
    assert!(!plan.reactive_consumers_for(output_cell).contains(&node_id));
    assert!(plan.sampled_consumers_for(output_cell).contains(&node_id));
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_node_preserves_dependency_order() {
    let (output, output_cell) = scalar(1.0);
    let (first, first_cell) = scalar(2.0);
    let (second, second_cell) = scalar(3.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan.register(Box::new(TestFunction::with_output("register", output).with_node_kind(ReactiveNodeKind::Register)), &[first, second]).unwrap();
    assert_eq!(plan.node(node_id).unwrap().inputs, vec![
      ReactiveDependency { cell: output_cell, kind: ReactiveDependencyKind::Sampled },
      ReactiveDependency { cell: first_cell, kind: ReactiveDependencyKind::Reactive },
      ReactiveDependency { cell: second_cell, kind: ReactiveDependencyKind::Reactive },
    ]);
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_node_validation_failure_does_not_mutate_plan() {
    let (output, _) = scalar(1.0);
    let (source, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();
    assert!(plan.register(Box::new(TestFunction::with_output("register", output).with_node_kind(ReactiveNodeKind::Register).with_dependency_kinds(Some(vec![]))), &[source]).is_err());
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn function_args_returns_only_inputs() {
    let (out, _) = scalar(0.0);
    let (a, _) = scalar(1.0);
    let (b, _) = scalar(2.0);
    let (c, _) = scalar(3.0);
    let (d, _) = scalar(4.0);

    assert_eq!(FunctionArgs::Nullary(out.clone()).input_values(), Vec::<Value>::new());
    assert_eq!(FunctionArgs::Unary(out.clone(), a.clone()).input_values(), vec![a.clone()]);
    assert_eq!(
      FunctionArgs::Binary(out.clone(), a.clone(), b.clone()).input_values(),
      vec![a.clone(), b.clone()],
    );
    assert_eq!(
      FunctionArgs::Ternary(out.clone(), a.clone(), b.clone(), c.clone()).input_values(),
      vec![a.clone(), b.clone(), c.clone()],
    );
    assert_eq!(
      FunctionArgs::Quaternary(
        out.clone(),
        a.clone(),
        b.clone(),
        c.clone(),
        d.clone(),
      )
      .input_values(),
      vec![a.clone(), b.clone(), c.clone(), d.clone()],
    );
    assert_eq!(
      FunctionArgs::Variadic(out, vec![a.clone(), b.clone(), c.clone(), d.clone()])
        .input_values(),
      vec![a, b, c, d],
    );
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_defaults_arguments_to_reactive() {
    let (first, first_cell) = scalar(1.0);
    let (second, second_cell) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(
        Box::new(TestFunction::new("default")),
        &[first, second],
      )
      .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
      node.inputs,
      vec![
        ReactiveDependency { cell: first_cell, kind: ReactiveDependencyKind::Reactive },
        ReactiveDependency { cell: second_cell, kind: ReactiveDependencyKind::Reactive },
      ],
    );
    assert_eq!(plan.reactive_consumers_for(first_cell), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(second_cell), &[node_id]);
    assert!(plan.sampled_consumers_for(first_cell).is_empty());
    assert!(plan.sampled_consumers_for(second_cell).is_empty());
  }

  #[cfg(all(feature = "set", feature = "f64"))]
  #[test]
  fn register_defaults_dependency_scope_to_recursive() {
    let (set, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(Box::new(TestFunction::new("recursive")), &[set])
      .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
      node.inputs,
      vec![
        ReactiveDependency { cell: outer, kind: ReactiveDependencyKind::Reactive },
        ReactiveDependency { cell: first, kind: ReactiveDependencyKind::Reactive },
        ReactiveDependency { cell: second, kind: ReactiveDependencyKind::Reactive },
      ],
    );
    assert_eq!(plan.reactive_consumers_for(outer), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(first), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(second), &[node_id]);
    assert!(plan.sampled_consumers.is_empty());
  }

  #[cfg(all(feature = "set", feature = "f64"))]
  #[test]
  fn register_root_scope_uses_only_root_cell() {
    let (set, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(
        Box::new(
          TestFunction::new("root").with_dependency_scopes(Some(vec![
            ReactiveDependencyScope::Root,
          ])),
        ),
        &[set],
      )
      .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
      node.inputs,
      vec![ReactiveDependency { cell: outer, kind: ReactiveDependencyKind::Reactive }],
    );
    assert_eq!(plan.reactive_consumers_for(outer), &[node_id]);
    assert!(plan.reactive_consumers_for(first).is_empty());
    assert!(plan.reactive_consumers_for(second).is_empty());
    assert_eq!(plan.reactive_consumers.len(), 1);
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_none_scope_ignores_argument_cells() {
    let (value, _) = scalar(1.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(
        Box::new(
          TestFunction::new("none").with_dependency_scopes(Some(vec![
            ReactiveDependencyScope::None,
          ])),
        ),
        &[value],
      )
      .unwrap();

    assert!(plan.node(node_id).unwrap().inputs.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_records_sampled_dependencies_separately() {
    let (first, first_cell) = scalar(1.0);
    let (second, second_cell) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(
        Box::new(
          TestFunction::new("sampled")
            .with_dependency_kinds(Some(vec![
              ReactiveDependencyKind::Sampled,
              ReactiveDependencyKind::Reactive,
            ])),
        ),
        &[first, second],
      )
      .unwrap();

    assert_eq!(plan.sampled_consumers_for(first_cell), &[node_id]);
    assert!(plan.reactive_consumers_for(first_cell).is_empty());
    assert_eq!(plan.reactive_consumers_for(second_cell), &[node_id]);
    assert!(plan.sampled_consumers_for(second_cell).is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_deduplicates_same_cell_same_kind() {
    let (value, cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(
        Box::new(TestFunction::new("dedupe")),
        &[value.clone(), value],
      )
      .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(node.inputs, vec![ReactiveDependency { cell, kind: ReactiveDependencyKind::Reactive }]);
    assert_eq!(plan.reactive_consumers_for(cell), &[node_id]);
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_rejects_same_cell_with_conflicting_kinds() {
    let (value, _cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();

    let error = plan
      .register(
        Box::new(
          TestFunction::new("conflict")
            .with_dependency_kinds(Some(vec![
              ReactiveDependencyKind::Sampled,
              ReactiveDependencyKind::Reactive,
            ])),
        ),
        &[value.clone(), value],
      )
      .unwrap_err();

    assert!(format!("{:?}", error).contains("ReactiveDependencyKindConflict"));
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_rejects_dependency_arity_mismatch() {
    let (first, _) = scalar(1.0);
    let (second, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let error = plan
      .register(
        Box::new(
          TestFunction::new("arity")
            .with_dependency_kinds(Some(vec![ReactiveDependencyKind::Reactive])),
        ),
        &[first, second],
      )
      .unwrap_err();

    assert!(format!("{:?}", error).contains("ReactiveDependencyArityMismatch"));
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_rejects_dependency_scope_arity_mismatch() {
    let (first, _) = scalar(1.0);
    let (second, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let error = plan
      .register(
        Box::new(
          TestFunction::new("scope arity").with_dependency_scopes(Some(vec![
            ReactiveDependencyScope::Recursive,
          ])),
        ),
        &[first, second],
      )
      .unwrap_err();

    assert!(format!("{:?}", error).contains("ReactiveDependencyScopeArityMismatch"));
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
  }

  #[cfg(feature = "f64")]
  #[test]
  fn register_records_outputs_and_kind() {
    let output = Ref::new(42.0);
    let output_cell = ReactiveCellId::new(output.id());
    let mut plan = ReactivePlan::new();

    let node_id = plan
      .register(
        Box::new(
          TestFunction::with_output("register", Value::F64(output))
            .with_node_kind(ReactiveNodeKind::Register),
        ),
        &[],
      )
      .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert!(node.outputs.contains(&output_cell));
  }

  #[cfg(feature = "f64")]
  struct SchedulerFunction {
    label: &'static str, output: Value, kind: ReactiveNodeKind,
    status: ReactiveSolveStatus, count: Rc<RefCell<usize>>, log: Rc<RefCell<Vec<&'static str>>>, error: bool,
  }
  #[cfg(feature = "f64")]
  impl MechFunctionImpl for SchedulerFunction {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
      *self.count.borrow_mut() += 1; self.log.borrow_mut().push(self.label);
      if self.error { Err(MechError::new(GenericError { msg: self.label.into() }, None)) } else { Ok(self.status) }
    }
    fn out(&self) -> Value { self.output.clone() }
    fn reactive_node_kind(&self) -> ReactiveNodeKind { self.kind }
    fn to_string(&self) -> String { self.label.into() }
  }
  #[cfg(all(feature = "f64", feature = "compiler"))]
  impl MechFunctionCompiler for SchedulerFunction { fn compile(&self, _: &mut CompileCtx) -> MResult<Register> { Ok(0) } }

  #[cfg(feature = "f64")]
  fn scheduler_node(plan: &mut ReactivePlan, label: &'static str, inputs: &[Value], kind: ReactiveNodeKind, status: ReactiveSolveStatus, log: Rc<RefCell<Vec<&'static str>>>, error: bool) -> (ReactiveNodeId, Value, Rc<RefCell<usize>>) {
    let output = Value::F64(Ref::new(0.0)); let count = Rc::new(RefCell::new(0));
    let function = SchedulerFunction { label, output: output.clone(), kind, status, count: count.clone(), log, error };
    (plan.register(Box::new(function), inputs).unwrap(), output, count)
  }
  #[cfg(feature = "f64")]
  fn scheduler_source() -> Value { Value::F64(Ref::new(0.0)) }

  #[cfg(feature = "f64")]
  #[test]
  fn reactive_dirty_scheduler_runs_linear_chain() {
    let mut p=ReactivePlan::new(); let l=Rc::new(RefCell::new(vec![])); let d=scheduler_source();
    let (a,ao,_)=scheduler_node(&mut p,"A",&[d.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);
    let (b,bo,_)=scheduler_node(&mut p,"B",&[ao],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);
    let (c,_,_)=scheduler_node(&mut p,"C",&[bo],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);
    let o=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap(); assert_eq!(o.executed_nodes,vec![a,b,c]); assert_eq!(o.changed_nodes,vec![a,b,c]); assert!(o.unchanged_nodes.is_empty()&&o.pending_register_nodes.is_empty()); assert_eq!(*l.borrow(),vec!["A","B","C"]);
  }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_orders_independent_branches_by_plan_index() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let x=scheduler_source();let y=scheduler_source();let(a,_,_)=scheduler_node(&mut p,"A",&[x.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);let(b,_,_)=scheduler_node(&mut p,"B",&[y.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);assert_eq!(p.solve_dirty_cells(&[y.reactive_root_cell_ids()[0],x.reactive_root_cell_ids()[0]]).unwrap().executed_nodes,vec![a,b]); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_skips_unrelated_nodes() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(_a,_,_)=scheduler_node(&mut p,"A",&[d.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);let(u,_,uc)=scheduler_node(&mut p,"U",&[],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);let o=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();assert_eq!(*uc.borrow(),0);assert!(!o.executed_nodes.contains(&u)); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_deduplicates_dirty_cells() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(_,_,c)=scheduler_node(&mut p,"A",&[d.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);let cell=d.reactive_root_cell_ids()[0];p.solve_dirty_cells(&[cell,cell,cell]).unwrap();assert_eq!(*c.borrow(),1); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_executes_fan_in_consumer_once() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let x=scheduler_source();let y=scheduler_source();let(_,lo,_)=scheduler_node(&mut p,"L",&[x.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);let(_,ro,_)=scheduler_node(&mut p,"R",&[y.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),false);let(_,_,c)=scheduler_node(&mut p,"J",&[lo,ro],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);p.solve_dirty_cells(&[x.reactive_root_cell_ids()[0],y.reactive_root_cell_ids()[0]]).unwrap();assert_eq!(*c.borrow(),1); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_propagates_changed_outputs() { reactive_dirty_scheduler_runs_linear_chain(); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_stops_on_unchanged() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(a,ao,ac)=scheduler_node(&mut p,"A",&[d.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Unchanged,l.clone(),false);let(b,_,bc)=scheduler_node(&mut p,"B",&[ao],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);let o=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();assert_eq!(*ac.borrow(),1);assert_eq!(*bc.borrow(),0);assert_eq!(o.unchanged_nodes,vec![a]);assert!(!o.executed_nodes.contains(&b)); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_ignores_sampled_consumers() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(n,_,c)=scheduler_node(&mut p,"R",&[],ReactiveNodeKind::Register,ReactiveSolveStatus::Changed,l,false);p.sampled_consumers.entry(d.reactive_root_cell_ids()[0]).or_default().push(n);let o=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();assert_eq!(*c.borrow(),0);assert!(!o.pending_register_nodes.contains(&n)); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_reports_register_pending_without_execution() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(r,_,c)=scheduler_node(&mut p,"R",&[d.clone()],ReactiveNodeKind::Register,ReactiveSolveStatus::Changed,l,false);let o=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();assert_eq!(o.pending_register_nodes,vec![r]);assert_eq!(*c.borrow(),0);assert!(o.executed_nodes.is_empty()); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_stops_at_register_boundary() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(r,ro,rc)=scheduler_node(&mut p,"R",&[d.clone()],ReactiveNodeKind::Register,ReactiveSolveStatus::Changed,l.clone(),false);let(_,_,dc)=scheduler_node(&mut p,"D",&[ro],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);let o=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap();assert_eq!(o.pending_register_nodes,vec![r]);assert_eq!(*rc.borrow(),0);assert_eq!(*dc.borrow(),0); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_dirty_register_output_runs_downstream_only() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(r,ro,rc)=scheduler_node(&mut p,"R",&[d.clone()],ReactiveNodeKind::Register,ReactiveSolveStatus::Changed,l.clone(),false);let(_,_,dc)=scheduler_node(&mut p,"D",&[ro.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);let cell=ro.reactive_root_cell_ids()[0];let o=p.solve_dirty_cells(&[cell]).unwrap();assert!(!o.pending_register_nodes.contains(&r));assert_eq!(*rc.borrow(),0);assert_eq!(*dc.borrow(),1); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_stops_on_error() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(_,ao,ac)=scheduler_node(&mut p,"A",&[d.clone()],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l.clone(),true);let(_,_,bc)=scheduler_node(&mut p,"B",&[ao],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);let e=p.solve_dirty_cells(&d.reactive_root_cell_ids()).unwrap_err();assert!(e.kind_message().contains("A"));assert_eq!(*ac.borrow(),1);assert_eq!(*bc.borrow(),0); }
  #[cfg(feature = "f64")] #[test]
  fn reactive_dirty_scheduler_empty_dirty_set_is_noop() { let mut p=ReactivePlan::new();let l=Rc::new(RefCell::new(vec![]));let d=scheduler_source();let(_,_,c)=scheduler_node(&mut p,"A",&[d],ReactiveNodeKind::Combinational,ReactiveSolveStatus::Changed,l,false);assert_eq!(p.solve_dirty_cells(&[]).unwrap(),ReactivePlanSolveOutcome::default());assert_eq!(*c.borrow(),0); }
}


#[cfg(all(test, feature = "f64"))]
mod reactive_turn_tests {
  use super::*;

  struct Commit { sink: Ref<f64>, next: f64, cells: Vec<ReactiveCellId>, count: Rc<RefCell<usize>> }
  impl ReactiveRegisterCommit for Commit {
    fn output_cells(&self) -> &[ReactiveCellId] { &self.cells }
    fn commit(self: Box<Self>) { *self.sink.borrow_mut() = self.next; *self.count.borrow_mut() += 1; }
  }
  struct TestRegister { source: Ref<f64>, sink: Ref<f64>, solve: Rc<RefCell<usize>>, stage: Rc<RefCell<usize>>, commit: Rc<RefCell<usize>>, fail: bool }
  impl MechFunctionImpl for TestRegister {
    fn solve(&self) { *self.solve.borrow_mut() += 1; }
    fn out(&self) -> Value { self.sink.to_value() }
    fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
      *self.stage.borrow_mut() += 1;
      if self.fail { return Err(MechError::new(GenericError { msg: "stage failure".into() }, None)); }
      Ok(Box::new(Commit { sink: self.sink.clone(), next: *self.source.borrow(), cells: self.reactive_output_cell_ids(), count: self.commit.clone() }))
    }
    fn to_string(&self) -> String { "test register".into() }
  }
  #[cfg(feature = "compiler")] impl MechFunctionCompiler for TestRegister { fn compile(&self, _: &mut CompileCtx) -> MResult<Register> { Ok(0) } }
  struct Comb { source: Ref<f64>, sink: Ref<f64>, add: f64, count: Rc<RefCell<usize>>, fail: bool }
  impl MechFunctionImpl for Comb {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> { *self.count.borrow_mut() += 1; if self.fail { return Err(MechError::new(GenericError { msg: "solve failure".into() }, None)); } *self.sink.borrow_mut() = *self.source.borrow() + self.add; Ok(ReactiveSolveStatus::Changed) }
    fn out(&self) -> Value { self.sink.to_value() }
    fn to_string(&self) -> String { "test combinational".into() }
  }
  #[cfg(feature = "compiler")] impl MechFunctionCompiler for Comb { fn compile(&self, _: &mut CompileCtx) -> MResult<Register> { Ok(0) } }
  fn counters() -> (Rc<RefCell<usize>>, Rc<RefCell<usize>>, Rc<RefCell<usize>>) { (Rc::new(RefCell::new(0)), Rc::new(RefCell::new(0)), Rc::new(RefCell::new(0))) }
  fn reg(p: &mut ReactivePlan, source: Ref<f64>, sink: Ref<f64>, fail: bool) -> (ReactiveNodeId, Rc<RefCell<usize>>, Rc<RefCell<usize>>, Rc<RefCell<usize>>) { let (solve,stage,commit)=counters(); let node=p.register(Box::new(TestRegister { source: source.clone(), sink, solve: solve.clone(), stage: stage.clone(), commit: commit.clone(), fail }), &[source.to_value()]).unwrap(); (node,solve,stage,commit) }
  fn comb(p: &mut ReactivePlan, source: Ref<f64>, sink: Ref<f64>, fail: bool) -> (ReactiveNodeId, Rc<RefCell<usize>>) { let count=Rc::new(RefCell::new(0)); let node=p.register(Box::new(Comb { source: source.clone(), sink, add: 1., count: count.clone(), fail }), &[source.to_value()]).unwrap(); (node,count) }
  fn chain() -> (ReactivePlan, Ref<f64>, Ref<f64>, Ref<f64>, Ref<f64>, ReactiveNodeId, ReactiveNodeId, ReactiveNodeId, Rc<RefCell<usize>>, Rc<RefCell<usize>>) { let mut p=ReactivePlan::new(); let input=Ref::new(1.);let a=Ref::new(1.);let middle=Ref::new(2.);let b=Ref::new(2.);let final_value=Ref::new(3.);let(ra,_,_,ca)=reg(&mut p,input.clone(),a.clone(),false);let(mid,_) = comb(&mut p,a.clone(),middle.clone(),false);let(rb,_,_,cb)=reg(&mut p,middle.clone(),b.clone(),false);let(final_node,_)=comb(&mut p,b.clone(),final_value.clone(),false);(p,input,a,middle,b,ra,rb,final_node,ca,cb) }
  #[test] fn reactive_turn_propagates_register_outputs_after_commit() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let a=Ref::new(1.);let out=Ref::new(2.);let(r,solve,stage,commit)=reg(&mut p,input.clone(),a.clone(),false);let(d,down)=comb(&mut p,a.clone(),out.clone(),false);*input.borrow_mut()=10.;let mut s=ReactiveTurnState::default();let o=p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap();assert_eq!(o.before_commit.pending_register_nodes,vec![r]);assert_eq!(o.register_commit.staged_nodes,vec![r]);assert_eq!(o.register_commit.committed_nodes,vec![r]);assert!(o.after_commit.executed_nodes.contains(&d));assert_eq!((*a.borrow(),*out.borrow(),*solve.borrow(),*stage.borrow(),*commit.borrow(),*down.borrow()),(10.,11.,0,1,1,1));assert!(s.pending_register_nodes.is_empty()); }
  #[test] fn reactive_turn_defers_post_commit_registers_until_next_turn() { let(mut p,input,a,middle,b,ra,rb,_,ca,cb)=chain();let final_value=p.nodes.last().unwrap().function.out().as_f64().unwrap().clone();*input.borrow_mut()=10.;let mut s=ReactiveTurnState::default();let first=p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap();assert_eq!(first.register_commit.committed_nodes,vec![ra]);assert_eq!(first.after_commit.pending_register_nodes,vec![rb]);assert_eq!(s.pending_register_nodes,vec![rb]);assert_eq!((*a.borrow(),*middle.borrow(),*b.borrow(),*final_value.borrow(),*cb.borrow()),(10.,11.,2.,3.,0));let second=p.advance_reactive_turn(&mut s,&[]).unwrap();assert_eq!(second.register_commit.committed_nodes,vec![rb]);assert_eq!((*ca.borrow(),*cb.borrow()),(1,1));assert!(!s.has_pending_registers()); }
  #[test] fn reactive_turn_commits_each_register_layer_at_most_once() { let(mut p,input,_,_,_,ra,rb,_,ca,cb)=chain();*input.borrow_mut()=10.;let mut s=ReactiveTurnState::default();p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap();assert_eq!((*ca.borrow(),*cb.borrow()),(1,0));p.advance_reactive_turn(&mut s,&[]).unwrap();assert_eq!((*ca.borrow(),*cb.borrow()),(1,1));assert_ne!(ra,rb); }
  #[test] fn reactive_turn_combines_carried_and_new_registers() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let(a,_,sa,ca)=reg(&mut p,input.clone(),Ref::new(0.),false);let(b,_,sb,cb)=reg(&mut p,input.clone(),Ref::new(0.),false);let mut s=ReactiveTurnState { pending_register_nodes: vec![b] };let o=p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap();assert_eq!(o.register_commit.staged_nodes,vec![a,b]);assert_eq!(o.register_commit.committed_nodes,vec![a,b]);assert_eq!((*sa.borrow(),*sb.borrow(),*ca.borrow(),*cb.borrow()),(1,1,1,1)); }
  #[test] fn reactive_turn_combinational_only_has_empty_commit() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let a=Ref::new(2.);let b=Ref::new(3.);let(na,_)=comb(&mut p,input.clone(),a.clone(),false);let(nb,_)=comb(&mut p,a.clone(),b.clone(),false);*input.borrow_mut()=10.;let mut s=ReactiveTurnState::default();let o=p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap();assert_eq!(o.before_commit.executed_nodes,vec![na,nb]);assert_eq!(o.register_commit,ReactiveRegisterCommitOutcome::default());assert_eq!(o.after_commit,ReactivePlanSolveOutcome::default());assert_eq!(*b.borrow(),12.); }
  #[test] fn reactive_turn_empty_is_noop() { let mut p=ReactivePlan::new();let mut s=ReactiveTurnState::default();assert_eq!(p.advance_reactive_turn(&mut s,&[]).unwrap(),ReactiveTurnOutcome::default());assert_eq!(s,ReactiveTurnState::default()); }
  #[test] fn reactive_turn_commit_failure_skips_post_commit_propagation() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let sink=Ref::new(1.);let(r,solve,stage,commit)=reg(&mut p,input.clone(),sink.clone(),true);let(_,down)=comb(&mut p,sink.clone(),Ref::new(2.),false);let mut s=ReactiveTurnState::default();let e=p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap_err();assert!(e.kind_message().contains("stage failure"));assert_eq!((*solve.borrow(),*stage.borrow(),*commit.borrow(),*down.borrow(),*sink.borrow()),(0,1,0,0,1.));assert_eq!(s.pending_register_nodes,vec![r]); }
  #[test] fn reactive_turn_post_commit_failure_does_not_requeue_committed_registers() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let sink=Ref::new(1.);let(_,_,_,commit)=reg(&mut p,input.clone(),sink.clone(),false);let(_,down)=comb(&mut p,sink.clone(),Ref::new(2.),true);*input.borrow_mut()=10.;let mut s=ReactiveTurnState::default();assert!(p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).is_err());assert_eq!((*sink.borrow(),*commit.borrow(),*down.borrow()),(10.,1,1));assert!(s.pending_register_nodes.is_empty()); }
  #[test]
  fn reactive_turn_post_commit_failure_preserves_deferred_registers() {
    let mut p = ReactivePlan::new();
    let input = Ref::new(1.);
    let a = Ref::new(1.);
    let middle = Ref::new(2.);
    let b = Ref::new(2.);
    let (a_register, _, _, a_commits) = reg(&mut p, input.clone(), a.clone(), false);
    let (_, middle_solves) = comb(&mut p, a.clone(), middle.clone(), false);
    let (b_register, _, _, b_commits) = reg(&mut p, middle.clone(), b.clone(), false);
    let (_, error_solves) = comb(&mut p, middle.clone(), Ref::new(0.), true);

    *input.borrow_mut() = 10.;
    let mut state = ReactiveTurnState::default();
    let error = p
      .advance_reactive_turn(&mut state, &input.to_value().reactive_root_cell_ids())
      .unwrap_err();

    assert!(error.kind_message().contains("solve failure"));
    assert_eq!((*a.borrow(), *middle.borrow(), *b.borrow()), (10., 11., 2.));
    assert_eq!((*a_commits.borrow(), *b_commits.borrow()), (1, 0));
    assert_eq!((*middle_solves.borrow(), *error_solves.borrow()), (1, 1));
    assert_eq!(state.pending_register_nodes, vec![b_register]);
    assert!(!state.pending_register_nodes.contains(&a_register));

    let retry = p.advance_reactive_turn(&mut state, &[]).unwrap();
    assert_eq!(retry.register_commit.committed_nodes, vec![b_register]);
    assert_eq!((*a_commits.borrow(), *b_commits.borrow()), (1, 1));
    assert_eq!(*b.borrow(), 11.);
    assert!(state.pending_register_nodes.is_empty());
  }
  #[test] fn reactive_turn_reuses_existing_plan() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let sink=Ref::new(1.);reg(&mut p,input.clone(),sink.clone(),false);comb(&mut p,sink.clone(),Ref::new(2.),false);let len=p.len();let ids=p.nodes.iter().map(|n|n.id).collect::<Vec<_>>();let outputs=p.nodes.iter().map(|n|n.outputs.clone()).collect::<Vec<_>>();let mut s=ReactiveTurnState::default();for value in [10.,20.] {*input.borrow_mut()=value;p.advance_reactive_turn(&mut s,&input.to_value().reactive_root_cell_ids()).unwrap();assert_eq!(p.len(),len);assert_eq!(p.nodes.iter().map(|n|n.id).collect::<Vec<_>>(),ids);assert_eq!(p.nodes.iter().map(|n|n.outputs.clone()).collect::<Vec<_>>(),outputs);} }
  #[test] fn reactive_turn_pre_commit_failure_preserves_carried_registers() { let mut p=ReactivePlan::new();let input=Ref::new(1.);let(carried,solve,stage,commit)=reg(&mut p,Ref::new(2.),Ref::new(3.),false);comb(&mut p,input.clone(),Ref::new(0.),true);let mut state=ReactiveTurnState{pending_register_nodes:vec![carried]};let error=p.advance_reactive_turn(&mut state,&input.to_value().reactive_root_cell_ids()).unwrap_err();assert!(error.kind_message().contains("solve failure"));assert_eq!((*solve.borrow(),*stage.borrow(),*commit.borrow()),(0,0,0));assert_eq!(state.pending_register_nodes,vec![carried]); }
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

#[cfg(all(test, feature = "f64"))]
mod reactive_register_commit_tests {
  use super::*;

  struct RegisterStageTestCommit { label: &'static str, sink: Ref<f64>, next: f64, output_cells: Vec<ReactiveCellId>, commit_count: Rc<RefCell<usize>>, commit_log: Rc<RefCell<Vec<&'static str>>>, total_commit_count: Rc<RefCell<usize>> }
  impl ReactiveRegisterCommit for RegisterStageTestCommit {
    fn output_cells(&self) -> &[ReactiveCellId] { &self.output_cells }
    fn commit(self: Box<Self>) { *self.sink.borrow_mut() = self.next; *self.commit_count.borrow_mut() += 1; *self.total_commit_count.borrow_mut() += 1; self.commit_log.borrow_mut().push(self.label); }
  }
  struct RegisterStageTestFunction { label: &'static str, sink: Ref<f64>, sources: Vec<Ref<f64>>, stage_count: Rc<RefCell<usize>>, solve_count: Rc<RefCell<usize>>, commit_count: Rc<RefCell<usize>>, stage_log: Rc<RefCell<Vec<&'static str>>>, commit_log: Rc<RefCell<Vec<&'static str>>>, total_commit_count: Rc<RefCell<usize>>, commit_counts_observed_during_stage: Rc<RefCell<Vec<usize>>>, fail_stage: bool, mismatch_outputs: Option<Vec<ReactiveCellId>> }
  impl MechFunctionImpl for RegisterStageTestFunction {
    fn solve(&self) { *self.solve_count.borrow_mut() += 1; }
    fn out(&self) -> Value { self.sink.to_value() }
    fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
      *self.stage_count.borrow_mut() += 1; self.stage_log.borrow_mut().push(self.label); let total = *self.total_commit_count.borrow(); self.commit_counts_observed_during_stage.borrow_mut().push(total);
      if self.fail_stage { return Err(MechError::new(GenericError { msg: self.label.to_string() }, None)); }
      let next = self.sources.iter().map(|source| *source.borrow()).sum::<f64>();
      let output_cells = self.mismatch_outputs.clone().unwrap_or_else(|| self.reactive_output_cell_ids());
      Ok(Box::new(RegisterStageTestCommit { label: self.label, sink: self.sink.clone(), next, output_cells, commit_count: self.commit_count.clone(), commit_log: self.commit_log.clone(), total_commit_count: self.total_commit_count.clone() }))
    }
    fn to_string(&self) -> String { self.label.to_string() }
  }
  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for RegisterStageTestFunction { fn compile(&self, _: &mut CompileCtx) -> MResult<Register> { Ok(0) } }
  struct Fixture { node: ReactiveNodeId, sink: Ref<f64>, stage: Rc<RefCell<usize>>, solve: Rc<RefCell<usize>>, commit: Rc<RefCell<usize>> }
  fn add(plan: &mut ReactivePlan, label: &'static str, sink: Ref<f64>, sources: Vec<Ref<f64>>, stage_log: Rc<RefCell<Vec<&'static str>>>, commit_log: Rc<RefCell<Vec<&'static str>>>, total: Rc<RefCell<usize>>, fail: bool, mismatch: Option<Vec<ReactiveCellId>>) -> Fixture {
    let stage=Rc::new(RefCell::new(0)); let solve=Rc::new(RefCell::new(0)); let commit=Rc::new(RefCell::new(0)); let observed=Rc::new(RefCell::new(vec![]));
    let node=plan.register(Box::new(RegisterStageTestFunction { label, sink:sink.clone(), sources, stage_count:stage.clone(), solve_count:solve.clone(), commit_count:commit.clone(), stage_log, commit_log, total_commit_count:total, commit_counts_observed_during_stage:observed, fail_stage:fail, mismatch_outputs:mismatch }), &[]).unwrap(); Fixture {node,sink,stage,solve,commit}
  }
  fn shared() -> (Rc<RefCell<Vec<&'static str>>>, Rc<RefCell<Vec<&'static str>>>, Rc<RefCell<usize>>) { (Rc::new(RefCell::new(vec![])),Rc::new(RefCell::new(vec![])),Rc::new(RefCell::new(0))) }
  #[test] fn reactive_register_commit_stages_all_before_any_commit() { let mut p=ReactivePlan::new();let(xl,cl,t)=shared();let x=Ref::new(1.);let y=Ref::new(2.);let a=add(&mut p,"X",x.clone(),vec![x.clone(),y.clone()],xl.clone(),cl.clone(),t.clone(),false,None);let b=add(&mut p,"Y",y.clone(),vec![y.clone(),x.clone()],xl.clone(),cl.clone(),t,false,None);let o=p.commit_pending_registers(&[b.node,a.node]).unwrap();assert_eq!((*x.borrow(),*y.borrow()),(3.,3.));assert_eq!(o.staged_nodes,vec![a.node,b.node]);assert_eq!(*xl.borrow(),vec!["X","Y"]);assert_eq!(*cl.borrow(),vec!["X","Y"]);assert_eq!((*a.solve.borrow(),*b.solve.borrow(),*a.stage.borrow(),*b.stage.borrow(),*a.commit.borrow(),*b.commit.borrow()),(0,0,1,1,1,1)); }
  #[test] fn reactive_register_commit_deduplicates_and_orders_pending_nodes() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(0.),vec![],l.clone(),c.clone(),t.clone(),false,None);let b=add(&mut p,"B",Ref::new(0.),vec![],l.clone(),c.clone(),t.clone(),false,None);let d=add(&mut p,"C",Ref::new(0.),vec![],l.clone(),c.clone(),t,false,None);let o=p.commit_pending_registers(&[d.node,a.node,b.node,a.node,d.node,b.node]).unwrap();assert_eq!(o.staged_nodes,vec![a.node,b.node,d.node]);assert_eq!(*l.borrow(),vec!["A","B","C"]);assert_eq!(*c.borrow(),vec!["A","B","C"]);for f in [&a,&b,&d] {assert_eq!((*f.stage.borrow(),*f.commit.borrow(),*f.solve.borrow()),(1,1,0));} }
  #[test] fn reactive_register_commit_is_atomic_on_stage_error() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(1.),vec![Ref::new(4.)],l.clone(),c.clone(),t.clone(),false,None);let b=add(&mut p,"B",Ref::new(2.),vec![],l,c,t.clone(),true,None);let e=p.commit_pending_registers(&[a.node,b.node]).unwrap_err();assert!(e.kind_message().contains("B"));assert_eq!((*a.sink.borrow(),*b.sink.borrow(),*a.commit.borrow(),*b.commit.borrow(),*t.borrow()),(1.,2.,0,0,0)); }
  #[test] fn reactive_register_commit_rejects_missing_node_without_staging() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(1.),vec![],l,c,t,false,None);let missing=p.nodes.len()+100;let e=p.commit_pending_registers(&[a.node,missing]).unwrap_err();assert_eq!(e.kind_name(),"ReactiveRegisterNodeNotFound");assert_eq!((*a.stage.borrow(),*a.commit.borrow(),*a.solve.borrow(),*a.sink.borrow()),(0,0,0,1.)); }
  #[test] fn reactive_register_commit_rejects_combinational_node_without_staging() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(1.),vec![],l,c,t,false,None);struct Combinational; impl MechFunctionImpl for Combinational { fn solve(&self) {} fn out(&self) -> Value { Value::Empty } fn to_string(&self)->String { "C".into() } } #[cfg(feature="compiler")] impl MechFunctionCompiler for Combinational { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Ok(0)} } let combinational=p.push(Box::new(Combinational));let e=p.commit_pending_registers(&[a.node,combinational]).unwrap_err();assert_eq!(e.kind_name(),"ReactiveRegisterNodeKind");assert_eq!((*a.stage.borrow(),*a.commit.borrow(),*a.solve.borrow()),(0,0,0)); }
  #[test] fn reactive_register_commit_rejects_overlapping_outputs_before_staging() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let sink=Ref::new(1.);let a=add(&mut p,"A",sink.clone(),vec![],l.clone(),c.clone(),t.clone(),false,None);let b=add(&mut p,"B",sink.clone(),vec![],l,c,t,false,None);let e=p.commit_pending_registers(&[a.node,b.node]).unwrap_err();assert_eq!(e.kind_name(),"ReactiveRegisterOutputConflict");assert_eq!((*a.stage.borrow(),*b.stage.borrow(),*a.commit.borrow(),*b.commit.borrow(),*sink.borrow()),(0,0,0,0,1.)); }
  #[test] fn reactive_register_commit_rejects_staged_output_mismatch_without_commit() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let sink=Ref::new(1.);let other=Ref::new(2.).to_value().reactive_root_cell_ids();let a=add(&mut p,"A",sink.clone(),vec![],l,c,t,false,Some(other));let e=p.commit_pending_registers(&[a.node]).unwrap_err();assert_eq!(e.kind_name(),"ReactiveRegisterStagedOutputMismatch");assert_eq!((*a.stage.borrow(),*a.commit.borrow(),*a.solve.borrow(),*sink.borrow()),(1,0,0,1.)); }
  #[test] fn reactive_register_commit_returns_ordered_unique_dirty_cells() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(1.),vec![],l.clone(),c.clone(),t.clone(),false,None);let b=add(&mut p,"B",Ref::new(2.),vec![],l,c,t,false,None);let cells=vec![p.nodes[a.node].outputs[0],p.nodes[b.node].outputs[0]];let o=p.commit_pending_registers(&[b.node,a.node,b.node]).unwrap();assert_eq!(o.dirty_cells,cells);assert_eq!(o.committed_nodes,vec![a.node,b.node]); }
  struct RegisterWithoutStaging { sink: Ref<f64>, solves: Rc<RefCell<usize>> }
  impl MechFunctionImpl for RegisterWithoutStaging { fn solve(&self) {*self.solves.borrow_mut()+=1;} fn out(&self)->Value {self.sink.to_value()} fn reactive_node_kind(&self)->ReactiveNodeKind {ReactiveNodeKind::Register} fn to_string(&self)->String {"unsupported".into()} }
  #[cfg(feature="compiler")] impl MechFunctionCompiler for RegisterWithoutStaging { fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Ok(0)} }
  #[test] fn reactive_register_commit_does_not_execute_downstream_nodes() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(1.),vec![Ref::new(2.)],l,c,t,false,None);let downstream=Rc::new(RefCell::new(0)); struct C(Rc<RefCell<usize>>);impl MechFunctionImpl for C {fn solve(&self){*self.0.borrow_mut()+=1;}fn out(&self)->Value{Value::Empty}fn to_string(&self)->String{"C".into()}} #[cfg(feature="compiler")] impl MechFunctionCompiler for C {fn compile(&self,_:&mut CompileCtx)->MResult<Register>{Ok(0)}} p.push(Box::new(C(downstream.clone())));let o=p.commit_pending_registers(&[a.node]).unwrap();assert_eq!(*a.commit.borrow(),1);assert!(o.dirty_cells.contains(&p.nodes[a.node].outputs[0]));assert_eq!(*downstream.borrow(),0); }
  #[test] fn reactive_register_commit_rejects_unsupported_register_staging() { let mut p=ReactivePlan::new();let sink=Ref::new(1.);let solves=Rc::new(RefCell::new(0));let n=p.register(Box::new(RegisterWithoutStaging{sink:sink.clone(),solves:solves.clone()}),&[]).unwrap();let e=p.commit_pending_registers(&[n]).unwrap_err();assert_eq!(e.kind_name(),"ReactiveRegisterStagingUnsupported");assert_eq!((*solves.borrow(),*sink.borrow()),(0,1.)); }
  #[test] fn reactive_register_commit_empty_pending_set_is_noop() { let mut p=ReactivePlan::new();let(l,c,t)=shared();let a=add(&mut p,"A",Ref::new(1.),vec![],l,c,t,false,None);assert_eq!(p.commit_pending_registers(&[]).unwrap(),ReactiveRegisterCommitOutcome::default());assert_eq!((*a.stage.borrow(),*a.solve.borrow(),*a.commit.borrow()),(0,0,0)); }
}
