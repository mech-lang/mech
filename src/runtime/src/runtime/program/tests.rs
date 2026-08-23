use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult,
    OperationContractDeclaration, ParsedProgram, Ref, ValueData, hash_str,
};
use mech_engine::{
    __resident::ResidentStorageClass, ArtifactSource, BindingDeclaration, ProgramArtifactDraft,
    SlotRole, decode_program_artifact_bytecode_v1, encode_program_artifact_bytecode_v1,
    resident::ResidentValueBorrow,
};
use sha2::{Digest, Sha256};

use crate::{
    BasicCapability, BasicConstraints, Capability, CapabilityDecision, CapabilityId,
    CapabilityRequest, InMemorySourceResolver, ModuleBuildOptions, PreparedRuntimeEffect,
    RuntimeAfterCommitEffect, RuntimeBuilder, RuntimeEffectCost, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimeIngress, RuntimeResidentResourceWriteRequest, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, SourceRequest,
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
struct TypedObservationProvider {
    planned: LegacyValue,
}

impl RuntimeResourceProvider for TypedObservationProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["test://typed/value".to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.planned.try_deep_snapshot()
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        panic!("typed resident host packets must not re-read the provider")
    }
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
        assert!(matches!(request.path.as_str(), "points" | "replace"));
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
        let values = product_scene_points(&request.path, &request.value);
        self.trace.lock().unwrap().preparations += 1;
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            ProductSceneDelivery {
                trace: self.trace.clone(),
                values,
                operation: request.path,
            },
        )))
    }
}

fn product_scene_points(path: &str, value: &LegacyValue) -> Vec<f64> {
    if let LegacyValue::MutableReference(reference) = value {
        return product_scene_points(path, &reference.borrow());
    }
    if path == "replace" {
        let LegacyValue::Record(scene) = value else {
            panic!("scene replacement must be a record, got {value:?}")
        };
        let scene = scene.borrow();
        let point_sets = scene
            .get(&hash_str("point-sets"))
            .expect("public N-body scene point-sets");
        return product_scene_point_set_positions(point_sets);
    }
    product_scene_matrix_values(value)
}

fn product_scene_point_set_positions(value: &LegacyValue) -> Vec<f64> {
    if let LegacyValue::MutableReference(reference) = value {
        return product_scene_point_set_positions(&reference.borrow());
    }
    let point_set = match value {
        LegacyValue::Record(record) => record.clone(),
        LegacyValue::Tuple(tuple) if tuple.borrow().elements.len() == 1 => {
            let tuple = tuple.borrow();
            let LegacyValue::Record(record) = tuple.elements[0].as_ref() else {
                panic!("scene point-set tuple must contain a record")
            };
            record.clone()
        }
        other => panic!("scene point-sets must contain a record, got {other:?}"),
    };
    let point_set = point_set.borrow();
    product_scene_matrix_values(
        point_set
            .get(&hash_str("positions"))
            .expect("public N-body point-set positions"),
    )
}

fn product_scene_matrix_values(value: &LegacyValue) -> Vec<f64> {
    if let LegacyValue::MutableReference(reference) = value {
        return product_scene_matrix_values(&reference.borrow());
    }
    match value {
        LegacyValue::MatrixF64(matrix) => matrix.as_vec(),
        LegacyValue::MatrixValue(matrix) => matrix
            .as_vec()
            .into_iter()
            .map(|value| *value.as_f64().unwrap().borrow())
            .collect(),
        other => panic!("scene points must be an f64 matrix, got {other:?}"),
    }
}

#[derive(Debug)]
struct ProductSceneDelivery {
    trace: Arc<Mutex<ProductSceneTrace>>,
    values: Vec<f64>,
    operation: String,
}

