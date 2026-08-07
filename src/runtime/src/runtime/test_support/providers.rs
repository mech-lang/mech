use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use mech_core::{FunctionCatalogBuilder, MResult, MechError, MechErrorKind, Ref, Value};

use super::super::{MechRuntime, RuntimeBuilder};
use crate::{
    PlannedPureHostFunction, PreparedRuntimeEffect, RegisteredHostFunction,
    RuntimeAfterCommitEffect, RuntimeCallContext, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, RuntimeValueSnapshot,
};

pub(crate) const TEST_OUTPUT_BASE_URI: &str = "test://effects/output";

#[derive(Debug, Clone)]
struct TestFixtureError(String);

impl MechErrorKind for TestFixtureError {
    fn name(&self) -> &str {
        "TestFixtureError"
    }

    fn message(&self) -> String {
        self.0.clone()
    }
}

pub(crate) struct TestAfterCommitEffect {
    metadata: RuntimeEffectMetadata,
    delivery: Box<dyn FnMut() -> MResult<()>>,
}

impl TestAfterCommitEffect {
    pub(crate) fn new(
        metadata: RuntimeEffectMetadata,
        delivery: impl FnMut() -> MResult<()> + 'static,
    ) -> Self {
        Self {
            metadata,
            delivery: Box::new(delivery),
        }
    }
}

impl std::fmt::Debug for TestAfterCommitEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TestAfterCommitEffect")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

impl RuntimeAfterCommitEffect for TestAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        self.metadata.clone()
    }

    fn deliver(&mut self) -> MResult<()> {
        (self.delivery)()
    }
}

#[derive(Debug, Default)]
pub(crate) struct TestResourceProvider {
    values: BTreeMap<String, BTreeMap<String, Value>>,
}

impl TestResourceProvider {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_value(mut self, base_uri: &str, path: &str, value: Value) -> Self {
        assert!(
            base_uri.starts_with("test://"),
            "test fixture resource must use test://",
        );
        assert!(!path.is_empty(), "test fixture path must not be empty");
        self.values
            .entry(base_uri.to_string())
            .or_default()
            .insert(path.to_string(), value);
        self
    }
}

impl RuntimeResourceProvider for TestResourceProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.values
            .get(&request.base_uri)
            .and_then(|paths| paths.get(&request.path))
            .cloned()
            .ok_or_else(|| {
                MechError::new(
                    TestFixtureError(format!(
                        "missing test resource {} / {}",
                        request.base_uri, request.path,
                    )),
                    None,
                )
            })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RecordingTestOutput {
    lines: Rc<RefCell<Vec<String>>>,
}

impl RecordingTestOutput {
    pub(crate) fn lines(&self) -> Vec<String> {
        self.lines.borrow().clone()
    }
}

#[derive(Debug)]
pub(crate) struct TestOutputProvider {
    backend: RecordingTestOutput,
}

impl TestOutputProvider {
    pub(crate) fn new(backend: RecordingTestOutput) -> Self {
        Self { backend }
    }
}

impl RuntimeResourceProvider for TestOutputProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![TEST_OUTPUT_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(MechError::new(
            TestFixtureError(format!(
                "test output is write-only: {} / {}",
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
                TestFixtureError(format!(
                    "invalid test output write: {} / {}",
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
        let lines = self.backend.lines.clone();
        let rendered = format!("{}", request.value);
        let metadata = RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "test".to_string(),
            },
            "send",
        )
        .with_resource(format!("{}/{}", request.base_uri, request.path,));
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            TestAfterCommitEffect::new(metadata, move || {
                lines.borrow_mut().push(rendered.clone());
                Ok(())
            }),
        )))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DeliberateHostCallError;

impl MechErrorKind for DeliberateHostCallError {
    fn name(&self) -> &str {
        "DeliberateHostCallError"
    }

    fn message(&self) -> String {
        "deliberate host call failure".to_string()
    }
}

pub(crate) fn test_provider_with(base_uri: &str, path: &str, value: f64) -> TestResourceProvider {
    TestResourceProvider::new().with_value(base_uri, path, Value::F64(Ref::new(value)))
}

pub(crate) fn test_runtime_builder() -> RuntimeBuilder {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut catalog).unwrap();
    mech_engine::install_intrinsic_source(&mut catalog).unwrap();
    RuntimeBuilder::new().function_catalog(std::sync::Arc::new(catalog.build().unwrap()))
}

pub(crate) fn test_runtime(provider: TestResourceProvider) -> MechRuntime {
    test_runtime_builder()
        .resource_provider(Box::new(provider))
        .build()
        .unwrap()
}

pub(crate) fn test_runtime_with_host(
    provider: TestResourceProvider,
    function: impl Into<RegisteredHostFunction>,
) -> MechRuntime {
    test_runtime_builder()
        .resource_provider(Box::new(provider))
        .host_function(function)
        .unwrap()
        .build()
        .unwrap()
}

pub(crate) fn test_runtime_with_output(
    provider: TestResourceProvider,
) -> (MechRuntime, RecordingTestOutput) {
    let output = RecordingTestOutput::default();
    let runtime = test_runtime_builder()
        .resource_provider(Box::new(provider))
        .resource_provider(Box::new(TestOutputProvider::new(output.clone())))
        .build()
        .unwrap();
    (runtime, output)
}

pub(crate) fn test_runtime_with_output_host(
    provider: TestResourceProvider,
    function: impl Into<RegisteredHostFunction>,
) -> (MechRuntime, RecordingTestOutput) {
    let output = RecordingTestOutput::default();
    let runtime = test_runtime_builder()
        .resource_provider(Box::new(provider))
        .resource_provider(Box::new(TestOutputProvider::new(output.clone())))
        .host_function(function)
        .unwrap()
        .build()
        .unwrap();
    (runtime, output)
}

pub(crate) fn sleep_host(name: &str) -> RegisteredHostFunction {
    PlannedPureHostFunction::new(
        name,
        |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
            Ok(args
                .first()
                .cloned()
                .unwrap_or_else(RuntimeValueSnapshot::empty))
        },
        move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
            thread::sleep(Duration::from_millis(5));

            let argument = args.first().map(RuntimeValueSnapshot::to_value);
            let value = match argument {
                Some(Value::F64(value)) => Value::F64(Ref::new(*value.borrow())),
                Some(Value::MutableReference(value)) => match &*value.borrow() {
                    Value::F64(value) => Value::F64(Ref::new(*value.borrow())),
                    other => panic!("expected f64 mutable reference, got {other:?}"),
                },
                other => panic!("expected f64 argument, got {other:?}"),
            };
            RuntimeValueSnapshot::try_capture(&value)
        },
    )
    .into()
}
