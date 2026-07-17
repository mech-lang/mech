use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::thread;
use std::time::Duration;

use mech_core::{hash_str, MResult, MechError, MechErrorKind, Ref, Value};

use super::*;
use crate::{
  BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId,
  ConfigValue, HostContextManifest, HostInstanceConfig, HostManifestConfig, InMemoryDocsProvider,
  RuntimeCapabilityGrant, RuntimeHostFactory, RuntimeHostInstallation, RuntimeHostInput,
  RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue, RuntimeIngress,
  RuntimeResourceProvider, ClosureHostFunction, materialize_host_manifest,
};

fn f64_value(value: &Value) -> f64 {
  match value {
    Value::F64(value) => *value.borrow(),
    other => panic!("expected f64, got {other:?}"),
  }
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

fn docs_provider_with(base_uri: &str, path: &str, value: f64) -> InMemoryDocsProvider {
  InMemoryDocsProvider::new()
    .with_value(base_uri, path, Value::F64(Ref::new(value)))
    .unwrap()
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

fn docs_runtime(provider: InMemoryDocsProvider) -> MechRuntime {
  RuntimeBuilder::new()
    .resource_provider(Box::new(provider))
    .build()
    .unwrap()
}

#[test]
fn canonical_host_input_updates_live_context_read() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "docs://clock/ticks", "value");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();

  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
  let canonical = RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap();
  assert!(runtime.live_input_bindings.contains_key(&canonical));
  assert!(runtime.live_input_bindings.keys().all(|source| !source.base_uri().contains("pulse") && !source.path().contains("pulse")));

  let alias = RuntimeHostInputSource::new("docs://pulse", "value").unwrap();
  let outcome = runtime
    .apply_host_input(RuntimeHostInput::single(alias, RuntimeHostInputValue::F64(5.0)))
    .unwrap();
  assert_eq!(outcome.update_count, 1);
  assert_eq!(outcome.ignored_update_count, 1);
  assert_eq!(outcome.binding_count, 0);
  assert!(outcome.solve.is_none());
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);

  runtime.ingress().submit(RuntimeHostInput::single(canonical, RuntimeHostInputValue::F64(5.0))).unwrap();
  let outcomes = runtime.drain_host_inputs(1).unwrap();
  assert_eq!(outcomes.len(), 1);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
}

#[test]
fn logical_packet_updates_all_inputs_and_solves_once() {
  let provider = InMemoryDocsProvider::new()
    .with_value("docs://clock/ticks", "a", Value::F64(Ref::new(1.0))).unwrap()
    .with_value("docs://clock/ticks", "b", Value::F64(Ref::new(2.0))).unwrap();
  let mut runtime = docs_runtime(provider);
  grant_read(&mut runtime, "docs://clock/ticks", "a");
  grant_read(&mut runtime, "docs://clock/ticks", "b");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(a), :read(b)}\nsum := @pulse/a + @pulse/b").unwrap();
  assert_eq!(f64_value(&symbol_value(&runtime, "sum")), 3.0);

  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://clock/ticks", "a").unwrap(), value: RuntimeHostInputValue::F64(10.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://clock/ticks", "b").unwrap(), value: RuntimeHostInputValue::F64(20.0) },
  ]).unwrap()).unwrap();

  assert!(outcome.solve.as_ref().unwrap().plan_len > 0);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("docs://clock/ticks", "a").unwrap())), 10.0);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("docs://clock/ticks", "b").unwrap())), 20.0);
  assert_eq!(f64_value(&symbol_value(&runtime, "sum")), 30.0);
}

#[test]
fn packet_with_bound_and_unbound_sources_updates_bound_inputs() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "docs://clock/ticks", "value");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();
  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap(), value: RuntimeHostInputValue::F64(5.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://clock/ticks", "missing").unwrap(), value: RuntimeHostInputValue::F64(9.0) },
  ]).unwrap()).unwrap();

  assert_eq!(outcome.update_count, 2);
  assert_eq!(outcome.ignored_update_count, 1);
  assert_eq!(outcome.binding_count, 1);
  assert!(outcome.solve.is_some());
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap())), 5.0);
}


