use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Ref};

use super::super::{MechRuntime, RuntimeBuilder};
use crate::{
    BasicCapability, CapabilityId, ConfigValue, HostContextManifest, HostInstanceConfig,
    HostManifestConfig, ResidentDurabilityPolicy, RuntimeHostFactory, RuntimeHostInput,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue, RuntimeHostInstallation,
    RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest, materialize_host_manifest,
};

#[derive(Debug, Clone)]
struct MockDriver {
    name: String,
    state: Rc<RefCell<MockDriverState>>,
    events: Rc<RefCell<Vec<String>>>,
}

const MOCK_DRIVER_BASE_URI: &str = "test-input://clock/ticks";

#[derive(Debug, Default)]
struct MockDriverState {
    attach_count: usize,
    start_count: usize,
    stop_count: usize,
    live: bool,
    fail_attach: bool,
    fail_start: bool,
    fail_stop: bool,
    panic_start: bool,
    attached_ingress: Option<RuntimeIngress>,
    stop_observed_closed_ingress: bool,
    log: Vec<String>,
}

impl MockDriver {
    fn new(name: &str, state: Rc<RefCell<MockDriverState>>) -> Self {
        Self::with_events(name, state, Rc::new(RefCell::new(Vec::new())))
    }

    fn with_events(
        name: &str,
        state: Rc<RefCell<MockDriverState>>,
        events: Rc<RefCell<Vec<String>>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            state,
            events,
        }
    }
}

impl RuntimeHostInputDriver for MockDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == MOCK_DRIVER_BASE_URI && source.path() == self.name
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut state = self.state.borrow_mut();
        state.attach_count += 1;
        state.attached_ingress = Some(ingress);
        let event = format!("attach:{}", self.name);
        state.log.push(event.clone());
        self.events.borrow_mut().push(event);
        if state.fail_attach {
            return Err(mock_error(
                "MockAttachError",
                format!("attach failed for {}", self.name),
            ));
        }
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        let mut state = self.state.borrow_mut();
        state.start_count += 1;
        let event = format!("start:{}", self.name);
        state.log.push(event.clone());
        self.events.borrow_mut().push(event);
        if state.panic_start {
            panic!("deliberate input driver start panic");
        }
        if state.fail_start {
            return Err(mock_error(
                "MockStartError",
                format!("start failed for {}", self.name),
            ));
        }
        state.live = true;
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        let mut state = self.state.borrow_mut();
        state.stop_count += 1;
        let observed_closed = state
            .attached_ingress
            .as_ref()
            .map(|ingress| ingress.is_closed().unwrap_or(false))
            .unwrap_or(false);
        state.stop_observed_closed_ingress |= observed_closed;
        let event = format!("stop:{}", self.name);
        state.log.push(event.clone());
        self.events.borrow_mut().push(event);
        state.live = false;
        if state.fail_stop {
            return Err(mock_error(
                "MockStopError",
                format!("stop failed for {}", self.name),
            ));
        }
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.state.borrow().live
    }
}

#[derive(Debug)]
struct MockResourceProvider;

impl RuntimeResourceProvider for MockResourceProvider {
    fn scheme(&self) -> &str {
        "test-input"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![MOCK_DRIVER_BASE_URI.to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(Ref::new(1.0)))
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(Ref::new(1.0)))
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
                contexts: vec![HostContextManifest {
                    name: "ticks".to_string(),
                    base_uri_template: "test-input://{instance}/ticks".to_string(),
                    operations: vec!["read".to_string()],
                }],
            },
            drivers,
        }
    }
}

impl RuntimeHostFactory for MockDriverFactory {
    fn provider_name(&self) -> &str {
        "test-input"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, _settings: &ConfigValue) -> MResult<()> {
        Ok(())
    }
    fn instantiate(
        &self,
        instance_name: &str,
        _settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(MockResourceProvider)],
            input_drivers: self
                .drivers
                .iter()
                .cloned()
                .map(|driver| Box::new(driver) as Box<dyn RuntimeHostInputDriver>)
                .collect(),
        })
    }
}

fn drivers_with_events(
    states: &[(&str, Rc<RefCell<MockDriverState>>)],
) -> (Vec<MockDriver>, Rc<RefCell<Vec<String>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let drivers = states
        .iter()
        .map(|(name, state)| MockDriver::with_events(name, state.clone(), events.clone()))
        .collect();
    (drivers, events)
}

