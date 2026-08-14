use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult,
    OperationContractDeclaration, Ref, ValueData,
};
use mech_engine::{SlotRole, encode_program_artifact_bytecode_v1, resident::ResidentValueBorrow};
use sha2::{Digest, Sha256};

use crate::{
    BasicCapability, BasicConstraints, Capability, CapabilityDecision, CapabilityId,
    CapabilityRequest, InMemorySourceResolver, ModuleBuildOptions, PreparedRuntimeEffect,
    RuntimeAfterCommitEffect, RuntimeBuilder, RuntimeEffectCost, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeIngress,
    RuntimeResidentResourceWriteRequest, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

use super::*;

const PURE_SOURCE: &str =
    include_str!("../../../../../tests/architecture/resident-activation/n-body-source-v1.mec");
const PRODUCT_NBODY_SOURCE: &str =
    include_str!("../../../../../examples/resident-n-body/n-body.mec");
const PUBLIC_NBODY_VIEWER_SOURCE: &str = include_str!("../../../../../examples/n-body/n-body.mec");

fn runtime() -> crate::MechRuntime {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .input_driver(ResidentTestInputDriver)
        .build()
        .unwrap()
}

#[derive(Debug)]
struct MutableResidentCapability {
    capability: BasicCapability,
    enabled: Arc<AtomicBool>,
}

impl Capability for MutableResidentCapability {
    fn id(&self) -> CapabilityId {
        self.capability.id()
    }

    fn subject_key(&self) -> &str {
        self.capability.subject_key()
    }

    fn validate(&self) -> MResult<()> {
        self.capability.validate()
    }

    fn check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        self.preview_check(request)
    }

    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        if self.enabled.load(Ordering::SeqCst) {
            self.capability.preview_check(request)
        } else {
            Ok(CapabilityDecision::deny(
                "mutable resident test capability is disabled",
            ))
        }
    }
}

fn replace_with_mutable_capability(
    runtime: &mut crate::MechRuntime,
    replaced: CapabilityId,
    replacement: CapabilityId,
    resource: &str,
    operations: impl IntoIterator<Item = &'static str>,
) -> Arc<AtomicBool> {
    runtime.revoke_capability(replaced).unwrap();
    let enabled = Arc::new(AtomicBool::new(true));
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(MutableResidentCapability {
            capability: BasicCapability::from_keys(replacement, subject, resource, operations),
            enabled: enabled.clone(),
        }))
        .unwrap();
    enabled
}

#[derive(Debug)]
struct ResidentTestInputDriver;

impl RuntimeHostInputDriver for ResidentTestInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri().starts_with("test://") || source.base_uri() == "timer://clock/tick"
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        false
    }
}

#[derive(Debug)]
struct PlanningObservationProvider {
    plans: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    value_bits: Arc<AtomicU64>,
}

#[derive(Debug)]
struct IndependentObservationProvider {
    reads: Arc<AtomicUsize>,
    fast_bits: Arc<AtomicU64>,
    slow_bits: Arc<AtomicU64>,
}

impl RuntimeResourceProvider for IndependentObservationProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![
            "test://clock/fast".to_owned(),
            "test://clock/slow".to_owned(),
        ]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.value(&request)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.value(&request)
    }
}

impl IndependentObservationProvider {
    fn value(&self, request: &RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        let bits = match request.base_uri.as_str() {
            "test://clock/fast" => self.fast_bits.load(Ordering::SeqCst),
            "test://clock/slow" => self.slow_bits.load(Ordering::SeqCst),
            other => panic!("unexpected independent observation URI {other}"),
        };
        Ok(LegacyValue::F64(Ref::new(f64::from_bits(bits))))
    }
}

impl RuntimeResourceProvider for PlanningObservationProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["test://clock/tick".to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.plans.fetch_add(1, Ordering::SeqCst);
        Ok(LegacyValue::F64(Ref::new(f64::from_bits(
            self.value_bits.load(Ordering::SeqCst),
        ))))
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(LegacyValue::F64(Ref::new(f64::from_bits(
            self.value_bits.load(Ordering::SeqCst),
        ))))
    }
}

fn external_source() -> &'static str {
    r#"
@clock := test://clock/tick{:read(delta-seconds)}
delta := @clock/delta-seconds
~state := 0.0
state += delta
output := state
"#
}

static PRODUCT_SCENE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        }),
    });

static PRODUCT_RETRY_SCENE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::IdempotentRetry,
            idempotency: IdempotencyRequirement::Required,
        }),
    });

#[derive(Clone, Copy, Debug)]
enum ProductSceneContract {
    AtMostOnce,
    IdempotentRetry,
}

#[derive(Debug)]
struct ProductSceneProvider {
    trace: Arc<Mutex<ProductSceneTrace>>,
    contract: ProductSceneContract,
    prepare_delay: Duration,
}

#[derive(Debug, Default)]
struct ProductSceneTrace {
    preparations: usize,
    delivery_attempts: usize,
    delivery_failures_remaining: usize,
    deliveries: usize,
    latest: Vec<f64>,
    max_retained_values: usize,
}

impl RuntimeResourceProvider for ProductSceneProvider {
    fn scheme(&self) -> &str {
        "scene"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["scene://orbit/frame".to_owned()]
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(match self.contract {
            ProductSceneContract::AtMostOnce => &PRODUCT_SCENE_CONTRACT,
            ProductSceneContract::IdempotentRetry => &PRODUCT_RETRY_SCENE_CONTRACT,
        })
    }

