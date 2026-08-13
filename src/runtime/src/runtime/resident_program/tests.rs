use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult,
    OperationContractDeclaration, Ref, ValueData,
};
use mech_engine::{SlotRole, encode_program_artifact_bytecode_v1, resident::ResidentValueBorrow};
use sha2::{Digest, Sha256};

use crate::{
    BasicCapability, CapabilityId, PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeBuilder,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeResidentResourceWriteRequest, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

use super::*;

const PURE_SOURCE: &str =
    include_str!("../../../../../tests/architecture/resident-activation/n-body-source-v1.mec");
const PRODUCT_NBODY_SOURCE: &str =
    include_str!("../../../../../examples/resident-n-body/n-body.mec");

fn runtime() -> crate::MechRuntime {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build()
        .unwrap()
}

#[derive(Debug)]
struct PlanningObservationProvider {
    plans: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    value_bits: Arc<AtomicU64>,
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
}

#[derive(Debug, Default)]
struct ProductSceneTrace {
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
            }))
            .unwrap();
    }
    let subject = runtime.runtime_context().unwrap().subject;
    let mut grants = vec![(9_100, "timer://clock/tick/delta-seconds", vec!["read"])];
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

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(Ref::new(1.0 / 60.0)))
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(Ref::new(1.0 / 60.0)))
    }
}

fn configured_external_runtime() -> (
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
    (runtime, plans, reads, value_bits)
}

fn external_runtime(
    durability: crate::ResidentDurabilityPolicy,
) -> (
    crate::MechRuntime,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
    Arc<AtomicU64>,
) {
    let (mut runtime, plans, reads, value_bits) = configured_external_runtime();
    runtime
        .load_source_program(
            external_source(),
            RuntimeProgramLoadOptions {
                routing: crate::ResidentRoutingPolicy::RequireResident,
                durability,
            },
        )
        .unwrap();
    (runtime, plans, reads, value_bits)
}

#[test]
fn load_options_default_to_prefer_resident_and_volatile() {
    let options = RuntimeProgramLoadOptions::default();
    assert_eq!(
        options.routing,
        crate::ResidentRoutingPolicy::PreferResident
    );
    assert_eq!(
        options.durability,
        crate::ResidentDurabilityPolicy::Volatile
    );
}

