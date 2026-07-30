use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mech_core::{GenericError, MResult, MechError, Ref, Value};
use mech_runtime::{
    ConfigValue, HostContextManifest, HostInstanceConfig, HostManifestConfig, MechRuntime,
    RunResourceGrantConfig, RuntimeBuilder, RuntimeHostFactory, RuntimeHostInput,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue, RuntimeHostInstallation,
    RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest, materialize_host_manifest,
};
use mech_syntax::ReplCommand;

use super::{CliOutcome, MechRepl, RuntimeReplInput, run_runtime_repl_event_loop};

const TEST_PROVIDER: &str = "replinput";
const TEST_INSTANCE: &str = "clock";
const TEST_CONTEXT: &str = "ticks";
const TEST_BASE_URI: &str = "replinput://clock/ticks";
const TEST_PATH: &str = "value";

#[derive(Debug, Default)]
struct TestDriverState {
    attach_count: usize,
    start_count: usize,
    stop_count: usize,
    live: bool,
    ingress: Option<RuntimeIngress>,
}

#[derive(Clone, Debug)]
struct TestDriver {
    state: Arc<Mutex<TestDriverState>>,
    submitted: RuntimeHostInputValue,
}

impl RuntimeHostInputDriver for TestDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == TEST_BASE_URI && source.path() == TEST_PATH
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut state = self.state.lock().unwrap();
        state.attach_count += 1;
        state.ingress = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        let ingress = {
            let mut state = self.state.lock().unwrap();
            state.start_count += 1;
            state.live = true;
            state
                .ingress
                .clone()
                .expect("test driver must be attached before start")
        };
        ingress.submit(RuntimeHostInput::single(
            RuntimeHostInputSource::new(TEST_BASE_URI, TEST_PATH)?,
            self.submitted.clone(),
        ))
    }

    fn stop(&mut self) -> MResult<()> {
        let mut state = self.state.lock().unwrap();
        state.stop_count += 1;
        state.live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.state.lock().unwrap().live
    }
}

#[derive(Debug)]
struct TestProvider;

impl RuntimeResourceProvider for TestProvider {
    fn scheme(&self) -> &str {
        TEST_PROVIDER
    }

    fn base_uris(&self) -> Vec<String> {
        vec![TEST_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri == TEST_BASE_URI && request.path == TEST_PATH {
            return Ok(Value::F64(Ref::new(1.0)));
        }
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "unexpected test provider read: {}/{}",
                    request.base_uri, request.path,
                ),
            },
            None,
        ))
    }
}

#[derive(Debug)]
struct TestFactory {
    manifest: HostManifestConfig,
    state: Arc<Mutex<TestDriverState>>,
    submitted: RuntimeHostInputValue,
}

impl TestFactory {
    fn new(state: Arc<Mutex<TestDriverState>>, submitted: RuntimeHostInputValue) -> Self {
        Self {
            manifest: HostManifestConfig {
                provider: TEST_PROVIDER.to_string(),
                contexts: vec![HostContextManifest {
                    name: TEST_CONTEXT.to_string(),
                    base_uri_template: format!("{TEST_PROVIDER}://{{instance}}/{TEST_CONTEXT}"),
                    operations: vec!["read".to_string()],
                }],
            },
            state,
            submitted,
        }
    }
}

impl RuntimeHostFactory for TestFactory {
    fn provider_name(&self) -> &str {
        TEST_PROVIDER
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
            resource_providers: vec![Box::new(TestProvider)],
            input_drivers: vec![Box::new(TestDriver {
                state: self.state.clone(),
                submitted: self.submitted.clone(),
            })],
        })
    }
}

fn runtime_with_driver(
    state: Arc<Mutex<TestDriverState>>,
    submitted: RuntimeHostInputValue,
) -> MechRuntime {
    RuntimeBuilder::new()
        .host_factory(Box::new(TestFactory::new(state, submitted)))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: TEST_INSTANCE.to_string(),
            provider: TEST_PROVIDER.to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: format!("{TEST_INSTANCE}/{TEST_CONTEXT}"),
            operations: vec!["read".to_string()],
            paths: vec![TEST_PATH.to_string()],
        })
        .build()
        .unwrap()
}

fn bind_live_input(runtime: &mut MechRuntime) {
    runtime
        .run_string(
            "@pulse := replinput://clock/ticks{:read(value)}\n\
             output := @pulse/value\n",
        )
        .unwrap();
    assert!(runtime.has_driven_live_input_bindings().unwrap());
}

