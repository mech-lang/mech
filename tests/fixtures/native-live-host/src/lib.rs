#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Ref};
use mech_runtime::{
    ConfigValue, HostContextManifest, HostManifestConfig, MaterializedHostInterface,
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeHostFactory, RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, materialize_host_manifest,
};

pub const TEST_LIVE_PROVIDER: &str = "test-live";
pub const TEST_LIVE_INSTANCE: &str = "clock";
pub const TEST_LIVE_CONTEXT: &str = "clock";
pub const TEST_LIVE_BASE_URI: &str = "test-live://clock/clock";
pub const TEST_LIVE_PATH: &str = "value";
pub const TEST_LIVE_OUTPUT_CONTEXT: &str = "output";
pub const TEST_LIVE_OUTPUT_BASE_URI: &str = "test-live://clock/output";
pub const TEST_LIVE_TUPLE_PATH: &str = "tuple";
pub const TEST_LIVE_RECORD_PATH: &str = "frame";
pub const TEST_LIVE_START_MARKER_ENV: &str = "MECH_TEST_LIVE_START_MARKER";
pub const TEST_LIVE_STOP_MARKER_ENV: &str = "MECH_TEST_LIVE_STOP_MARKER";
pub const TEST_LIVE_FAIL_AFTER_START_ENV: &str = "MECH_TEST_LIVE_FAIL_AFTER_START";

fn write_process_marker(variable: &str) -> MResult<()> {
    let Some(path) = std::env::var_os(variable) else {
        return Ok(());
    };
    fs::write(&path, b"observed\n").map_err(|failure| {
        error(
            "TestLiveMarkerFailed",
            format!("failed to write {}: {failure}", path.to_string_lossy()),
        )
    })
}

pub fn test_live_manifest() -> MResult<HostManifestConfig> {
    Ok(HostManifestConfig {
        provider: TEST_LIVE_PROVIDER.to_owned(),
        contexts: vec![
            HostContextManifest {
                name: TEST_LIVE_CONTEXT.to_owned(),
                base_uri_template: "test-live://{instance}/clock".to_owned(),
                operations: vec!["read".to_owned()],
            },
            HostContextManifest {
                name: TEST_LIVE_OUTPUT_CONTEXT.to_owned(),
                base_uri_template: "test-live://{instance}/output".to_owned(),
                operations: vec!["write".to_owned()],
            },
        ],
    })
}

pub fn empty_settings() -> ConfigValue {
    ConfigValue::Map(BTreeMap::new())
}

