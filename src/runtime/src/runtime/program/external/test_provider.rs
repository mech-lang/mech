//! Deterministic D3 providers shared by unit tests and controlled evidence.
//!
//! This module is never present in normal product builds.

use std::sync::{Arc, LazyLock, Mutex};

use mech_core::{
    AccessMode, ChangeDetectionPolicy, DeliveryMode, EffectContract, EffectDeliveryPolicy,
    ExternalInteraction, IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue,
    MResult, MechError, ObservationContract, ObservationReplayPolicy, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, ShapeRule, TransactionalEffectProtocol,
    TransactionalExternalContract,
};

use crate::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeCompensatableEffect, RuntimeEffectCost,
    RuntimeEffectId, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeResidentResourceWriteRequest, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
};

pub static D3_OBSERVATION_CONTRACT: LazyLock<OperationContractDeclaration> =
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

pub static D3_SCENE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    external_contract(ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::IdempotentRetry,
        idempotency: IdempotencyRequirement::Required,
    }))
});

pub static D3_TRANSACTIONAL_CONTRACT: LazyLock<OperationContractDeclaration> =
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
pub struct D3ProviderTrace {
    pub plan_calls: u64,
    pub read_calls: u64,
    pub prepared: Vec<(RuntimeEffectId, String)>,
    pub delivered: u64,
    pub applied: u64,
    pub compensated: u64,
    pub delivery_failures: u64,
}

pub type SharedD3ProviderTrace = Arc<Mutex<D3ProviderTrace>>;

#[derive(Debug)]
pub struct D3InputProvider {
    sample: f64,
    trace: SharedD3ProviderTrace,
    fail_reads: Arc<Mutex<u64>>,
}

impl D3InputProvider {
    pub fn new(sample: f64, trace: SharedD3ProviderTrace) -> Self {
        Self {
            sample,
            trace,
            fail_reads: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_read_failures(sample: f64, trace: SharedD3ProviderTrace, failures: u64) -> Self {
        Self {
            sample,
            trace,
            fail_reads: Arc::new(Mutex::new(failures)),
        }
    }
}

impl RuntimeResourceProvider for D3InputProvider {
    fn scheme(&self) -> &str {
        "gate-d3"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["gate-d3://input/value".to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&D3_OBSERVATION_CONTRACT)
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.trace.lock().expect("D3 provider trace").plan_calls += 1;
        Ok(LegacyValue::F64(mech_core::Ref::new(self.sample)))
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.trace.lock().expect("D3 provider trace").read_calls += 1;
        let mut failures = self.fail_reads.lock().expect("D3 input failure count");
        if *failures > 0 {
            *failures -= 1;
            return Err(provider_error("injected D3 input read failure"));
        }
        Ok(LegacyValue::F64(mech_core::Ref::new(self.sample)))
    }
}

#[derive(Debug)]
pub struct D3SceneProvider {
    trace: SharedD3ProviderTrace,
    fail_preparations: Arc<Mutex<u64>>,
    fail_deliveries: Arc<Mutex<u64>>,
}

impl D3SceneProvider {
    pub fn new(trace: SharedD3ProviderTrace) -> Self {
        Self {
            trace,
            fail_preparations: Arc::new(Mutex::new(0)),
            fail_deliveries: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_preparation_failures(trace: SharedD3ProviderTrace, failures: u64) -> Self {
        Self {
            trace,
            fail_preparations: Arc::new(Mutex::new(failures)),
            fail_deliveries: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_delivery_failures(trace: SharedD3ProviderTrace, failures: u64) -> Self {
        Self {
            trace,
            fail_preparations: Arc::new(Mutex::new(0)),
            fail_deliveries: Arc::new(Mutex::new(failures)),
        }
    }
}

impl RuntimeResourceProvider for D3SceneProvider {
    fn scheme(&self) -> &str {
        "gate-d3"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["gate-d3://scene/output".to_owned()]
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&D3_SCENE_CONTRACT)
    }

    fn supports_resident_idempotency(&self, intent: RuntimeResourceWriteIntent) -> bool {
        intent == RuntimeResourceWriteIntent::Send
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(provider_error(&format!(
            "D3 scene provider is write-only: {}#{}",
            request.base_uri, request.path
        )))
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        validate_d3_write(&request, "gate-d3://scene/output", "frame")
    }

    fn prepare_resident_write(
        &self,
        request: RuntimeResidentResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        if request.idempotency_key.is_empty() {
            return Err(provider_error(
                "D3 scene effect requires an idempotency key",
            ));
        }
        self.trace
            .lock()
            .expect("D3 provider trace")
            .prepared
            .push((request.effect_id, request.idempotency_key));
        let mut failures = self
            .fail_preparations
            .lock()
            .expect("D3 preparation failure count");
        if *failures > 0 {
            *failures -= 1;
            return Err(provider_error("injected D3 scene preparation failure"));
        }
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            D3SceneDelivery {
                trace: self.trace.clone(),
                fail_deliveries: self.fail_deliveries.clone(),
            },
        )))
    }
}

#[derive(Debug)]
struct D3SceneDelivery {
    trace: SharedD3ProviderTrace,
    fail_deliveries: Arc<Mutex<u64>>,
}

impl RuntimeAfterCommitEffect for D3SceneDelivery {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata("scene")
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut remaining = self.fail_deliveries.lock().expect("D3 failure count");
        if *remaining > 0 {
            *remaining -= 1;
            self.trace
                .lock()
                .expect("D3 provider trace")
                .delivery_failures += 1;
            return Err(provider_error("injected D3 scene delivery failure"));
        }
        self.trace.lock().expect("D3 provider trace").delivered += 1;
        Ok(())
    }
}

#[derive(Debug)]
pub struct D3TransactionalProvider {
    trace: SharedD3ProviderTrace,
}

impl D3TransactionalProvider {
    pub fn new(trace: SharedD3ProviderTrace) -> Self {
        Self { trace }
    }
}

impl RuntimeResourceProvider for D3TransactionalProvider {
    fn scheme(&self) -> &str {
        "gate-d3"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["gate-d3://transactional/state".to_owned()]
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&D3_TRANSACTIONAL_CONTRACT)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(provider_error(&format!(
            "D3 transactional provider is write-only: {}#{}",
            request.base_uri, request.path
        )))
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        validate_d3_write(&request, "gate-d3://transactional/state", "value")
    }