#[test]
fn partial_snapshot_updates_bound_fields_and_ignores_unbound_fields() {
  let provider = InMemoryDocsProvider::new()
    .with_value("docs://timer/state", "tick", Value::F64(Ref::new(1.0))).unwrap()
    .with_value("docs://timer/state", "delta-seconds", Value::F64(Ref::new(0.1))).unwrap();
  let mut runtime = docs_runtime(provider);
  grant_read(&mut runtime, "docs://timer/state", "tick");
  grant_read(&mut runtime, "docs://timer/state", "delta-seconds");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@timer := docs://timer/state{:read(tick), :read(delta-seconds)}\noutput := @timer/tick + @timer/delta-seconds").unwrap();

  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://timer/state", "tick").unwrap(), value: RuntimeHostInputValue::F64(10.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://timer/state", "elapsed-ms").unwrap(), value: RuntimeHostInputValue::F64(1000.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://timer/state", "delta-ms").unwrap(), value: RuntimeHostInputValue::F64(16.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://timer/state", "elapsed-seconds").unwrap(), value: RuntimeHostInputValue::F64(1.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://timer/state", "delta-seconds").unwrap(), value: RuntimeHostInputValue::F64(0.25) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://timer/state", "skipped-steps").unwrap(), value: RuntimeHostInputValue::F64(0.0) },
  ]).unwrap()).unwrap();

  assert_eq!(outcome.update_count, 6);
  assert_eq!(outcome.ignored_update_count, 4);
  assert_eq!(outcome.binding_count, 2);
  assert!(outcome.solve.is_some());
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("docs://timer/state", "tick").unwrap())), 10.0);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("docs://timer/state", "delta-seconds").unwrap())), 0.25);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.25);
}

#[test]
fn all_unbound_packet_skips_plan_solve() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "docs://clock/ticks", "value");
  let mut context = runtime.runtime_context().unwrap();
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();

  let outcome = runtime.apply_host_input(RuntimeHostInput::new(vec![
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://clock/ticks", "missing-a").unwrap(), value: RuntimeHostInputValue::F64(5.0) },
    RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("docs://clock/ticks", "missing-b").unwrap(), value: RuntimeHostInputValue::F64(9.0) },
  ]).unwrap()).unwrap();

  assert_eq!(outcome.ignored_update_count, outcome.update_count);
  assert_eq!(outcome.binding_count, 0);
  assert!(outcome.solve.is_none());
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
}


#[test]
fn live_input_recomputes_runtime_host_function() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  grant_read(&mut runtime, "docs://clock/ticks", "value");
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
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := demo/live-plus-one(@pulse/value) + 0").unwrap();
  let initial_calls = calls.load(Ordering::SeqCst);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);

  runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(9.0))).unwrap();

  assert!(calls.load(Ordering::SeqCst) > initial_calls);
  assert_eq!(f64_value(&source_value(&runtime, &RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap())), 9.0);
  assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
}


fn grant_read_to(runtime: &mut MechRuntime, subject: &str, resource: &str, path: &str) {
  runtime.grant_capability(RuntimeCapabilityGrant {
    subject: subject.to_string(),
    resource: resource.to_string(),
    operations: vec![RuntimeCapabilityOperation::Read],
    paths: vec![path.to_string()],
  }).unwrap();
}

fn grant_host_call(runtime: &mut MechRuntime, subject: &str, id: u64, resource: &str) {
  runtime
    .grant_capability(Arc::new(BasicCapability::new(
      CapabilityId(id.into()),
      &BasicSubject::new(subject),
      &BasicResource::new(resource),
      [BasicOperation::new("call")],
    )))
    .unwrap();
}

