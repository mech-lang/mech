//! Deterministic providers shared by resident external tests.
//!
//! This module is never present in normal product builds.

use std::sync::{Arc, LazyLock, Mutex};

use mech_core::{
    AccessMode, ChangeDetectionPolicy, DeliveryMode, EffectContract, EffectDeliveryPolicy,
    ExternalInteraction, IdempotencyRequirement, InputPortLayout, InputPortPolicy, MResult,
    MechError, ObservationContract, ObservationReplayPolicy, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, ShapeRule, TransactionalEffectProtocol,
    TransactionalExternalContract, Value, ValueData,
};

use crate::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeCompensatableEffect, RuntimeEffectCost,
    RuntimeEffectId, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostInputValue,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteCommand,
    RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
};

pub static TEST_OBSERVATION_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([])),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: mech_core::AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    });

pub static TEST_SCENE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    external_contract(ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::IdempotentRetry,
        idempotency: IdempotencyRequirement::Required,
    }))
});

pub static TEST_TRANSACTIONAL_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| {
        external_contract(ExternalInteraction::TransactionalExternal(
            TransactionalExternalContract {
                protocol: TransactionalEffectProtocol::PrepareCommitCompensate,
            },
        ))
    });

fn external_contract(interaction: ExternalInteraction) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction,
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExternalTestProviderTrace {
    pub plan_calls: u64,
    pub read_calls: u64,
    pub prepared: Vec<(RuntimeEffectId, String)>,
    pub delivered: u64,
    pub applied: u64,
    pub compensated: u64,
    pub delivery_failures: u64,
}

pub type SharedExternalTestProviderTrace = Arc<Mutex<ExternalTestProviderTrace>>;

#[derive(Debug)]
pub struct ExternalTestInputProvider {
    sample: f64,
    trace: SharedExternalTestProviderTrace,
    fail_reads: Arc<Mutex<u64>>,
}

