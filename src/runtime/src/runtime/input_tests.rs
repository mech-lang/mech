use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::thread;
use std::time::Duration;

use mech_core::{hash_str, MResult, MechError, MechErrorKind, ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, Ref, Value};

use super::*;
use crate::{
  BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId,
  ConfigValue, HostContextManifest, HostInstanceConfig, HostManifestConfig,
  RuntimeCapabilityGrant, RuntimeHostFactory, RuntimeHostInstallation, RuntimeHostInput,
  RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue, RuntimeIngress,
  RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, RuntimeResourceWriteIntent, ClosureHostFunction, materialize_host_manifest,
};

const TEST_CLOCK_BASE_URI: &str =
  "test://clock/ticks";
const TEST_TIMER_BASE_URI: &str =
  "test://timer/state";
const TEST_SIGNALS_BASE_URI: &str =
  "test://signals/inputs";
const TEST_OUTPUT_BASE_URI: &str =
  "test://effects/output";

#[derive(Debug, Clone)]
struct TestFixtureError(String);
impl MechErrorKind for TestFixtureError {
  fn name(&self) -> &str { "TestFixtureError" }
  fn message(&self) -> String { self.0.clone() }
}

#[derive(Debug, Default)]
struct TestResourceProvider { values: BTreeMap<String, BTreeMap<String, Value>> }
impl TestResourceProvider {
  fn new() -> Self { Self::default() }
  fn with_value(mut self, base_uri: &str, path: &str, value: Value) -> Self {
    assert!(base_uri.starts_with("test://"), "test fixture resource must use test://");
    assert!(!path.is_empty(), "test fixture path must not be empty");
    self.values.entry(base_uri.to_string()).or_default().insert(path.to_string(), value);
    self
  }
}
impl RuntimeResourceProvider for TestResourceProvider {
  fn scheme(&self) -> &str { "test" }
  fn base_uris(&self) -> Vec<String> { self.values.keys().cloned().collect() }
  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    self.values.get(&request.base_uri).and_then(|paths| paths.get(&request.path)).cloned().ok_or_else(|| MechError::new(TestFixtureError(format!("missing test resource {} / {}", request.base_uri, request.path)), None))
  }
}

#[derive(Clone, Debug, Default)]
struct RecordingTestOutput { lines: Rc<RefCell<Vec<String>>> }
impl RecordingTestOutput { fn lines(&self) -> Vec<String> { self.lines.borrow().clone() } }
#[derive(Debug)]
struct TestOutputProvider { backend: RecordingTestOutput }
impl RuntimeResourceProvider for TestOutputProvider {
  fn scheme(&self) -> &str { "test" }
  fn base_uris(&self) -> Vec<String> { vec![TEST_OUTPUT_BASE_URI.to_string()] }
  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> { Err(MechError::new(TestFixtureError(format!("test output is write-only: {} / {}", request.base_uri, request.path)), None)) }
  fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
    if request.base_uri == TEST_OUTPUT_BASE_URI && request.path == "line" && request.intent == RuntimeResourceWriteIntent::Send { Ok(()) } else { Err(MechError::new(TestFixtureError(format!("invalid test output write: {} / {}", request.base_uri, request.path)), None)) }
  }
  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    self.preflight_write(RuntimeResourceWritePreflightRequest { base_uri: request.base_uri.clone(), path: request.path.clone(), context_name: request.context_name.clone(), operation: request.operation.clone(), intent: request.intent })?;
    self.backend.lines.borrow_mut().push(format!("{}", request.value)); Ok(())
  }
}

fn f64_value(value: &Value) -> f64 {
  match value {
    Value::F64(value) => *value.borrow(),
    other => panic!("expected f64, got {other:?}"),
  }
}

fn host_f64_argument(value: &Value) -> f64 {
  match value {
    Value::F64(value) => *value.borrow(),
    Value::MutableReference(value) => match &*value.borrow() {
      Value::F64(value) => *value.borrow(),
      other => panic!("expected f64 mutable reference, got {other:?}"),
    },
    other => panic!("expected f64 host argument, got {other:?}"),
  }
}

fn string_value(value: &Value) -> String {
  match value {
    Value::String(value) => value.borrow().clone(),
    other => panic!("expected string, got {other:?}"),
  }
}

#[derive(Debug, Clone)]
struct DeliberateHostCallError;
impl MechErrorKind for DeliberateHostCallError {
  fn name(&self) -> &str { "DeliberateHostCallError" }
  fn message(&self) -> String { "deliberate host call failure".to_string() }
}

fn symbol_value(runtime: &MechRuntime, name: &str) -> Value {
  runtime
    .program
    .interpreter()
    .symbols()
    .borrow()
    .get(hash_str(name))
    .unwrap_or_else(|| panic!("missing symbol {name}"))
    .borrow()
    .clone()
}


fn source_value(runtime: &MechRuntime, source: &RuntimeHostInputSource) -> Value {
  let input = runtime
    .live_input_bindings
    .get(source)
    .and_then(|inputs| inputs.first())
    .unwrap_or_else(|| panic!("missing binding for {} / {}", source.base_uri(), source.path()));
  runtime
    .program
    .interpreter()
    .symbols()
    .borrow()
    .get(input.symbol_id)
    .unwrap_or_else(|| panic!("missing symbol {}", input.symbol_id))
    .borrow()
    .clone()
}

fn symbol_cell(runtime: &MechRuntime, name: &str) -> ReactiveCellId {
  let cells = symbol_value(runtime, name).reactive_root_cell_ids();
  assert_eq!(cells.len(), 1, "symbol {name} must have one root cell");
  cells[0]
}

fn source_cell(runtime: &MechRuntime, source: &RuntimeHostInputSource) -> ReactiveCellId {
  let cells = source_value(runtime, source).reactive_root_cell_ids();
  assert_eq!(cells.len(), 1, "source must have one root cell");
  cells[0]
}

fn register_node_for_symbol(runtime: &MechRuntime, name: &str) -> ReactiveNodeId {
  let output = symbol_cell(runtime, name);
  let plan = runtime.program.interpreter().plan();
  let plan = plan.borrow();
  let nodes = plan.nodes.iter().filter(|node| node.kind == ReactiveNodeKind::Register && node.outputs.contains(&output)).map(|node| node.id).collect::<Vec<_>>();
  assert_eq!(nodes.len(), 1, "symbol {name} must have one register node");
  nodes[0]
}

fn combinational_node_for_output_and_inputs(runtime: &MechRuntime, output: ReactiveCellId, required_inputs: &[ReactiveCellId]) -> ReactiveNodeId {
  let plan = runtime.program.interpreter().plan();
  let plan = plan.borrow();
  let nodes = plan.nodes.iter().filter(|node| node.kind == ReactiveNodeKind::Combinational && node.outputs.contains(&output) && required_inputs.iter().all(|required| node.inputs.iter().any(|dependency| dependency.cell == *required && dependency.kind == ReactiveDependencyKind::Reactive))).map(|node| node.id).collect::<Vec<_>>();
  assert_eq!(nodes.len(), 1, "expected one matching combinational node");
  nodes[0]
}

fn plan_snapshot(runtime: &MechRuntime) -> (usize, Vec<ReactiveNodeId>, Vec<Vec<ReactiveCellId>>) {
  let plan = runtime.program.interpreter().plan();
  let plan = plan.borrow();
  (plan.len(), plan.nodes.iter().map(|node| node.id).collect(), plan.nodes.iter().map(|node| node.outputs.clone()).collect())
}

fn test_provider_with(base_uri: &str, path: &str, value: f64) -> TestResourceProvider {
  TestResourceProvider::new()
    .with_value(base_uri, path, Value::F64(Ref::new(value)))
}