impl RuntimeAfterCommitEffect for ProductSceneDelivery {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "scene".to_owned(),
            },
            self.operation.clone(),
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
        grants.push((
            9_102,
            "scene://orbit/frame/replace",
            vec!["write", "replace"],
        ));
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
        Some(
            output
                .interactive_binding
                .as_ref()
                .map(|binding| binding.lexical_name.clone())
                .unwrap_or_else(|| output.name.clone())
        )
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
fn ordinary_output_names_are_never_inferred_as_interactive_symbols() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_source(include_str!(
            "../../../../../tests/fixtures/shims/all-slots.mec"
        ))
        .unwrap();
    let compiled = product.artifact();
    let mut outputs = compiled.outputs().to_vec();
    assert!(
        outputs.len() >= 2,
        "the rich document fixture must expose multiple outputs"
    );
    for (index, output) in outputs.iter_mut().enumerate() {
        output.name = if index == 0 {
            "mech-repl-symbol-61".to_owned()
        } else {
            format!("ordinary-output-{index}")
        };
        output.interactive_binding = None;
    }
    let expected_names = outputs
        .iter()
        .map(|output| output.name.clone())
        .collect::<Vec<_>>();
    let ordinary = ProgramArtifactDraft {
        schemas: compiled.schemas().clone(),
        constants: compiled.constants().clone(),
        contracts: compiled.contracts().clone(),
        requirements: compiled.requirements().clone(),
        inputs: compiled.inputs().to_vec().into_boxed_slice(),
        slots: compiled.slots().to_vec().into_boxed_slice(),
        nodes: compiled.nodes().to_vec().into_boxed_slice(),
        bindings: compiled.bindings().to_vec().into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
        constraints: compiled.constraints().to_vec().into_boxed_slice(),
        compute_regions: compiled.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .unwrap();
    let first_id = ordinary.outputs()[0].output;

    let mut runtime = runtime();
    runtime
        .load_compiled_program(ordinary, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(
        runtime.output_name(first_id).as_deref(),
        Some("mech-repl-symbol-61")
    );
    assert_eq!(
        runtime
            .root_symbol_values_all()
            .unwrap()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
        expected_names
    );
}

#[test]
fn formatted_document_outputs_survive_source_and_bytecode_publication() {
    let source = include_str!("../../../../../examples/working/fizzbuzz.mec");
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler.compile_source(source).unwrap();
    let source_outputs = product
        .artifact()
        .outputs()
        .iter()
        .map(|output| output.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        source_outputs,
        ["y"],
        "integrity constraints are not ordinary published outputs"
    );
    let mut source_runtime = runtime();
    let source_loaded = source_runtime
        .load_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(source_loaded.route, RuntimeProgramRoute::ResidentPure);
    let mut interactive_runtime = runtime();
    let interactive_loaded = interactive_runtime
        .load_interactive_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let program_output_id = interactive_runtime
        .program_output_id()
        .expect("FizzBuzz must retain its final non-constraint result");
    assert_eq!(
        interactive_runtime
            .output_name(program_output_id)
            .as_deref(),
        Some("y"),
        "the trailing integrity constraint must not replace the program output"
    );
    assert!(
        interactive_loaded
            .initial_value
            .to_value()
            .format_canonical_inline()
            .contains("✨🐝"),
        "interactive loading must select the final ordinary result after formatted-document outputs"
    );
    let mut bytecode_runtime = runtime();
    let bytecode_loaded = bytecode_runtime
        .load_bytecode_program(
            product.bytecode(),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(bytecode_loaded.route, RuntimeProgramRoute::ResidentPure);
    assert_eq!(
        source_loaded.info.program_revision,
        bytecode_loaded.info.program_revision
    );
    let decoded = decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
    assert_eq!(
        decoded
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        source_outputs,
        "bytecode v1 must preserve formatted-document output symbols"
    );

    let mut resolver = InMemorySourceResolver::new();
    resolver.insert_string("fizzbuzz.mec", source).unwrap();
    let mut rooted_compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let rooted = rooted_compiler
        .compile_root(
            SourceRequest::new("fizzbuzz.mec"),
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();
    assert_eq!(
        rooted
            .artifact()
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        source_outputs,
        "rooted formatted documents must publish the same output symbols"
    );

    let rich_source = include_str!("../../../../../tests/fixtures/shims/all-slots.mec");
    let rich = compiler.compile_source(rich_source).unwrap();
    let rich_outputs = rich
        .artifact()
        .outputs()
        .iter()
        .map(|output| output.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        rich_outputs.len(),
        3,
        "inline, fenced, and root outputs publish"
    );
    assert_eq!(
        decode_program_artifact_bytecode_v1(rich.bytecode())
            .unwrap()
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        rich_outputs,
        "rich document outputs must survive bytecode-v1 encoding"
    );

    let mut rich_runtime = runtime();
    rich_runtime
        .load_source_program(rich_source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let inline = rich_runtime
        .output_value(mech_core::OutputId::new(0))
        .unwrap()
        .unwrap()
        .into_value();
    assert!(
        matches!(inline, LegacyValue::F64(ref value) if *value.borrow() == 42.0),
        "the first published output must retain the evaluated inline value: {inline:?}"
    );
}

#[test]
fn interactive_program_output_is_the_final_statement_without_a_fenced_output() {
    let source = include_str!("../../../../../examples/working/factorial.mec");
    let mut runtime = runtime();
    let loaded = runtime
        .load_interactive_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let output_id = runtime
        .program_output_id()
        .expect("factorial must publish its final statement");

    assert_eq!(runtime.output_name(output_id).as_deref(), Some("res"));
    assert_eq!(loaded.initial_value.to_value().to_string(), "120");
    assert_eq!(
        runtime
            .output_value(output_id)
            .unwrap()
            .unwrap()
            .to_value()
            .to_string(),
        "120"
    );
}

#[test]
fn activation_only_compilation_preserves_the_artifact_without_retaining_bytecode() {
    const SOURCE: &str = "answer := 40f32 + 2f32\nanswer";
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let activation = compiler.compile_source_artifact(SOURCE).unwrap();
    let durable = compiler.compile_source(SOURCE).unwrap();

    assert_eq!(
        activation
            .artifact()
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        durable
            .artifact()
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        activation.artifact().nodes().len(),
        durable.artifact().nodes().len()
    );
    assert_eq!(
        activation.artifact().compute_regions(),
        durable.artifact().compute_regions()
    );
}

#[test]
fn static_initialization_returns_detached_column_major_matrix_values() {
    let tree = mech_syntax::parse("matrix := [1f32 2f32; 3f32 4f32]").unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mut first = compiler
        .evaluate_static_tree_symbols(&tree, &["matrix"])
        .unwrap();
    assert_eq!(
        first.remove("matrix"),
        Some(RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 2,
            values: vec![1.0, 3.0, 2.0, 4.0],
        })
    );

    let second = compiler
        .evaluate_static_tree_symbols(&tree, &["matrix"])
        .unwrap();
    assert_eq!(
        second["matrix"],
        RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 2,
            values: vec![1.0, 3.0, 2.0, 4.0],
        }
    );
}

#[test]
fn planning_values_seed_explicit_live_inputs_while_literals_remain_constants() {
    let tree =
        mech_syntax::parse("supplied-port := supplied\nvalue := supplied-port + 2f32\nvalue")
            .unwrap();
    let inputs = BTreeMap::from([("supplied".to_owned(), RuntimeHostInputValue::F32(40.0))]);
    let external = BTreeSet::from(["supplied-port".to_owned()]);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let (product, initial_inputs) = compiler
        .compile_tree_artifact_with_input_initializers(&tree, &inputs, &external)
        .unwrap();

    assert_eq!(
        product
            .artifact()
            .inputs()
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        ["supplied-port"]
    );
    assert_eq!(
        initial_inputs["supplied-port"],
        RuntimeHostInputValue::F32(40.0)
    );
    assert!(
        (0..product.artifact().constants().len()).any(|index| {
            product
                .artifact()
                .constants()
                .get(mech_core::ConstantId::new(index as u32))
                .is_some_and(
                    |value| matches!(value.data(), ValueData::F32(value) if value.to_f32() == 2.0),
                )
        }),
        "the source literal must remain an embedded artifact constant"
    );
}

#[test]
fn matrix_declaration_defaults_become_typed_live_inputs() {
    let tree =
        mech_syntax::parse("matrix := [1f32 2f32; 3f32 4f32]\nresult := matrix + 1f32\nresult")
            .unwrap();
    let external = BTreeSet::from(["matrix".to_owned()]);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let (product, initial_inputs) = compiler
        .compile_tree_artifact_with_input_initializers(&tree, &BTreeMap::new(), &external)
        .unwrap();

    let input = product
        .artifact()
        .inputs()
        .iter()
        .find(|input| input.name == "matrix")
        .expect("the matrix declaration must become an artifact input");
    assert_eq!(
        initial_inputs["matrix"],
        RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 2,
            values: vec![1.0, 3.0, 2.0, 4.0],
        }
    );
    assert!(product.artifact().bindings().iter().any(|binding| {
        matches!(
            binding,
            BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            } if *slot == input.slot
        )
    }));
}

#[cfg(feature = "compute")]
const MIXED_COMPUTE_SOURCE: &str = r#"
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
@compute/input/x <- 2f32
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
x := 1f32
result := x + 2f32
result
"#;

#[cfg(feature = "compute")]
#[test]
fn mixed_tree_compilation_owns_partitioning_and_typed_initializers() {
    let tree = mech_syntax::parse(MIXED_COMPUTE_SOURCE).unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_tree(&tree).unwrap();

    assert!(mixed.coordinator.artifact().compute_regions().is_empty());
    assert_eq!(mixed.compute.declaration.name.as_ref(), "calculation");
    assert_eq!(mixed.compute.interface.inputs.len(), 1);
    assert_eq!(mixed.compute.interface.outputs.len(), 1);
    assert_eq!(mixed.compute.interface.outputs[0].name.as_ref(), "result");
    let input = &mixed.compute.interface.inputs[0];
    assert_eq!(input.name.as_ref(), "x");
    assert!(input.dimensions.is_empty());
    assert_eq!(
        mixed.compute.initializers.get(input.id),
        Some(&mech_compute::ComputeValue::ScalarF32(1.0))
    );
}

#[cfg(feature = "compute")]
#[test]
fn mixed_tree_coordinator_retains_interactive_root_symbols() {
    let tree = mech_syntax::parse(
        r#"
visible := 41
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
@compute/input/x <- visible
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
x := 1f32
result := x + 1f32
result
"#,
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_tree(&tree).unwrap();
    let names = mixed
        .coordinator
        .artifact()
        .outputs()
        .iter()
        .filter_map(|output| output.interactive_binding.as_ref())
        .map(|binding| binding.lexical_name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"visible"));
}