impl ExternalTestInputProvider {
    pub fn new(sample: f64, trace: SharedExternalTestProviderTrace) -> Self {
        Self {
            sample,
            trace,
            fail_reads: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_read_failures(
        sample: f64,
        trace: SharedExternalTestProviderTrace,
        failures: u64,
    ) -> Self {
        Self {
            sample,
            trace,
            fail_reads: Arc::new(Mutex::new(failures)),
        }
    }
}

impl RuntimeResourceProvider for ExternalTestInputProvider {
    fn scheme(&self) -> &str {
        "test-resource"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["test-resource://input/value".to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&TEST_OBSERVATION_CONTRACT)
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.trace
            .lock()
            .expect("external test provider trace")
            .plan_calls += 1;
        RuntimeHostInputValue::F64(self.sample).into_value()
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.trace
            .lock()
            .expect("external test provider trace")
            .read_calls += 1;
        let mut failures = self
            .fail_reads
            .lock()
            .expect("external input failure count");
        if *failures > 0 {
            *failures -= 1;
            return Err(provider_error("injected external input read failure"));
        }
        RuntimeHostInputValue::F64(self.sample).into_value()
    }
}

#[derive(Debug)]
pub struct ExternalTestSceneProvider {
    trace: SharedExternalTestProviderTrace,
    fail_preparations: Arc<Mutex<u64>>,
    fail_deliveries: Arc<Mutex<u64>>,
}

impl ExternalTestSceneProvider {
    pub fn new(trace: SharedExternalTestProviderTrace) -> Self {
        Self {
            trace,
            fail_preparations: Arc::new(Mutex::new(0)),
            fail_deliveries: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_preparation_failures(
        trace: SharedExternalTestProviderTrace,
        failures: u64,
    ) -> Self {
        Self {
            trace,
            fail_preparations: Arc::new(Mutex::new(failures)),
            fail_deliveries: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_delivery_failures(trace: SharedExternalTestProviderTrace, failures: u64) -> Self {
        Self {
            trace,
            fail_preparations: Arc::new(Mutex::new(0)),
            fail_deliveries: Arc::new(Mutex::new(failures)),
        }
    }
}

impl RuntimeResourceProvider for ExternalTestSceneProvider {
    fn scheme(&self) -> &str {
        "test-resource"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["test-resource://scene/output".to_owned()]
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&TEST_SCENE_CONTRACT)
    }

    fn supports_resident_idempotency(&self, intent: RuntimeResourceWriteIntent) -> bool {
        intent == RuntimeResourceWriteIntent::Send
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(provider_error(&format!(
            "external scene provider is write-only: {}#{}",
            request.base_uri, request.path
        )))
    }

    fn plan_write(&self, request: RuntimeResourceWriteCommand) -> MResult<()> {
        validate_test_write_command(&request, "test-resource://scene/output", "frame")
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        if request.idempotency_key.is_empty() {
            return Err(provider_error(
                "external scene effect requires an idempotency key",
            ));
        }
        self.trace
            .lock()
            .expect("external test provider trace")
            .prepared
            .push((request.effect_id, request.idempotency_key));
        let mut failures = self
            .fail_preparations
            .lock()
            .expect("external preparation failure count");
        if *failures > 0 {
            *failures -= 1;
            return Err(provider_error(
                "injected external scene preparation failure",
            ));
        }
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            ExternalTestSceneDelivery {
                trace: self.trace.clone(),
                fail_deliveries: self.fail_deliveries.clone(),
            },
        )))
    }
}

#[derive(Debug)]
struct ExternalTestSceneDelivery {
    trace: SharedExternalTestProviderTrace,
    fail_deliveries: Arc<Mutex<u64>>,
}

impl RuntimeAfterCommitEffect for ExternalTestSceneDelivery {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata("scene")
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut remaining = self.fail_deliveries.lock().expect("external failure count");
        if *remaining > 0 {
            *remaining -= 1;
            self.trace
                .lock()
                .expect("external test provider trace")
                .delivery_failures += 1;
            return Err(provider_error("injected external scene delivery failure"));
        }
        self.trace
            .lock()
            .expect("external test provider trace")
            .delivered += 1;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ExternalTestTransactionalProvider {
    trace: SharedExternalTestProviderTrace,
}

impl ExternalTestTransactionalProvider {
    pub fn new(trace: SharedExternalTestProviderTrace) -> Self {
        Self { trace }
    }
}

impl RuntimeResourceProvider for ExternalTestTransactionalProvider {
    fn scheme(&self) -> &str {
        "test-resource"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["test-resource://transactional/state".to_owned()]
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&TEST_TRANSACTIONAL_CONTRACT)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(provider_error(&format!(
            "external transactional provider is write-only: {}#{}",
            request.base_uri, request.path
        )))
    }

    fn plan_write(&self, request: RuntimeResourceWriteCommand) -> MResult<()> {
        validate_test_write_command(&request, "test-resource://transactional/state", "value")
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.trace
            .lock()
            .expect("external test provider trace")
            .prepared
            .push((request.effect_id, request.idempotency_key));
        Ok(PreparedRuntimeEffect::Compensatable(Box::new(
            ExternalTestCompensatableWrite {
                trace: self.trace.clone(),
                applied: false,
            },
        )))
    }
}

#[derive(Debug)]
struct ExternalTestCompensatableWrite {
    trace: SharedExternalTestProviderTrace,
    applied: bool,
}

impl RuntimeCompensatableEffect for ExternalTestCompensatableWrite {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata("transactional")
    }

    fn apply(&mut self) -> MResult<()> {
        self.applied = true;
        self.trace
            .lock()
            .expect("external test provider trace")
            .applied += 1;
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        if self.applied {
            self.trace
                .lock()
                .expect("external test provider trace")
                .compensated += 1;
            self.applied = false;
        }
        Ok(())
    }
}

fn metadata(name: &str) -> RuntimeEffectMetadata {
    RuntimeEffectMetadata::new(
        RuntimeEffectSource::ResourceProvider {
            scheme: "test-resource".to_owned(),
        },
        "write",
    )
    .with_resource(name)
    .with_cost(RuntimeEffectCost { bytes: 8, items: 1 })
}

fn validate_test_write_command(
    request: &RuntimeResourceWriteCommand,
    expected_base_uri: &str,
    expected_path: &str,
) -> MResult<()> {
    if request.base_uri != expected_base_uri
        || request.path != expected_path
        || request.intent != RuntimeResourceWriteIntent::Send
        || !matches!(request.value.data(), ValueData::F64(_))
    {
        return Err(provider_error(
            "external test fixture write does not match its declared numeric send target",
        ));
    }
    Ok(())
}

fn provider_error(message: &str) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.to_owned(),
        },
        None,
    )
}