fn grant_read(runtime: &mut MechRuntime, resource: &str, path: &str) {
  let subject = runtime.runtime_context().unwrap().subject;
  runtime.grant_capability(RuntimeCapabilityGrant {
    subject,
    resource: resource.to_string(),
    operations: vec![RuntimeCapabilityOperation::Read],
    paths: vec![path.to_string()],
  }).unwrap();
}

fn grant_write(runtime: &mut MechRuntime, resource: &str, path: &str) {
  let subject = runtime.runtime_context().unwrap().subject;
  runtime.grant_capability(RuntimeCapabilityGrant { subject, resource: resource.to_string(), operations: vec![RuntimeCapabilityOperation::Write], paths: vec![path.to_string()] }).unwrap();
}

fn test_runtime(provider: TestResourceProvider) -> MechRuntime {
  RuntimeBuilder::new()
    .resource_provider(Box::new(provider))
    .build()
    .unwrap()
}

fn test_runtime_with_output(provider: TestResourceProvider) -> (MechRuntime, RecordingTestOutput) {
  let output = RecordingTestOutput::default();
  let runtime = RuntimeBuilder::new()
    .resource_provider(Box::new(provider))
    .resource_provider(Box::new(TestOutputProvider { backend: output.clone() }))
    .build()
    .unwrap();
  (runtime, output)
}

#[test]
fn canonical_host_input_updates_live_context_read() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "test://clock/ticks", "value");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();

  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
  let canonical = RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap();
  assert!(runtime.live_input_bindings.contains_key(&canonical));
  assert!(runtime.live_input_bindings.keys().all(|source| !source.base_uri().contains("pulse") && !source.path().contains("pulse")));

  let alias = RuntimeHostInputSource::new("test://pulse", "value").unwrap();
  let outcome = runtime
    .apply_host_input(RuntimeHostInput::single(alias, RuntimeHostInputValue::F64(5.0)))
    .unwrap();
  assert_eq!(outcome.update_count, 1);
  assert_eq!(outcome.ignored_update_count, 1);
  assert_eq!(outcome.binding_count, 0);
  assert!(outcome.turn.is_none());
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);

  runtime.ingress().submit(RuntimeHostInput::single(canonical, RuntimeHostInputValue::F64(5.0))).unwrap();
  let outcomes = runtime.drain_host_inputs(1).unwrap();
  assert_eq!(outcomes.len(), 1);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
}

#[test]
fn runtime_reactive_host_input_batches_bound_updates_into_one_turn() {
  let provider = TestResourceProvider::new()
    .with_value(TEST_CLOCK_BASE_URI, "a", Value::F64(Ref::new(1.0)))
    .with_value(TEST_CLOCK_BASE_URI, "b", Value::F64(Ref::new(2.0)));
  let mut runtime = test_runtime(provider);
  grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "a");
  grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "b");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(a), :read(b)}\nsum := @pulse/a + @pulse/b").unwrap();
  let a_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "a").unwrap();
  let b_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "b").unwrap();
  let sum_node = combinational_node_for_output_and_inputs(&runtime, symbol_cell(&runtime, "sum"), &[source_cell(&runtime, &a_source), source_cell(&runtime, &b_source)]);
  let root_interpreter_id = runtime.program.interpreter().id;
  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: a_source.clone(), value: RuntimeHostInputValue::F64(10.0) },
    RuntimeHostInputUpdate { source: b_source.clone(), value: RuntimeHostInputValue::F64(20.0) },
  ]).unwrap()).unwrap();
  let program_turn = outcome.turn.as_ref().unwrap();
  let interpreter_turn = &program_turn.interpreter_turns[0];
  let reactive_turn = &interpreter_turn.turn;
  assert_eq!(outcome.update_count, 2); assert_eq!(outcome.ignored_update_count, 0); assert_eq!(outcome.binding_count, 2);
  assert_eq!(program_turn.updated_count, 2); assert_eq!(program_turn.interpreter_turns.len(), 1); assert_eq!(interpreter_turn.interpreter_id, root_interpreter_id); assert!(!interpreter_turn.dirty_cells.is_empty());
  assert_eq!(reactive_turn.before_commit.executed_nodes.iter().filter(|node| **node == sum_node).count(), 1);
  assert!(reactive_turn.register_commit.staged_nodes.is_empty()); assert!(reactive_turn.register_commit.committed_nodes.is_empty()); assert!(reactive_turn.register_commit.dirty_cells.is_empty());
  assert_eq!(f64_value(&source_value(&runtime, &a_source)), 10.0); assert_eq!(f64_value(&source_value(&runtime, &b_source)), 20.0); assert_eq!(f64_value(&symbol_value(&runtime, "sum")), 30.0);
}

#[test]
fn packet_with_bound_and_unbound_sources_updates_bound_inputs() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "test://clock/ticks", "value");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();
  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), value: RuntimeHostInputValue::F64(5.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://clock/ticks", "missing").unwrap(), value: RuntimeHostInputValue::F64(9.0) },
  ]).unwrap()).unwrap();

  assert_eq!(outcome.update_count, 2);
  assert_eq!(outcome.ignored_update_count, 1);
  assert_eq!(outcome.binding_count, 1);
  assert!(outcome.turn.is_some());
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap())), 5.0);
}


#[test]
fn partial_snapshot_updates_bound_fields_and_ignores_unbound_fields() {
  let provider = TestResourceProvider::new()
    .with_value(TEST_TIMER_BASE_URI, "tick", Value::F64(Ref::new(1.0)))
    .with_value(TEST_TIMER_BASE_URI, "delta-seconds", Value::F64(Ref::new(0.1)));
  let mut runtime = test_runtime(provider);
  grant_read(&mut runtime, "test://timer/state", "tick");
  grant_read(&mut runtime, "test://timer/state", "delta-seconds");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@timer := test://timer/state{:read(tick), :read(delta-seconds)}\noutput := @timer/tick + @timer/delta-seconds").unwrap();

  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://timer/state", "tick").unwrap(), value: RuntimeHostInputValue::F64(10.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://timer/state", "elapsed-ms").unwrap(), value: RuntimeHostInputValue::F64(1000.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://timer/state", "delta-ms").unwrap(), value: RuntimeHostInputValue::F64(16.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://timer/state", "elapsed-seconds").unwrap(), value: RuntimeHostInputValue::F64(1.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://timer/state", "delta-seconds").unwrap(), value: RuntimeHostInputValue::F64(0.25) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("test://timer/state", "skipped-steps").unwrap(), value: RuntimeHostInputValue::F64(0.0) },
  ]).unwrap()).unwrap();

  assert_eq!(outcome.update_count, 6);
  assert_eq!(outcome.ignored_update_count, 4);
  assert_eq!(outcome.binding_count, 2);
  assert!(outcome.turn.is_some());
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("test://timer/state", "tick").unwrap())), 10.0);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("test://timer/state", "delta-seconds").unwrap())), 0.25);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.25);
}