#[cfg(feature = "compute")]
#[test]
fn mixed_tree_retains_array_activation_as_outer_broadcast_extent() {
    let tree = mech_syntax::parse(
        r#"
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
lanes := [1f32 2f32 3f32 4f32]
@compute/input/x <- lanes
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
x := 0f32
result := x + 1f32
result
"#,
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_tree(&tree).unwrap();

    assert_eq!(
        mixed.activation_inputs["x"],
        mech_compute::ComputeValue::TensorF32 {
            dimensions: vec![4].into_boxed_slice(),
            layout: mech_compute::TensorLayout::RowMajor,
            values: Arc::from([1.0, 2.0, 3.0, 4.0]),
        }
    );
    let input = mixed.compute.interface.input_named("x").unwrap();
    assert_eq!(
        mixed.compute.initializers.get(input.id),
        Some(&mech_compute::ComputeValue::ScalarF32(0.0)),
        "the one-lane port initializer must remain distinct from its outer activation batch",
    );
}

#[cfg(feature = "compute")]
#[test]
fn mixed_tree_normalizes_matrix_initializers_to_canonical_row_major_layout() {
    let tree = mech_syntax::parse(
        r#"
@compute := compute://worker/kernel{:write(input/matrix), :write(turn)}
@compute/input/matrix <- [0f32 0f32; 0f32 0f32]
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
matrix := [1f32 2f32; 3f32 4f32]
result := matrix + 1f32
result
"#,
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_tree(&tree).unwrap();
    let input = mixed.compute.interface.input_named("matrix").unwrap();

    assert_eq!(input.dimensions.as_ref(), [2, 2]);
    assert_eq!(
        mixed.compute.initializers.get(input.id),
        Some(&mech_compute::ComputeValue::TensorF32 {
            dimensions: vec![2, 2].into_boxed_slice(),
            layout: mech_compute::TensorLayout::RowMajor,
            values: Arc::from([1.0, 2.0, 3.0, 4.0]),
        })
    );
}

#[cfg(feature = "compute")]
#[test]
fn mixed_tree_rejects_coordinator_input_with_the_wrong_shape_without_a_provider() {
    let tree = mech_syntax::parse(
        r#"
@compute := compute://worker/kernel{:write(input/matrix), :write(turn)}
@compute/input/matrix <- [1f32; 2f32]
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
matrix := [1f32 2f32; 3f32 4f32]
result := matrix + 1f32
result
"#,
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let error = compiler.compile_mixed_tree(&tree).unwrap_err();
    let rendered = format!("{error:?}");
    assert!(rendered.contains("compute boundary planning failed"));
    assert!(rendered.contains("DimensionMismatch"));
}

#[cfg(feature = "compute")]
#[test]
fn mixed_root_preserves_imports_in_coordinator_and_compute_products() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string(
            "main.mec",
            r#"
+> ./dep.mec
+> math
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
coordinator-value := math/sin(dep/value) + 1f32
@compute/input/x <- coordinator-value
@compute/turn <- 1
coordinator-value

calculation @compute
-------------------------------------------------------------------------------
x := 1f32
result := math/cos(x) + dep/value
result
"#,
        )
        .unwrap();
    resolver
        .insert_string("dep.mec", "value := 2f32\n<+ value\nvalue\n")
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();

    let mixed = compiler
        .compile_mixed_root(
            SourceRequest::new("main.mec"),
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();

    assert!(
        mixed
            .coordinator
            .artifact()
            .outputs()
            .iter()
            .any(|output| output.name == "coordinator-value")
    );
    assert!(
        mixed
            .coordinator
            .artifact()
            .outputs()
            .iter()
            .all(|output| output.name != "result"),
        "the coordinator must execute its generated partition, not the cached full source tree",
    );
    assert!(
        mixed
            .compute
            .artifact
            .outputs()
            .iter()
            .any(|output| output.name == "result")
    );
    assert!(
        mixed
            .compute
            .artifact
            .outputs()
            .iter()
            .all(|output| output.name != "coordinator-value"),
        "the compute artifact must execute its generated partition, not the cached full source tree",
    );
    let input = mixed.compute.interface.input_named("x").unwrap();
    assert_eq!(
        mixed.compute.initializers.get(input.id),
        Some(&mech_compute::ComputeValue::ScalarF32(1.0))
    );
}

#[cfg(feature = "compute")]
#[test]
fn mixed_compilation_rejects_multiple_compute_regions_for_v04() {
    let tree = mech_syntax::parse(
        r#"
first @compute
-------------------------------------------------------------------------------
a := 1f32 + 1f32

second @cpu
-------------------------------------------------------------------------------
b := 2f32 + 2f32
"#,
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let error = compiler.compile_mixed_tree(&tree).unwrap_err();

    assert!(
        error
            .kind_message()
            .contains("exactly one executable compute region")
    );
}

#[test]
fn variable_definition_metadata_and_state_survive_resident_bytecode_admission() {
    const SOURCE: &str = "input := 1.0\n~state := 2.0\nstate";

    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler.compile_source(SOURCE).unwrap();
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    let input_id = hash_str("input");
    let state_id = hash_str("state");
    assert!(parsed.symbols.contains_key(&input_id));
    assert!(parsed.symbols.contains_key(&state_id));
    assert_eq!(parsed.dictionary.get(&input_id).unwrap(), "input");
    assert_eq!(parsed.dictionary.get(&state_id).unwrap(), "state");
    assert!(!parsed.mutable_symbols.contains(&input_id));
    assert!(parsed.mutable_symbols.contains(&state_id));
    assert!(
        product
            .artifact()
            .slots()
            .iter()
            .any(|slot| slot.role == SlotRole::State)
    );

    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let mut bytecode_runtime = runtime();
    let bytecode = bytecode_runtime
        .load_bytecode_program(
            product.bytecode(),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(source.route, RuntimeProgramRoute::ResidentPure);
    assert_eq!(bytecode.route, RuntimeProgramRoute::ResidentPure);
    assert_eq!(source.initial_value, bytecode.initial_value);
    assert_eq!(source.info.program_revision, bytecode.info.program_revision);
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
fn integrity_constraints_match_across_source_and_bytecode_resident_admission() {
    const PASSING: &str = "x := 1.0\nsafe! := x <= 2.0";
    const FAILING: &str = "x := 3.0\nsafe! := x <= 2.0";

    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let passing = compiler.compile_source(PASSING).unwrap();
    assert_eq!(passing.artifact().constraints().len(), 1);

    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(PASSING, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let mut bytecode_runtime = runtime();
    let bytecode = bytecode_runtime
        .load_bytecode_program(
            passing.bytecode(),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(source.initial_value, bytecode.initial_value);
    assert_eq!(source.info.program_revision, bytecode.info.program_revision);

    let failing = compiler.compile_source(FAILING).unwrap();
    let mut rejected_source = runtime();
    assert!(
        rejected_source
            .load_source_program(FAILING, crate::ResidentDurabilityPolicy::Volatile)
            .is_err()
    );
    assert_eq!(rejected_source.program_route(), RuntimeProgramRoute::None);
    let mut rejected_bytecode = runtime();
    assert!(
        rejected_bytecode
            .load_bytecode_program(
                failing.bytecode(),
                crate::ResidentDurabilityPolicy::Volatile,
            )
            .is_err()
    );
    assert_eq!(rejected_bytecode.program_route(), RuntimeProgramRoute::None);
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
fn resident_matrix_comprehensions_feed_mutable_vertical_concatenation() {
    const SOURCE: &str = r#"
samples := 1..=3
x-row := [1.0 | sample <- samples]
y-row := [2.0 | sample <- samples]
x := x-row'
y := y-row'
~trail := [x y]
new-row := ([7.0 8.0])
next-trail := matrix/vertcat(trail[2..=3,:], new-row)
trail = next-trail
trail
"#;

    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let compiled = compiler.compile_source(SOURCE).unwrap();

    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let mut bytecode_runtime = runtime();
    let bytecode = bytecode_runtime
        .load_bytecode_program(
            compiled.bytecode(),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();

    for loaded in [source, bytecode] {
        assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);
        let LegacyValue::MatrixF64(matrix) = loaded.initial_value.to_value() else {
            panic!("compact resident trail must remain an f64 matrix")
        };
        assert_eq!((matrix.rows(), matrix.cols()), (3, 2));
        assert_eq!(matrix.as_vec(), [1.0, 1.0, 7.0, 2.0, 2.0, 8.0]);
    }
}

#[test]
fn empty_matrix_comprehension_activates_through_source_and_bytecode() {
    const SOURCE: &str = r#"
samples := 1..=3
empty := [sample | sample <- samples, sample > 4.0]
empty
"#;

    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let compiled = compiler.compile_source(SOURCE).unwrap();

    let mut source_runtime = runtime();
    let source = source_runtime
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let mut bytecode_runtime = runtime();
    let bytecode = bytecode_runtime
        .load_bytecode_program(
            compiled.bytecode(),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();

    for loaded in [source, bytecode] {
        assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);
        let LegacyValue::MatrixIndex(matrix) = loaded.initial_value.to_value() else {
            panic!("empty comprehension must use the canonical empty resident matrix carrier")
        };
        assert_eq!((matrix.rows(), matrix.cols()), (0, 0));
        assert!(matrix.as_vec().is_empty());
    }
}

#[test]
fn live_comprehension_membership_is_rejected_before_resident_activation() {
    const FILTER: &str = r#"
~gate := 0.0
samples := 1..=3
values := [sample | sample <- samples, gate > 0.0]
values
"#;
    const GENERATOR: &str = r#"
~samples := [1.0 2.0 3.0]
values := [sample | sample <- samples]
values
"#;

    for (qualifier, source) in [("filter", FILTER), ("generator", GENERATOR)] {
        let error = runtime()
            .load_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        let failure = error.kind_as::<ResidentRouteFailure>().unwrap();
        assert!(
            failure.class == ResidentRouteFailureClass::SemanticUnsupported
                && failure
                    .reason
                    .contains("ReactiveComprehensionStructureUnsupported")
                && failure.reason.contains(qualifier),
            "live {qualifier} membership must fail explicitly instead of freezing its initial cardinality: {error:?}",
        );
    }
}

#[test]
fn matrix_comprehension_scope_preserves_the_enclosing_live_plan() {
    const SOURCE: &str = r#"
@clock := test://clock/tick{:read(delta-seconds)}
delta := @clock/delta-seconds
samples := 1..=3
values := [delta + 1.0 | sample <- samples]
values
"#;

    let (mut runtime, _, _, _) = configured_external_runtime();
    let loaded = runtime
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();

    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal);
    assert_eq!(
        runtime.program_execution_info().observation_count,
        1,
        "planning a comprehension must not erase live nodes accumulated by its enclosing program",
    );

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(2.0),
        ))
        .unwrap();
    runtime.drain_resident_host_inputs(1).unwrap();

    let LegacyValue::MatrixF64(values) = runtime.root_symbol_value("values").unwrap().into_value()
    else {
        panic!("live comprehension output must remain an f64 matrix")
    };
    assert_eq!(
        values.as_vec(),
        [3.0, 3.0, 3.0],
        "operations inside the comprehension must execute on every accepted turn",
    );
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
valid! := output < 10.0
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

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(10.0),
        ))
        .unwrap();
    let rejected = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        rejected.turn,
        Some(crate::ResidentExternalTurnOutcome::Rejected { .. })
    ));
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    assert!(matches!(
        execution.coordinator.instance().output_borrow(0),
        Some(ResidentValueBorrow::F64 { values, .. }) if values == [9.0]
    ));
}

#[test]
fn dynamic_matrix_selector_recomputes_resident_scalar_access() {
    let configured_runtime = || {
        let mut runtime = runtime();
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
                CapabilityId(9_024),
                subject,
                "test://clock/tick/delta-seconds",
                ["read"],
            )))
            .unwrap();
        runtime
    };
    let mut runtime = configured_runtime();
    runtime
        .load_source_program(
            r#"
@clock := test://clock/tick{:read(delta-seconds)}
values := [10.0 20.0 30.0]
index := @clock/delta-seconds
selected := values[1,index]
selected
"#,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(2.0),
        ))
        .unwrap();
    runtime.drain_resident_host_inputs(1).unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("dynamic scalar access must remain resident")
    };
    assert!(execution.artifact.nodes().iter().any(|node| {
        node.operation.module_path.as_ref() == ["access"]
            && node.operation.operation_name == "index"
    }));
    assert!(matches!(
        execution.coordinator.instance().output_borrow(0),
        Some(ResidentValueBorrow::F64 { values, .. }) if values == [20.0]
    ));
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();

    let mut bytecode_runtime = configured_runtime();
    bytecode_runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    bytecode_runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(3.0),
        ))
        .unwrap();
    bytecode_runtime.drain_resident_host_inputs(1).unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &bytecode_runtime.active_program
    else {
        panic!("bytecode scalar access must remain resident")
    };
    assert!(matches!(
        execution.coordinator.instance().output_borrow(0),
        Some(ResidentValueBorrow::F64 { values, .. }) if values == [30.0]
    ));
}