fn register_sleep_host(runtime: &mut MechRuntime, name: &str) {
  runtime.register_mech_host_function(ClosureHostFunction::new(name, move |_services, _context, args| {
    thread::sleep(Duration::from_millis(5));
    match args.first() {
      Some(Value::F64(value)) => Ok(Value::F64(Ref::new(*value.borrow()))),
      Some(Value::MutableReference(value)) => match &*value.borrow() {
        Value::F64(value) => Ok(Value::F64(Ref::new(*value.borrow()))),
        other => panic!("expected f64 mutable reference, got {other:?}"),
      },
      other => panic!("expected f64 argument, got {other:?}"),
    }
  })).unwrap();
}

#[test]
fn transactional_context_cannot_arm_live_program() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  let mut context = runtime.runtime_context().unwrap();
  grant_read_to(&mut runtime, &context.subject, "docs://clock/ticks", "value");
  runtime.begin_transaction(&mut context).unwrap();
  let error = format!("{:?}", runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap_err());
  assert!(error.contains("RuntimeTransactionalLiveProgramUnsupported"), "{error}");
  assert!(runtime.live_context_template.is_none());
  assert!(runtime.live_input_bindings.is_empty());
  assert!(runtime.persistent_sends.is_empty());
}

#[test]
fn failed_source_does_not_leave_live_state_armed() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  let mut context_a = runtime.runtime_context().unwrap().with_subject("subject-a");
  grant_read_to(&mut runtime, "subject-a", "docs://clock/ticks", "value");
  let error = runtime.run_string_with_context(&mut context_a, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value\nmissing := @pulse/missing");
  assert!(error.is_err());
  assert!(runtime.live_context_template.is_none());
  assert!(runtime.live_input_bindings.is_empty());
  assert!(runtime.persistent_sends.is_empty());

  let mut context_b = runtime.runtime_context().unwrap().with_subject("subject-b");
  grant_read_to(&mut runtime, "subject-b", "docs://clock/ticks", "value");
  runtime.run_string_with_context(&mut context_b, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap();
}

#[test]
fn duration_failure_restores_live_state() {
  let mut config = RuntimeConfig::default();
  config.limits.max_turn_duration_ms = Some(1);
  let mut runtime = RuntimeBuilder::new()
    .config(config)
    .resource_provider(Box::new(docs_provider_with("docs://clock/ticks", "value", 1.0)))
    .build()
    .unwrap();
  let mut context = runtime.runtime_context().unwrap();
  grant_read_to(&mut runtime, &context.subject, "docs://clock/ticks", "value");
  grant_host_call(&mut runtime, &context.subject, 43, "host:demo/source-sleep");
  register_sleep_host(&mut runtime, "demo/source-sleep");
  let error = format!("{:?}", runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\nslow := demo/source-sleep(@pulse/value)").unwrap_err());
  assert!(error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"), "{error}");
  assert!(runtime.live_context_template.is_none());
  assert!(runtime.live_input_bindings.is_empty());
  assert!(runtime.persistent_sends.is_empty());
  runtime.config.limits.max_turn_duration_ms = None;
  runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn live_turn_enforces_step_budget() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  let mut context = runtime.runtime_context().unwrap().with_budget(ResourceBudget::default().with_max_steps(1));
  grant_read_to(&mut runtime, &context.subject, "docs://clock/ticks", "value");
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();
  runtime.live_context_template.as_mut().unwrap().budget_limits.max_steps = Some(0);
  let error = format!("{:?}", runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(2.0))).unwrap_err());
  assert!(error.contains("ResourceBudgetExceeded"), "{error}");

  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  let mut context = runtime.runtime_context().unwrap().with_budget(ResourceBudget::default().with_max_steps(1));
  grant_read_to(&mut runtime, &context.subject, "docs://clock/ticks", "value");
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value * 2").unwrap();
  let outcome = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(2.0))).unwrap();
  assert!(outcome.solve.is_some());
  let outcome = runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(3.0))).unwrap();
  assert!(outcome.solve.is_some());
}