#[test]
fn runtime_reactive_host_input_unbound_packet_does_not_advance_pending_registers() {
  let (mut runtime, output) = test_runtime_with_output(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0));
  grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value"); grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
  let mut context = runtime.runtime_context().unwrap(); runtime.run_string_with_context(&mut context, "@out := test://effects/output{:write(line)}\n@pulse := test://clock/ticks{:read(value)}\n~a := 0.0\n~b := 0.0\na = @pulse/value\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0\n@out/line <- output").unwrap();
  assert_eq!(output.lines().len(), 1);
  let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap(); runtime.apply_host_input(RuntimeHostInput::single(source, RuntimeHostInputValue::F64(10.0))).unwrap();
  assert_eq!(f64_value(&symbol_value(&runtime, "b")), 2.0); assert_eq!(f64_value(&symbol_value(&runtime, "output")), 3.0); assert!(runtime.program.interpreter().has_pending_reactive_registers()); assert_eq!(output.lines().len(), 2);
  let b_before = f64_value(&symbol_value(&runtime, "b")); let output_before = f64_value(&symbol_value(&runtime, "output")); let lines_before = output.lines();
  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "missing-a").unwrap(), value: RuntimeHostInputValue::F64(5.0) }, RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "missing-b").unwrap(), value: RuntimeHostInputValue::F64(9.0) }]).unwrap()).unwrap();
  assert_eq!(outcome.update_count, 2); assert_eq!(outcome.ignored_update_count, 2); assert_eq!(outcome.binding_count, 0); assert!(outcome.turn.is_none()); assert_eq!(f64_value(&symbol_value(&runtime, "b")), b_before); assert_eq!(f64_value(&symbol_value(&runtime, "output")), output_before); assert!(runtime.program.interpreter().has_pending_reactive_registers()); assert_eq!(output.lines(), lines_before);
}

#[test]
fn live_input_recomputes_runtime_host_function() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "test://clock/ticks", "value");
  runtime
    .grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(42),
      &BasicSubject::new(&runtime.runtime_context().unwrap().subject),
      &BasicResource::new("host:demo/live-plus-one"),
      [BasicOperation::new("call")],
    )))
    .unwrap();
  let calls = Arc::new(AtomicUsize::new(0));
  let host_calls = calls.clone();
  runtime.register_mech_host_function(ClosureHostFunction::new("demo/live-plus-one", move |_services, _context, args| {
    host_calls.fetch_add(1, Ordering::SeqCst);
    match &args[0] {
      Value::F64(value) => Ok(Value::F64(Ref::new(*value.borrow() + 1.0))),
      Value::MutableReference(value) => match &*value.borrow() {
        Value::F64(value) => Ok(Value::F64(Ref::new(*value.borrow() + 1.0))),
        other => panic!("expected f64 mutable reference, got {other:?}"),
      },
      other => panic!("expected f64 argument, got {other:?}"),
    }
  })).unwrap();
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := demo/live-plus-one(@pulse/value) + 0").unwrap();
  let initial_calls = calls.load(Ordering::SeqCst);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);

  runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(9.0))).unwrap();

  assert!(calls.load(Ordering::SeqCst) > initial_calls);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap())), 9.0);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
}

#[test]
fn runtime_reactive_host_input_executes_only_reachable_branch() {
  let provider = TestResourceProvider::new().with_value(TEST_CLOCK_BASE_URI, "left", Value::F64(Ref::new(1.0))).with_value(TEST_CLOCK_BASE_URI, "right", Value::F64(Ref::new(2.0)));
  let mut runtime = test_runtime(provider);
  grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "left"); grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "right");
  let subject = runtime.runtime_context().unwrap().subject;
  grant_host_call(&mut runtime, &subject, 91, "host:demo/left-branch"); grant_host_call(&mut runtime, &subject, 92, "host:demo/right-branch");
  let left_calls = Arc::new(AtomicUsize::new(0)); let right_calls = Arc::new(AtomicUsize::new(0));
  for (name, calls, add) in [("demo/left-branch", left_calls.clone(), 100.0), ("demo/right-branch", right_calls.clone(), 200.0)] {
    runtime.register_mech_host_function(ClosureHostFunction::new(name, move |_services, _context, args| { calls.fetch_add(1, Ordering::SeqCst); let input = host_f64_argument(&args[0]); Ok(Value::F64(Ref::new(input + add))) })).unwrap();
  }
  let mut context = runtime.runtime_context().unwrap(); runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(left), :read(right)}\nleft-output := demo/left-branch(@pulse/left)\nright-output := demo/right-branch(@pulse/right)").unwrap();
  let left_calls_before = left_calls.load(Ordering::SeqCst);
  let right_calls_before = right_calls.load(Ordering::SeqCst);
  assert_eq!(f64_value(&symbol_value(&runtime, "left-output")), 101.0); assert_eq!(f64_value(&symbol_value(&runtime, "right-output")), 202.0);
  let left_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "left").unwrap(); let right_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "right").unwrap();
  let plan = runtime.program.interpreter().plan(); let left_consumers = plan.borrow().reactive_consumers_for(source_cell(&runtime, &left_source)).to_vec(); let right_consumers = plan.borrow().reactive_consumers_for(source_cell(&runtime, &right_source)).to_vec(); drop(plan);
  let outcome = runtime.apply_host_input(RuntimeHostInput::single(left_source, RuntimeHostInputValue::F64(10.0))).unwrap();
  let program_turn = outcome.turn.as_ref().unwrap();
  assert_eq!(program_turn.interpreter_turns.len(), 1);
  let reactive_turn = &program_turn.interpreter_turns[0].turn;
  assert_eq!(left_calls.load(Ordering::SeqCst), left_calls_before + 1);
  assert_eq!(right_calls.load(Ordering::SeqCst), right_calls_before);
  assert!(reactive_turn.register_commit.staged_nodes.is_empty()); assert!(reactive_turn.register_commit.committed_nodes.is_empty()); assert!(reactive_turn.register_commit.dirty_cells.is_empty());
  assert!(left_consumers.iter().any(|id| reactive_turn.before_commit.executed_nodes.contains(id))); assert!(!right_consumers.iter().any(|id| reactive_turn.before_commit.executed_nodes.contains(id))); assert_eq!(f64_value(&symbol_value(&runtime, "left-output")), 110.0); assert_eq!(f64_value(&symbol_value(&runtime, "right-output")), 202.0);
}

#[test]
fn live_host_string_output_recomputes_without_replacing_reference() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "test://clock/ticks", "value");
  let subject = runtime.runtime_context().unwrap().subject;
  grant_host_call(&mut runtime, &subject, 45, "host:demo/live-label");
  runtime.register_mech_host_function(ClosureHostFunction::new("demo/live-label", |_services, _context, args| {
    let value = match &args[0] {
      Value::F64(value) => *value.borrow(),
      Value::MutableReference(value) => match &*value.borrow() {
        Value::F64(value) => *value.borrow(),
        other => panic!("expected f64 mutable reference, got {other:?}"),
      },
      other => panic!("expected f64 argument, got {other:?}"),
    };
    Ok(Value::String(Ref::new(format!("tick:{value}"))))
  })).unwrap();
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := demo/live-label(@pulse/value)").unwrap();
  let before = runtime.program.interpreter().symbols().borrow().get(hash_str("output")).unwrap();
  let outer_pointer = before.as_ptr();
  let inner_pointer = match &*before.borrow() {
    Value::String(value) => value.as_ptr(),
    other => panic!("expected string, got {other:?}"),
  };
  assert_eq!(string_value(&before.borrow()), "tick:1");

  runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(9.0))).unwrap();

  let after = runtime.program.interpreter().symbols().borrow().get(hash_str("output")).unwrap();
  assert_eq!(outer_pointer, after.as_ptr());
  match &*after.borrow() {
    Value::String(value) => {
      assert_eq!(inner_pointer, value.as_ptr());
      assert_eq!(&*value.borrow(), "tick:9");
    }
    other => panic!("expected string, got {other:?}"),
  }
}