#[cfg(feature = "compiler_default")]
fn assert_dynamic_scalar_selector_round_trip(
    planned: LegacyValue,
    source_packet: crate::RuntimeHostInputValue,
    bytecode_packet: crate::RuntimeHostInputValue,
) {
    const SOURCE: &str = r#"
@typed := test://typed/value{:read(data)}
values := [10.0 20.0 30.0]
selected := values[1,@typed/data]
selected
"#;

    let configured_runtime = |planned: LegacyValue| {
        let mut runtime = runtime();
        runtime
            .register_resource_provider(Box::new(TypedObservationProvider { planned }))
            .unwrap();
        let subject = runtime.runtime_context().unwrap().subject;
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(9_025),
                subject,
                "test://typed/value/data",
                ["read"],
            )))
            .unwrap();
        runtime
    };

    let planned_for_bytecode = planned.try_deep_snapshot().unwrap();
    let mut source = configured_runtime(planned);
    source
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    source
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            source_packet,
        ))
        .unwrap();
    source.drain_resident_host_inputs(1).unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &source.active_program else {
        panic!("dynamic scalar access must remain resident")
    };
    assert!(matches!(
        execution.coordinator.instance().output_borrow(0),
        Some(ResidentValueBorrow::F64 { values, .. }) if values == [20.0]
    ));
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();

    let mut decoded = configured_runtime(planned_for_bytecode);
    decoded
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    decoded
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            bytecode_packet,
        ))
        .unwrap();
    decoded.drain_resident_host_inputs(1).unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &decoded.active_program else {
        panic!("bytecode scalar access must remain resident")
    };
    assert!(matches!(
        execution.coordinator.instance().output_borrow(0),
        Some(ResidentValueBorrow::F64 { values, .. }) if values == [30.0]
    ));
}

#[test]
#[cfg(feature = "compiler_default")]
fn dynamic_matrix_selectors_preserve_every_supported_scalar_kind() {
    macro_rules! assert_kind {
        ($value:ident, $planned:expr, $source:expr, $bytecode:expr) => {
            assert_dynamic_scalar_selector_round_trip(
                LegacyValue::$value(Ref::new($planned)),
                crate::RuntimeHostInputValue::$value($source),
                crate::RuntimeHostInputValue::$value($bytecode),
            );
        };
    }

    assert_kind!(U8, 1, 2, 3);
    assert_kind!(U16, 1, 2, 3);
    assert_kind!(U32, 1, 2, 3);
    assert_kind!(U64, 1, 2, 3);
    assert_kind!(U128, 1, 2, 3);
    assert_kind!(I8, 1, 2, 3);
    assert_kind!(I16, 1, 2, 3);
    assert_kind!(I32, 1, 2, 3);
    assert_kind!(I64, 1, 2, 3);
    assert_kind!(I128, 1, 2, 3);
    assert_kind!(F32, 1.0, 2.0, 3.0);
    assert_kind!(F64, 1.0, 2.0, 3.0);
    assert_kind!(Index, 1, 2, 3);
}

#[test]
#[cfg(feature = "compiler_default")]
fn dynamic_matrix_selectors_reject_values_outside_the_portable_width() {
    const SOURCE: &str = r#"
@typed := test://typed/value{:read(data)}
values := [10.0 20.0 30.0]
selected := values[1,@typed/data]
selected
"#;
    const OUTSIDE_PORTABLE_INDEX: u64 = u32::MAX as u64 + 1;

    let configured_runtime = |planned| {
        let mut runtime = runtime();
        runtime
            .register_resource_provider(Box::new(TypedObservationProvider {
                planned: LegacyValue::U64(Ref::new(planned)),
            }))
            .unwrap();
        let subject = runtime.runtime_context().unwrap().subject;
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(9_026),
                subject,
                "test://typed/value/data",
                ["read"],
            )))
            .unwrap();
        runtime
    };

    let mut initially_oversized = configured_runtime(OUTSIDE_PORTABLE_INDEX);
    assert!(
        initially_oversized
            .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
            .is_err()
    );

    let mut source = configured_runtime(1);
    source
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &source.active_program else {
        panic!("dynamic scalar access must remain resident")
    };
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();

    source
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            crate::RuntimeHostInputValue::U64(OUTSIDE_PORTABLE_INDEX),
        ))
        .unwrap();
    let rejected = source.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        rejected.turn,
        Some(crate::ResidentExternalTurnOutcome::Rejected { .. })
    ));

    let mut decoded = configured_runtime(1);
    decoded
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    decoded
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            crate::RuntimeHostInputValue::U64(OUTSIDE_PORTABLE_INDEX),
        ))
        .unwrap();
    let rejected = decoded.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        rejected.turn,
        Some(crate::ResidentExternalTurnOutcome::Rejected { .. })
    ));
}