    fn supports_resident_idempotency(&self, intent: RuntimeResourceWriteIntent) -> bool {
        intent == RuntimeResourceWriteIntent::Send
            && matches!(self.contract, ProductSceneContract::IdempotentRetry)
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        panic!("the deterministic scene is write-only")
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        assert_eq!(request.base_uri, "scene://orbit/frame");
        assert_eq!(request.path, "points");
        assert_eq!(request.intent, RuntimeResourceWriteIntent::Send);
        Ok(())
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })
    }

    fn prepare_resident_write(
        &self,
        request: RuntimeResidentResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        std::thread::sleep(self.prepare_delay);
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        let values = match request.value {
            LegacyValue::MatrixF64(matrix) => matrix.as_vec(),
            LegacyValue::MutableReference(reference) => match &*reference.borrow() {
                LegacyValue::MatrixF64(matrix) => matrix.as_vec(),
                other => panic!("scene points must be typed f64 matrix, got {other:?}"),
            },
            other => panic!("scene points must be typed f64 matrix, got {other:?}"),
        };
        self.trace.lock().unwrap().preparations += 1;
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            ProductSceneDelivery {
                trace: self.trace.clone(),
                values,
            },
        )))
    }
}

#[derive(Debug)]
struct ProductSceneDelivery {
    trace: Arc<Mutex<ProductSceneTrace>>,
    values: Vec<f64>,
}

impl RuntimeAfterCommitEffect for ProductSceneDelivery {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "scene".to_owned(),
            },
            "points",
        )
        .with_resource("scene://orbit/frame")
        .with_cost(RuntimeEffectCost {
            bytes: self.values.len() as u64 * 8,
            items: 1,
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.delivery_attempts += 1;
        if trace.delivery_failures_remaining > 0 {
            trace.delivery_failures_remaining -= 1;
            return Err(mech_core::MechError::new(
                mech_core::GenericError {
                    msg: "injected resident scene delivery failure".to_owned(),
                },
                None,
            ));
        }
        trace.deliveries += 1;
        trace.latest.clone_from(&self.values);
        trace.max_retained_values = trace.max_retained_values.max(trace.latest.len());
        Ok(())
    }
}

fn product_nbody_runtime() -> (crate::MechRuntime, Arc<Mutex<ProductSceneTrace>>) {
    configured_product_nbody_runtime(ProductSceneContract::AtMostOnce, true, true)
}

fn configured_product_nbody_runtime(
    contract: ProductSceneContract,
    include_scene: bool,
    grant_scene: bool,
) -> (crate::MechRuntime, Arc<Mutex<ProductSceneTrace>>) {
    configured_product_nbody_runtime_with_delay(
        contract,
        include_scene,
        grant_scene,
        Duration::ZERO,
    )
}

fn configured_product_nbody_runtime_with_delay(
    contract: ProductSceneContract,
    include_scene: bool,
    grant_scene: bool,
    prepare_delay: Duration,
) -> (crate::MechRuntime, Arc<Mutex<ProductSceneTrace>>) {
    let trace = Arc::new(Mutex::new(ProductSceneTrace::default()));
    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(ProductTimerProvider))
        .unwrap();
    if include_scene {
        runtime
            .register_resource_provider(Box::new(ProductSceneProvider {
                trace: trace.clone(),
                contract,
                prepare_delay,
            }))
            .unwrap();
    }
    let subject = runtime.runtime_context().unwrap().subject;
    let mut grants = vec![(9_100, "timer://clock/tick/tick", vec!["read"])];
    if grant_scene {
        grants.push((9_101, "scene://orbit/frame/points", vec!["write", "points"]));
    }
    for (id, resource, operations) in grants {
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(id),
                subject.clone(),
                resource,
                operations,
            )))
            .unwrap();
    }
    (runtime, trace)
}

#[derive(Debug)]
struct ProductTimerProvider;

impl RuntimeResourceProvider for ProductTimerProvider {
    fn scheme(&self) -> &str {
        "timer"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["timer://clock/tick".to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        assert_eq!(request.path, "tick");
        Ok(LegacyValue::F64(Ref::new(0.0)))
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        assert_eq!(request.path, "tick");
        Ok(LegacyValue::F64(Ref::new(0.0)))
    }
}

fn external_runtime(
    durability: crate::ResidentDurabilityPolicy,
) -> (
    crate::MechRuntime,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
    Arc<AtomicU64>,
) {
    let plans = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let value_bits = Arc::new(AtomicU64::new((1.0 / 60.0_f64).to_bits()));
    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(PlanningObservationProvider {
            plans: plans.clone(),
            reads: reads.clone(),
            value_bits: value_bits.clone(),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_001),
            subject,
            "test://clock/tick/delta-seconds",
            ["read"],
        )))
        .unwrap();
    runtime
        .load_source_program(external_source(), durability)
        .unwrap();
    (runtime, plans, reads, value_bits)
}

fn independent_external_runtime() -> (crate::MechRuntime, Arc<AtomicUsize>) {
    let reads = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(IndependentObservationProvider {
            reads: reads.clone(),
            fast_bits: Arc::new(AtomicU64::new(2.0_f64.to_bits())),
            slow_bits: Arc::new(AtomicU64::new(3.0_f64.to_bits())),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    for (id, resource) in [
        (9_020, "test://clock/fast/delta-seconds"),
        (9_021, "test://clock/slow/delta-seconds"),
    ] {
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(id),
                subject.clone(),
                resource,
                ["read"],
            )))
            .unwrap();
    }
    runtime
        .load_source_program(
            r#"
@fast := test://clock/fast{:read(delta-seconds)}
@slow := test://clock/slow{:read(delta-seconds)}
fast := @fast/delta-seconds
slow := @slow/delta-seconds
~state := 0.0
state += fast + slow
output := state
"#,
            crate::ResidentDurabilityPolicy::Retained,
        )
        .unwrap();
    (runtime, reads)
}