#[test]
fn live_host_output_kind_change_preserves_previous_output() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "test://clock/ticks", "value");
  let subject = runtime.runtime_context().unwrap().subject;
  grant_host_call(&mut runtime, &subject, 46, "host:demo/kind-change");
  let calls = Arc::new(AtomicUsize::new(0));
  let host_calls = calls.clone();
  runtime.register_mech_host_function(ClosureHostFunction::new("demo/kind-change", move |_services, _context, args| {
    let call = host_calls.fetch_add(1, Ordering::SeqCst);
    if call == 0 {
      let value = match &args[0] {
        Value::F64(value) => *value.borrow(),
        Value::MutableReference(value) => match &*value.borrow() {
          Value::F64(value) => *value.borrow(),
          other => panic!("expected f64 mutable reference, got {other:?}"),
        },
        other => panic!("expected f64 argument, got {other:?}"),
      };
      Ok(Value::F64(Ref::new(value + 1.0)))
    } else {
      Ok(Value::String(Ref::new("bad-kind".to_string())))
    }
  })).unwrap();
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\nhost-result := demo/kind-change(@pulse/value)\noutput := host-result + 0").unwrap();
  assert_eq!(f64_value(&symbol_value(&runtime, "host-result")), 2.0);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
  let host_result = runtime.program.interpreter().symbols().borrow().get(hash_str("host-result")).unwrap();
  let host_result_inner = match &*host_result.borrow() {
    Value::F64(value) => value.as_ptr(),
    other => panic!("expected f64 host result, got {other:?}"),
  };

  let error = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(9.0))).unwrap_err();
  let rendered = format!("{error:?}");
  assert!(rendered.contains("RuntimeHostOutputUpdateError"), "{rendered}");
  assert!(calls.load(Ordering::SeqCst) >= 2);
  let host_result = runtime.program.interpreter().symbols().borrow().get(hash_str("host-result")).unwrap();
  match &*host_result.borrow() {
    Value::F64(value) => {
      assert_eq!(host_result_inner, value.as_ptr());
      assert_eq!(*value.borrow(), 2.0);
    }
    other => panic!("expected f64 host result, got {other:?}"),
  }
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
  runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn runtime_reactive_host_input_turn_failure_preserves_admitted_inputs() {
  let (mut runtime, output) = test_runtime_with_output(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0)); grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value"); grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
  let subject = runtime.runtime_context().unwrap().subject; grant_host_call(&mut runtime, &subject, 47, "host:demo/fails-after-first");
  let calls = Arc::new(AtomicUsize::new(0)); let host_calls = calls.clone(); let fail_host = Arc::new(AtomicBool::new(false)); let fail_host_for_call = fail_host.clone();
  runtime.register_mech_host_function(ClosureHostFunction::new("demo/fails-after-first", move |_services, _context, args| { host_calls.fetch_add(1, Ordering::SeqCst); if fail_host_for_call.load(Ordering::SeqCst) { return Err(MechError::new(DeliberateHostCallError, None)); } let input = host_f64_argument(&args[0]); Ok(Value::F64(Ref::new(input + 1.0))) })).unwrap();
  let mut context = runtime.runtime_context().unwrap(); runtime.run_string_with_context(&mut context, "@out := test://effects/output{:write(line)}\n@pulse := test://clock/ticks{:read(value)}\nhost-result := demo/fails-after-first(@pulse/value)\noutput := host-result + 0\n@out/line <- output").unwrap();
  let calls_before = calls.load(Ordering::SeqCst); let input_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap(); let host_result = runtime.program.interpreter().symbols().borrow().get(hash_str("host-result")).unwrap(); let host_result_pointer = match &*host_result.borrow() { Value::F64(value) => value.as_ptr(), other => panic!("expected f64 host result, got {other:?}") }; let plan_before = plan_snapshot(&runtime); let lines_before = output.lines();
  assert_eq!(f64_value(&source_value(&runtime, &input_source)), 1.0); assert_eq!(f64_value(&symbol_value(&runtime, "host-result")), 2.0); assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0); assert_eq!(output.lines().len(), 1); assert_eq!(runtime.persistent_send_count(), 1);
  fail_host.store(true, Ordering::SeqCst); let error = runtime.apply_host_input(RuntimeHostInput::single(input_source.clone(), RuntimeHostInputValue::F64(9.0))).unwrap_err(); assert!(format!("{error:?}").contains("DeliberateHostCallError")); assert!(calls.load(Ordering::SeqCst) > calls_before); assert_eq!(f64_value(&source_value(&runtime, &input_source)), 9.0); let host_result = runtime.program.interpreter().symbols().borrow().get(hash_str("host-result")).unwrap(); match &*host_result.borrow() { Value::F64(value) => assert_eq!(value.as_ptr(), host_result_pointer), other => panic!("expected f64 host result, got {other:?}") }; assert_eq!(f64_value(&symbol_value(&runtime, "host-result")), 2.0); assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0); assert_eq!(plan_snapshot(&runtime), plan_before); assert_eq!(output.lines(), lines_before); assert_eq!(runtime.persistent_send_count(), 1); runtime.run_string("recovery := 1").unwrap(); assert_eq!(f64_value(&symbol_value(&runtime, "recovery")), 1.0);
}

#[test]
fn live_host_empty_output_can_recompute() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "test://clock/ticks", "value");
  let subject = runtime.runtime_context().unwrap().subject;
  grant_host_call(&mut runtime, &subject, 48, "host:demo/live-empty");
  let calls = Arc::new(AtomicUsize::new(0));
  let host_calls = calls.clone();
  runtime.register_mech_host_function(ClosureHostFunction::new("demo/live-empty", move |_services, _context, _args| {
    host_calls.fetch_add(1, Ordering::SeqCst);
    Ok(Value::Empty)
  })).unwrap();
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := demo/live-empty(@pulse/value)").unwrap();
  assert_eq!(symbol_value(&runtime, "output"), Value::Empty);

  let outcome = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(9.0))).unwrap();
  assert!(outcome.turn.is_some());
  assert!(calls.load(Ordering::SeqCst) >= 2);
  assert_eq!(symbol_value(&runtime, "output"), Value::Empty);
}

#[test]
fn runtime_reactive_host_input_preflight_failure_mutates_nothing() {
  let provider = TestResourceProvider::new().with_value(TEST_SIGNALS_BASE_URI, "a", Value::F64(Ref::new(1.0))).with_value(TEST_SIGNALS_BASE_URI, "b", Value::F64(Ref::new(2.0)));
  let (mut runtime, output) = test_runtime_with_output(provider); grant_read(&mut runtime, TEST_SIGNALS_BASE_URI, "a"); grant_read(&mut runtime, TEST_SIGNALS_BASE_URI, "b"); grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
  let mut context = runtime.runtime_context().unwrap(); runtime.run_string_with_context(&mut context, "@out := test://effects/output{:write(line)}\n@signals := test://signals/inputs{:read(a), :read(b)}\nsum := @signals/a + @signals/b\n@out/line <- sum").unwrap();
  let a_source = RuntimeHostInputSource::new(TEST_SIGNALS_BASE_URI, "a").unwrap(); let b_source = RuntimeHostInputSource::new(TEST_SIGNALS_BASE_URI, "b").unwrap(); let a_before = f64_value(&source_value(&runtime, &a_source)); let b_before = f64_value(&source_value(&runtime, &b_source)); let sum_before = f64_value(&symbol_value(&runtime, "sum")); let plan_before = plan_snapshot(&runtime); let lines_before = output.lines(); assert!(!runtime.program.interpreter().has_pending_reactive_registers());
  let error = runtime.apply_host_input(RuntimeHostInput::new(vec![RuntimeHostInputUpdate { source: a_source.clone(), value: RuntimeHostInputValue::F64(10.0) }, RuntimeHostInputUpdate { source: b_source.clone(), value: RuntimeHostInputValue::String("bad".to_string()) }]).unwrap()).unwrap_err();
  assert!(format!("{error:?}").contains("StableValueUpdateKindMismatch")); assert_eq!(f64_value(&source_value(&runtime, &a_source)), a_before); assert_eq!(f64_value(&source_value(&runtime, &b_source)), b_before); assert_eq!(f64_value(&symbol_value(&runtime, "sum")), sum_before); assert_eq!(plan_snapshot(&runtime), plan_before); assert!(!runtime.program.interpreter().has_pending_reactive_registers()); assert_eq!(output.lines(), lines_before);
}

