use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::thread;
use std::time::Duration;

use mech_core::{hash_str, MResult, MechError, MechErrorKind, ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, ReactiveTurnOutcome, Ref, Value};

use super::*;
use crate::{
  BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId,
  ConfigValue, HostContextManifest, HostInstanceConfig, HostManifestConfig,
  RuntimeCapabilityGrant, RuntimeHostFactory, RuntimeHostInstallation, RuntimeHostInput,
  RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue, RuntimeHostInputOutcome, RuntimeIngress,
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

fn grant_read_to(
  runtime: &mut MechRuntime,
  subject: &str,
  resource: &str,
  path: &str,
) {
  runtime
    .grant_capability(
      RuntimeCapabilityGrant {
        subject: subject.to_string(),
        resource: resource.to_string(),
        operations: vec![
          RuntimeCapabilityOperation::Read,
        ],
        paths: vec![path.to_string()],
      },
    )
    .unwrap();
}

fn grant_host_call(
  runtime: &mut MechRuntime,
  subject: &str,
  id: u64,
  resource: &str,
) {
  runtime
    .grant_capability(
      Arc::new(
        BasicCapability::new(
          CapabilityId(id.into()),
          &BasicSubject::new(subject),
          &BasicResource::new(resource),
          [
            BasicOperation::new("call"),
          ],
        ),
      ),
    )
    .unwrap();
}

fn register_sleep_host(
  runtime: &mut MechRuntime,
  name: &str,
) {
  runtime
    .register_mech_host_function(
      ClosureHostFunction::new(
        name,
        move |_services, _context, args| {
          thread::sleep(
            Duration::from_millis(5),
          );

          match args.first() {
            Some(Value::F64(value)) =>
              Ok(Value::F64(
                Ref::new(*value.borrow()),
              )),

            Some(Value::MutableReference(value)) =>
              match &*value.borrow() {
                Value::F64(value) =>
                  Ok(Value::F64(
                    Ref::new(*value.borrow()),
                  )),

                other =>
                  panic!(
                    "expected f64 mutable reference, got {other:?}",
                  ),
              },

            other =>
              panic!(
                "expected f64 argument, got {other:?}",
              ),
          }
        },
      ),
    )
    .unwrap();
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
fn patterned_activation_guard_rejects_runtime_host_before_elaboration() {
  let mut runtime = test_runtime(TestResourceProvider::new());
  let subject = runtime.runtime_context().unwrap().subject;
  grant_host_call(&mut runtime, &subject, 43, "host:demo/audit");
  let calls = Arc::new(AtomicUsize::new(0));
  let host_calls = calls.clone();
  runtime
    .register_mech_host_function(ClosureHostFunction::new(
      "demo/audit",
      move |_services, _context, _args| {
        host_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Bool(Ref::new(true)))
      },
    ))
    .unwrap();
  runtime
    .run_string("event := (:released, 1.0)\nsentinel := 7.0")
    .unwrap();
  let plan_before = plan_snapshot(&runtime);
  let symbols_before = runtime.program.interpreter().symbols().borrow().snapshot();
  let registrations_before = runtime
    .program
    .interpreter()
    .plan()
    .pattern_activation_registrations()
    .to_vec();

  let error = runtime
    .run_string(
      r#"
~> event
  | :pressed(x), demo/audit(x) => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
    )
    .unwrap_err();

  assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
  assert_eq!(calls.load(Ordering::SeqCst), 0);
  assert_eq!(plan_snapshot(&runtime), plan_before);
  assert_eq!(
    runtime.program.interpreter().symbols().borrow().snapshot(),
    symbols_before,
  );
  let registrations_after = runtime
    .program
    .interpreter()
    .plan()
    .pattern_activation_registrations()
    .to_vec();
  assert_eq!(registrations_after, registrations_before);
  assert!(!runtime
    .program
    .interpreter()
    .plan()
    .activation_registration_active());
  assert!(!runtime.program.interpreter().has_pending_reactive_registers());
  assert!(runtime
    .program
    .interpreter()
    .symbols()
    .borrow()
    .get(hash_str("selected"))
    .is_none());
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

fn bind_single_mock_input(runtime: &mut MechRuntime) {
  runtime
    .live_input_bindings
    .insert(
      RuntimeHostInputSource::new(MOCK_DRIVER_BASE_URI, MOCK_DRIVER_PATH).unwrap(),
      vec![mech_program::ProgramInputId { interpreter_id: 1, symbol_id: 1 }],
    );
}

#[test]
fn build_attaches_and_starts_driven_input_drivers() {
  let state = Rc::new(RefCell::new(MockDriverState::default()));
  let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
  bind_single_mock_input(&mut runtime);
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
  bind_single_mock_input(&mut runtime);
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
  bind_single_mock_input(&mut runtime);
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
  bind_single_mock_input(&mut runtime);
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
    bind_single_mock_input(&mut runtime);
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
  use crate::runtime::execution::ACTIVATION_EFFECT_BARRIER_NAME;
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

    fn grant_write_to(runtime: &mut MechRuntime, subject: &str, resource: &str, path: &str) {
        runtime
            .grant_capability(RuntimeCapabilityGrant {
                subject: subject.to_string(),
                resource: resource.to_string(),
                operations: vec![RuntimeCapabilityOperation::Write],
                paths: vec![path.to_string()],
            })
            .unwrap();
    }

    #[derive(Clone, Debug, Default)]
    struct SequencedOutput {
        attempts: Rc<RefCell<Vec<String>>>,
        successes: Rc<RefCell<Vec<String>>>,
        fail_once_at: Rc<RefCell<Option<usize>>>,
    }

    impl SequencedOutput {
        fn attempts(&self) -> Vec<String> {
            self.attempts.borrow().clone()
        }
        fn successes(&self) -> Vec<String> {
            self.successes.borrow().clone()
        }
        fn fail_once_at(&self, attempt: usize) {
            assert!(attempt > 0);
            *self.fail_once_at.borrow_mut() = Some(attempt);
        }
    }

    #[derive(Debug)]
    struct SequencedOutputProvider {
        backend: SequencedOutput,
    }

    impl RuntimeResourceProvider for SequencedOutputProvider {
        fn scheme(&self) -> &str {
            "test"
        }
        fn base_uris(&self) -> Vec<String> {
            vec![TEST_OUTPUT_BASE_URI.to_string()]
        }
        fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
            Err(MechError::new(
                PersistentSendTestError(format!(
                    "sequenced output is write-only: {} / {}",
                    request.base_uri, request.path,
                )),
                None,
            ))
        }
        fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
            if request.base_uri == TEST_OUTPUT_BASE_URI
                && request.path == "line"
                && request.intent == RuntimeResourceWriteIntent::Send
            {
                Ok(())
            } else {
                Err(MechError::new(
                    PersistentSendTestError(format!(
                        "invalid sequenced output write: {} / {}",
                        request.base_uri, request.path,
                    )),
                    None,
                ))
            }
        }
        fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
            self.preflight_write(RuntimeResourceWritePreflightRequest {
                base_uri: request.base_uri.clone(),
                path: request.path.clone(),
                context_name: request.context_name.clone(),
                operation: request.operation.clone(),
                intent: request.intent,
            })?;
            let rendered = format!("{}", request.value);
            let attempt_number = {
                let mut attempts = self.backend.attempts.borrow_mut();
                attempts.push(rendered.clone());
                attempts.len()
            };
            let should_fail = {
                let mut fail_once_at = self.backend.fail_once_at.borrow_mut();
                if *fail_once_at == Some(attempt_number) {
                    *fail_once_at = None;
                    true
                } else {
                    false
                }
            };
            if should_fail {
                return Err(MechError::new(
                    PersistentSendTestError(format!(
                        "intentional output failure on attempt {attempt_number}"
                    )),
                    None,
                ));
            }
            self.backend.successes.borrow_mut().push(rendered);
            Ok(())
        }
    }


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


  #[derive(Clone, Debug, PartialEq, Eq)]
  struct ActivationPlanSnapshot { nodes: Vec<ActivationNodeSnapshot>, reactive_consumers: Vec<(ReactiveCellId, Vec<ReactiveNodeId>)>, sampled_consumers: Vec<(ReactiveCellId, Vec<ReactiveNodeId>)> }
  #[derive(Clone, Debug, PartialEq, Eq)]
  struct ActivationNodeSnapshot { id: ReactiveNodeId, plan_index: usize, kind: ReactiveNodeKind, inputs: Vec<(ReactiveCellId, ReactiveDependencyKind)>, outputs: Vec<ReactiveCellId> }
  fn activation_plan_snapshot(runtime: &MechRuntime) -> ActivationPlanSnapshot { let plan=runtime.program.interpreter().plan(); let plan=plan.borrow(); let nodes=plan.nodes.iter().map(|n| ActivationNodeSnapshot{id:n.id,plan_index:n.plan_index,kind:n.kind,inputs:n.inputs.iter().map(|d|(d.cell,d.kind)).collect(),outputs:n.outputs.clone()}).collect(); let mut reactive_consumers=plan.reactive_consumers.iter().map(|(c,n)|(*c,n.clone())).collect::<Vec<_>>();let mut sampled_consumers=plan.sampled_consumers.iter().map(|(c,n)|(*c,n.clone())).collect::<Vec<_>>();reactive_consumers.sort_by_key(|(c,_)|c.get());sampled_consumers.sort_by_key(|(c,_)|c.get());ActivationPlanSnapshot{nodes,reactive_consumers,sampled_consumers} }
  fn activation_nodes_for_trigger(runtime:&MechRuntime, trigger_name:&str, kind:ReactiveNodeKind)->Vec<ReactiveNodeId>{let c=symbol_cell(runtime,trigger_name);let p=runtime.program.interpreter().plan();p.borrow().nodes.iter().filter(|n|n.kind==kind&&n.inputs.iter().any(|d|d.cell==c&&d.kind==ReactiveDependencyKind::Reactive)).map(|n|n.id).collect()}
  fn activation_barrier_for_trigger(runtime:&MechRuntime, trigger_name:&str)->ReactiveNodeId {let c=symbol_cell(runtime,trigger_name);let p=runtime.program.interpreter().plan();let barriers=p.borrow().nodes.iter().filter(|n|n.kind==ReactiveNodeKind::Combinational&&n.function.to_string()==ACTIVATION_EFFECT_BARRIER_NAME&&n.outputs.is_empty()&&n.inputs.iter().any(|d|d.cell==c&&d.kind==ReactiveDependencyKind::Reactive)).map(|n|n.id).collect::<Vec<_>>();assert_eq!(barriers.len(),1,"expected exactly one activation-effect barrier for trigger {trigger_name}");barriers[0]}
  fn only_reactive_turn(outcome:&RuntimeHostInputOutcome)->&ReactiveTurnOutcome{let p=outcome.turn.as_ref().expect("expected a program input turn");assert_eq!(p.interpreter_turns.len(),1,"expected exactly one affected interpreter");&p.interpreter_turns[0].turn}
  fn executed_count(turn:&ReactiveTurnOutcome,id:ReactiveNodeId)->usize{turn.before_commit.executed_nodes.iter().chain(turn.after_commit.executed_nodes.iter()).filter(|x|**x==id).count()}
  fn apply_f64_input(r:&mut MechRuntime,b:&str,p:&str,v:f64)->RuntimeHostInputOutcome{r.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new(b,p).unwrap(),RuntimeHostInputValue::F64(v))).unwrap()}
  fn recorded_f64(o:&RecordingTestOutput,i:usize)->f64{o.lines()[i].trim().parse().unwrap()}
  fn activation_send_count(r:&MechRuntime)->usize{r.persistent_sends.iter().filter(|s|matches!(s.schedule,RuntimePersistentSendSchedule::Activation{..})).count()}
  fn activation_rejection_runtime()->(MechRuntime,RecordingTestOutput){let (mut r,o)=test_runtime_with_output(TestResourceProvider::new().with_value("test://render/timer","tick",Value::F64(Ref::new(0.0))));grant_read(&mut r,"test://render/timer","tick");grant_write(&mut r,TEST_OUTPUT_BASE_URI,"line");(r,o)}
  fn assert_rejected_activation_left_no_state(r:&MechRuntime,o:&RecordingTestOutput,p:&ActivationPlanSnapshot,s:usize,b:&HashMap<RuntimeHostInputSource,Vec<mech_program::ProgramInputId>>){assert_eq!(activation_plan_snapshot(r),*p);assert_eq!(r.persistent_sends.len(),s);assert_eq!(r.live_input_bindings,*b);assert!(o.lines().is_empty());assert!(!r.program.interpreter().plan().activation_registration_active());}

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

  #[test]
  fn activation_send_registers_one_barrier_per_scope_and_replays_equal_triggers() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    runtime.run_string(r#"@out := console://console/output{:write(line)}
@clock := time://clock/clock{:read(hour)}
render-tick := @clock/hour
~> render-tick {
  @out/line <- "first"
  @out/line <- "second"
  @out/line <- "third"
}
"#).unwrap();

    // Activation effects are registered, rather than evaluated, during load.
    assert!(console.lines().is_empty());
    assert_eq!(runtime.persistent_send_count(), 3);
    let schedules: Vec<_> = runtime.persistent_sends.iter().map(|send| match send.schedule {
      RuntimePersistentSendSchedule::Activation { barrier_node_id, .. } => barrier_node_id,
      RuntimePersistentSendSchedule::EveryAcceptedTurn => panic!("activation send used top-level schedule"),
    }).collect();
    assert!(schedules.windows(2).all(|ids| ids[0] == ids[1]));
    let barriers = runtime.program.interpreter().plan().borrow().nodes.iter().filter(|node| {
      node.kind == mech_core::ReactiveNodeKind::Combinational
        && node.function.to_string() == ACTIVATION_EFFECT_BARRIER_NAME
        && node.outputs.is_empty()
    }).count();
    assert_eq!(barriers, 1);

    // Equal admitted values still execute the barrier and replay every send.
    publish(&mut runtime, &driver, snapshot(5.0, 2.0, 3.0, 4.0));
    publish(&mut runtime, &driver, snapshot(5.0, 2.0, 3.0, 4.0));
    assert_eq!(console.lines(), vec!["\"first\"", "\"second\"", "\"third\"", "\"first\"", "\"second\"", "\"third\""]);
  }

  #[test]
  fn activation_send_snapshots_fixed_payloads_before_same_trigger_register_commit() {
    let provider = TestResourceProvider::new().with_value(
      "test://render/timer",
      "tick",
      Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~state := 0.0

~> render-tick {
  state = state + 1.0
}

~> render-tick {
  @out/line <- state
  @out/line <- state + 0.0
}
"#).unwrap();

    let plan = activation_plan_snapshot(&runtime);
    let register = register_node_for_symbol(&runtime, "state");
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 2);

    let first = apply_f64_input(
      &mut runtime,
      "test://render/timer",
      "tick",
      1.0,
    );
    assert_eq!(
      only_reactive_turn(&first).register_commit.committed_nodes,
      vec![register]
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 1.0);
    assert_eq!(output.lines(), vec!["0", "0"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let equal = apply_f64_input(
      &mut runtime,
      "test://render/timer",
      "tick",
      1.0,
    );
    assert_eq!(
      only_reactive_turn(&equal).register_commit.committed_nodes,
      vec![register]
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 2.0);
    assert_eq!(output.lines(), vec!["0", "0", "1", "1"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);
  }

  #[test]
  fn activation_send_registration_is_atomic_on_fixed_elaboration_failure() {
    let (mut runtime, output) = activation_rejection_runtime();
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
render-tick := @tick/tick
"#).unwrap();
    let plan = activation_plan_snapshot(&runtime);
    let sends = runtime.persistent_sends.len();
    let bindings = runtime.live_input_bindings.clone();

    let error = runtime.run_string(r#"@out := test://effects/output{:write(line)}
~> render-tick {
  @out/line <- render-tick
  registered-first := render-tick + 1.0
  failure :=
    function-that-does-not-exist(registered-first)
}
"#).unwrap_err();

    assert!(
      error.kind_name().contains("Function"),
      "unexpected elaboration error: {error:?}"
    );
    assert_rejected_activation_left_no_state(
      &runtime,
      &output,
      &plan,
      sends,
      &bindings,
    );
    assert!(
      !runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .contains(hash_str("registered-first"))
    );
    assert!(
      !runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .contains(hash_str("failure"))
    );
  }

  #[test]
  fn activation_send_duration_failure_does_not_release_fixed_or_patterned_effects() {
    let provider = TestResourceProvider::new().with_value(
      "test://render/timer",
      "tick",
      Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    let subject = runtime.runtime_context().unwrap().subject;
    grant_host_call(
      &mut runtime,
      &subject,
      195,
      "host:demo/activation-duration-sleep",
    );
    register_sleep_host(
      &mut runtime,
      "demo/activation-duration-sleep",
    );
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick

~> render-tick {
  @out/line <-
    demo/activation-duration-sleep(render-tick)
}

~> render-tick
  | selected => {
      @out/line <- selected + 10.0
    }
"#).unwrap();

    let plan = activation_plan_snapshot(&runtime);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 2);

    runtime.config.limits.max_turn_duration_ms = Some(1);
    let error = runtime.apply_host_input(RuntimeHostInput::single(
      RuntimeHostInputSource::new(
        "test://render/timer",
        "tick",
      ).unwrap(),
      RuntimeHostInputValue::F64(1.0),
    )).unwrap_err();
    let error = format!("{error:?}");
    assert!(
      error.contains("turn_duration_ms")
        || error.contains("ResourceBudgetExceeded"),
      "{error}"
    );
    assert!(output.lines().is_empty());
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);

    runtime.config.limits.max_turn_duration_ms = None;
    let retry = apply_f64_input(
      &mut runtime,
      "test://render/timer",
      "tick",
      1.0,
    );
    assert!(retry.turn.is_some());
    assert_eq!(output.lines(), vec!["1", "11"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);
  }

  #[test]
  fn patterned_activation_sends_only_from_the_selected_arm() {
    let provider=TestResourceProvider::new().with_value("test://render/timer","tick",Value::F64(Ref::new(0.0)));let(mut runtime,output)=test_runtime_with_output(provider);grant_read(&mut runtime,"test://render/timer","tick");grant_write(&mut runtime,TEST_OUTPUT_BASE_URI,"line");
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick
  | 99.0 => {
      @out/line <- 99.0
    }
  | selected, selected > 0.0 => {
      @out/line <- selected
    }
  | * => {
      @out/line <- -1.0
    }
"#).unwrap();
    assert!(output.lines().is_empty(),"patterned effects ran during load");assert_eq!(activation_send_count(&runtime),3);
    let barriers=runtime.persistent_sends.iter().map(|send|match send.schedule{RuntimePersistentSendSchedule::Activation{barrier_node_id,..}=>barrier_node_id,RuntimePersistentSendSchedule::EveryAcceptedTurn=>panic!("patterned activation send used top-level schedule")}).collect::<Vec<_>>();
    assert_eq!(barriers.iter().copied().collect::<HashSet<_>>().len(),3,"each effectful arm must own a distinct barrier");
    let plan=activation_plan_snapshot(&runtime);
    let first=apply_f64_input(&mut runtime,"test://render/timer","tick",5.0);let turn=only_reactive_turn(&first);assert_eq!(output.lines(),vec!["5"]);assert_eq!((executed_count(turn,barriers[0]),executed_count(turn,barriers[1]),executed_count(turn,barriers[2])),(0,1,0));assert_eq!(activation_plan_snapshot(&runtime),plan);
    let equal=apply_f64_input(&mut runtime,"test://render/timer","tick",5.0);let turn=only_reactive_turn(&equal);assert_eq!(output.lines(),vec!["5","5"]);assert_eq!((executed_count(turn,barriers[0]),executed_count(turn,barriers[1]),executed_count(turn,barriers[2])),(0,1,0));assert_eq!(activation_plan_snapshot(&runtime),plan);
    let fallback=apply_f64_input(&mut runtime,"test://render/timer","tick",-5.0);let turn=only_reactive_turn(&fallback);assert_eq!(output.lines(),vec!["5","5","-1"]);assert_eq!((executed_count(turn,barriers[0]),executed_count(turn,barriers[1]),executed_count(turn,barriers[2])),(0,0,1));assert_eq!(activation_plan_snapshot(&runtime),plan);assert_eq!(activation_send_count(&runtime),3);
  }

  #[test]
  fn patterned_activation_samples_outer_effect_values_only_on_its_trigger() {
    let provider = TestResourceProvider::new()
      .with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
      )
      .with_value(
        TEST_SIGNALS_BASE_URI,
        "value",
        Value::F64(Ref::new(1.0)),
      );
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_read(&mut runtime, TEST_SIGNALS_BASE_URI, "value");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@signals := test://signals/inputs{:read(value)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
scene := @signals/value
~> render-tick
  | *, scene > 0.0 => {
      @out/line <- scene
    }
  | * => {}
"#).unwrap();

    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let barrier = runtime
      .persistent_sends
      .iter()
      .find_map(|send| match send.schedule {
        RuntimePersistentSendSchedule::Activation { barrier_node_id, .. } => {
          Some(barrier_node_id)
        }
        RuntimePersistentSendSchedule::EveryAcceptedTurn => None,
      })
      .unwrap();
    let plan = activation_plan_snapshot(&runtime);

    let scene_only = apply_f64_input(
      &mut runtime,
      TEST_SIGNALS_BASE_URI,
      "value",
      -1.0,
    );
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&scene_only), barrier), 0);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let guard_false = apply_f64_input(
      &mut runtime,
      "test://render/timer",
      "tick",
      1.0,
    );
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&guard_false), barrier), 0);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let scene_only = apply_f64_input(
      &mut runtime,
      TEST_SIGNALS_BASE_URI,
      "value",
      10.0,
    );
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&scene_only), barrier), 0);

    let render = apply_f64_input(
      &mut runtime,
      "test://render/timer",
      "tick",
      1.0,
    );
    assert_eq!(output.lines(), vec!["10"]);
    assert_eq!(executed_count(only_reactive_turn(&render), barrier), 1);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let scene_only = apply_f64_input(
      &mut runtime,
      TEST_SIGNALS_BASE_URI,
      "value",
      20.0,
    );
    assert_eq!(output.lines(), vec!["10"]);
    assert_eq!(executed_count(only_reactive_turn(&scene_only), barrier), 0);

    let equal_render = apply_f64_input(
      &mut runtime,
      "test://render/timer",
      "tick",
      1.0,
    );
    assert_eq!(output.lines(), vec!["10", "20"]);
    assert_eq!(executed_count(only_reactive_turn(&equal_render), barrier), 1);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 1);
  }

  #[test]
  fn patterned_activation_send_registration_is_atomic_on_elaboration_failure() {
    let(mut runtime,output)=activation_rejection_runtime();
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
render-tick := @tick/tick
"#).unwrap();
    let plan=activation_plan_snapshot(&runtime);let sends=runtime.persistent_sends.len();let bindings=runtime.live_input_bindings.clone();
    let error=runtime.run_string(r#"@out := test://effects/output{:write(line)}
~> render-tick
  | selected => {
      @out/line <- selected
    }
  | * => {
      failure := function-that-does-not-exist(1.0)
    }
"#).unwrap_err();
    assert!(error.kind_name().contains("Function"),"unexpected elaboration error: {error:?}");
    assert_rejected_activation_left_no_state(&runtime,&output,&plan,sends,&bindings);
  }

  #[test]
  fn patterned_activation_send_rejects_isolated_registration() {
    let(mut runtime,output)=activation_rejection_runtime();let plan=activation_plan_snapshot(&runtime);let sends=runtime.persistent_sends.len();let bindings=runtime.live_input_bindings.clone();let mut context=runtime.runtime_context().unwrap();
    let error=runtime.run_string_with_isolated_registration_for_test(&mut context,r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick
  | selected, selected > 0.0 => {
      @out/line <- selected
    }
  | * => {
      fallback := 0.0
    }
"#).unwrap_err();
    assert!(error.kind_as::<RuntimeIsolatedActivationSendUnsupported>().is_some(),"unexpected error: {error:?}");
    assert_rejected_activation_left_no_state(&runtime,&output,&plan,sends,&bindings);
  }

  #[test]
  fn patterned_activation_send_preserves_custom_live_authority() {
    let provider=TestResourceProvider::new().with_value("test://render/timer","tick",Value::F64(Ref::new(0.0)));let(mut runtime,output)=test_runtime_with_output(provider);let default_subject=runtime.runtime_context().unwrap().subject;let custom_subject="task:patterned-activation-send-custom";grant_read_to(&mut runtime,custom_subject,"test://render/timer","tick");grant_write_to(&mut runtime,custom_subject,TEST_OUTPUT_BASE_URI,"line");
    assert!(!runtime.has_capability_grant(&default_subject,"test://render/timer",&RuntimeCapabilityOperation::Read,"tick"));assert!(!runtime.has_capability_grant(&default_subject,TEST_OUTPUT_BASE_URI,&RuntimeCapabilityOperation::Write,"line"));
    let mut context=runtime.runtime_context().unwrap().with_subject(custom_subject);
    runtime.run_string_with_context(&mut context,r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick
  | selected, selected > 0.0 => {
      @out/line <- selected
    }
  | * => {
      fallback := 0.0
    }
"#).unwrap();
    assert!(output.lines().is_empty());assert_eq!(activation_send_count(&runtime),1);
    let outcome=apply_f64_input(&mut runtime,"test://render/timer","tick",9.0);assert!(outcome.turn.is_some());assert_eq!(output.lines(),vec!["9"]);assert_eq!(activation_send_count(&runtime),1);
  }

  #[test]
  fn patterned_activation_snapshots_effects_before_commit_and_releases_after_success() {
    let provider = TestResourceProvider::new().with_value(
      "test://render/timer",
      "tick",
      Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~state := 0.0
~> render-tick
  | amount, amount > 0.0 => {
      state = state + amount
      @out/line <- state
      @out/line <- state + 0.0
    }
  | ignored => {
      fallback := ignored
    }
"#).unwrap();
    let plan = activation_plan_snapshot(&runtime);
    let register = register_node_for_symbol(&runtime, "state");
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());

    let first = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    let first_turn = only_reactive_turn(&first);
    assert_eq!(
      first_turn.register_commit.committed_nodes,
      vec![register]
    );
    assert!(first_turn.after_commit.pending_register_nodes.is_empty());
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 1.0);
    assert_eq!(output.lines(), vec!["0", "0"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let equal = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    let equal_turn = only_reactive_turn(&equal);
    assert_eq!(
      equal_turn.register_commit.committed_nodes,
      vec![register]
    );
    assert!(equal_turn.after_commit.pending_register_nodes.is_empty());
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 2.0);
    assert_eq!(output.lines(), vec!["0", "0", "1", "1"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);
  }

  #[test]
  fn patterned_activation_register_batch_failure_does_not_leak_or_replay_send() {
    let provider=TestResourceProvider::new().with_value("test://render/timer","tick",Value::F64(Ref::new(0.0))).with_value("test://other/timer","tick",Value::F64(Ref::new(0.0)));let(mut runtime,output)=test_runtime_with_output(provider);grant_read(&mut runtime,"test://render/timer","tick");grant_read(&mut runtime,"test://other/timer","tick");grant_write(&mut runtime,TEST_OUTPUT_BASE_URI,"line");
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@other := test://other/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
other-tick := @other/tick
other-value := other-tick + 1.0
~state := 0.0
~> render-tick
  | amount, amount > 0.0 => {
      state = state + amount
      state = state + amount + 1.0
      @out/line <- state
    }
  | * => {
      fallback := 0.0
    }
"#).unwrap();
    let plan=activation_plan_snapshot(&runtime);assert_eq!(f64_value(&symbol_value(&runtime,"state")),0.0);assert!(output.lines().is_empty());assert_eq!(activation_send_count(&runtime),1);
    let error=runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://render/timer","tick").unwrap(),RuntimeHostInputValue::F64(1.0))).unwrap_err();assert!(format!("{error:?}").contains("ReactiveRegisterOutputConflict"),"unexpected error: {error:?}");assert_eq!(f64_value(&symbol_value(&runtime,"state")),0.0);assert!(output.lines().is_empty());assert_eq!(activation_plan_snapshot(&runtime),plan);
    let unrelated_error=runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://other/timer","tick").unwrap(),RuntimeHostInputValue::F64(7.0))).unwrap_err();assert!(format!("{unrelated_error:?}").contains("ReactiveRegisterOutputConflict"),"unexpected carried-batch error: {unrelated_error:?}");assert_eq!(f64_value(&symbol_value(&runtime,"state")),0.0);assert!(output.lines().is_empty());assert_eq!(activation_plan_snapshot(&runtime),plan);assert_eq!(activation_send_count(&runtime),1);
  }

  #[test]
  fn failed_patterned_body_does_not_replay_send_on_unrelated_turn() {
    let provider=TestResourceProvider::new().with_value("test://render/timer","tick",Value::F64(Ref::new(0.0))).with_value("test://other/timer","tick",Value::F64(Ref::new(0.0)));let(mut runtime,output)=test_runtime_with_output(provider);grant_read(&mut runtime,"test://render/timer","tick");grant_read(&mut runtime,"test://other/timer","tick");grant_write(&mut runtime,TEST_OUTPUT_BASE_URI,"line");
    let subject=runtime.runtime_context().unwrap().subject;grant_host_call(&mut runtime,&subject,194,"host:demo/patterned-body-fail-second");let calls=Arc::new(AtomicUsize::new(0));let host_calls=calls.clone();
    runtime.register_mech_host_function(ClosureHostFunction::new("demo/patterned-body-fail-second",move |_services,_context,args|{let call=host_calls.fetch_add(1,Ordering::SeqCst)+1;if call==2{return Err(MechError::new(DeliberateHostCallError,None));}Ok(Value::F64(Ref::new(host_f64_argument(&args[0]))))})).unwrap();
    runtime.run_string(r#"@tick := test://render/timer{:read(tick)}
@other := test://other/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
other-tick := @other/tick
other-value := other-tick + 1.0
~state := 0.0
~> render-tick
  | amount, amount > 0.0 => {
      state = demo/patterned-body-fail-second(state + amount)
      @out/line <- state
    }
  | * => {
      fallback := 0.0
    }
"#).unwrap();
    let plan=activation_plan_snapshot(&runtime);assert_eq!(calls.load(Ordering::SeqCst),1);assert!(output.lines().is_empty());assert_eq!(activation_send_count(&runtime),1);
    let error=runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("test://render/timer","tick").unwrap(),RuntimeHostInputValue::F64(1.0))).unwrap_err();assert!(error.kind_as::<DeliberateHostCallError>().is_some());assert_eq!(calls.load(Ordering::SeqCst),2);assert_eq!(f64_value(&symbol_value(&runtime,"state")),0.0);assert!(output.lines().is_empty());assert_eq!(activation_plan_snapshot(&runtime),plan);
    let unrelated=apply_f64_input(&mut runtime,"test://other/timer","tick",7.0);assert!(unrelated.turn.is_some());assert_eq!(f64_value(&symbol_value(&runtime,"state")),0.0);assert!(output.lines().is_empty());assert_eq!(activation_plan_snapshot(&runtime),plan);
    let retry=apply_f64_input(&mut runtime,"test://render/timer","tick",1.0);assert!(retry.turn.is_some());assert_eq!(calls.load(Ordering::SeqCst),3);assert_eq!(f64_value(&symbol_value(&runtime,"state")),1.0);assert_eq!(output.lines(),vec!["0"]);assert_eq!(activation_plan_snapshot(&runtime),plan);assert_eq!(activation_send_count(&runtime),1);
  }

  #[test]
  fn activation_two_clock_physics_render_acceptance() {
    let provider=TestResourceProvider::new().with_value("test://physics/timer","tick",Value::F64(Ref::new(0.0))).with_value("test://render/timer","tick",Value::F64(Ref::new(0.0)));let(mut runtime,output)=test_runtime_with_output(provider);grant_read(&mut runtime,"test://physics/timer","tick");grant_read(&mut runtime,"test://render/timer","tick");grant_write(&mut runtime,TEST_OUTPUT_BASE_URI,"line");
    runtime.run_string(r#"@physics := test://physics/timer{:read(tick)}
@render := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
physics-tick := @physics/tick
render-tick := @render/tick
~x := 0.0
~> physics-tick {
  next-x := x + 1.0
  x = next-x
}
~> render-tick {
  @out/line <- x
}
"#).unwrap();let initial_plan=activation_plan_snapshot(&runtime);let render_barrier=activation_barrier_for_trigger(&runtime,"render-tick");let physics_combinational_nodes=activation_nodes_for_trigger(&runtime,"physics-tick",ReactiveNodeKind::Combinational);let x_register=register_node_for_symbol(&runtime,"x");assert!(!physics_combinational_nodes.is_empty());assert_eq!(f64_value(&symbol_value(&runtime,"x")),0.0);assert!(output.lines().is_empty());assert_eq!(activation_send_count(&runtime),1);
    let a=apply_f64_input(&mut runtime,"test://physics/timer","tick",1.0);let t=only_reactive_turn(&a);assert_eq!(f64_value(&symbol_value(&runtime,"x")),1.0);assert!(output.lines().is_empty());assert_eq!(executed_count(t,render_barrier),0);for n in &physics_combinational_nodes{assert_eq!(executed_count(t,*n),1)}assert_eq!(t.register_commit.committed_nodes,vec![x_register]);assert_eq!(activation_plan_snapshot(&runtime),initial_plan);assert_eq!(activation_send_count(&runtime),1);
    let a=apply_f64_input(&mut runtime,"test://physics/timer","tick",2.0);let t=only_reactive_turn(&a);assert_eq!(f64_value(&symbol_value(&runtime,"x")),2.0);assert!(output.lines().is_empty());assert_eq!(executed_count(t,render_barrier),0);for n in &physics_combinational_nodes{assert_eq!(executed_count(t,*n),1)}assert_eq!(t.register_commit.committed_nodes,vec![x_register]);assert_eq!(activation_plan_snapshot(&runtime),initial_plan);assert_eq!(activation_send_count(&runtime),1);
    let a=apply_f64_input(&mut runtime,"test://render/timer","tick",1.0);let t=only_reactive_turn(&a);assert_eq!(f64_value(&symbol_value(&runtime,"x")),2.0);assert_eq!(output.lines().len(),1);assert_eq!(recorded_f64(&output,0),2.0);assert_eq!(executed_count(t,render_barrier),1);for n in &physics_combinational_nodes{assert_eq!(executed_count(t,*n),0)}assert!(!t.before_commit.pending_register_nodes.contains(&x_register));assert!(!t.register_commit.staged_nodes.contains(&x_register));assert!(!t.register_commit.committed_nodes.contains(&x_register));assert!(!t.after_commit.pending_register_nodes.contains(&x_register));assert_eq!(activation_plan_snapshot(&runtime),initial_plan);assert_eq!(activation_send_count(&runtime),1);
    let a=apply_f64_input(&mut runtime,"test://render/timer","tick",1.0);assert_eq!(f64_value(&symbol_value(&runtime,"x")),2.0);assert_eq!(output.lines().len(),2);assert_eq!(recorded_f64(&output,0),2.0);assert_eq!(recorded_f64(&output,1),2.0);assert_eq!(executed_count(only_reactive_turn(&a),render_barrier),1);assert_eq!(activation_plan_snapshot(&runtime),initial_plan);assert_eq!(activation_send_count(&runtime),1);
  }
  #[test]
  fn activation_send_samples_latest_value_and_ignores_other_updates(){let(mut r,o)=test_runtime_with_output(TestResourceProvider::new().with_value("test://render/timer","tick",Value::F64(Ref::new(0.0))).with_value("test://other/timer","tick",Value::F64(Ref::new(0.0))).with_value(TEST_SIGNALS_BASE_URI,"value",Value::F64(Ref::new(1.0))));for(b,p)in [("test://render/timer","tick"),("test://other/timer","tick"),(TEST_SIGNALS_BASE_URI,"value")]{grant_read(&mut r,b,p)}grant_write(&mut r,TEST_OUTPUT_BASE_URI,"line");r.run_string(r#"@render := test://render/timer{:read(tick)}
@other := test://other/timer{:read(tick)}
@signals := test://signals/inputs{:read(value)}
@out := test://effects/output{:write(line)}
render-tick := @render/tick
other-tick := @other/tick
sampled-value := @signals/value
~> render-tick {
  @out/line <- sampled-value
}
~> other-tick {
  other-result := sampled-value + 1.0
}
"#).unwrap();let b=activation_barrier_for_trigger(&r,"render-tick");let ns=activation_nodes_for_trigger(&r,"other-tick",ReactiveNodeKind::Combinational);assert!(!ns.is_empty());assert!(o.lines().is_empty());let q=apply_f64_input(&mut r,TEST_SIGNALS_BASE_URI,"value",10.0);assert!(o.lines().is_empty());assert_eq!(executed_count(only_reactive_turn(&q),b),0);let q=apply_f64_input(&mut r,"test://other/timer","tick",1.0);assert!(o.lines().is_empty());assert_eq!(executed_count(only_reactive_turn(&q),b),0);for n in ns{assert_eq!(executed_count(only_reactive_turn(&q),n),1)}let q=apply_f64_input(&mut r,"test://render/timer","tick",1.0);assert_eq!(o.lines().len(),1);assert_eq!(recorded_f64(&o,0),10.0);assert_eq!(executed_count(only_reactive_turn(&q),b),1);let q=apply_f64_input(&mut r,TEST_SIGNALS_BASE_URI,"value",20.0);assert_eq!(o.lines().len(),1);assert_eq!(executed_count(only_reactive_turn(&q),b),0);let q=apply_f64_input(&mut r,"test://render/timer","tick",1.0);assert_eq!(o.lines().len(),2);assert_eq!(recorded_f64(&o,0),10.0);assert_eq!(recorded_f64(&o,1),20.0);assert_eq!(executed_count(only_reactive_turn(&q),b),1);}

  fn rejected(source:&str, check:impl FnOnce(MechError)){let(mut r,o)=activation_rejection_runtime();let p=activation_plan_snapshot(&r);let n=r.persistent_sends.len();let b=r.live_input_bindings.clone();check(r.run_string(source).unwrap_err());assert_rejected_activation_left_no_state(&r,&o,&p,n,&b);}
  #[test] fn activation_send_rejects_local_register_mix(){rejected(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~x := 0.0
~> render-tick {
  x = x + 1.0
  @out/line <- x
}
"#,|e|assert!(e.kind_as::<ActivationScopeEffectWithRegisterUnsupported>().is_some(),"unexpected error: {e:?}"));}
  #[test] fn activation_send_rejects_local_op_assign_mix(){rejected(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~x := 0.0
~> render-tick {
  x += 1.0
  @out/line <- x
}
"#,|e|assert!(e.kind_as::<ActivationScopeEffectWithRegisterUnsupported>().is_some(),"unexpected error: {e:?}"));}
  #[test] fn activation_send_rejects_context_assignment_in_scope(){rejected(r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
x := 1.0
~> render-tick {
  @out/line = x
  @out/line <- x
}
"#,|e|{let k=e.kind_as::<RuntimeInvalidOperationError>().expect("expected RuntimeInvalidOperationError");assert_eq!(k.operation,"direct_context_effect_placement");assert!(k.reason.contains("context assignment"),"unexpected placement reason: {}",k.reason);assert!(e.kind_as::<ActivationScopeEffectWithRegisterUnsupported>().is_none());});}
  #[test] fn activation_send_rejects_isolated_registration(){let(mut r,o)=activation_rejection_runtime();let p=activation_plan_snapshot(&r);let n=r.persistent_sends.len();let b=r.live_input_bindings.clone();let mut c=r.runtime_context().unwrap();let e=r.run_string_with_isolated_registration_for_test(&mut c,r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick {
  @out/line <- render-tick
}
"#).unwrap_err();assert!(e.kind_as::<RuntimeIsolatedActivationSendUnsupported>().is_some(),"unexpected error: {e:?}");assert_rejected_activation_left_no_state(&r,&o,&p,n,&b);}

    #[test]
    fn activation_send_reactive_failure_produces_no_writes() {
        let provider = TestResourceProvider::new().with_value(
            "test://render/timer",
            "tick",
            Value::F64(Ref::new(0.0)),
        );
        let (mut runtime, output) = test_runtime_with_output(provider);
        grant_read(&mut runtime, "test://render/timer", "tick");
        grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
        let subject = runtime.runtime_context().unwrap().subject;
        grant_host_call(
            &mut runtime,
            &subject,
            193,
            "host:demo/activation-fail-second",
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let host_calls = calls.clone();
        runtime
            .register_mech_host_function(ClosureHostFunction::new(
                "demo/activation-fail-second",
                move |_services, _context, args| {
                    let call_number = host_calls.fetch_add(1, Ordering::SeqCst) + 1;
                    if call_number == 2 {
                        return Err(MechError::new(DeliberateHostCallError, None));
                    }
                    Ok(Value::F64(Ref::new(host_f64_argument(&args[0]))))
                },
            ))
            .unwrap();
        runtime
            .run_string(
                r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}

render-tick := @tick/tick

~> render-tick {
  @out/line <-
    demo/activation-fail-second(render-tick)
}
"#,
            )
            .unwrap();
        let plan_before = activation_plan_snapshot(&runtime);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "host function must compile once during load"
        );
        assert!(output.lines().is_empty());
        assert_eq!(activation_send_count(&runtime), 1);
        let error = runtime
            .apply_host_input(RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
                RuntimeHostInputValue::F64(1.0),
            ))
            .unwrap_err();
        assert!(
            error.kind_as::<DeliberateHostCallError>().is_some(),
            "unexpected error: {error:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(output.lines().is_empty());
        assert_eq!(activation_send_count(&runtime), 1);
        assert_eq!(activation_plan_snapshot(&runtime), plan_before);
        let retry = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
        assert!(retry.turn.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(output.lines().len(), 1);
        assert_eq!(recorded_f64(&output, 0), 1.0);
        assert_eq!(activation_send_count(&runtime), 1);
        assert_eq!(activation_plan_snapshot(&runtime), plan_before);
    }

    #[test]
    fn activation_send_provider_failure_is_fail_fast_and_registration_is_retained() {
        let provider = TestResourceProvider::new().with_value(
            "test://render/timer",
            "tick",
            Value::F64(Ref::new(0.0)),
        );
        let output = SequencedOutput::default();
        let mut runtime = RuntimeBuilder::new()
            .resource_provider(Box::new(provider) as Box<dyn RuntimeResourceProvider>)
            .resource_provider(Box::new(SequencedOutputProvider {
                backend: output.clone(),
            }) as Box<dyn RuntimeResourceProvider>)
            .build()
            .unwrap();
        grant_read(&mut runtime, "test://render/timer", "tick");
        grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
        runtime
            .run_string(
                r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}

render-tick := @tick/tick
latest := render-tick + 1.0

~> render-tick {
  @out/line <- "first"
  @out/line <- "second"
  @out/line <- "third"
}
"#,
            )
            .unwrap();
        assert!(output.attempts().is_empty());
        assert!(output.successes().is_empty());
        assert_eq!(activation_send_count(&runtime), 3);
        let plan_before = activation_plan_snapshot(&runtime);
        output.fail_once_at(2);
        let error = runtime
            .apply_host_input(RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
                RuntimeHostInputValue::F64(1.0),
            ))
            .unwrap_err();
        assert!(
            error.kind_as::<PersistentSendTestError>().is_some(),
            "unexpected error: {error:?}"
        );
        assert_eq!(
            output.attempts(),
            vec!["\"first\"".to_string(), "\"second\"".to_string()]
        );
        assert_eq!(output.successes(), vec!["\"first\"".to_string()]);
        assert_eq!(
            f64_value(&symbol_value(&runtime, "latest")),
            2.0,
            "reactive state must remain committed"
        );
        assert_eq!(activation_send_count(&runtime), 3);
        assert_eq!(activation_plan_snapshot(&runtime), plan_before);
        let retry = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
        assert!(retry.turn.is_some());
        assert_eq!(
            output.attempts(),
            vec![
                "\"first\"".to_string(),
                "\"second\"".to_string(),
                "\"first\"".to_string(),
                "\"second\"".to_string(),
                "\"third\"".to_string()
            ]
        );
        assert_eq!(
            output.successes(),
            vec![
                "\"first\"".to_string(),
                "\"first\"".to_string(),
                "\"second\"".to_string(),
                "\"third\"".to_string()
            ]
        );
        assert_eq!(activation_send_count(&runtime), 3);
        assert_eq!(activation_plan_snapshot(&runtime), plan_before);
    }

    #[test]
    fn activation_send_preserves_custom_live_authority() {
        let provider = TestResourceProvider::new().with_value(
            "test://render/timer",
            "tick",
            Value::F64(Ref::new(0.0)),
        );
        let (mut runtime, output) = test_runtime_with_output(provider);
        let default_subject = runtime.runtime_context().unwrap().subject;
        let custom_subject = "task:activation-send-custom";
        grant_read_to(&mut runtime, custom_subject, "test://render/timer", "tick");
        grant_write_to(&mut runtime, custom_subject, TEST_OUTPUT_BASE_URI, "line");
        assert!(!runtime.has_capability_grant(
            &default_subject,
            "test://render/timer",
            &RuntimeCapabilityOperation::Read,
            "tick"
        ));
        assert!(!runtime.has_capability_grant(
            &default_subject,
            TEST_OUTPUT_BASE_URI,
            &RuntimeCapabilityOperation::Write,
            "line"
        ));
        let mut context = runtime
            .runtime_context()
            .unwrap()
            .with_subject(custom_subject);
        let source = r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}

render-tick := @tick/tick

~> render-tick {
  @out/line <- render-tick
}
"#;
        runtime
            .run_string_with_context(&mut context, source)
            .unwrap();
        assert!(output.lines().is_empty());
        assert_eq!(activation_send_count(&runtime), 1);
        let outcome = apply_f64_input(&mut runtime, "test://render/timer", "tick", 9.0);
        assert!(outcome.turn.is_some());
        assert_eq!(output.lines().len(), 1);
        assert_eq!(recorded_f64(&output, 0), 9.0);
        assert_eq!(activation_send_count(&runtime), 1);
    }

    #[test]
    fn activation_send_internal_payload_name_does_not_collide_with_user_binding() {
        let provider = TestResourceProvider::new().with_value(
            "test://render/timer",
            "tick",
            Value::F64(Ref::new(0.0)),
        );
        let (mut runtime, output) = test_runtime_with_output(provider);
        grant_read(&mut runtime, "test://render/timer", "tick");
        grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
        runtime
            .run_string(
                r#"mech-internal-activation-send-value-0 := 41.0

@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}

render-tick := @tick/tick

~> render-tick {
  @out/line <- 7.0
}

after :=
  mech-internal-activation-send-value-0 + 1.0
"#,
            )
            .unwrap();
        assert_eq!(
            f64_value(&symbol_value(
                &runtime,
                "mech-internal-activation-send-value-0"
            )),
            41.0
        );
        assert_eq!(f64_value(&symbol_value(&runtime, "after")), 42.0);
        assert!(output.lines().is_empty());
        assert_eq!(activation_send_count(&runtime), 1);
        let outcome = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
        assert!(outcome.turn.is_some());
        assert_eq!(output.lines().len(), 1);
        assert_eq!(recorded_f64(&output, 0), 7.0);
    }

  #[test]
  fn activation_internal_barrier_is_not_user_callable() {
    let (mut runtime, _driver, _console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    let error = runtime.run_string("mech/runtime/activation-effect-barrier()").unwrap_err();
    assert!(format!("{error:?}").contains("MissingFunction"));
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