fn unactivated_external_runtime(driver_count: usize) -> crate::MechRuntime {
    let mut builder = RuntimeBuilder::new().function_catalog(mech_stdlib::source_catalog());
    for _ in 0..driver_count {
        builder = builder.input_driver(ResidentTestInputDriver);
    }
    let mut runtime = builder.build().unwrap();
    runtime
        .register_resource_provider(Box::new(PlanningObservationProvider {
            plans: Arc::new(AtomicUsize::new(0)),
            reads: Arc::new(AtomicUsize::new(0)),
            value_bits: Arc::new(AtomicU64::new(1.0_f64.to_bits())),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_003),
            subject,
            "test://clock/tick/delta-seconds",
            ["read"],
        )))
        .unwrap();
    runtime
}

#[test]
fn pure_source_and_bytecode_choose_resident_with_equivalent_identity_and_output() {
    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(PURE_SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(source.route, RuntimeProgramRoute::ResidentPure);
    assert!(!source.initial_value.is_empty());
    let ActiveProgramExecution::ResidentPure(source_execution) = &source_runtime.active_program
    else {
        panic!("source route must own a pure resident instance")
    };
    assert_eq!(
        source_runtime.root_plan_len(),
        source_execution.instance.plan.execution_node_count()
    );
    let output = source_execution
        .artifact
        .outputs()
        .first()
        .expect("the resident fixture must expose an output");
    assert_eq!(
        source_runtime.output_name(output.output),
        Some(output.name.clone())
    );
    assert!(
        source_runtime
            .output_value(output.output)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        source_runtime.root_symbol_values_all().unwrap().len(),
        source_execution.artifact.outputs().len()
    );
    let bytecode = encode_program_artifact_bytecode_v1(&source_execution.artifact).unwrap();

    let mut bytecode_runtime = runtime();
    let bytecode = bytecode_runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(bytecode.route, RuntimeProgramRoute::ResidentPure);
    assert_eq!(source.initial_value, bytecode.initial_value);
    assert_eq!(source.info.program_revision, bytecode.info.program_revision);
    assert_eq!(source.info.plan_generation, bytecode.info.plan_generation);
    assert_eq!(
        source.info.layout_generation,
        bytecode.info.layout_generation
    );
}

#[test]
fn literal_scalar_output_is_published_without_fake_state() {
    let mut source_runtime = runtime();
    let loaded = source_runtime
        .load_source_program(
            "answer := 42.0\nanswer",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);
    assert!(matches!(
        loaded.initial_value.to_value(),
        LegacyValue::F64(value) if *value.borrow() == 42.0
    ));
    let ActiveProgramExecution::ResidentPure(execution) = &source_runtime.active_program else {
        panic!("literal source must own a resident instance")
    };
    let published = execution.artifact.outputs()[0].source;
    assert_eq!(
        execution.artifact.slots()[published.get() as usize].role,
        SlotRole::Output
    );
    assert!(execution.instance.state_borrow(published).is_none());
    let revision = execution.artifact.revision();
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();

    let mut decoded = runtime();
    let loaded = decoded
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(loaded.info.program_revision, Some(revision));
    assert!(matches!(
        loaded.initial_value.to_value(),
        LegacyValue::F64(value) if *value.borrow() == 42.0
    ));
}

#[test]
fn computed_scalar_output_is_materialized_during_activation() {
    let mut runtime = runtime();
    let loaded = runtime
        .load_source_program(
            "answer := 40.0 + 2.0\nanswer",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);
    assert!(matches!(
        loaded.initial_value.to_value(),
        LegacyValue::F64(value) if *value.borrow() == 42.0
    ));
}

#[test]
fn literal_bool_and_matrix_outputs_are_published() {
    let mut boolean = runtime();
    let loaded = boolean
        .load_source_program(
            "answer := true\nanswer",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert!(matches!(
        loaded.initial_value.to_value(),
        LegacyValue::Bool(value) if *value.borrow()
    ));

    let mut matrix = runtime();
    let loaded = matrix
        .load_source_program(
            "answer := [1.0 2.0; 3.0 4.0]\nanswer",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    let LegacyValue::MatrixF64(value) = loaded.initial_value.to_value() else {
        panic!("matrix output must retain its f64 matrix representation")
    };
    assert_eq!((value.rows(), value.cols()), (2, 2));
    assert_eq!(value.as_vec(), [1.0, 3.0, 2.0, 4.0]);
}

#[test]
fn turn_derived_output_is_published_from_resident_scratch() {
    let plans = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(PlanningObservationProvider {
            plans,
            reads,
            value_bits: Arc::new(AtomicU64::new(0.0_f64.to_bits())),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_023),
            subject,
            "test://clock/tick/delta-seconds",
            ["read"],
        )))
        .unwrap();
    runtime
        .load_source_program(
            r#"
@clock := test://clock/tick{:read(delta-seconds)}
delta := @clock/delta-seconds
output := delta + 1.0
output
"#,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(8.0),
        ))
        .unwrap();
    runtime.drain_resident_host_inputs(1).unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("observation-derived output must remain resident")
    };
    assert!(matches!(
        execution.coordinator.instance().output_borrow(0),
        Some(ResidentValueBorrow::F64 { values, .. }) if values == [9.0]
    ));
}