#[test]
fn transactional_context_cannot_arm_live_program() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  let mut context = runtime.runtime_context().unwrap();
  grant_read_to(&mut runtime, &context.subject, "test://clock/ticks", "value");
  runtime.begin_transaction(&mut context).unwrap();
  let error = format!("{:?}", runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap_err());
  assert!(error.contains("RuntimeTransactionalLiveProgramUnsupported"), "{error}");
  assert!(runtime.live_context_template.is_none());
  assert!(runtime.live_input_bindings.is_empty());
  assert!(runtime.persistent_sends.is_empty());
}

#[test]
fn failed_source_does_not_leave_live_state_armed() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  let mut context_a = runtime.runtime_context().unwrap().with_subject("subject-a");
  grant_read_to(&mut runtime, "subject-a", "test://clock/ticks", "value");
  let error = runtime.run_string_with_context(&mut context_a, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value\nmissing := @pulse/missing");
  assert!(error.is_err());
  assert!(runtime.live_context_template.is_none());
  assert!(runtime.live_input_bindings.is_empty());
  assert!(runtime.persistent_sends.is_empty());

  let mut context_b = runtime.runtime_context().unwrap().with_subject("subject-b");
  grant_read_to(&mut runtime, "subject-b", "test://clock/ticks", "value");
  runtime.run_string_with_context(&mut context_b, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap();
}

#[test]
fn duration_failure_restores_live_state() {
  let mut config = RuntimeConfig::default();
  config.limits.max_turn_duration_ms = Some(1);
  let mut runtime = RuntimeBuilder::new()
    .config(config)
    .resource_provider(Box::new(test_provider_with("test://clock/ticks", "value", 1.0)))
    .build()
    .unwrap();
  let mut context = runtime.runtime_context().unwrap();
  grant_read_to(&mut runtime, &context.subject, "test://clock/ticks", "value");
  grant_host_call(&mut runtime, &context.subject, 43, "host:demo/source-sleep");
  register_sleep_host(&mut runtime, "demo/source-sleep");
  let error = format!("{:?}", runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\nslow := demo/source-sleep(@pulse/value)").unwrap_err());
  assert!(error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"), "{error}");
  assert!(runtime.live_context_template.is_none());
  assert!(runtime.live_input_bindings.is_empty());
  assert!(runtime.persistent_sends.is_empty());
  runtime.config.limits.max_turn_duration_ms = None;
  runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn live_turn_enforces_step_budget() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  let mut context = runtime.runtime_context().unwrap().with_budget(ResourceBudget::default().with_max_steps(1));
  grant_read_to(&mut runtime, &context.subject, "test://clock/ticks", "value");
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();
  runtime.live_context_template.as_mut().unwrap().budget_limits.max_steps = Some(0);
  let error = format!("{:?}", runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(2.0))).unwrap_err());
  assert!(error.contains("ResourceBudgetExceeded"), "{error}");

  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  let mut context = runtime.runtime_context().unwrap().with_budget(ResourceBudget::default().with_max_steps(1));
  grant_read_to(&mut runtime, &context.subject, "test://clock/ticks", "value");
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();
  let outcome = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(2.0))).unwrap();
  assert!(outcome.turn.is_some());
  let outcome = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(3.0))).unwrap();
  assert!(outcome.turn.is_some());
}

#[test]
fn live_turn_enforces_duration_limit() {
  let mut runtime = RuntimeBuilder::new()
    .resource_provider(Box::new(test_provider_with("test://clock/ticks", "value", 1.0)))
    .build()
    .unwrap();
  let mut context = runtime.runtime_context().unwrap();
  grant_read_to(&mut runtime, &context.subject, "test://clock/ticks", "value");
  grant_host_call(&mut runtime, &context.subject, 44, "host:demo/live-sleep");
  register_sleep_host(&mut runtime, "demo/live-sleep");
  runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\nslow := demo/live-sleep(@pulse/value)").unwrap();
  runtime.config.limits.max_turn_duration_ms = Some(1);
  let error = format!("{:?}", runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(2.0))).unwrap_err());
  assert!(error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"), "{error}");
  assert!(runtime.program().interpreter().symbols().borrow().contains(hash_str("slow")));
  runtime.config.limits.max_turn_duration_ms = None;
  runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn changed_actor_message_rejects_live_context_reuse() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  let actor = ActorId(7);
  let state = ObjectId(9);
  let mut context_a = runtime.runtime_context().unwrap().with_subject("actor:7").with_actor(actor);
  context_a.actor_state = Some(state);
  context_a.actor_message = Some(MessageRecord::new(MessageId(1), actor, "tick", b"a".to_vec()));
  grant_read_to(&mut runtime, "actor:7", "test://clock/ticks", "value");
  runtime.run_string_with_context(&mut context_a, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap();

  let mut context_b = runtime.runtime_context().unwrap().with_subject("actor:7").with_actor(actor);
  context_b.actor_state = Some(state);
  context_b.actor_message = Some(MessageRecord::new(MessageId(2), actor, "tick", b"b".to_vec()));
  let error = format!("{:?}", runtime.run_string_with_context(&mut context_b, "@pulse := test://clock/ticks{:read(value)}\nother := @pulse/value").unwrap_err());
  assert!(error.contains("RuntimeLiveContextMismatch"), "{error}");
}

#[test]
fn live_context_capability_order_does_not_cause_mismatch() {
  let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
  let mut context_a = runtime.runtime_context().unwrap().with_capabilities(vec![CapabilityId(1), CapabilityId(2)]);
  let mut context_b = runtime.runtime_context().unwrap().with_capabilities(vec![CapabilityId(2), CapabilityId(1)]);
  grant_read_to(&mut runtime, &context_a.subject, "test://clock/ticks", "value");
  runtime.run_string_with_context(&mut context_a, "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap();
  runtime.run_string_with_context(&mut context_b, "@pulse := test://clock/ticks{:read(value)}\nother := @pulse/value").unwrap();
}

#[derive(Debug, Clone)]
struct MockDriver {
  name: String,
  state: Rc<RefCell<MockDriverState>>,
  events: Rc<RefCell<Vec<String>>>,
}

const MOCK_DRIVER_BASE_URI: &str =
  "test-input://clock/ticks";

const MOCK_DRIVER_PATH: &str =
  "value";

#[derive(Debug, Default)]
struct MockDriverState {
  attach_count: usize,
  start_count: usize,
  stop_count: usize,
  live: bool,
  fail_attach: bool,
  fail_start: bool,
  fail_stop: bool,
  attached_ingress: Option<RuntimeIngress>,
  stop_observed_closed_ingress: bool,
  log: Vec<String>,
}

impl MockDriver {
  fn new(name: &str, state: Rc<RefCell<MockDriverState>>) -> Self {
    Self::with_events(name, state, Rc::new(RefCell::new(Vec::new())))
  }

  fn with_events(name: &str, state: Rc<RefCell<MockDriverState>>, events: Rc<RefCell<Vec<String>>>) -> Self {
    Self { name: name.to_string(), state, events }
  }
}

impl RuntimeHostInputDriver for MockDriver {
  fn drives(
    &self,
    source: &RuntimeHostInputSource,
  ) -> bool {
    source.base_uri() == MOCK_DRIVER_BASE_URI
      && source.path() == MOCK_DRIVER_PATH
  }

  fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
    let mut state = self.state.borrow_mut();
    state.attach_count += 1;
    state.attached_ingress = Some(ingress);
    let event = format!("attach:{}", self.name);
    state.log.push(event.clone());
    self.events.borrow_mut().push(event);
    if state.fail_attach { return Err(mock_error("MockAttachError", format!("attach failed for {}", self.name))); }
    Ok(())
  }

  fn start(&mut self) -> MResult<()> {
    let mut state = self.state.borrow_mut();
    state.start_count += 1;
    let event = format!("start:{}", self.name);
    state.log.push(event.clone());
    self.events.borrow_mut().push(event);
    if state.fail_start { return Err(mock_error("MockStartError", format!("start failed for {}", self.name))); }
    state.live = true;
    Ok(())
  }

  fn stop(&mut self) -> MResult<()> {
    let mut state = self.state.borrow_mut();
    state.stop_count += 1;
    let observed_closed = state.attached_ingress.as_ref().map(|ingress| ingress.is_closed().unwrap_or(false)).unwrap_or(false);
    state.stop_observed_closed_ingress |= observed_closed;
    let event = format!("stop:{}", self.name);
    state.log.push(event.clone());
    self.events.borrow_mut().push(event);
    state.live = false;
    if state.fail_stop { return Err(mock_error("MockStopError", format!("stop failed for {}", self.name))); }
    Ok(())
  }

  fn is_live(&self) -> bool {
    self.state.borrow().live
  }
}

#[derive(Debug)]
struct MockDriverFactory {
  manifest: HostManifestConfig,
  drivers: Vec<MockDriver>,
}

impl MockDriverFactory {
  fn new(drivers: Vec<MockDriver>) -> Self {
    Self {
      manifest: HostManifestConfig {
        provider: "test-input".to_string(),
        contexts: vec![HostContextManifest { name: "ticks".to_string(), base_uri_template: "test-input://{instance}/ticks".to_string(), operations: vec!["read".to_string()] }],
      },
      drivers,
    }
  }
}

impl RuntimeHostFactory for MockDriverFactory {
  fn provider_name(&self) -> &str { "test-input" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, _settings: &ConfigValue) -> MResult<()> { Ok(()) }
  fn instantiate(&self, instance_name: &str, _settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      resource_providers: Vec::<Box<dyn RuntimeResourceProvider>>::new(),
      input_drivers: self.drivers.iter().cloned().map(|driver| Box::new(driver) as Box<dyn RuntimeHostInputDriver>).collect(),
    })
  }
}