#[derive(Debug, Default)]
struct DriverState {
    ingress: Mutex<Option<RuntimeIngress>>,
    attached: AtomicBool,
    live: AtomicBool,
    attach_count: AtomicUsize,
    start_count: AtomicUsize,
    stop_count: AtomicUsize,
    submit_count: AtomicUsize,
    writes: Mutex<Vec<ObservedResourceWrite>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObservedResourceValue {
    F64(f64),
    Tuple(Vec<ObservedResourceValue>),
    Record(BTreeMap<String, ObservedResourceValue>),
    Other(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservedResourceWrite {
    pub path: String,
    pub value: ObservedResourceValue,
}

#[derive(Clone, Debug, Default)]
pub struct TestLiveDriverHandle {
    state: Arc<DriverState>,
}

impl TestLiveDriverHandle {
    pub fn is_attached(&self) -> bool {
        self.state.attached.load(Ordering::SeqCst)
    }

    pub fn is_live(&self) -> bool {
        self.state.live.load(Ordering::SeqCst)
    }

    pub fn attach_count(&self) -> usize {
        self.state.attach_count.load(Ordering::SeqCst)
    }

    pub fn start_count(&self) -> usize {
        self.state.start_count.load(Ordering::SeqCst)
    }

    pub fn stop_count(&self) -> usize {
        self.state.stop_count.load(Ordering::SeqCst)
    }

    pub fn submit_count(&self) -> usize {
        self.state.submit_count.load(Ordering::SeqCst)
    }

    pub fn writes(&self) -> Vec<ObservedResourceWrite> {
        self.state
            .writes
            .lock()
            .expect("test-live write observation lock poisoned")
            .clone()
    }

    pub fn clear_writes(&self) {
        self.state
            .writes
            .lock()
            .expect("test-live write observation lock poisoned")
            .clear();
    }

    pub fn submit(&self, value: f64) -> MResult<()> {
        if !self.is_live() {
            return Err(error(
                "TestLiveDriverNotLive",
                "test-live input driver must be started before submission",
            ));
        }
        let ingress = self
            .state
            .ingress
            .lock()
            .map_err(|_| {
                error(
                    "TestLiveDriverUnavailable",
                    "test-live ingress lock poisoned",
                )
            })?
            .clone()
            .ok_or_else(|| {
                error(
                    "TestLiveDriverNotAttached",
                    "test-live input driver has no ingress attachment",
                )
            })?;
        ingress.submit(RuntimeHostInput::single(
            test_live_source()?,
            RuntimeHostInputValue::F64(value),
        ))?;
        self.state.submit_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub fn test_live_source() -> MResult<RuntimeHostInputSource> {
    RuntimeHostInputSource::new(TEST_LIVE_BASE_URI, TEST_LIVE_PATH)
}

#[derive(Clone, Debug)]
pub struct TestLiveInputDriver {
    handle: TestLiveDriverHandle,
}

impl TestLiveInputDriver {
    fn new(handle: TestLiveDriverHandle) -> Self {
        Self { handle }
    }
}

impl RuntimeHostInputDriver for TestLiveInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == TEST_LIVE_BASE_URI && source.path() == TEST_LIVE_PATH
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut attached = self.handle.state.ingress.lock().map_err(|_| {
            error(
                "TestLiveDriverUnavailable",
                "test-live ingress lock poisoned",
            )
        })?;
        if attached.is_some() {
            return Err(error(
                "TestLiveDriverAlreadyAttached",
                "test-live input driver is already attached",
            ));
        }
        *attached = Some(ingress);
        self.handle.state.attached.store(true, Ordering::SeqCst);
        self.handle
            .state
            .attach_count
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        if !self.handle.is_attached() {
            return Err(error(
                "TestLiveDriverNotAttached",
                "test-live input driver must be attached before start",
            ));
        }
        if !self.handle.state.live.swap(true, Ordering::SeqCst) {
            self.handle.state.start_count.fetch_add(1, Ordering::SeqCst);
        }
        write_process_marker(TEST_LIVE_START_MARKER_ENV)?;
        if std::env::var_os(TEST_LIVE_FAIL_AFTER_START_ENV).is_some() {
            let ingress = self
                .handle
                .state
                .ingress
                .lock()
                .map_err(|_| {
                    error(
                        "TestLiveDriverUnavailable",
                        "test-live ingress lock poisoned",
                    )
                })?
                .clone()
                .ok_or_else(|| {
                    error(
                        "TestLiveDriverNotAttached",
                        "test-live input driver has no ingress attachment",
                    )
                })?;
            ingress.submit(RuntimeHostInput::single(
                test_live_source()?,
                RuntimeHostInputValue::String("deliberately-invalid-live-input".to_owned()),
            ))?;
        }
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        if self.handle.state.live.swap(false, Ordering::SeqCst) {
            self.handle.state.stop_count.fetch_add(1, Ordering::SeqCst);
            write_process_marker(TEST_LIVE_STOP_MARKER_ENV)?;
        }
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.handle.is_live()
    }
}

#[derive(Debug)]
struct TestLiveResourceProvider {
    handle: TestLiveDriverHandle,
}

impl TestLiveResourceProvider {
    fn planned_value(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if request.base_uri != TEST_LIVE_BASE_URI
            || request.path != TEST_LIVE_PATH
            || request.context_name != TEST_LIVE_CONTEXT
        {
            return Err(error(
                "TestLiveResourceUnknown",
                format!(
                    "unsupported test-live resource {} / {} ({})",
                    request.base_uri, request.path, request.context_name,
                ),
            ));
        }
        Ok(LegacyValue::F64(Ref::new(0.0)))
    }
}

impl RuntimeResourceProvider for TestLiveResourceProvider {
    fn scheme(&self) -> &str {
        TEST_LIVE_PROVIDER
    }

    fn base_uris(&self) -> Vec<String> {
        vec![
            TEST_LIVE_BASE_URI.to_owned(),
            TEST_LIVE_OUTPUT_BASE_URI.to_owned(),
        ]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.planned_value(request)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.planned_value(request)
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != TEST_LIVE_OUTPUT_BASE_URI
            || request.context_name != TEST_LIVE_OUTPUT_CONTEXT
            || request.operation.name() != "write"
            || request.intent != RuntimeResourceWriteIntent::Send
            || !matches!(
                request.path.as_str(),
                TEST_LIVE_TUPLE_PATH | TEST_LIVE_RECORD_PATH
            )
        {
            return Err(error(
                "TestLiveResourceWriteUnknown",
                format!(
                    "unsupported test-live write {} / {} ({})",
                    request.base_uri, request.path, request.context_name,
                ),
            ));
        }
        Ok(())
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
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            RecordObservedWrite {
                writes: Arc::clone(&self.handle.state),
                observation: ObservedResourceWrite {
                    path: request.path,
                    value: observe_value(&request.value),
                },
            },
        )))
    }
}