#[test]
fn empty_runtime_step_fails_without_an_execution_fallback() {
    let mut runtime = runtime();

    assert_eq!(runtime.root_plan_len(), 0);
    assert!(runtime.root_symbol_values_all().unwrap().is_empty());
    assert!(
        runtime
            .output_value(mech_core::OutputId::new(u32::MAX))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        runtime
            .root_symbol_value("missing")
            .unwrap_err()
            .kind_name(),
        "RuntimeInvalidOperation"
    );
    let error = runtime.step_active_program().unwrap_err();

    assert_eq!(error.kind_name(), "ResidentRouteFailure");
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn resident_root_plans_the_resolved_source_import_closure_before_route_selection() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string("main.mec", format!("+> ./dep.mec\n{}", external_source()))
        .unwrap();
    resolver
        .insert_string("dep.mec", "loaded := true\n<+ loaded\n")
        .unwrap();
    let mut source_runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .input_driver(ResidentTestInputDriver)
        .build()
        .unwrap();
    source_runtime
        .register_resource_provider(Box::new(PlanningObservationProvider {
            plans: Arc::new(AtomicUsize::new(0)),
            reads: Arc::new(AtomicUsize::new(0)),
            value_bits: Arc::new(AtomicU64::new(1.0_f64.to_bits())),
        }))
        .unwrap();
    let subject = source_runtime.runtime_context().unwrap().subject;
    source_runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_110),
            subject,
            "test://clock/tick/delta-seconds",
            ["read"],
        )))
        .unwrap();

    let outcome = source_runtime
        .load_root_program(
            "main.mec".into(),
            ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();

    assert_eq!(outcome.route, RuntimeProgramRoute::ResidentExternal);
    let ActiveProgramExecution::ResidentExternal(execution) = &source_runtime.active_program else {
        panic!("import closure must install one resident artifact")
    };
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();
    let mut bytecode_runtime = unactivated_external_runtime(1);
    let decoded = bytecode_runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(decoded.route, RuntimeProgramRoute::ResidentExternal);
    assert_eq!(outcome.info.program_revision, decoded.info.program_revision);
    assert_eq!(outcome.initial_value, decoded.initial_value);
}