fn drivers_with_events(states: &[(&str, Rc<RefCell<MockDriverState>>)]) -> (Vec<MockDriver>, Rc<RefCell<Vec<String>>>) {
  let events = Rc::new(RefCell::new(Vec::new()));
  let drivers = states
    .iter()
    .map(|(name, state)| MockDriver::with_events(name, state.clone(), events.clone()))
    .collect();
  (drivers, events)
}

fn runtime_with_drivers(drivers: Vec<MockDriver>) -> MResult<MechRuntime> {
  RuntimeBuilder::new()
    .host_factory(Box::new(MockDriverFactory::new(drivers)))?
    .host_instance(HostInstanceConfig { name: "clock".to_string(), provider: "test-input".to_string(), settings: ConfigValue::Map(Default::default()) })
    .build()
}

#[test]
fn build_attaches_but_does_not_start_input_drivers() {
  let state = Rc::new(RefCell::new(MockDriverState::default()));
  let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
  assert_eq!(state.borrow().attach_count, 1);
  assert_eq!(state.borrow().start_count, 0);
  assert_eq!(state.borrow().stop_count, 0);
  assert!(!state.borrow().live);
  runtime.start_input_drivers().unwrap();
  assert_eq!(state.borrow().start_count, 1);
  assert!(state.borrow().live);
}

#[test]
fn attach_failure_closes_ingress_and_rolls_back_in_reverse_order() {
  let a = Rc::new(RefCell::new(MockDriverState::default()));
  let b = Rc::new(RefCell::new(MockDriverState { fail_attach: true, ..Default::default() }));
  let c = Rc::new(RefCell::new(MockDriverState::default()));
  let (drivers, events) = drivers_with_events(&[("a", a.clone()), ("b", b.clone()), ("c", c.clone())]);
  let error = format!("{:?}", runtime_with_drivers(drivers).unwrap_err());
  assert!(error.contains("MockAttachError"));
  assert_eq!(a.borrow().attach_count, 1);
  assert_eq!(b.borrow().attach_count, 1);
  assert_eq!(c.borrow().attach_count, 0);
  assert!(a.borrow().attached_ingress.as_ref().unwrap().is_closed().unwrap());
  let stop_events: Vec<String> = events.borrow().iter().filter(|event| event.starts_with("stop:")).cloned().collect();
  assert_eq!(stop_events, vec!["stop:b", "stop:a"]);
  assert_eq!(a.borrow().stop_count, 1);
  assert_eq!(b.borrow().stop_count, 1);
  assert_eq!(c.borrow().stop_count, 0);
}

#[test]
fn start_failure_stops_every_driver_in_reverse_order() {
  let a = Rc::new(RefCell::new(MockDriverState::default()));
  let b = Rc::new(RefCell::new(MockDriverState { fail_start: true, ..Default::default() }));
  let c = Rc::new(RefCell::new(MockDriverState::default()));
  let (drivers, events) = drivers_with_events(&[("a", a.clone()), ("b", b.clone()), ("c", c.clone())]);
  let mut runtime = runtime_with_drivers(drivers).unwrap();
  let error = format!("{:?}", runtime.start_input_drivers().unwrap_err());
  assert!(error.contains("MockStartError"));
  let stop_events: Vec<String> = events.borrow().iter().filter(|event| event.starts_with("stop:")).cloned().collect();
  assert_eq!(stop_events, vec!["stop:c", "stop:b", "stop:a"]);
  assert!(!a.borrow().live && !b.borrow().live && !c.borrow().live);
}

#[test]
fn stop_input_drivers_attempts_every_driver() {
  let a = Rc::new(RefCell::new(MockDriverState::default()));
  let b = Rc::new(RefCell::new(MockDriverState { fail_stop: true, ..Default::default() }));
  let c = Rc::new(RefCell::new(MockDriverState::default()));
  let (drivers, events) = drivers_with_events(&[("a", a.clone()), ("b", b.clone()), ("c", c.clone())]);
  let mut runtime = runtime_with_drivers(drivers).unwrap();
  runtime.start_input_drivers().unwrap();
  let error = format!("{:?}", runtime.stop_input_drivers().unwrap_err());
  assert!(error.contains("MockStopError"));
  assert_eq!(a.borrow().stop_count, 1);
  assert_eq!(b.borrow().stop_count, 1);
  assert_eq!(c.borrow().stop_count, 1);
  let stop_events: Vec<String> = events.borrow().iter().filter(|event| event.starts_with("stop:")).cloned().collect();
  assert_eq!(stop_events, vec!["stop:c", "stop:b", "stop:a"]);
}

#[test]
fn shutdown_closes_ingress_before_stopping_drivers() {
  let state = Rc::new(RefCell::new(MockDriverState::default()));
  let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
  runtime.start_input_drivers().unwrap();
  let ingress = state.borrow().attached_ingress.clone().unwrap();
  runtime.shutdown().unwrap();
  assert_eq!(state.borrow().stop_count, 1);
  assert!(state.borrow().stop_observed_closed_ingress);
  drop(runtime);
  assert_eq!(state.borrow().stop_count, 1);
  let source = RuntimeHostInputSource::new("test-input://clock/ticks", "value").unwrap();
  let error = format!("{:?}", ingress.submit(RuntimeHostInput::single(source, RuntimeHostInputValue::F64(1.0))).unwrap_err());
  assert!(error.contains("RuntimeIngressClosed"));
}

#[test]
fn drop_stops_live_input_drivers() {
  let state = Rc::new(RefCell::new(MockDriverState::default()));
  {
    let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
    runtime.start_input_drivers().unwrap();
    assert!(state.borrow().live);
  }
  assert_eq!(state.borrow().stop_count, 1);
  assert!(!state.borrow().live);
}