#[test]
fn runtime_repl_drains_live_inputs_while_idle_and_stops_driver_once() {
    let state = Arc::new(Mutex::new(TestDriverState::default()));
    let mut runtime = runtime_with_driver(state.clone(), RuntimeHostInputValue::F64(9.0));
    bind_live_input(&mut runtime);
    let mut repl = MechRepl::from_runtime(runtime);
    let (sender, input) = crossbeam_channel::unbounded();
    let idle_drain_completed = Arc::new(AtomicBool::new(false));
    let idle_for_sender = idle_drain_completed.clone();
    let sender_thread = thread::spawn(move || {
        for _ in 0..1_000 {
            if idle_for_sender.load(Ordering::SeqCst) {
                sender
                    .send(RuntimeReplInput::Line("observed := output\n".to_string()))
                    .unwrap();
                sender
                    .send(RuntimeReplInput::Line(":quit\n".to_string()))
                    .unwrap();
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("runtime REPL never completed an idle host-input drain");
    });
    let mut before_input = || {};
    let mut outputs = Vec::new();
    let mut output = |value| outputs.push(value);
    let mut after_command = || {};
    let mut after_idle_drain = || {
        idle_drain_completed.store(true, Ordering::SeqCst);
    };

    let outcome = run_runtime_repl_event_loop(
        &mut repl,
        &input,
        Duration::from_millis(1),
        &mut before_input,
        &mut output,
        &mut after_command,
        &mut after_idle_drain,
    )
    .unwrap();

    sender_thread.join().unwrap();
    assert!(matches!(outcome, CliOutcome::Exit(0)));
    assert!(idle_drain_completed.load(Ordering::SeqCst));
    assert!(
        outputs.iter().any(|value| value.contains('9')),
        "subsequent REPL command did not observe the live update: {outputs:?}",
    );
    assert!(
        repl.execute_repl_command(ReplCommand::Whos(vec!["observed".to_string()]))
            .unwrap()
            .contains('9'),
    );
    let state = state.lock().unwrap();
    assert_eq!(state.attach_count, 1);
    assert_eq!(state.start_count, 1);
    assert_eq!(state.stop_count, 1);
    assert!(!state.live);
}

#[test]
fn runtime_repl_drain_failure_still_stops_driver() {
    let state = Arc::new(Mutex::new(TestDriverState::default()));
    let mut runtime = runtime_with_driver(
        state.clone(),
        RuntimeHostInputValue::String("wrong kind".to_string()),
    );
    bind_live_input(&mut runtime);
    let mut repl = MechRepl::from_runtime(runtime);
    let (_sender, input) = crossbeam_channel::unbounded();
    let mut before_input = || {};
    let mut output = |_value| {};
    let mut after_command = || {};
    let mut after_idle_drain = || {};

    let error = match run_runtime_repl_event_loop(
        &mut repl,
        &input,
        Duration::from_millis(1),
        &mut before_input,
        &mut output,
        &mut after_command,
        &mut after_idle_drain,
    ) {
        Ok(_) => panic!("invalid live input unexpectedly completed the REPL loop"),
        Err(error) => error,
    };

    assert!(
        error.kind_name().contains("KindMismatch"),
        "unexpected drain error: {error:?}",
    );
    let state = state.lock().unwrap();
    assert_eq!(state.start_count, 1);
    assert_eq!(state.stop_count, 1);
    assert!(!state.live);
}

#[test]
fn non_live_runtime_repl_does_not_start_input_drivers() {
    let state = Arc::new(Mutex::new(TestDriverState::default()));
    let runtime = runtime_with_driver(state.clone(), RuntimeHostInputValue::F64(9.0));
    assert!(!runtime.has_driven_live_input_bindings().unwrap());
    let mut repl = MechRepl::from_runtime(runtime);
    let (sender, input) = crossbeam_channel::unbounded();
    sender
        .send(RuntimeReplInput::Line(":quit\n".to_string()))
        .unwrap();
    let mut before_input = || {};
    let mut output = |_value| {};
    let mut after_command = || {};
    let mut after_idle_drain = || {};

    let outcome = run_runtime_repl_event_loop(
        &mut repl,
        &input,
        Duration::from_millis(1),
        &mut before_input,
        &mut output,
        &mut after_command,
        &mut after_idle_drain,
    )
    .unwrap();

    assert!(matches!(outcome, CliOutcome::Exit(0)));
    let state = state.lock().unwrap();
    assert_eq!(state.attach_count, 1);
    assert_eq!(state.start_count, 0);
    assert_eq!(state.stop_count, 1);
    assert!(!state.live);
}