#[test]
fn live_turn_enforces_duration_limit() {
  let mut runtime = RuntimeBuilder::new()
    .resource_provider(Box::new(docs_provider_with("docs://clock/ticks", "value", 1.0)))
    .build()
    .unwrap();
  let mut context = runtime.runtime_context().unwrap();
  grant_read_to(&mut runtime, &context.subject, "docs://clock/ticks", "value");
  grant_host_call(&mut runtime, &context.subject, 44, "host:demo/live-sleep");
  register_sleep_host(&mut runtime, "demo/live-sleep");
  runtime.run_string_with_context(&mut context, "@pulse := docs://clock/ticks{:read(value)}\nslow := demo/live-sleep(@pulse/value)").unwrap();
  runtime.config.limits.max_turn_duration_ms = Some(1);
  let error = format!("{:?}", runtime.apply_host_input(RuntimeHostInput::single(RuntimeHostInputSource::new("docs://clock/ticks", "value").unwrap(), RuntimeHostInputValue::F64(2.0))).unwrap_err());
  assert!(error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"), "{error}");
  assert!(runtime.program().interpreter().symbols().borrow().contains(hash_str("slow")));
  runtime.config.limits.max_turn_duration_ms = None;
  runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn changed_actor_message_rejects_live_context_reuse() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  let actor = ActorId(7);
  let state = ObjectId(9);
  let mut context_a = runtime.runtime_context().unwrap().with_subject("actor:7").with_actor(actor);
  context_a.actor_state = Some(state);
  context_a.actor_message = Some(MessageRecord::new(MessageId(1), actor, "tick", b"a".to_vec()));
  grant_read_to(&mut runtime, "actor:7", "docs://clock/ticks", "value");
  runtime.run_string_with_context(&mut context_a, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap();

  let mut context_b = runtime.runtime_context().unwrap().with_subject("actor:7").with_actor(actor);
  context_b.actor_state = Some(state);
  context_b.actor_message = Some(MessageRecord::new(MessageId(2), actor, "tick", b"b".to_vec()));
  let error = format!("{:?}", runtime.run_string_with_context(&mut context_b, "@pulse := docs://clock/ticks{:read(value)}\nother := @pulse/value").unwrap_err());
  assert!(error.contains("RuntimeLiveContextMismatch"), "{error}");
}

#[test]
fn live_context_capability_order_does_not_cause_mismatch() {
  let mut runtime = docs_runtime(docs_provider_with("docs://clock/ticks", "value", 1.0));
  let mut context_a = runtime.runtime_context().unwrap().with_capabilities(vec![CapabilityId(1), CapabilityId(2)]);
  let mut context_b = runtime.runtime_context().unwrap().with_capabilities(vec![CapabilityId(2), CapabilityId(1)]);
  grant_read_to(&mut runtime, &context_a.subject, "docs://clock/ticks", "value");
  runtime.run_string_with_context(&mut context_a, "@pulse := docs://clock/ticks{:read(value)}\noutput := @pulse/value").unwrap();
  runtime.run_string_with_context(&mut context_b, "@pulse := docs://clock/ticks{:read(value)}\nother := @pulse/value").unwrap();
}

#[derive(Debug, Clone)]
struct MockDriver {
  name: String,
  state: Rc<RefCell<MockDriverState>>,
  events: Rc<RefCell<Vec<String>>>,
}

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
    fn base_uris(&self) -> Vec<String> { vec!["time://clock/clock".to_string()] }
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
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("time://clock/clock", "unix-ms")?, value: RuntimeHostInputValue::F64(snapshot.unix_ms) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("time://clock/clock", "hour")?, value: RuntimeHostInputValue::F64(snapshot.hour) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("time://clock/clock", "minute")?, value: RuntimeHostInputValue::F64(snapshot.minute) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("time://clock/clock", "second")?, value: RuntimeHostInputValue::F64(snapshot.second) },
        RuntimeHostInputUpdate { source: RuntimeHostInputSource::new("time://clock/clock", "millisecond")?, value: RuntimeHostInputValue::F64(snapshot.millisecond) },
      ])?)
    }
  }

  impl RuntimeHostInputDriver for ManualTimeInputDriver {
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
    assert!(outcome.solve.is_some());
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