fn observe_value(value: &LegacyValue) -> ObservedResourceValue {
    match value {
        LegacyValue::MutableReference(value) => observe_value(&value.borrow()),
        LegacyValue::Typed(value, _) => observe_value(value),
        LegacyValue::F64(value) => ObservedResourceValue::F64(*value.borrow()),
        LegacyValue::Tuple(value) => ObservedResourceValue::Tuple(
            value
                .borrow()
                .elements
                .iter()
                .map(|value| observe_value(value))
                .collect(),
        ),
        LegacyValue::Record(value) => {
            let value = value.borrow();
            ObservedResourceValue::Record(
                value
                    .data
                    .iter()
                    .map(|(id, field)| {
                        (
                            value
                                .field_names
                                .get(id)
                                .cloned()
                                .unwrap_or_else(|| format!("{id}")),
                            observe_value(field),
                        )
                    })
                    .collect(),
            )
        }
        other => ObservedResourceValue::Other(format!("{other}")),
    }
}

#[derive(Debug)]
struct RecordObservedWrite {
    writes: Arc<DriverState>,
    observation: ObservedResourceWrite,
}

impl RuntimeAfterCommitEffect for RecordObservedWrite {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: TEST_LIVE_PROVIDER.to_owned(),
            },
            "record-write",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        self.writes
            .writes
            .lock()
            .map_err(|_| error("TestLiveWriteUnavailable", "test-live write lock poisoned"))?
            .push(self.observation.clone());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TestLiveHostFactory {
    manifest: HostManifestConfig,
    driver: TestLiveDriverHandle,
}

impl TestLiveHostFactory {
    pub fn new() -> MResult<(Self, TestLiveDriverHandle)> {
        let driver = TestLiveDriverHandle::default();
        Ok((
            Self {
                manifest: test_live_manifest()?,
                driver: driver.clone(),
            },
            driver,
        ))
    }

    /// Construct the production-shaped factory used by generated native
    /// applications. Tests that drive live input retain the handle from `new`.
    pub fn native() -> MResult<Self> {
        Self::new().map(|(factory, _driver)| factory)
    }

    fn interface(&self, instance_name: &str) -> MResult<MaterializedHostInterface> {
        materialize_host_manifest(instance_name, &self.manifest)
    }
}

impl RuntimeHostFactory for TestLiveHostFactory {
    fn provider_name(&self) -> &str {
        TEST_LIVE_PROVIDER
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        validate_settings(instance_name, settings)
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        Ok(RuntimeHostInstallation {
            interface: self.interface(instance_name)?,
            resource_providers: vec![Box::new(TestLiveResourceProvider {
                handle: self.driver.clone(),
            })],
            input_drivers: vec![Box::new(TestLiveInputDriver::new(self.driver.clone()))],
        })
    }
}

pub fn validate_settings(_instance_name: &str, settings: &ConfigValue) -> MResult<()> {
    match settings {
        ConfigValue::Map(settings) if settings.is_empty() => Ok(()),
        _ => Err(error(
            "TestLiveSettingsInvalid",
            "test-live settings must be an empty map",
        )),
    }
}

#[derive(Clone, Debug)]
struct TestLiveHostError {
    name: &'static str,
    message: String,
}

impl MechErrorKind for TestLiveHostError {
    fn name(&self) -> &str {
        self.name
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

fn error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        TestLiveHostError {
            name,
            message: message.into(),
        },
        None,
    )
}