#[test]
fn external_activation_requires_exactly_one_input_driver() {
    for (driver_count, expected) in [
        (0, "ProviderUnavailable:"),
        (2, "ProviderContractMismatch:"),
    ] {
        let mut runtime = unactivated_external_runtime(driver_count);
        let error = runtime
            .load_source_program(external_source(), crate::ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert!(
            error.kind_message().starts_with(expected),
            "expected {expected}, got {error:?}",
        );
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
    }
}

#[test]
fn finite_use_capability_cannot_authorize_a_resident_session() {
    let mut runtime = unactivated_external_runtime(1);
    runtime.revoke_capability(CapabilityId(9_003)).unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(
            BasicCapability::from_keys(
                CapabilityId(9_004),
                subject,
                "test://clock/tick/delta-seconds",
                ["read"],
            )
            .with_constraints(BasicConstraints::default().with_max_uses(1)),
        ))
        .unwrap();

    let error = runtime
        .load_source_program(external_source(), crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert!(error.kind_message().starts_with("AuthorizationDenied:"));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn invalidated_admitted_grant_blocks_next_drain_before_dequeue_or_publication() {
    let (mut runtime, scene) = product_nbody_runtime();
    let timer_grant = replace_with_mutable_capability(
        &mut runtime,
        CapabilityId(9_100),
        CapabilityId(9_200),
        "timer://clock/tick/tick",
        ["read"],
    );
    runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    let published_epoch = execution.coordinator.instance().published_epoch();
    let accepted_turns = runtime.program_execution_info().resident_accepted_turns;
    let scene_before = scene.lock().unwrap();
    let preparations = scene_before.preparations;
    let delivery_attempts = scene_before.delivery_attempts;
    let deliveries = scene_before.deliveries;
    drop(scene_before);

    timer_grant.store(false, Ordering::SeqCst);
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("timer://clock/tick", "tick").unwrap(),
            crate::RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();

    let error = runtime.drain_resident_host_inputs(1).unwrap_err();
    assert!(error.kind_message().starts_with("AuthorizationDenied:"));
    assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
    assert_eq!(
        runtime.program_execution_info().resident_accepted_turns,
        accepted_turns
    );
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    assert_eq!(
        execution.coordinator.instance().published_epoch(),
        published_epoch
    );
    let scene = scene.lock().unwrap();
    assert_eq!(scene.preparations, preparations);
    assert_eq!(scene.delivery_attempts, delivery_attempts);
    assert_eq!(scene.deliveries, deliveries);
}

#[test]
fn invalidated_admitted_grant_blocks_outbox_retry_before_preparation_or_delivery() {
    let (mut runtime, scene) =
        configured_product_nbody_runtime(ProductSceneContract::IdempotentRetry, true, true);
    let scene_grant = replace_with_mutable_capability(
        &mut runtime,
        CapabilityId(9_101),
        CapabilityId(9_201),
        "scene://orbit/frame/points",
        ["write", "points"],
    );
    scene.lock().unwrap().delivery_failures_remaining = 1;
    runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    advance_product_nbody(&mut runtime);
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    assert_eq!(execution.coordinator.pending_outbox_count(), 1);
    let scene_before = scene.lock().unwrap();
    assert_eq!(scene_before.preparations, 1);
    assert_eq!(scene_before.delivery_attempts, 1);
    assert_eq!(scene_before.deliveries, 0);
    drop(scene_before);

    scene_grant.store(false, Ordering::SeqCst);
    let error = runtime.retry_resident_outbox().unwrap_err();
    assert!(error.kind_message().starts_with("AuthorizationDenied:"));
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    assert_eq!(execution.coordinator.pending_outbox_count(), 1);
    let scene = scene.lock().unwrap();
    assert_eq!(scene.preparations, 1);
    assert_eq!(scene.delivery_attempts, 1);
    assert_eq!(scene.deliveries, 0);
}

#[test]
fn production_source_and_bytecode_load_residently_without_engine_selection() {
    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(PURE_SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(source.route, RuntimeProgramRoute::ResidentPure);

    let ActiveProgramExecution::ResidentPure(execution) = &source_runtime.active_program else {
        unreachable!()
    };
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();
    let mut bytecode_runtime = runtime();
    let decoded = bytecode_runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(decoded.route, RuntimeProgramRoute::ResidentPure);
}

#[test]
fn production_unsupported_semantics_fail_without_installing_legacy() {
    let mut runtime = runtime();
    let error = runtime
        .load_source_program(
            r#"message := "resident strings are deliberately unsupported""#,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap_err();
    let failure = error.kind_as::<ResidentRouteFailure>().unwrap();
    assert_eq!(
        failure.class,
        ResidentRouteFailureClass::SemanticUnsupported
    );
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn external_source_plans_without_a_live_provider_read_and_freezes_environment() {
    let (mut runtime, plans, reads, _) =
        external_runtime(crate::ResidentDurabilityPolicy::Volatile);
    assert_eq!(
        runtime.program_route(),
        RuntimeProgramRoute::ResidentExternal
    );
    assert!(plans.load(Ordering::SeqCst) >= 1);
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.program_execution_info().observation_count, 1);
    assert!(
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(9_002),
                "irrelevant",
                "irrelevant",
                ["read"],
            )))
            .is_err()
    );
    runtime.unload_active_program().unwrap();
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn resident_loaders_enforce_source_limits_before_planning_or_decoding() {
    let mut config = crate::RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);

    let mut source_runtime = crate::MechRuntime::new(config.clone()).unwrap();
    let source_error = source_runtime
        .load_source_program("1234", crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    let source_budget = source_error
        .kind_as::<crate::ResourceBudgetExceededError>()
        .unwrap();
    assert_eq!(source_budget.resource, "source_bytes");
    assert_eq!(source_budget.requested, 4);
    assert_eq!(source_runtime.program_route(), RuntimeProgramRoute::None);

    let mut bytecode_runtime = crate::MechRuntime::new(config).unwrap();
    let bytecode_error = bytecode_runtime
        .load_bytecode_program(&[0, 1, 2, 3], crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    let bytecode_budget = bytecode_error
        .kind_as::<crate::ResourceBudgetExceededError>()
        .unwrap();
    assert_eq!(bytecode_budget.resource, "source_bytes");
    assert_eq!(bytecode_budget.requested, 4);
    assert_eq!(bytecode_runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn active_runtime_transaction_blocks_program_load_without_freezing_transaction_control() {
    let mut runtime = runtime();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let error = runtime
        .load_source_program(PURE_SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert_eq!(error.kind_name(), "ResidentRouteFailure");
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);

    runtime
        .abort_runtime_transaction(&mut context, "load correctly refused")
        .unwrap();
    assert!(context.transaction.is_none());
}

#[test]
fn queued_resident_packet_prevents_unload_until_it_is_drained() {
    let (mut runtime, _, _, _) = external_runtime(crate::ResidentDurabilityPolicy::Volatile);
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();

    let error = runtime.unload_active_program().unwrap_err();
    assert_eq!(error.kind_name(), "ResidentProgramNotQuiescent");
    runtime.drain_resident_host_inputs(1).unwrap();
    runtime.unload_active_program().unwrap();
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn retained_evidence_drain_releases_capacity_for_long_running_sessions() {
    let (mut runtime, _, _, _) = external_runtime(crate::ResidentDurabilityPolicy::Retained);
    let trigger = crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap();
    let mut drained_inputs = 0usize;
    let mut drained_receipts = 0usize;
    for turn in 0..1_200 {
        runtime
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                trigger.clone(),
                crate::RuntimeHostInputValue::F64(turn as f64),
            ))
            .unwrap();
        runtime.drain_resident_host_inputs(1).unwrap();
        if (turn + 1) % 400 == 0 {
            let evidence = runtime.drain_resident_evidence().unwrap();
            drained_inputs += evidence.input_batches.len();
            drained_receipts += evidence.receipts.len();
        }
    }
    assert_eq!(drained_inputs, 1_200);
    assert_eq!(drained_receipts, 1_200);
    assert_eq!(
        runtime.program_execution_info().resident_accepted_turns,
        1_200
    );
}

#[test]
fn retained_admission_failure_leaves_the_ordered_packet_available_for_retry() {
    let (mut runtime, _, _, _) = external_runtime(crate::ResidentDurabilityPolicy::Retained);
    let trigger = crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap();
    for turn in 0..1_024 {
        runtime
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                trigger.clone(),
                crate::RuntimeHostInputValue::F64(turn as f64),
            ))
            .unwrap();
        runtime.drain_resident_host_inputs(1).unwrap();
    }
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            trigger,
            crate::RuntimeHostInputValue::F64(1_024.0),
        ))
        .unwrap();

    let error = runtime.drain_resident_host_inputs(1).unwrap_err();
    assert_eq!(error.kind_name(), "LedgerCapacityExceeded");
    assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
    let evidence = runtime.drain_resident_evidence().unwrap();
    assert_eq!(evidence.input_batches.len(), 1_024);
    assert_eq!(evidence.receipts.len(), 1_024);

    let retried = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        retried.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
}

#[test]
fn every_root_loader_rejects_unimplemented_durability_with_the_route_classification() {
    let mut resolver = InMemorySourceResolver::new();
    resolver.insert_string("main.mec", PURE_SOURCE).unwrap();
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build()
        .unwrap();
    let error = runtime
        .load_root_program(
            "main.mec".into(),
            ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]),
            crate::ResidentDurabilityPolicy::SynchronousDurable,
        )
        .unwrap_err();
    assert!(error.kind_message().starts_with("InternalFailure:"));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn resident_host_packets_coalesce_and_capture_the_latest_packet_value() {
    let (mut runtime, _, reads, value_bits) =
        external_runtime(crate::ResidentDurabilityPolicy::Retained);
    let trigger = crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap();
    let ingress = runtime.ingress();
    ingress
        .submit(crate::RuntimeHostInput::single(
            trigger.clone(),
            crate::RuntimeHostInputValue::F64(7.0),
        ))
        .unwrap();
    ingress
        .submit(crate::RuntimeHostInput::single(
            trigger,
            crate::RuntimeHostInputValue::F64(8.0),
        ))
        .unwrap();

    value_bits.store(0.125_f64.to_bits(), Ordering::SeqCst);
    let outcome = runtime.drain_resident_host_inputs(64).unwrap();
    assert_eq!(outcome.dequeued_packets, 2);
    assert_eq!(outcome.matched_packets, 2);
    assert_eq!(outcome.coalesced_packets, 1);
    assert_eq!(outcome.ignored_packets, 0);
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.program_execution_info().resident_accepted_turns, 1);
    assert_eq!(runtime.program_execution_info().coalesced_host_packets, 1);

    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("external route must remain active")
    };
    let batch = execution
        .coordinator
        .input_facts()
        .next()
        .expect("retained input fact")
        .1;
    let ValueData::F64(value) = batch.facts[0].value.data() else {
        panic!("timer observation must capture f64")
    };
    assert_eq!(value.bits(), 8.0_f64.to_bits());
}

