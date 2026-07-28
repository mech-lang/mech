use std::cell::RefCell;
use std::rc::Rc;

use mech_core::{MResult, MechError, MechErrorKind, Ref, Value};

use super::super::{MechRuntime, RuntimeBuilder};
use super::scheduling::{
    activation_plan_snapshot, activation_send_count, apply_f64_input, recorded_f64,
};
use crate::runtime::test_support::{
    capabilities::{grant_read, grant_read_to, grant_resource, grant_write},
    providers::{
        TEST_OUTPUT_BASE_URI, TestAfterCommitEffect, TestResourceProvider, test_runtime_with_output,
    },
    values::{f64_value, symbol_value},
};
use crate::{
    CapabilityRequest, PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeEventKind, RuntimeHostInput, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue, RuntimeIngress,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

const TEST_TIME_BASE_URI: &str = "time://clock/clock";

const TEST_TIME_PATHS: [&str; 5] = ["unix-ms", "hour", "minute", "second", "millisecond"];

fn test_time_source_matches(source: &RuntimeHostInputSource) -> bool {
    source.base_uri() == TEST_TIME_BASE_URI && TEST_TIME_PATHS.contains(&source.path())
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TimeSnapshot {
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
    fn scheme(&self) -> &str {
        "time"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![TEST_TIME_BASE_URI.to_string()]
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let snapshot = *self.snapshot.borrow();
        let value = match request.path.as_str() {
            "unix-ms" => snapshot.unix_ms,
            "hour" => snapshot.hour,
            "minute" => snapshot.minute,
            "second" => snapshot.second,
            "millisecond" => snapshot.millisecond,
            other => {
                return Err(MechError::new(
                    PersistentSendTestError(format!("unknown time path {other}")),
                    None,
                ));
            }
        };
        Ok(Value::F64(Ref::new(value)))
    }
}

#[derive(Clone, Debug)]
pub(super) struct ManualTimeInputDriver {
    snapshot: Rc<RefCell<TimeSnapshot>>,
    ingress: Rc<RefCell<Option<RuntimeIngress>>>,
    live: Rc<RefCell<bool>>,
}

impl ManualTimeInputDriver {
    fn new(snapshot: Rc<RefCell<TimeSnapshot>>) -> Self {
        Self {
            snapshot,
            ingress: Rc::new(RefCell::new(None)),
            live: Rc::new(RefCell::new(false)),
        }
    }

    fn publish(&self, snapshot: TimeSnapshot) -> MResult<()> {
        *self.snapshot.borrow_mut() = snapshot;
        let ingress = self.ingress.borrow().clone().ok_or_else(|| {
            MechError::new(
                PersistentSendTestError("driver is not attached".to_string()),
                None,
            )
        })?;
        ingress.submit(RuntimeHostInput::new(vec![
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "unix-ms")?,
                value: RuntimeHostInputValue::F64(snapshot.unix_ms),
            },
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "hour")?,
                value: RuntimeHostInputValue::F64(snapshot.hour),
            },
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "minute")?,
                value: RuntimeHostInputValue::F64(snapshot.minute),
            },
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "second")?,
                value: RuntimeHostInputValue::F64(snapshot.second),
            },
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(TEST_TIME_BASE_URI, "millisecond")?,
                value: RuntimeHostInputValue::F64(snapshot.millisecond),
            },
        ])?)
    }
}