fn assert_typed_observation_round_trip(planned: LegacyValue, packet: crate::RuntimeHostInputValue) {
    const SOURCE: &str = r#"
@typed := test://typed/value{:read(data)}
@typed/data
"#;

    let planned_for_bytecode = planned.try_deep_snapshot().unwrap();
    let mut source = runtime();
    source
        .register_resource_provider(Box::new(TypedObservationProvider { planned }))
        .unwrap();
    let subject = source.runtime_context().unwrap().subject;
    source
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_024),
            subject,
            "test://typed/value/data",
            ["read"],
        )))
        .unwrap();
    source
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &source.active_program else {
        panic!("typed observation must remain resident")
    };
    let revision = execution.artifact.revision();
    let output = execution.artifact.outputs()[0].output;
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();
    source
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            packet.clone(),
        ))
        .unwrap();
    let source_turn = source.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        source_turn.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    let source_value = source.output_value(output).unwrap().unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &source.active_program else {
        unreachable!()
    };
    assert_eq!(execution.coordinator.input_facts().count(), 1);

    let mut decoded = runtime();
    decoded
        .register_resource_provider(Box::new(TypedObservationProvider {
            planned: planned_for_bytecode,
        }))
        .unwrap();
    let subject = decoded.runtime_context().unwrap().subject;
    decoded
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_024),
            subject,
            "test://typed/value/data",
            ["read"],
        )))
        .unwrap();
    let loaded = decoded
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    assert_eq!(loaded.info.program_revision, Some(revision));
    decoded
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            packet,
        ))
        .unwrap();
    let decoded_turn = decoded.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        decoded_turn.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(source_value, decoded.output_value(output).unwrap().unwrap());
}