fn runtime_with_drivers(drivers: Vec<MockDriver>) -> MResult<MechRuntime> {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(MockDriverFactory::new(drivers)))?
        .host_instance(HostInstanceConfig {
            name: "clock".to_string(),
            provider: "test-input".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .build()
}

fn compiler_with_drivers(drivers: Vec<MockDriver>) -> MResult<super::super::ProgramCompiler> {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(MockDriverFactory::new(drivers)))?
        .host_instance(HostInstanceConfig {
            name: "clock".to_string(),
            provider: "test-input".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .build_compiler()
}

fn activate_mock_inputs(runtime: &mut MechRuntime, names: &[&str]) {
    let subject = runtime.runtime_context().unwrap().subject;
    let mut source = String::new();
    let mut values = Vec::new();
    for (index, name) in names.iter().enumerate() {
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(8_800 + index as u128),
                &subject,
                format!("{MOCK_DRIVER_BASE_URI}/{name}"),
                ["read"],
            )))
            .unwrap();
        source.push_str(&format!(
            "@input-{name} := {MOCK_DRIVER_BASE_URI}{{:read({name})}}\n{name}-value := @input-{name}/{name}\n",
        ));
        values.push(format!("{name}-value"));
    }
    source.push_str("~state := 0.0\nstate += ");
    source.push_str(&values.join(" + "));
    source.push_str("\noutput := state\n");
    runtime
        .load_production_source_program(&source, ResidentDurabilityPolicy::Volatile)
        .unwrap();
}

#[derive(Debug, Clone)]
struct MockDriverError {
    name: &'static str,
    message: String,
}
impl MechErrorKind for MockDriverError {
    fn name(&self) -> &str {
        self.name
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}
fn mock_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        MockDriverError {
            name,
            message: message.into(),
        },
        None,
    )
}

#[test]
fn build_attaches_and_starts_driven_input_drivers() {
    let state = Rc::new(RefCell::new(MockDriverState::default()));
    let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
    assert_eq!(state.borrow().attach_count, 1);
    assert_eq!(state.borrow().start_count, 0);
    assert_eq!(state.borrow().stop_count, 0);
    assert!(!state.borrow().live);
    activate_mock_inputs(&mut runtime, &["a"]);
    runtime.start_input_drivers().unwrap();
    assert_eq!(state.borrow().start_count, 1);
    assert!(state.borrow().live);
}

#[test]
fn compiler_never_attaches_starts_or_stops_input_drivers() {
    let state = Rc::new(RefCell::new(MockDriverState::default()));
    {
        let _compiler = compiler_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
        assert_eq!(state.borrow().attach_count, 0);
        assert_eq!(state.borrow().start_count, 0);
        assert_eq!(state.borrow().stop_count, 0);
        assert!(!state.borrow().live);
    }
    assert_eq!(state.borrow().stop_count, 0);
}