#[derive(Debug, Clone)]
struct MockDriverError { name: &'static str, message: String }
impl MechErrorKind for MockDriverError {
  fn name(&self) -> &str { self.name }
  fn message(&self) -> String { self.message.clone() }
}
fn mock_error(name: &'static str, message: impl Into<String>) -> MechError {
  MechError::new(MockDriverError { name, message: message.into() }, None)
}

#[cfg(test)]
mod persistent_send_tests {
  use super::*;
  use crate::RuntimeResourceWritePreflightRequest;

  const TEST_TIME_BASE_URI: &str =
    "time://clock/clock";

  const TEST_TIME_PATHS: [&str; 5] = [
    "unix-ms",
    "hour",
    "minute",
    "second",
    "millisecond",
  ];

  fn test_time_source_matches(
    source: &RuntimeHostInputSource,
  ) -> bool {
    source.base_uri() == TEST_TIME_BASE_URI
      && TEST_TIME_PATHS.contains(&source.path())
  }

  #[derive(Clone, Copy, Debug)]
  struct TimeSnapshot {
    unix_ms: f64,
    hour: f64,
    minute: f64,
    second: f64,
    millisecond: f64,
  }

  #[derive(Debug)]
  struct TimeResourceProvider {
    snapshot: Rc<RefCell<TimeSnapshot>>,
  }

  impl RuntimeResourceProvider for TimeResourceProvider {
    fn scheme(&self) -> &str { "time" }
    fn base_uris(&self) -> Vec<String> { vec![TEST_TIME_BASE_URI.to_string()] }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
      let snapshot = *self.snapshot.borrow();
      let value = match request.path.as_str() {
        "unix-ms" => snapshot.unix_ms,
        "hour" => snapshot.hour,
        "minute" => snapshot.minute,
        "second" => snapshot.second,
        "millisecond" => snapshot.millisecond,
        other => return Err(MechError::new(PersistentSendTestError(format!("unknown time path {other}")), None)),
      };
      Ok(Value::F64(Ref::new(value)))
    }
  }

  #[derive(Clone, Debug)]
  struct ManualTimeInputDriver {
    snapshot: Rc<RefCell<TimeSnapshot>>,
    ingress: Rc<RefCell<Option<RuntimeIngress>>>,
    live: Rc<RefCell<bool>>,
  }

  impl ManualTimeInputDriver {
    fn new(snapshot: Rc<RefCell<TimeSnapshot>>) -> Self {
      Self { snapshot, ingress: Rc::new(RefCell::new(None)), live: Rc::new(RefCell::new(false)) }
    }