impl RuntimeHostInputDriver for ManualTimeInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        test_time_source_matches(source)
    }
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        *self.ingress.borrow_mut() = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        *self.live.borrow_mut() = true;
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        *self.live.borrow_mut() = false;
        Ok(())
    }
    fn is_live(&self) -> bool {
        *self.live.borrow()
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct RecordingConsoleBackend {
    lines: Rc<RefCell<Vec<String>>>,
    fail_next: Rc<RefCell<Option<String>>>,
}

impl RecordingConsoleBackend {
    pub(super) fn lines(&self) -> Vec<String> {
        self.lines.borrow().clone()
    }
    fn fail_next(&self, reason: impl Into<String>) {
        *self.fail_next.borrow_mut() = Some(reason.into());
    }
}

#[derive(Debug)]
struct ConsoleResourceProvider {
    backend: RecordingConsoleBackend,
}

impl RuntimeResourceProvider for ConsoleResourceProvider {
    fn scheme(&self) -> &str {
        "console"
    }
    fn base_uris(&self) -> Vec<String> {
        vec!["console://console/output".to_string()]
    }
    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(MechError::new(
            PersistentSendTestError("console is write-only".to_string()),
            None,
        ))
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.path == "line" && request.intent == RuntimeResourceWriteIntent::Send {
            Ok(())
        } else {
            Err(MechError::new(
                PersistentSendTestError("bad console write".to_string()),
                None,
            ))
        }
    }
    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        let backend = self.backend.clone();
        let rendered = format!("{}", request.value);
        let metadata = RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "console".to_string(),
            },
            "send",
        )
        .with_resource(format!("{}/{}", request.base_uri, request.path,));
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            TestAfterCommitEffect::new(metadata, move || {
                if let Some(reason) = backend.fail_next.borrow_mut().take() {
                    return Err(MechError::new(PersistentSendTestError(reason), None));
                }
                backend.lines.borrow_mut().push(rendered.clone());
                Ok(())
            }),
        )))
    }
}

#[derive(Debug, Clone)]
struct PersistentSendTestError(String);

impl MechErrorKind for PersistentSendTestError {
    fn name(&self) -> &str {
        "PersistentSendTestError"
    }
    fn message(&self) -> String {
        self.0.clone()
    }
}

const TIME_PATHS: &[&str] = &["unix-ms", "hour", "minute", "second", "millisecond"];

fn grant_write_to(runtime: &mut MechRuntime, subject: &str, resource: &str, path: &str) {
    grant_resource(
        runtime,
        subject,
        resource,
        RuntimeCapabilityOperation::Write,
        &[path],
    );
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
    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        let rendered = format!("{}", request.value);
        let backend = self.backend.clone();
        let metadata = RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "test".to_string(),
            },
            "send",
        )
        .with_resource(format!("{}/{}", request.base_uri, request.path));
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            TestAfterCommitEffect::new(metadata, move || {
                let attempt_number = {
                    let mut attempts = backend.attempts.borrow_mut();
                    attempts.push(rendered.clone());
                    attempts.len()
                };
                let should_fail = {
                    let mut fail_once_at = backend.fail_once_at.borrow_mut();
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
                backend.successes.borrow_mut().push(rendered.clone());
                Ok(())
            }),
        )))
    }
}

pub(super) fn snapshot(hour: f64, minute: f64, second: f64, millisecond: f64) -> TimeSnapshot {
    TimeSnapshot {
        unix_ms: hour * 3_600_000.0 + minute * 60_000.0 + second * 1000.0 + millisecond,
        hour,
        minute,
        second,
        millisecond,
    }
}

fn grant(
    runtime: &mut MechRuntime,
    resource: &str,
    operation: RuntimeCapabilityOperation,
    paths: &[&str],
) {
    let subject = runtime.runtime_context().unwrap().subject;
    grant_resource(runtime, &subject, resource, operation, paths);
}

pub(super) fn runtime_with_console(
    initial: TimeSnapshot,
    fail_next: bool,
) -> (MechRuntime, ManualTimeInputDriver, RecordingConsoleBackend) {
    let shared = Rc::new(RefCell::new(initial));
    let console = RecordingConsoleBackend::default();
    if fail_next {
        console.fail_next("intentional console failure");
    }
    let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(TimeResourceProvider {
            snapshot: shared.clone(),
        }) as Box<dyn RuntimeResourceProvider>)
        .resource_provider(Box::new(ConsoleResourceProvider {
            backend: console.clone(),
        }) as Box<dyn RuntimeResourceProvider>)
        .build()
        .unwrap();
    grant(
        &mut runtime,
        "time://clock/clock",
        RuntimeCapabilityOperation::Read,
        TIME_PATHS,
    );
    grant(
        &mut runtime,
        "console://console/output",
        RuntimeCapabilityOperation::Write,
        &["line"],
    );
    let mut driver = ManualTimeInputDriver::new(shared);
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    (runtime, driver, console)
}

fn load(runtime: &mut MechRuntime, send_expression: &str) {
    let source = format!(
        r#"@out := console://console/output{{:write(line)}}
@clock := time://clock/clock{{:read(unix-ms), :read(hour), :read(minute), :read(second), :read(millisecond)}}
unix-ms := @clock/unix-ms
hour := @clock/hour
minute := @clock/minute
second := @clock/second
millisecond := @clock/millisecond
scalar-output := hour + minute
clock-output := (hour, minute, second)
@out/line <- {send_expression}
"#
    );
    runtime.run_string(&source).unwrap();
}