#[test]
fn resident_observation_profile_covers_scalars_and_dense_matrices() {
    assert_typed_observation_round_trip(
        LegacyValue::Bool(Ref::new(false)),
        crate::RuntimeHostInputValue::Bool(true),
    );
    assert_typed_observation_round_trip(
        LegacyValue::Index(Ref::new(0)),
        crate::RuntimeHostInputValue::Index(7),
    );
    assert_typed_observation_round_trip(
        LegacyValue::F64(Ref::new(0.0)),
        crate::RuntimeHostInputValue::F64(7.5),
    );
    assert_typed_observation_round_trip(
        LegacyValue::MatrixBool(ValueMatrix::from_vec(vec![false; 4], 2, 2)),
        crate::RuntimeHostInputValue::BoolMatrix {
            rows: 2,
            columns: 2,
            values: vec![true, false, false, true],
        },
    );
    assert_typed_observation_round_trip(
        LegacyValue::MatrixIndex(ValueMatrix::from_vec(vec![0; 4], 2, 2)),
        crate::RuntimeHostInputValue::IndexMatrix {
            rows: 2,
            columns: 2,
            values: vec![1, 2, 3, 4],
        },
    );
    assert_typed_observation_round_trip(
        LegacyValue::MatrixF64(ValueMatrix::from_vec(vec![0.0; 4], 2, 2)),
        crate::RuntimeHostInputValue::F64Matrix {
            rows: 2,
            columns: 2,
            values: vec![1.0, 2.0, 3.0, 4.0],
        },
    );
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
fn interactive_root_loader_retains_document_symbols_and_reports_the_root_result() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string(
            "document.mec",
            "source := 41\nanswer := source + 1\nanswer\n",
        )
        .unwrap();
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build()
        .unwrap();

    let outcome = runtime
        .load_interactive_root_program(
            "document.mec".into(),
            ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();

    assert_eq!(outcome.initial_value.to_string(), "42");
    assert_eq!(
        runtime
            .root_symbol_values_all()
            .unwrap()
            .into_iter()
            .map(|(name, value)| (name, value.to_string()))
            .collect::<Vec<_>>(),
        [
            ("ans".to_string(), "42".to_string()),
            ("answer".to_string(), "42".to_string()),
            ("source".to_string(), "41".to_string()),
        ],
    );
    let answer_id = runtime
        .root_symbol_output_id("answer")
        .expect("answer must retain its resident binding identity");
    let source_id = runtime
        .root_symbol_output_id("source")
        .expect("source must retain its resident binding identity");
    assert_ne!(answer_id, source_id);
    assert_eq!(runtime.output_name(answer_id).as_deref(), Some("answer"));
}

#[test]
fn explicit_root_imported_by_an_earlier_root_still_joins_the_combined_artifact() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string(
            "main.mec",
            "+> ./dep.mec\nanswer := dep/value + 1\nanswer\n",
        )
        .unwrap();
    resolver
        .insert_string("dep.mec", "value := 41\n<+ value\nvalue\n")
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();

    let product = compiler
        .compile_roots(
            &[
                SourceRequest::new("main.mec"),
                SourceRequest::new("dep.mec"),
            ],
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();
    let outputs = product
        .artifact()
        .outputs()
        .iter()
        .map(|output| output.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        outputs,
        ["answer", "value"],
        "explicit roots must be published in caller order"
    );
    let decoded = decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
    assert_eq!(
        decoded
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        outputs,
        "bytecode v1 must retain every explicit root output"
    );
}

#[test]
fn explicit_dependency_root_plans_provider_reads_exactly_once() {
    let plans = Arc::new(AtomicUsize::new(0));
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string(
            "main.mec",
            "+> ./dep.mec\nanswer := dep/value + 1.0\nanswer\n",
        )
        .unwrap();
    resolver
        .insert_string(
            "dep.mec",
            r#"
@clock := test://clock/tick{:read(delta-seconds)}
value := @clock/delta-seconds
<+ value
value
"#,
        )
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .resource_provider(Box::new(PlanningObservationProvider {
            plans: plans.clone(),
            reads: Arc::new(AtomicUsize::new(0)),
            value_bits: Arc::new(AtomicU64::new(41.0_f64.to_bits())),
        }))
        .build_compiler()
        .unwrap();

    let product = compiler
        .compile_roots(
            &[
                SourceRequest::new("main.mec"),
                SourceRequest::new("dep.mec"),
            ],
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();

    assert_eq!(plans.load(Ordering::SeqCst), 1);
    let outputs = product
        .artifact()
        .outputs()
        .iter()
        .map(|output| output.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(outputs, ["answer", "value"]);
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
fn parsed_tree_can_be_loaded_as_a_production_resident_program() {
    let tree = mech_syntax::parser::parse(external_source().trim()).unwrap();
    let (mut runtime, _, _, _) = configured_external_runtime();
    let outcome = runtime
        .load_tree_program(&tree, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();

    assert_eq!(outcome.route, RuntimeProgramRoute::ResidentExternal);
    assert!(!outcome.initial_value.is_empty());
    assert!(outcome.info.program_revision.is_some());
    assert_eq!(outcome.info.route, RuntimeProgramRoute::ResidentExternal);
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
            "tuple := (1, 2); tuple.2",
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
fn production_scalar_string_source_loads_residently_without_fallback() {
    let mut runtime = runtime();
    let outcome = runtime
        .load_source_program(
            r#"message := "resident scalar string""#,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();

    assert_eq!(outcome.route, RuntimeProgramRoute::ResidentPure);
    assert!(matches!(
        outcome.initial_value.to_value(),
        LegacyValue::String(_)
    ));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::ResidentPure);
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
fn resident_recurrence_advances_when_a_same_turn_parent_is_unchanged() {
    let (mut runtime, _, _, _) = configured_external_runtime();
    runtime
        .load_source_program(
            r#"
@clock := test://clock/tick{:read(delta-seconds)}
pulse := @clock/delta-seconds
step := pulse * 0.0 + 1.0
~state := 0.0
next-state := state + step
state = next-state
state
"#,
            crate::ResidentDurabilityPolicy::Retained,
        )
        .unwrap();
    let trigger = crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap();

    for pulse in [7.0, 8.0] {
        runtime
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                trigger.clone(),
                crate::RuntimeHostInputValue::F64(pulse),
            ))
            .unwrap();
        let outcome = runtime.drain_resident_host_inputs(1).unwrap();
        assert!(matches!(
            outcome.turn,
            Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
        ));
    }

    let state = runtime
        .root_symbol_value("state")
        .unwrap()
        .into_value()
        .as_f64()
        .unwrap();
    assert_eq!(*state.borrow(), 2.0);
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

#[derive(Clone, Debug)]
struct ScalarNbodyReference {
    positions: Vec<f64>,
    velocities: Vec<f64>,
    masses: Vec<f64>,
}

#[derive(Clone, Copy, Debug)]
struct PublicNbodyStateSlots {
    positions: mech_core::CellSlotId,
    velocities: mech_core::CellSlotId,
}

impl ScalarNbodyReference {
    fn from_runtime(runtime: &crate::MechRuntime, slots: PublicNbodyStateSlots) -> Self {
        Self {
            positions: public_nbody_state_slot(runtime, slots.positions),
            velocities: public_nbody_state_slot(runtime, slots.velocities),
            masses: public_nbody_masses(runtime),
        }
    }

    fn benchmark_game() -> Self {
        let year = 365.24;
        let solar_mass = 4.0 * std::f64::consts::PI.powi(2);
        let rows = [
            ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0),
            (
                [
                    4.84143144246472090,
                    -1.16032004402742839,
                    -0.103622044471123109,
                ],
                [
                    0.00166007664274403694 * year,
                    0.00769901118419740425 * year,
                    -0.0000690460016972063023 * year,
                ],
                0.000954791938424326609,
            ),
            (
                [
                    8.34336671824457987,
                    4.12479856412430479,
                    -0.403523417114321381,
                ],
                [
                    -0.00276742510726862411 * year,
                    0.00499852801234917238 * year,
                    0.0000230417297573763929 * year,
                ],
                0.000285885980666130812,
            ),
            (
                [
                    12.8943695621391310,
                    -15.1111514016986312,
                    -0.223307578892655734,
                ],
                [
                    0.00296460137564761618 * year,
                    0.00237847173959480950 * year,
                    -0.0000296589568540237556 * year,
                ],
                0.0000436624404335156298,
            ),
            (
                [
                    15.3796971148509165,
                    -25.9193146099879641,
                    0.179258772950371181,
                ],
                [
                    0.00268067772490389322 * year,
                    0.00162824170038242295 * year,
                    -0.000095159225451971587 * year,
                ],
                0.0000515138902046611451,
            ),
        ];
        let mut reference = Self {
            positions: (0..3)
                .flat_map(|axis| rows.iter().map(move |row| row.0[axis]))
                .collect(),
            velocities: (0..3)
                .flat_map(|axis| rows.iter().map(move |row| row.1[axis]))
                .collect(),
            masses: rows.iter().map(|row| row.2 * solar_mass).collect(),
        };
        reference.offset_solar_momentum();
        reference
    }

    fn body_count(&self) -> usize {
        self.masses.len()
    }

    fn advance(&mut self, dt: f64) {
        let body_count = self.body_count();
        let mut pairs = Vec::with_capacity(body_count * (body_count - 1) / 2);
        for left in 0..body_count {
            for right in left + 1..body_count {
                let delta: [f64; 3] = core::array::from_fn(|axis| {
                    self.positions[left + axis * body_count]
                        - self.positions[right + axis * body_count]
                });
                let distance_squared = delta.iter().map(|value| value * value).sum::<f64>();
                pairs.push((left, right, delta, dt * distance_squared.powf(-1.5)));
            }
        }
        for (left, right, delta, magnitude) in &pairs {
            for axis in 0..3 {
                self.velocities[*left + axis * body_count] -=
                    delta[axis] * self.masses[*right] * magnitude;
            }
        }
        for (left, right, delta, magnitude) in pairs {
            for axis in 0..3 {
                self.velocities[right + axis * body_count] +=
                    delta[axis] * self.masses[left] * magnitude;
            }
        }
        for index in 0..self.positions.len() {
            self.positions[index] += self.velocities[index] * dt;
        }
    }

    fn energy(&self) -> f64 {
        let body_count = self.body_count();
        let kinetic = (0..body_count)
            .map(|body| {
                0.5 * self.masses[body]
                    * (0..3)
                        .map(|axis| self.velocities[body + axis * body_count].powi(2))
                        .sum::<f64>()
            })
            .sum::<f64>();
        let potential = (0..body_count)
            .flat_map(|left| (left + 1..body_count).map(move |right| (left, right)))
            .map(|(left, right)| {
                let distance = (0..3)
                    .map(|axis| {
                        (self.positions[left + axis * body_count]
                            - self.positions[right + axis * body_count])
                            .powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                self.masses[left] * self.masses[right] / distance
            })
            .sum::<f64>();
        kinetic - potential
    }

    fn momentum(&self) -> [f64; 3] {
        let body_count = self.body_count();
        core::array::from_fn(|axis| {
            (0..body_count)
                .map(|body| self.velocities[body + axis * body_count] * self.masses[body])
                .sum()
        })
    }

    fn offset_solar_momentum(&mut self) {
        let body_count = self.body_count();
        for axis in 0..3 {
            let momentum = (1..body_count)
                .map(|body| self.velocities[body + axis * body_count] * self.masses[body])
                .sum::<f64>();
            self.velocities[axis * body_count] = -momentum / self.masses[0];
        }
    }

    fn relative_body_state(&self, body: usize) -> ([f64; 3], [f64; 3]) {
        let body_count = self.body_count();
        (
            core::array::from_fn(|axis| {
                self.positions[body + axis * body_count] - self.positions[axis * body_count]
            }),
            core::array::from_fn(|axis| {
                self.velocities[body + axis * body_count] - self.velocities[axis * body_count]
            }),
        )
    }

    fn mercury_state(&self) -> (f64, f64, f64) {
        let (position, velocity) = self.relative_body_state(1);
        let radius = position
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let speed = velocity
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let cross = [
            position[1] * velocity[2] - position[2] * velocity[1],
            position[2] * velocity[0] - position[0] * velocity[2],
            position[0] * velocity[1] - position[1] * velocity[0],
        ];
        let areal_rate = 0.5 * cross.iter().map(|value| value * value).sum::<f64>().sqrt();
        (radius, speed, areal_rate)
    }

    fn body_orbital_elements(&self, body: usize) -> (f64, f64) {
        let (position, velocity) = self.relative_body_state(body);
        let radius = position
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        let speed_squared = velocity.iter().map(|value| value * value).sum::<f64>();
        let cross = [
            position[1] * velocity[2] - position[2] * velocity[1],
            position[2] * velocity[0] - position[0] * velocity[2],
            position[0] * velocity[1] - position[1] * velocity[0],
        ];
        let gravitational_parameter = self.masses[0] + self.masses[body];
        let semimajor_axis = 1.0 / (2.0 / radius - speed_squared / gravitational_parameter);
        let eccentricity = (1.0
            - cross.iter().map(|value| value * value).sum::<f64>()
                / (gravitational_parameter * semimajor_axis))
            .sqrt();
        (semimajor_axis, eccentricity)
    }
}

fn public_nbody_state_slots(runtime: &crate::MechRuntime) -> PublicNbodyStateSlots {
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("public N-body must remain on the resident-external route")
    };
    let instance = execution.coordinator.instance();
    let slots = instance
        .plan
        .slots
        .iter()
        .filter(|slot| {
            slot.storage == ResidentStorageClass::State
                && slot.region.kind == mech_core::ResidentValueKind::F64
                && slot.region.shape.rows == 10
                && slot.region.shape.columns == 3
        })
        .map(|slot| slot.artifact_id)
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 2, "public N-body has exactly two state cells");
    let first = public_nbody_state_slot(runtime, slots[0]);
    let second = public_nbody_state_slot(runtime, slots[1]);
    if (first[1] - (-0.1407280797108344)).abs() < 1.0e-12 {
        PublicNbodyStateSlots {
            positions: slots[0],
            velocities: slots[1],
        }
    } else {
        assert!((second[1] - (-0.1407280797108344)).abs() < 1.0e-12);
        PublicNbodyStateSlots {
            positions: slots[1],
            velocities: slots[0],
        }
    }
}

fn public_nbody_state_slot(runtime: &crate::MechRuntime, slot: mech_core::CellSlotId) -> Vec<f64> {
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("public N-body must remain on the resident-external route")
    };
    let ResidentValueBorrow::F64 { values, .. } = execution
        .coordinator
        .instance()
        .state_borrow(slot)
        .expect("public N-body state slot")
    else {
        panic!("public N-body state must be f64")
    };
    values.to_vec()
}

fn public_nbody_masses(runtime: &crate::MechRuntime) -> Vec<f64> {
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("public N-body must remain on the resident-external route")
    };
    let instance = execution.coordinator.instance();
    for slot in instance.plan.slots.iter().filter(|slot| {
        slot.storage == ResidentStorageClass::Constant
            && slot.region.kind == mech_core::ResidentValueKind::F64
            && slot.region.shape.rows == 10
            && slot.region.shape.columns == 1
    }) {
        let values = &instance.activation.f64_storage()
            [slot.region.offset..slot.region.offset + slot.region.len];
        if values.first().is_some_and(|value| *value > 30.0)
            && values.iter().all(|value| value.is_finite() && *value > 0.0)
        {
            return values.to_vec();
        }
    }
    panic!("public N-body mass vector must be resident activation storage")
}

fn assert_public_nbody_matches_raw(
    runtime: &crate::MechRuntime,
    slots: PublicNbodyStateSlots,
    raw: &ScalarNbodyReference,
) {
    for (name, actual, expected) in [
        (
            "positions",
            public_nbody_state_slot(runtime, slots.positions),
            &raw.positions,
        ),
        (
            "velocities",
            public_nbody_state_slot(runtime, slots.velocities),
            &raw.velocities,
        ),
    ] {
        assert_eq!(actual.len(), expected.len());
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 1.0e-10,
                "public N-body {name}[{index}] differs from the raw Rust recurrence: {actual:?} != {expected:?}",
            );
        }
    }
}

fn public_nbody_python_state_hash(raw: &ScalarNbodyReference) -> String {
    let mut hash = Sha256::new();
    for value in raw.positions.iter().chain(&raw.velocities) {
        hash.update(((value / 1.0e-8).round() as i64).to_le_bytes());
    }
    finish_hash(hash)
}

#[test]
fn public_nbody_viewer_integrates_mutual_gravity_residently() {
    let (mut runtime, scene) = product_nbody_runtime();
    let loaded = runtime
        .load_source_program(
            PUBLIC_NBODY_VIEWER_SOURCE,
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal);
    let state_slots = public_nbody_state_slots(&runtime);
    let mut raw = ScalarNbodyReference::from_runtime(&runtime, state_slots);
    let initial_raw = raw.clone();
    assert!(
        raw.momentum()
            .iter()
            .all(|component| component.abs() < 1.0e-12),
        "the public source must start in the zero-momentum center-of-mass frame",
    );
    // Exact heliocentric ecliptic vectors returned by Horizons for every body
    // at the one shared epoch used by the public source. Comparing the live
    // program state component-by-component prevents a plausible-looking orbit
    // from concealing a wrong body, epoch, axis, or velocity.
    let jpl_states = [
        (
            "Mercury",
            [
                -0.1407280797108344,
                -0.4439009580270337,
                -0.02334555919971206,
            ],
            [
                0.02116887135892671,
                -0.007097975420557316,
                -0.002522831030718983,
            ],
        ),
        (
            "Venus",
            [
                -0.7186302169204941,
                -0.02250380069428597,
                0.04117184128636830,
            ],
            [
                0.0005135327471455046,
                -0.02030614162247666,
                -0.0003071745200681565,
            ],
        ),
        (
            "Earth",
            [
                -0.1685246483858782,
                0.9687833049070306,
                -0.000004120490278477264,
            ],
            [
                -0.01723394583247693,
                -0.003007660249861311,
                0.00000003562572888400974,
            ],
        ),
        (
            "Mars",
            [
                1.390361066039004,
                -0.02100972225898463,
                -0.03461801440927048,
            ],
            [
                0.0007479271359517672,
                0.01518629867665782,
                0.0002997532106431930,
            ],
        ),
        (
            "Jupiter",
            [4.003460488693537, 2.935353187887882, -0.1018230443988181],
            [
                -0.004563750795379206,
                0.006447274222742638,
                0.00007547009668026901,
            ],
        ),
        (
            "Saturn",
            [6.408556035505925, 6.568042752621957, -0.3691272880681217],
            [
                -0.004290540498899068,
                0.003891990893152464,
                0.0001026097543467762,
            ],
        ),
        (
            "Uranus",
            [14.43051609648136, -13.73565967460644, -0.2381293855338772],
            [
                0.002678465627884438,
                0.002672426903071822,
                -0.00002475113494134723,
            ],
        ),
        (
            "Neptune",
            [16.81075807703606, -24.99265146883861, 0.1272705680239183],
            [
                0.002579217015470078,
                0.001776355038562099,
                -0.00009620006049034144,
            ],
        ),
        (
            "Pluto",
            [-9.876866563865008, -27.95802013288459, 5.850814086362886],
            [
                0.003039003425722539,
                -0.001529889055125659,
                -0.0007172321784828390,
            ],
        ),
    ];
    for (index, (name, expected_position, expected_velocity_per_day)) in
        jpl_states.iter().enumerate()
    {
        let (position, velocity) = raw.relative_body_state(index + 1);
        for axis in 0..3 {
            assert!(
                (position[axis] - expected_position[axis]).abs() < 1.0e-12,
                "{name} JPL position component {axis} differs: {:?} != {:?}",
                position[axis],
                expected_position[axis],
            );
            let expected_velocity = expected_velocity_per_day[axis] * 365.24;
            assert!(
                (velocity[axis] - expected_velocity).abs() < 1.0e-12,
                "{name} JPL velocity component {axis} differs: {:?} != {:?}",
                velocity[axis],
                expected_velocity,
            );
        }
    }

    let jpl_orbits = [
        ("Mercury", 0.3870982252718477, 0.2056302515978038),
        ("Venus", 0.7233268496756070, 0.006755697268576816),
        ("Earth", 1.000371833994387, 0.01704239718110438),
        ("Mars", 1.523678184286835, 0.09331460654156362),
        ("Jupiter", 5.205108585205607, 0.04892305962953223),
        ("Saturn", 9.581451990528134, 0.05559928887285597),
        ("Uranus", 19.22993812529615, 0.04439367187710320),
        ("Neptune", 30.09700542229719, 0.01114790154011905),
        ("Pluto", 39.50058973957585, 0.2478572758892915),
    ];
    let orbital_elements = jpl_orbits
        .iter()
        .enumerate()
        .map(|(index, (name, expected_axis, expected_eccentricity))| {
            let (axis, eccentricity) = raw.body_orbital_elements(index + 1);
            let axis_tolerance = f64::max(0.002, expected_axis * 0.0004);
            assert!(
                (axis - expected_axis).abs() < axis_tolerance,
                "{name} semimajor axis differs from its independent JPL J2000 element: {axis:?} != {expected_axis:?}",
            );
            assert!(
                (eccentricity - expected_eccentricity).abs() < 0.001,
                "{name} eccentricity differs from its independent JPL J2000 element: {eccentricity:?} != {expected_eccentricity:?}",
            );
            (axis, eccentricity)
        })
        .collect::<Vec<_>>();
    let neptune_axis = orbital_elements[7].0;
    let (pluto_axis, pluto_eccentricity) = orbital_elements[8];
    assert!(
        pluto_axis * (1.0 - pluto_eccentricity) < neptune_axis
            && neptune_axis < pluto_axis * (1.0 + pluto_eccentricity),
        "Pluto's independently derived eccentric orbit must span Neptune's semimajor axis",
    );
    let mut mercury = vec![raw.mercury_state()];

    let published_energy = |runtime: &crate::MechRuntime| match runtime
        .program_output_value()
        .unwrap()
        .expect("N-body publishes total energy")
        .into_value()
    {
        LegacyValue::MatrixF64(value) if (value.rows(), value.cols()) == (1, 1) => {
            value.as_vec()[0]
        }
        value => panic!("N-body energy must be a 1-by-1 f64 matrix, got {value:?}"),
    };
    let expected_initial_energy = raw.energy();
    raw.advance(0.002);
    advance_product_nbody(&mut runtime);
    assert_public_nbody_matches_raw(&runtime, state_slots, &raw);
    let initial_frame = scene.lock().unwrap().latest.clone();
    assert_eq!(initial_frame.len(), 20);
    let initial_display_radii = (0..10)
        .map(|body| (initial_frame[body] - 430.0).hypot(initial_frame[10 + body] - 380.0))
        .collect::<Vec<_>>();
    for (body, radius, expected) in [
        ("Sun", initial_display_radii[0], 0.0..1.0),
        ("Mercury", initial_display_radii[1], 20.0..35.0),
        ("Venus", initial_display_radii[2], 30.0..45.0),
        ("Earth", initial_display_radii[3], 35.0..52.0),
        ("Mars", initial_display_radii[4], 45.0..65.0),
        ("Jupiter", initial_display_radii[5], 85.0..115.0),
        ("Saturn", initial_display_radii[6], 120.0..155.0),
        ("Uranus", initial_display_radii[7], 175.0..215.0),
        ("Neptune", initial_display_radii[8], 220.0..260.0),
        ("Pluto", initial_display_radii[9], 215.0..265.0),
    ] {
        assert!(
            expected.contains(&radius),
            "{body} rendered at display radius {radius}, expected {expected:?}",
        );
    }
    let initial_mercury_radius =
        (initial_frame[1] - initial_frame[0]).hypot(initial_frame[11] - initial_frame[10]);
    let initial_energy = published_energy(&runtime);
    assert!(
        (initial_energy - expected_initial_energy).abs() < 1.0e-10,
        "the public Mech energy must equal the independent raw-Rust energy for the published turn: {initial_energy:?} != {expected_initial_energy:?}",
    );

    let mut expected_final_energy = expected_initial_energy;
    for _ in 1..4_096 {
        expected_final_energy = raw.energy();
        raw.advance(0.002);
        advance_product_nbody(&mut runtime);
        assert_public_nbody_matches_raw(&runtime, state_slots, &raw);
        mercury.push(raw.mercury_state());
        let frame = scene.lock().unwrap().latest.clone();
        assert_eq!(frame.len(), 20);
        assert!(frame.iter().all(|value| value.is_finite()));
        assert!(
            (frame[0] - 430.0).hypot(frame[10] - 380.0) < 1.0e-9,
            "the heliocentric camera must keep the rendered Sun at the orbit-guide center",
        );
    }
    let final_frame = scene.lock().unwrap().latest.clone();
    let final_mercury_radius =
        (final_frame[1] - final_frame[0]).hypot(final_frame[11] - final_frame[10]);
    assert_ne!(&initial_frame[1..], &final_frame[1..]);
    assert_eq!((initial_frame[0], initial_frame[10]), (430.0, 380.0));
    assert_eq!((final_frame[0], final_frame[10]), (430.0, 380.0));
    assert_ne!(
        (&initial_raw.positions[0], &initial_raw.positions[10]),
        (&raw.positions[0], &raw.positions[10]),
        "the physical Sun must respond to the other bodies even though the camera follows it",
    );
    assert!(
        (final_mercury_radius - initial_mercury_radius).abs() > 1.0e-6,
        "a mutual-gravity orbit must not preserve the old prescribed radius",
    );

    let final_energy = published_energy(&runtime);
    assert!(
        (final_energy - expected_final_energy).abs() < 1.0e-10,
        "the public Mech energy must equal the independent raw-Rust energy for the published turn: {final_energy:?} != {expected_final_energy:?}",
    );
    assert!(initial_energy.is_finite() && final_energy.is_finite());
    let relative_energy_drift = ((final_energy - initial_energy) / initial_energy).abs();
    assert!(
        relative_energy_drift < 0.01,
        "symplectic integration energy drifted by {relative_energy_drift:e}",
    );

    assert_eq!(
        public_nbody_python_state_hash(&raw),
        "f9194e44d424d554c68dfcecbd8f0af95e764b65aff5387fd99d1a781c78d3c8",
        "the Mech/raw-Rust trajectory must match the independent Python reference",
    );
    let perihelion = mercury
        .iter()
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .unwrap();
    let aphelion = mercury
        .iter()
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .unwrap();
    assert!(
        (0.295..=0.320).contains(&perihelion.0),
        "Mercury's perihelion must remain on its independently specified nominal orbit: {perihelion:?}",
    );
    assert!(
        (0.450..=0.480).contains(&aphelion.0),
        "Mercury's aphelion must remain on its independently specified nominal orbit: {aphelion:?}",
    );
    assert!(
        perihelion.1 > aphelion.1 * 1.2,
        "Mercury must move faster at perihelion than at aphelion: {perihelion:?} versus {aphelion:?}",
    );
    let (minimum_areal_rate, maximum_areal_rate) = mercury.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(minimum, maximum), sample| (minimum.min(sample.2), maximum.max(sample.2)),
    );
    let mean_areal_rate = mercury.iter().map(|sample| sample.2).sum::<f64>() / mercury.len() as f64;
    assert!(
        (maximum_areal_rate - minimum_areal_rate) / mean_areal_rate < 0.01,
        "Mercury's equal-area rate varied beyond the mutual-gravity perturbation bound",
    );

    let mut benchmark = ScalarNbodyReference::benchmark_game();
    assert!((benchmark.energy() - (-0.169075164)).abs() < 5.0e-10);
    for _ in 0..1_000 {
        benchmark.advance(0.01);
    }
    assert!((benchmark.energy() - (-0.169087605)).abs() < 5.0e-10);

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
    assert!(
        error
            .kind_as::<super::ResidentHostTurnFailed>()
            .expect("resident host turn failure kind")
            .is_recoverable()
    );
    assert!(
        error
            .kind_message()
            .contains("ResourceBudgetExceeded: MechError")
    );
    let source = error.source.as_ref().expect("rejection cause is preserved");
    assert_eq!(source.kind_name(), "ResourceBudgetExceeded");
    assert!(source.kind_message().contains("turn_duration_ms"));
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

    runtime.config.limits.max_turn_duration_ms = None;
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("timer://clock/tick", "tick").unwrap(),
            crate::RuntimeHostInputValue::F64(2.0),
        ))
        .unwrap();
    let recovered = runtime.drain_host_inputs(1).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(scene.lock().unwrap().deliveries, 1);
    let info = runtime.program_execution_info();
    assert_eq!(info.resident_rejected_turns, 1);
    assert_eq!(info.resident_accepted_turns, 1);
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