#[test]
fn duplicate_observations_share_one_authoritative_host_update() {
    let plans = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(PlanningObservationProvider {
            plans,
            reads: reads.clone(),
            value_bits: Arc::new(AtomicU64::new(1.0_f64.to_bits())),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_022),
            subject,
            "test://clock/tick/delta-seconds",
            ["read"],
        )))
        .unwrap();
    runtime
        .load_source_program(
            r#"
@first := test://clock/tick{:read(delta-seconds)}
@second := test://clock/tick{:read(delta-seconds)}
first := @first/delta-seconds
second := @second/delta-seconds
~state := 0.0
state += first + second
output := state
"#,
            crate::ResidentDurabilityPolicy::Retained,
        )
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("duplicate observations must remain resident")
    };
    assert_eq!(execution.trigger_sources.len(), 1);

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();
    let outcome = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    let batch = execution.coordinator.input_facts().next().unwrap().1;
    assert_eq!(batch.facts.len(), 2);
    for fact in &batch.facts {
        let ValueData::F64(value) = fact.value.data() else {
            panic!("duplicate timer observation must remain f64")
        };
        assert_eq!(value.bits(), 9.0_f64.to_bits());
    }
}

#[test]
fn independent_observations_capture_absent_values_from_the_bound_provider() {
    let (mut runtime, reads) = independent_external_runtime();
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(7.0),
        ))
        .unwrap();

    let outcome = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    let mut values = execution
        .coordinator
        .input_facts()
        .next()
        .unwrap()
        .1
        .facts
        .iter()
        .map(|fact| match fact.value.data() {
            ValueData::F64(value) => value.to_f64(),
            _ => panic!("clock observations must remain f64"),
        })
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    assert_eq!(values, vec![3.0, 7.0]);
}

#[test]
fn public_host_drain_exposes_the_clean_resident_turn() {
    let (mut runtime, _, _, _) = external_runtime(crate::ResidentDurabilityPolicy::Volatile);
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();
    let outcomes = runtime.drain_host_inputs(1).unwrap();
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0].resident_turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
}

#[test]
fn nonmatching_host_packets_do_not_execute_a_resident_turn() {
    let (mut runtime, _, reads, _) = external_runtime(crate::ResidentDurabilityPolicy::Volatile);
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://other/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();

    let outcome = runtime.drain_resident_host_inputs(64).unwrap();
    assert_eq!(outcome.dequeued_packets, 1);
    assert_eq!(outcome.matched_packets, 0);
    assert_eq!(outcome.ignored_packets, 1);
    assert!(outcome.turn.is_none());
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.program_execution_info().resident_accepted_turns, 0);
    assert_eq!(runtime.program_execution_info().ignored_host_packets, 1);
}

#[test]
fn missing_provider_and_malformed_bytecode_fail_closed() {
    let source = r#"
@clock := missing://clock/tick{:read(delta-seconds)}
delta := @clock/delta-seconds
output := delta
"#;
    let mut runtime = runtime();
    let provider_error = runtime
        .load_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert!(
        provider_error
            .kind_message()
            .starts_with("ProviderUnavailable:")
    );

    let bytecode_error = runtime
        .load_bytecode_program(
            b"not bytecode-v1",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap_err();
    assert!(
        bytecode_error
            .kind_message()
            .starts_with("InvalidBytecode:")
    );
}

fn product_nbody_bytecode() -> Vec<u8> {
    let (mut runtime, _) = product_nbody_runtime();
    runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    encode_program_artifact_bytecode_v1(&execution.artifact).unwrap()
}

#[test]
fn product_nbody_denied_grant_missing_provider_and_contract_mismatch_never_fallback() {
    let bytecode = product_nbody_bytecode();
    for (mut runtime, expected) in [
        (
            configured_product_nbody_runtime(ProductSceneContract::AtMostOnce, true, false).0,
            "AuthorizationDenied:",
        ),
        (
            configured_product_nbody_runtime(ProductSceneContract::AtMostOnce, false, true).0,
            "ProviderUnavailable:",
        ),
        (
            configured_product_nbody_runtime(ProductSceneContract::IdempotentRetry, true, true).0,
            "ProviderContractMismatch:",
        ),
    ] {
        let error = runtime
            .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert!(
            error.kind_message().starts_with(expected),
            "expected {expected}, got {error:?}"
        );
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
    }
}

#[test]
fn successful_resident_activation_never_falls_back_to_a_second_program() {
    let (mut runtime, _) = product_nbody_runtime();
    runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    let revision = runtime.program_execution_info().program_revision;
    let error = runtime
        .load_source_program(
            r#"message := "unsupported second program""#,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap_err();
    assert!(error.kind_message().starts_with("InternalFailure:"));
    assert_eq!(
        runtime.program_route(),
        RuntimeProgramRoute::ResidentExternal
    );
    assert_eq!(runtime.program_execution_info().program_revision, revision);
}

fn product_nbody_state_slots(
    runtime: &crate::MechRuntime,
) -> (mech_core::CellSlotId, mech_core::CellSlotId) {
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("n-body must remain on the resident-external route")
    };
    let positions = execution.artifact.outputs()[0].source;
    let velocity = execution
        .artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State && slot.slot != positions)
        .expect("n-body velocity state slot")
        .slot;
    (positions, velocity)
}