pub(super) fn publish(
    runtime: &mut MechRuntime,
    driver: &ManualTimeInputDriver,
    snapshot: TimeSnapshot,
) {
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
        .resource_provider(Box::new(TimeResourceProvider {
            snapshot: shared.clone(),
        }) as Box<dyn RuntimeResourceProvider>)
        .resource_provider(Box::new(ConsoleResourceProvider {
            backend: console.clone(),
        }) as Box<dyn RuntimeResourceProvider>)
        .build()
        .unwrap();
    let subject = "task:live-custom";
    grant_resource(
        &mut runtime,
        subject,
        "time://clock/clock",
        RuntimeCapabilityOperation::Read,
        TIME_PATHS,
    );
    grant_resource(
        &mut runtime,
        subject,
        "console://console/output",
        RuntimeCapabilityOperation::Write,
        &["line"],
    );

    let mut context = runtime.runtime_context().unwrap().with_subject(subject);
    runtime
        .run_string_with_context(
            &mut context,
            r#"@out := console://console/output{:write(line)}
@clock := time://clock/clock{:read(hour)}
hour := @clock/hour
output := hour + 1
@out/line <- output
"#,
        )
        .unwrap();
    assert_eq!(console.lines().len(), 1);
    *shared.borrow_mut() = snapshot(1.0, 9.0, 3.0, 4.0);
    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("time://clock/clock", "hour").unwrap(),
            RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();
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
    assert!(
        lines[1].contains("30"),
        "expected updated scalar in {:?}",
        lines
    );
}

#[test]
fn persistent_send_tuple_reads_new_values_after_solve() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "clock-output");
    publish(&mut runtime, &driver, snapshot(10.0, 20.0, 30.0, 40.0));
    let lines = console.lines();
    assert!(
        lines[1].contains("10"),
        "expected updated tuple in {:?}",
        lines
    );
    assert!(
        lines[1].contains("20"),
        "expected updated tuple in {:?}",
        lines
    );
    assert!(
        lines[1].contains("30"),
        "expected updated tuple in {:?}",
        lines
    );
}

#[test]
fn persistent_send_delivery_failure_keeps_committed_drain_healthy() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    load(&mut runtime, "scalar-output");
    console.fail_next("expected drain failure");
    driver.publish(snapshot(5.0, 6.0, 7.0, 8.0)).unwrap();
    let outcomes = runtime.drain_host_inputs(1).unwrap();
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].turn.is_some());
    assert!(!runtime.is_poisoned());
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
        matches!(
          &event.kind,
          RuntimeEventKind::EffectDeliveryFailed { message, .. }
            if message.contains("expected drain failure")
        )
    }));
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
    assert!(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                &default_subject,
                "read",
                "test://render/timer/tick",
            ))
            .is_err()
    );
    assert!(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                &default_subject,
                "write",
                format!("{TEST_OUTPUT_BASE_URI}/line"),
            ))
            .is_err()
    );
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
fn activation_send_delivery_failure_continues_and_registration_is_retained() {
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
    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
            RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();
    assert!(outcome.turn.is_some());
    assert!(!runtime.is_poisoned());
    assert!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::EffectDeliveryFailed { .. }) })
    );
    assert_eq!(
        output.attempts(),
        vec![
            "\"first\"".to_string(),
            "\"second\"".to_string(),
            "\"third\"".to_string()
        ]
    );
    assert_eq!(
        output.successes(),
        vec!["\"first\"".to_string(), "\"third\"".to_string()]
    );
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
            "\"third\"".to_string(),
            "\"first\"".to_string(),
            "\"second\"".to_string(),
            "\"third\"".to_string()
        ]
    );
    assert_eq!(
        output.successes(),
        vec![
            "\"first\"".to_string(),
            "\"third\"".to_string(),
            "\"first\"".to_string(),
            "\"second\"".to_string(),
            "\"third\"".to_string()
        ]
    );
    assert_eq!(activation_send_count(&runtime), 3);
    assert_eq!(activation_plan_snapshot(&runtime), plan_before);
}