    fn prepare_resident_write(
        &self,
        request: RuntimeResidentResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.trace
            .lock()
            .expect("D3 provider trace")
            .prepared
            .push((request.effect_id, request.idempotency_key));
        Ok(PreparedRuntimeEffect::Compensatable(Box::new(
            D3CompensatableWrite {
                trace: self.trace.clone(),
                applied: false,
            },
        )))
    }
}

#[derive(Debug)]
struct D3CompensatableWrite {
    trace: SharedD3ProviderTrace,
    applied: bool,
}

impl RuntimeCompensatableEffect for D3CompensatableWrite {
    fn metadata(&self) -> RuntimeEffectMetadata {
        metadata("transactional")
    }

    fn apply(&mut self) -> MResult<()> {
        self.applied = true;
        self.trace.lock().expect("D3 provider trace").applied += 1;
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        if self.applied {
            self.trace.lock().expect("D3 provider trace").compensated += 1;
            self.applied = false;
        }
        Ok(())
    }
}

fn metadata(name: &str) -> RuntimeEffectMetadata {
    RuntimeEffectMetadata::new(
        RuntimeEffectSource::ResourceProvider {
            scheme: "gate-d3".to_owned(),
        },
        "write",
    )
    .with_resource(name)
    .with_cost(RuntimeEffectCost { bytes: 8, items: 1 })
}

fn validate_d3_write(
    request: &RuntimeResourceWriteRequest,
    expected_base_uri: &str,
    expected_path: &str,
) -> MResult<()> {
    if request.base_uri != expected_base_uri
        || request.path != expected_path
        || request.intent != RuntimeResourceWriteIntent::Send
        || !matches!(request.value, LegacyValue::F64(_))
    {
        return Err(provider_error(
            "D3 fixture write does not match its declared numeric send target",
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