fn product_nbody_slot(runtime: &crate::MechRuntime, slot: mech_core::CellSlotId) -> Vec<f64> {
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("n-body must remain on the resident-external route")
    };
    let ResidentValueBorrow::F64 { values, .. } = execution
        .coordinator
        .instance()
        .state_borrow(slot)
        .expect("n-body state slot must be published")
    else {
        panic!("n-body state slots must be f64")
    };
    values.to_vec()
}

fn quantize_nbody(value: f64) -> i64 {
    (value / 1.0e-10).round() as i64
}

fn hash_quantized_nbody(hash: &mut Sha256, values: &[f64]) {
    for value in values {
        hash.update(quantize_nbody(*value).to_le_bytes());
    }
}

fn finish_hash(hash: Sha256) -> String {
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn advance_product_nbody(runtime: &mut crate::MechRuntime) {
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("timer://clock/tick", "tick").unwrap(),
            crate::RuntimeHostInputValue::F64(
                runtime.program_execution_info().resident_accepted_turns as f64 + 1.0,
            ),
        ))
        .unwrap();
    let outcome = runtime.drain_resident_host_inputs(64).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
}

#[test]
fn public_nbody_viewer_preserves_the_working_fixed_sun_orbits_residently() {
    let (mut runtime, scene) = product_nbody_runtime();
    let loaded = runtime
        .load_source_program(
            PUBLIC_NBODY_VIEWER_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal);

    let expected_radii = [
        0.3871_f64.sqrt() * 44.0,
        0.7233_f64.sqrt() * 44.0,
        1.0000_f64.sqrt() * 44.0,
        1.5237_f64.sqrt() * 44.0,
        5.2029_f64.sqrt() * 44.0,
        9.5370_f64.sqrt() * 44.0,
        19.1910_f64.sqrt() * 44.0,
        30.0690_f64.sqrt() * 44.0,
        39.4820_f64.sqrt() * 44.0,
    ];
    let mut first_mercury = None;
    for _ in 0..4_096 {
        advance_product_nbody(&mut runtime);
        let frame = scene.lock().unwrap().latest.clone();
        assert_eq!(frame.len(), 20);
        assert_eq!(frame[0], 300.0);
        assert_eq!(frame[10], 300.0);
        assert!(frame.iter().all(|value| value.is_finite()));
        first_mercury.get_or_insert((frame[1], frame[11]));
        for (body, expected_radius) in expected_radii.iter().enumerate() {
            let x = frame[body + 1] - 300.0;
            let y = frame[body + 11] - 300.0;
            let radius = x.hypot(y);
            assert!(
                (radius - expected_radius).abs() <= 1.0e-10,
                "body {body} drifted: expected radius {expected_radius}, got {radius}",
            );
        }
    }
    let final_frame = scene.lock().unwrap().latest.clone();
    assert_ne!(first_mercury.unwrap(), (final_frame[1], final_frame[11]));

    let info = runtime.program_execution_info();
    assert_eq!(info.resident_accepted_turns, 4_096);
    assert_eq!(info.resident_rejected_turns, 0);
}

#[test]
fn effect_only_resident_program_executes_once_during_activation() {
    let (mut runtime, scene) = product_nbody_runtime();
    let loaded = runtime
        .load_source_program(
            r#"
@scene := scene://orbit/frame{:write(points)}
points := [1.0 2.0]
@scene/points <- points
"#,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal);
    assert_eq!(loaded.info.observation_count, 0);
    assert_eq!(loaded.info.resident_accepted_turns, 1);
    let trace = scene.lock().unwrap();
    assert_eq!(trace.deliveries, 1);
    assert_eq!(trace.latest, vec![1.0, 2.0]);
}

#[test]
fn resident_turn_duration_rejects_before_scene_publication_and_surfaces_publicly() {
    let (mut runtime, scene) = configured_product_nbody_runtime_with_delay(
        ProductSceneContract::AtMostOnce,
        true,
        true,
        Duration::from_millis(5),
    );
    runtime.config.limits.max_turn_duration_ms = Some(1);
    runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            crate::ResidentDurabilityPolicy::Retained,
        )
        .unwrap();
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("timer://clock/tick", "tick").unwrap(),
            crate::RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();

    let error = runtime.drain_host_inputs(1).unwrap_err();
    assert_eq!(error.kind_name(), "ResidentHostTurnFailed");
    assert_eq!(scene.lock().unwrap().deliveries, 0);
    assert_eq!(runtime.program_execution_info().resident_rejected_turns, 1);
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    assert_eq!(execution.coordinator.instance().published_epoch().get(), 0);
    assert_eq!(
        execution
            .coordinator
            .receipts()
            .next()
            .unwrap()
            .1
            .header
            .failure
            .as_ref()
            .unwrap()
            .phase,
        crate::TurnFailurePhase::Execution,
    );
}