    fn publish(&self, snapshot: TimeSnapshot) -> MResult<()> {
      *self.snapshot.borrow_mut() = snapshot;
      let ingress = self.ingress.borrow().clone().ok_or_else(|| MechError::new(PersistentSendTestError("driver is not attached".to_string()), None))?;
      ingress.submit(RuntimeHostInput::new(vec![
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "unix-ms")?, value: RuntimeHostInputValue::F64(snapshot.unix_ms) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "hour")?, value: RuntimeHostInputValue::F64(snapshot.hour) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "minute")?, value: RuntimeHostInputValue::F64(snapshot.minute) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "second")?, value: RuntimeHostInputValue::F64(snapshot.second) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "millisecond")?, value: RuntimeHostInputValue::F64(snapshot.millisecond) },
      ])?)
    }
  }

  impl RuntimeHostInputDriver for ManualTimeInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool { test_time_source_matches(source) }
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> { *self.ingress.borrow_mut() = Some(ingress); Ok(()) }
    fn start(&mut self) -> MResult<()> { *self.live.borrow_mut() = true; Ok(()) }
    fn stop(&mut self) -> MResult<()> { *self.live.borrow_mut() = false; Ok(()) }
    fn is_live(&self) -> bool { *self.live.borrow() }
  }

  #[derive(Clone, Debug, Default)]
  struct RecordingConsoleBackend {
    lines: Rc<RefCell<Vec<String>>>,
    fail_next: Rc<RefCell<Option<String>>>,
  }

  impl RecordingConsoleBackend {
    fn lines(&self) -> Vec<String> { self.lines.borrow().clone() }
    fn fail_next(&self, reason: impl Into<String>) { *self.fail_next.borrow_mut() = Some(reason.into()); }
  }

  #[derive(Debug)]
  struct ConsoleResourceProvider { backend: RecordingConsoleBackend }

  impl RuntimeResourceProvider for ConsoleResourceProvider {
    fn scheme(&self) -> &str { "console" }
    fn base_uris(&self) -> Vec<String> { vec!["console://console/output".to_string()] }
    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> { Err(MechError::new(PersistentSendTestError("console is write-only".to_string()), None)) }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
      if request.path == "line" && request.intent == RuntimeResourceWriteIntent::Send { Ok(()) } else { Err(MechError::new(PersistentSendTestError("bad console write".to_string()), None)) }
    }
    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
      self.preflight_write(RuntimeResourceWritePreflightRequest {
        base_uri: request.base_uri,
        path: request.path,
        context_name: request.context_name,
        operation: request.operation,
        intent: request.intent,
      })?;
      if let Some(reason) = self.backend.fail_next.borrow_mut().take() {
        return Err(MechError::new(PersistentSendTestError(reason), None));
      }
      self.backend.lines.borrow_mut().push(format!("{}", request.value));
      Ok(())
    }
  }

  #[derive(Debug, Clone)]
  struct PersistentSendTestError(String);

  impl MechErrorKind for PersistentSendTestError {
    fn name(&self) -> &str { "PersistentSendTestError" }
    fn message(&self) -> String { self.0.clone() }
  }

  const TIME_PATHS: &[&str] = &["unix-ms", "hour", "minute", "second", "millisecond"];

  fn snapshot(hour: f64, minute: f64, second: f64, millisecond: f64) -> TimeSnapshot {
    TimeSnapshot { unix_ms: hour * 3_600_000.0 + minute * 60_000.0 + second * 1000.0 + millisecond, hour, minute, second, millisecond }
  }

  fn grant(runtime: &mut MechRuntime, resource: &str, operation: RuntimeCapabilityOperation, paths: &[&str]) {
    let subject = runtime.runtime_context().unwrap().subject;
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject,
      resource: resource.to_string(),
      operations: vec![operation],
      paths: paths.iter().map(|path| path.to_string()).collect(),
    }).unwrap();
  }

  fn runtime_with_console(initial: TimeSnapshot, fail_next: bool) -> (MechRuntime, ManualTimeInputDriver, RecordingConsoleBackend) {
    let shared = Rc::new(RefCell::new(initial));
    let console = RecordingConsoleBackend::default();
    if fail_next { console.fail_next("intentional console failure"); }
    let mut runtime = RuntimeBuilder::new()
      .resource_provider(Box::new(TimeResourceProvider { snapshot: shared.clone() }) as Box<dyn RuntimeResourceProvider>)
      .resource_provider(Box::new(ConsoleResourceProvider { backend: console.clone() }) as Box<dyn RuntimeResourceProvider>)
      .build()
      .unwrap();
    grant(&mut runtime, "time://clock/clock", RuntimeCapabilityOperation::Read, TIME_PATHS);
    grant(&mut runtime, "console://console/output", RuntimeCapabilityOperation::Write, &["line"]);
    let mut driver = ManualTimeInputDriver::new(shared);
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    (runtime, driver, console)
  }

  fn load(runtime: &mut MechRuntime, send_expression: &str) {
    let source = format!(r#"@out := console://console/output{{:write(line)}}
@clock := time://clock/clock{{:read(unix-ms), :read(hour), :read(minute), :read(second), :read(millisecond)}}
unix-ms := @clock/unix-ms
hour := @clock/hour
minute := @clock/minute
second := @clock/second
millisecond := @clock/millisecond
scalar-output := hour + minute
clock-output := (hour, minute, second)
@out/line <- {send_expression}
"#);
    runtime.run_string(&source).unwrap();
  }

  fn publish(runtime: &mut MechRuntime, driver: &ManualTimeInputDriver, snapshot: TimeSnapshot) {
    driver.publish(snapshot).unwrap();
    let outcomes = runtime.drain_host_inputs(1).unwrap();
    assert_eq!(outcomes.len(), 1);
  }


  #[test]
  fn persistent_send_uses_original_custom_subject() {
    let initial = snapshot(1.0, 2.0, 3.0, 4.0);
    let shared = Rc::new(RefCell::new(initial));
    let console = RecordingConsoleBackend::default();
    let mut runtime = RuntimeBuilder::new()
      .resource_provider(Box::new(TimeResourceProvider { snapshot: shared.clone() }) as Box<dyn RuntimeResourceProvider>)
      .resource_provider(Box::new(ConsoleResourceProvider { backend: console.clone() }) as Box<dyn RuntimeResourceProvider>)
      .build()
      .unwrap();
    let subject = "task:live-custom";
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject: subject.to_string(),
      resource: "time://clock/clock".to_string(),
      operations: vec![RuntimeCapabilityOperation::Read],
      paths: TIME_PATHS.iter().map(|path| path.to_string()).collect(),
    }).unwrap();
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject: subject.to_string(),
      resource: "console://console/output".to_string(),
      operations: vec![RuntimeCapabilityOperation::Write],
      paths: vec!["line".to_string()],
    }).unwrap();
    assert!(!runtime.has_capability_grant(&runtime.runtime_context().unwrap().subject, "console://console/output", &RuntimeCapabilityOperation::Write, "line"));

    let mut context = runtime.runtime_context().unwrap().with_subject(subject);
    runtime.run_string_with_context(&mut context, r#"@out := console://console/output{:write(line)}
@clock := time://clock/clock{:read(hour)}
hour := @clock/hour
output := hour + 1
@out/line <- output
"#).unwrap();
    assert_eq!(console.lines().len(), 1);
    *shared.borrow_mut() = snapshot(1.0, 9.0, 3.0, 4.0);
    let outcome = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("time://clock/clock", "hour").unwrap(), RuntimeHostInputValue::F64(9.0))).unwrap();
    assert!(outcome.turn.is_some());
    let lines = console.lines();
    assert_eq!(lines.len(), 2);
    assert!(lines.last().unwrap().contains("10"), "{lines:?}");
  }

  #[test]
  fn persistent_send_initial_evaluation_sends_once() {
    let (mut runtime, _driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "scalar-output");
    assert_eq!(console.lines().len(), 1);
    assert_eq!(runtime.persistent_send_count(), 1);
  }

  #[test]
  fn persistent_send_one_packet_sends_once_more_with_changed_value() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "scalar-output");
    let initial = console.lines();
    publish(&mut runtime, &driver, snapshot(5.0, 6.0, 7.0, 8.0));
    let lines = console.lines();
    assert_eq!(lines.len(), 2);
    assert_ne!(lines[1], initial[0]);
  }

  #[test]
  fn persistent_send_two_packets_produce_two_additional_values_in_order() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 1.0, 1.0, 0.0), false);
    load(&mut runtime, "scalar-output");
    publish(&mut runtime, &driver, snapshot(2.0, 3.0, 0.0, 0.0));
    publish(&mut runtime, &driver, snapshot(4.0, 5.0, 0.0, 0.0));
    let lines = console.lines();
    assert_eq!(lines.len(), 3);
    assert_ne!(lines[1], lines[2]);
  }

  #[test]
  fn persistent_send_logical_packet_with_five_fields_sends_once() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "clock-output");
    publish(&mut runtime, &driver, snapshot(5.0, 6.0, 7.0, 8.0));
    assert_eq!(console.lines().len(), 2);
  }

  #[test]
  fn persistent_send_scalar_reads_new_value_after_solve() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "scalar-output");
    publish(&mut runtime, &driver, snapshot(10.0, 20.0, 0.0, 0.0));
    let lines = console.lines();
    assert!(lines[1].contains("30"), "expected updated scalar in {:?}", lines);
  }

  #[test]
  fn persistent_send_tuple_reads_new_values_after_solve() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "clock-output");
    publish(&mut runtime, &driver, snapshot(10.0, 20.0, 30.0, 40.0));
    let lines = console.lines();
    assert!(lines[1].contains("10"), "expected updated tuple in {:?}", lines);
    assert!(lines[1].contains("20"), "expected updated tuple in {:?}", lines);
    assert!(lines[1].contains("30"), "expected updated tuple in {:?}", lines);
  }

  #[test]
  fn persistent_send_provider_failure_returns_from_drain() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "scalar-output");
    console.fail_next("expected drain failure");
    driver.publish(snapshot(5.0, 6.0, 7.0, 8.0)).unwrap();
    let err = runtime.drain_host_inputs(1).unwrap_err();
    assert!(format!("{err:?}").contains("expected drain failure"));
  }

  #[test]
  fn persistent_send_replay_does_not_register_another_send() {
    let (mut runtime, driver, _console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "scalar-output");
    assert_eq!(runtime.persistent_send_count(), 1);
    publish(&mut runtime, &driver, snapshot(5.0, 6.0, 7.0, 8.0));
    assert_eq!(runtime.persistent_send_count(), 1);
  }
}

#[test]
fn runtime_reactive_host_input_preserves_deferred_registers_across_packets() {
  let mut runtime = test_runtime(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0)); grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
  let mut context = runtime.runtime_context().unwrap(); runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\n~a := 0.0\n~b := 0.0\na = @pulse/value\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0").unwrap();
  let a = register_node_for_symbol(&runtime, "a"); let b = register_node_for_symbol(&runtime, "b"); let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
  assert_eq!(f64_value(&symbol_value(&runtime, "a")), 1.0); assert_eq!(f64_value(&symbol_value(&runtime, "middle")), 2.0); assert_eq!(f64_value(&symbol_value(&runtime, "b")), 2.0); assert_eq!(f64_value(&symbol_value(&runtime, "output")), 3.0);
  let first = runtime.apply_host_input(RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(10.0))).unwrap(); let turn = &first.turn.as_ref().unwrap().interpreter_turns[0].turn; assert_eq!(turn.register_commit.committed_nodes, vec![a]); assert_eq!(turn.after_commit.pending_register_nodes, vec![b]); assert_eq!((f64_value(&symbol_value(&runtime, "a")), f64_value(&symbol_value(&runtime, "middle")), f64_value(&symbol_value(&runtime, "b")), f64_value(&symbol_value(&runtime, "output"))), (10.0, 11.0, 2.0, 3.0)); assert!(runtime.program.interpreter().has_pending_reactive_registers());
  let second = runtime.apply_host_input(RuntimeHostInput::single(source, RuntimeHostInputValue::F64(20.0))).unwrap(); let turn = &second.turn.as_ref().unwrap().interpreter_turns[0].turn; assert_eq!(turn.before_commit.pending_register_nodes, vec![a]); assert_eq!(turn.register_commit.committed_nodes, vec![a, b]); assert_eq!(turn.after_commit.pending_register_nodes, vec![b]); assert_eq!((f64_value(&symbol_value(&runtime, "a")), f64_value(&symbol_value(&runtime, "middle")), f64_value(&symbol_value(&runtime, "b")), f64_value(&symbol_value(&runtime, "output"))), (20.0, 21.0, 11.0, 12.0)); assert!(runtime.program.interpreter().has_pending_reactive_registers());
}