#[test]
fn pure_source_and_bytecode_choose_resident_with_equivalent_identity_and_output() {
    let options = RuntimeProgramLoadOptions {
        routing: crate::ResidentRoutingPolicy::RequireResident,
        ..RuntimeProgramLoadOptions::default()
    };
    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(PURE_SOURCE, options)
        .unwrap();
    assert_eq!(source.route, RuntimeProgramRoute::ResidentPure);
    assert!(!source.initial_value.is_empty());
    let ActiveProgramExecution::ResidentPure(source_execution) = &source_runtime.active_program
    else {
        panic!("source route must own a pure resident instance")
    };
    let bytecode = encode_program_artifact_bytecode_v1(&source_execution.artifact).unwrap();

    let mut bytecode_runtime = runtime();
    let bytecode = bytecode_runtime
        .load_bytecode_program(&bytecode, options)
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
fn parsed_tree_can_be_loaded_as_a_resident_program() {
    let options = RuntimeProgramLoadOptions {
        routing: crate::ResidentRoutingPolicy::RequireResident,
        ..RuntimeProgramLoadOptions::default()
    };
    let tree = mech_syntax::parser::parse(external_source().trim()).unwrap();
    let (mut runtime, _, _, _) = configured_external_runtime();
    let outcome = runtime.load_tree_program(&tree, options).unwrap();

    assert_eq!(outcome.route, RuntimeProgramRoute::ResidentExternal);
    assert!(!outcome.initial_value.is_empty());
    assert!(outcome.info.program_revision.is_some());
}

#[test]
fn unsupported_source_falls_back_only_when_policy_allows_it() {
    let unsupported = r#"message := "resident strings are deliberately unsupported""#;
    let mut preferred = runtime();
    let outcome = preferred
        .load_source_program(unsupported, RuntimeProgramLoadOptions::default())
        .unwrap();
    assert_eq!(outcome.route, RuntimeProgramRoute::Legacy);
    assert_eq!(outcome.info.legacy_turns, 1);

    let mut required = runtime();
    let error = required
        .load_source_program(
            unsupported,
            RuntimeProgramLoadOptions {
                routing: crate::ResidentRoutingPolicy::RequireResident,
                ..RuntimeProgramLoadOptions::default()
            },
        )
        .unwrap_err();
    assert_eq!(error.kind_name(), "ResidentRouteFailure");
    assert!(error.kind_message().starts_with("SemanticUnsupported:"));
}

#[test]
fn legacy_only_bypasses_resident_planning() {
    let mut runtime = runtime();
    let outcome = runtime
        .load_source_program(
            PURE_SOURCE,
            RuntimeProgramLoadOptions {
                routing: crate::ResidentRoutingPolicy::LegacyOnly,
                ..RuntimeProgramLoadOptions::default()
            },
        )
        .unwrap();
    assert_eq!(outcome.route, RuntimeProgramRoute::Legacy);
    assert_eq!(runtime.next_resident_instance, 1);
    runtime.record_legacy_live_turns(3).unwrap();
    assert_eq!(runtime.program_execution_info().legacy_turns, 4);
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
fn resident_host_packets_coalesce_before_latest_snapshot_capture() {
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

    let latest = 0.125_f64;
    value_bits.store(latest.to_bits(), Ordering::SeqCst);
    let outcome = runtime.drain_resident_host_inputs(64).unwrap();
    assert_eq!(outcome.dequeued_packets, 2);
    assert_eq!(outcome.matched_packets, 2);
    assert_eq!(outcome.coalesced_packets, 1);
    assert_eq!(outcome.ignored_packets, 0);
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(reads.load(Ordering::SeqCst), 1);
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
    assert_eq!(value.bits(), latest.to_bits());
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
        .load_source_program(source, RuntimeProgramLoadOptions::default())
        .unwrap_err();
    assert_ne!(runtime.program_route(), RuntimeProgramRoute::Legacy);
    assert!(
        provider_error
            .kind_message()
            .starts_with("ProviderUnavailable:")
    );

    let bytecode_error = runtime
        .load_bytecode_program(b"not bytecode-v1", RuntimeProgramLoadOptions::default())
        .unwrap_err();
    assert!(
        bytecode_error
            .kind_message()
            .starts_with("InvalidBytecode:")
    );
    assert_ne!(runtime.program_route(), RuntimeProgramRoute::Legacy);
}

fn product_nbody_bytecode() -> Vec<u8> {
    let (mut runtime, _) = product_nbody_runtime();
    runtime
        .load_source_program(
            PRODUCT_NBODY_SOURCE,
            RuntimeProgramLoadOptions {
                routing: crate::ResidentRoutingPolicy::RequireResident,
                durability: crate::ResidentDurabilityPolicy::Volatile,
            },
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
            .load_bytecode_program(&bytecode, RuntimeProgramLoadOptions::default())
            .unwrap_err();
        assert!(
            error.kind_message().starts_with(expected),
            "expected {expected}, got {error:?}"
        );
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
        assert_eq!(runtime.program_execution_info().legacy_turns, 0);
    }
}

#[test]
fn successful_resident_activation_never_falls_back_to_a_second_program() {
    let (mut runtime, _) = product_nbody_runtime();
    runtime
        .load_source_program(PRODUCT_NBODY_SOURCE, RuntimeProgramLoadOptions::default())
        .unwrap();
    let revision = runtime.program_execution_info().program_revision;
    let error = runtime
        .load_source_program(
            r#"message := "unsupported second program""#,
            RuntimeProgramLoadOptions::default(),
        )
        .unwrap_err();
    assert!(error.kind_message().starts_with("InternalFailure:"));
    assert_eq!(
        runtime.program_route(),
        RuntimeProgramRoute::ResidentExternal
    );
    assert_eq!(runtime.program_execution_info().program_revision, revision);
    assert_eq!(runtime.program_execution_info().legacy_turns, 0);
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
            crate::RuntimeHostInputSource::new("timer://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(1.0 / 60.0),
        ))
        .unwrap();
    let outcome = runtime.drain_resident_host_inputs(64).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
}

#[test]
fn product_nbody_source_and_bytecode_match_d2_for_4096_accepted_turns() {
    let options = RuntimeProgramLoadOptions {
        routing: crate::ResidentRoutingPolicy::RequireResident,
        durability: crate::ResidentDurabilityPolicy::Volatile,
    };
    let (mut source_runtime, source_scene) = product_nbody_runtime();
    let source = source_runtime
        .load_source_program(PRODUCT_NBODY_SOURCE, options)
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
        .load_bytecode_program(&bytecode, options)
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
        assert_eq!(source_frame, source_x);
        assert_eq!(bytecode_frame, bytecode_x);
        assert_eq!(source_frame, bytecode_frame);
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
        assert_eq!(info.legacy_turns, 0);
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
    }
    for scene in [source_scene, bytecode_scene] {
        let trace = scene.lock().unwrap();
        assert_eq!(trace.deliveries, 4_096);
        assert_eq!(trace.latest.len(), 30);
        assert_eq!(trace.max_retained_values, 30);
    }
}