#[test]
fn product_nbody_source_and_bytecode_match_d2_for_4096_accepted_turns() {
    let (mut source_runtime, source_scene) = product_nbody_runtime();
    let source = source_runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(source.route, RuntimeProgramRoute::ResidentExternal);
    let bytecode = {
        let ActiveProgramExecution::ResidentExternal(execution) = &source_runtime.active_program
        else {
            panic!("source n-body route must own a resident artifact")
        };
        encode_program_artifact_bytecode_v1(&execution.artifact).unwrap()
    };

    let (mut bytecode_runtime, bytecode_scene) = product_nbody_runtime();
    let decoded = bytecode_runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(decoded.route, RuntimeProgramRoute::ResidentExternal);
    assert_eq!(source.info.program_revision, decoded.info.program_revision);

    let source_slots = product_nbody_state_slots(&source_runtime);
    let bytecode_slots = product_nbody_state_slots(&bytecode_runtime);
    let source_probe = match &source_runtime.active_program {
        ActiveProgramExecution::ResidentExternal(execution) => {
            execution.coordinator.instance().structural_probe()
        }
        _ => unreachable!(),
    };
    let bytecode_probe = match &bytecode_runtime.active_program {
        ActiveProgramExecution::ResidentExternal(execution) => {
            execution.coordinator.instance().structural_probe()
        }
        _ => unreachable!(),
    };
    assert_eq!(source_probe, bytecode_probe);
    assert_eq!(source_probe.commit_runtime_call_count, 0);
    assert_eq!(source_probe.legacy_journal_capture_count, 0);
    assert_eq!(
        source_probe.runtime_execution_transaction_construction_count,
        0
    );
    let mut source_trajectory = Sha256::new();
    let mut bytecode_trajectory = Sha256::new();
    let mut source_scene_trajectory = Sha256::new();
    let mut bytecode_scene_trajectory = Sha256::new();

    for turn in 0..4_096 {
        advance_product_nbody(&mut source_runtime);
        advance_product_nbody(&mut bytecode_runtime);

        let source_x = product_nbody_slot(&source_runtime, source_slots.0);
        let source_v = product_nbody_slot(&source_runtime, source_slots.1);
        let bytecode_x = product_nbody_slot(&bytecode_runtime, bytecode_slots.0);
        let bytecode_v = product_nbody_slot(&bytecode_runtime, bytecode_slots.1);
        assert_eq!(source_x, bytecode_x, "position mismatch at turn {turn}");
        assert_eq!(source_v, bytecode_v, "velocity mismatch at turn {turn}");
        hash_quantized_nbody(&mut source_trajectory, &source_x);
        hash_quantized_nbody(&mut source_trajectory, &source_v);
        hash_quantized_nbody(&mut bytecode_trajectory, &bytecode_x);
        hash_quantized_nbody(&mut bytecode_trajectory, &bytecode_v);

        let source_frame = source_scene.lock().unwrap().latest.clone();
        let bytecode_frame = bytecode_scene.lock().unwrap().latest.clone();
        assert_eq!(source_frame, bytecode_frame);
        assert_eq!(source_frame.len(), 20);
        assert!(source_frame.iter().all(|value| value.is_finite()));
        assert!(
            source_frame
                .iter()
                .all(|value| (0.0..=600.0).contains(value))
        );
        hash_quantized_nbody(&mut source_scene_trajectory, &source_frame);
        hash_quantized_nbody(&mut bytecode_scene_trajectory, &bytecode_frame);

        for runtime in [&source_runtime, &bytecode_runtime] {
            let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program
            else {
                unreachable!()
            };
            assert_eq!(execution.coordinator.input_facts().count(), 0);
            assert_eq!(execution.coordinator.receipts().count(), 0);
            assert_eq!(execution.coordinator.pending_outbox_count(), 0);
            assert!(!execution.coordinator.has_active_candidate());
        }
    }

    let expected_trajectory = match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => "c6b22824484158404a84bdd19de823d605aa31b5f35622b89af2fc61591268ac",
        ("x86_64", "linux") => "b4d33b7c35c30f890d22e8a7074e415cc54681c1789fac49a80c581204fe86db",
        ("x86_64", "macos") => "5aa064d6b4fcd14952d9391b21d8e4862e754c29180fb2768e29164baef1a9f2",
        platform => panic!("unsupported D2 trajectory platform {platform:?}"),
    };
    let source_trajectory = finish_hash(source_trajectory);
    let bytecode_trajectory = finish_hash(bytecode_trajectory);
    assert_eq!(source_trajectory, expected_trajectory);
    assert_eq!(bytecode_trajectory, expected_trajectory);
    assert_eq!(
        finish_hash(source_scene_trajectory),
        finish_hash(bytecode_scene_trajectory)
    );

    let mut final_state = Sha256::new();
    hash_quantized_nbody(
        &mut final_state,
        &product_nbody_slot(&source_runtime, source_slots.0),
    );
    hash_quantized_nbody(
        &mut final_state,
        &product_nbody_slot(&source_runtime, source_slots.1),
    );
    assert_eq!(
        finish_hash(final_state),
        "8f25d0b2dbdebb62e1ea1667e72a37eabbaf8a254f680935bb77275e1a9e640b"
    );

    for runtime in [&source_runtime, &bytecode_runtime] {
        let info = runtime.program_execution_info();
        assert_eq!(info.resident_accepted_turns, 4_096);
        assert_eq!(info.resident_rejected_turns, 0);
        assert_eq!(info.requirement_count, 2);
        assert_eq!(info.observation_count, 1);
        assert_eq!(info.effect_count, 1);
        let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
            unreachable!()
        };
        assert_eq!(
            execution.coordinator.instance().structural_probe(),
            source_probe
        );
        let probe = runtime.resident_production_probe();
        assert_eq!(probe.resident_turns, 4_096);
        assert_eq!(probe.resident_rejections, 0);
        assert_eq!(probe.scene_effects_prepared, 4_096);
        assert_eq!(probe.scene_effects_delivered, 4_096);
        assert_eq!(probe.scene_effects_before_publication, 0);
        assert_eq!(probe.scene_effects_for_rejected_turns, 0);
    }
    for scene in [source_scene, bytecode_scene] {
        let trace = scene.lock().unwrap();
        assert_eq!(trace.deliveries, 4_096);
        assert_eq!(trace.latest.len(), 20);
        assert_eq!(trace.max_retained_values, 20);
    }
}