#[test]
fn queued_input_without_resident_owner_fails_without_dequeue() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime
        .ingress()
        .submit(RuntimeHostInput::single(
            RuntimeHostInputSource::new(MOCK_DRIVER_BASE_URI, "a").unwrap(),
            RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();

    let error = runtime.drain_host_inputs(1).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeInvalidOperation");
    assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
}

#[test]
fn attach_failure_closes_ingress_and_rolls_back_in_reverse_order() {
    let a = Rc::new(RefCell::new(MockDriverState::default()));
    let b = Rc::new(RefCell::new(MockDriverState {
        fail_attach: true,
        ..Default::default()
    }));
    let c = Rc::new(RefCell::new(MockDriverState::default()));
    let (drivers, events) =
        drivers_with_events(&[("a", a.clone()), ("b", b.clone()), ("c", c.clone())]);
    let error = format!("{:?}", runtime_with_drivers(drivers).unwrap_err());
    assert!(error.contains("MockAttachError"));
    assert_eq!(a.borrow().attach_count, 1);
    assert_eq!(b.borrow().attach_count, 1);
    assert_eq!(c.borrow().attach_count, 0);
    assert!(
        a.borrow()
            .attached_ingress
            .as_ref()
            .unwrap()
            .is_closed()
            .unwrap()
    );
    let stop_events: Vec<String> = events
        .borrow()
        .iter()
        .filter(|event| event.starts_with("stop:"))
        .cloned()
        .collect();
    assert_eq!(stop_events, vec!["stop:b", "stop:a"]);
    assert_eq!(a.borrow().stop_count, 1);
    assert_eq!(b.borrow().stop_count, 1);
    assert_eq!(c.borrow().stop_count, 0);
}

#[test]
fn start_failure_stops_only_drivers_started_by_the_call() {
    let a = Rc::new(RefCell::new(MockDriverState::default()));
    let b = Rc::new(RefCell::new(MockDriverState {
        fail_start: true,
        ..Default::default()
    }));
    let c = Rc::new(RefCell::new(MockDriverState::default()));
    let (drivers, events) =
        drivers_with_events(&[("a", a.clone()), ("b", b.clone()), ("c", c.clone())]);
    let mut runtime = runtime_with_drivers(drivers).unwrap();
    activate_mock_inputs(&mut runtime, &["a", "b", "c"]);
    let error = format!("{:?}", runtime.start_input_drivers().unwrap_err());
    assert!(error.contains("MockStartError"));
    let stop_events: Vec<String> = events
        .borrow()
        .iter()
        .filter(|event| event.starts_with("stop:"))
        .cloned()
        .collect();
    assert_eq!(stop_events, vec!["stop:a"]);
    assert_eq!((a.borrow().start_count, a.borrow().stop_count), (1, 1));
    assert_eq!((b.borrow().start_count, b.borrow().stop_count), (1, 0));
    assert_eq!((c.borrow().start_count, c.borrow().stop_count), (0, 0));
    assert!(!a.borrow().live && !b.borrow().live && !c.borrow().live);
}

#[test]
fn input_driver_panic_is_converted_and_started_drivers_are_stopped() {
    let a = Rc::new(RefCell::new(MockDriverState::default()));
    let b = Rc::new(RefCell::new(MockDriverState {
        panic_start: true,
        ..Default::default()
    }));
    let (drivers, events) = drivers_with_events(&[("a", a.clone()), ("b", b.clone())]);
    let mut runtime = runtime_with_drivers(drivers).unwrap();
    activate_mock_inputs(&mut runtime, &["a", "b"]);

    let error = runtime.start_input_drivers().unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate input driver start panic"));
    assert_eq!(
        events.borrow().as_slice(),
        ["attach:a", "attach:b", "start:a", "start:b", "stop:a"],
    );
    assert_eq!((a.borrow().start_count, a.borrow().stop_count), (1, 1));
    assert_eq!((b.borrow().start_count, b.borrow().stop_count), (1, 0));
    assert!(!runtime.is_poisoned());
    assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
}

#[test]
fn stop_input_drivers_attempts_every_driver() {
    let a = Rc::new(RefCell::new(MockDriverState::default()));
    let b = Rc::new(RefCell::new(MockDriverState {
        fail_stop: true,
        ..Default::default()
    }));
    let c = Rc::new(RefCell::new(MockDriverState::default()));
    let (drivers, events) =
        drivers_with_events(&[("a", a.clone()), ("b", b.clone()), ("c", c.clone())]);
    let mut runtime = runtime_with_drivers(drivers).unwrap();
    activate_mock_inputs(&mut runtime, &["a", "b", "c"]);
    runtime.start_input_drivers().unwrap();
    let error = format!("{:?}", runtime.stop_input_drivers().unwrap_err());
    assert!(error.contains("MockStopError"));
    assert_eq!(a.borrow().stop_count, 1);
    assert_eq!(b.borrow().stop_count, 1);
    assert_eq!(c.borrow().stop_count, 1);
    let stop_events: Vec<String> = events
        .borrow()
        .iter()
        .filter(|event| event.starts_with("stop:"))
        .cloned()
        .collect();
    assert_eq!(stop_events, vec!["stop:c", "stop:b", "stop:a"]);
}

#[test]
fn shutdown_closes_ingress_before_stopping_drivers() {
    let state = Rc::new(RefCell::new(MockDriverState::default()));
    let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
    activate_mock_inputs(&mut runtime, &["a"]);
    runtime.start_input_drivers().unwrap();
    let ingress = state.borrow().attached_ingress.clone().unwrap();
    runtime.shutdown().unwrap();
    assert_eq!(state.borrow().stop_count, 1);
    assert!(state.borrow().stop_observed_closed_ingress);
    drop(runtime);
    assert_eq!(state.borrow().stop_count, 1);
    let source = RuntimeHostInputSource::new("test-input://clock/ticks", "value").unwrap();
    let error = format!(
        "{:?}",
        ingress
            .submit(RuntimeHostInput::single(
                source,
                RuntimeHostInputValue::F64(1.0)
            ))
            .unwrap_err()
    );
    assert!(error.contains("RuntimeIngressClosed"));
}

#[test]
fn drop_stops_live_input_drivers() {
    let state = Rc::new(RefCell::new(MockDriverState::default()));
    {
        let mut runtime = runtime_with_drivers(vec![MockDriver::new("a", state.clone())]).unwrap();
        activate_mock_inputs(&mut runtime, &["a"]);
        runtime.start_input_drivers().unwrap();
        assert!(state.borrow().live);
    }
    assert_eq!(state.borrow().stop_count, 1);
    assert!(!state.borrow().live);
}
