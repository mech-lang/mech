use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

use mech_core::{
    AccessMode, DeliveryMode, DimensionExpr, EffectContract, EffectDeliveryPolicy,
    ExternalInteraction, IdempotencyRequirement, InputPortLayout, InputPortPolicy, MResult,
    OperationContractDeclaration, ParsedProgram, SchemaBody, Value, ValueCell, ValueData,
    ValueDataDraft, hash_str, snapshot::SequenceView,
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
    RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteCommand, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
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

fn canonical_f64(value: &Value) -> f64 {
    let ValueData::F64(value) = value.data() else {
        panic!("expected canonical f64, got {value:?}")
    };
    value.to_f64()
}

fn canonical_matrix_shape(value: &Value) -> (usize, usize) {
    let schemas = value.schemas().expect("canonical matrix retains schemas");
    let SchemaBody::Matrix { dimensions, .. } = schemas
        .entry(value.schema())
        .expect("canonical matrix schema exists")
        .schema()
        .body()
    else {
        panic!("expected canonical matrix schema")
    };
    let [rows, columns] = dimensions.as_ref() else {
        panic!("runtime matrix must have two dimensions")
    };
    (
        value.shape().resolve_dimension(rows).unwrap() as usize,
        value.shape().resolve_dimension(columns).unwrap() as usize,
    )
}

fn canonical_f64_matrix(value: &Value) -> Vec<f64> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("expected canonical f64 matrix, got {value:?}")
    };
    match matrix.elements() {
        SequenceView::F64(values) => values.iter().map(|value| value.to_f64()).collect(),
        SequenceView::Values(values) => values
            .iter()
            .map(|value| match value {
                ValueData::F64(value) => value.to_f64(),
                other => panic!("expected f64 matrix element, got {other:?}"),
            })
            .collect(),
        other => panic!("expected f64 matrix storage, got {other:?}"),
    }
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
    planned: Value,
}

#[derive(Debug)]
struct DriverlessObservationProvider {
    reads: Arc<AtomicUsize>,
}

impl RuntimeResourceProvider for DriverlessObservationProvider {
    fn scheme(&self) -> &str {
        "snapshot"
    }

    fn base_uris(&self) -> Vec<String> {
        vec!["snapshot://clock/tick".to_owned()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::resource_observation_contract())
    }

    fn observation_requires_input_driver(&self, _request: &RuntimeResourceReadRequest) -> bool {
        false
    }

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        ValueCell::from_exact(true)?.snapshot()
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        ValueCell::from_exact(true)?.snapshot()
    }
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

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        Ok(self.planned.clone())
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
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

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.value(&request)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.value(&request)
    }
}

impl IndependentObservationProvider {
    fn value(&self, request: &RuntimeResourceReadRequest) -> MResult<Value> {
        let bits = match request.base_uri.as_str() {
            "test://clock/fast" => self.fast_bits.load(Ordering::SeqCst),
            "test://clock/slow" => self.slow_bits.load(Ordering::SeqCst),
            other => panic!("unexpected independent observation URI {other}"),
        };
        ValueCell::from_exact(f64::from_bits(bits))?.snapshot()
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

    fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.plans.fetch_add(1, Ordering::SeqCst);
        ValueCell::from_exact(f64::from_bits(self.value_bits.load(Ordering::SeqCst)))?.snapshot()
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        ValueCell::from_exact(f64::from_bits(self.value_bits.load(Ordering::SeqCst)))?.snapshot()
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

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        panic!("the deterministic scene is write-only")
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        assert_eq!(request.base_uri, "scene://orbit/frame");
        assert!(matches!(request.path.as_str(), "points" | "replace"));
        assert_eq!(request.intent, RuntimeResourceWriteIntent::Send);
        Ok(())
    }

    fn plan_write(&self, request: RuntimeResourceWriteCommand) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
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

fn product_scene_points(path: &str, value: &Value) -> Vec<f64> {
    let schemas = value.schemas().expect("scene value retains schemas");
    let schema = schemas
        .entry(value.schema())
        .expect("scene value schema exists")
        .schema()
        .body();
    if path == "replace" {
        let (SchemaBody::Record(fields), ValueData::Record(scene)) = (schema, value.data()) else {
            panic!("scene replacement must be a record, got {value:?}")
        };
        let index = fields
            .iter()
            .position(|field| field.name == "point-sets")
            .expect("public N-body scene point-sets");
        return product_scene_point_set_positions(&fields[index].schema, &scene.fields()[index]);
    }
    product_scene_matrix_values(schema, value.data())
}

fn product_scene_point_set_positions(schema: &SchemaBody, value: &ValueData) -> Vec<f64> {
    if let (SchemaBody::Table { columns, rows }, ValueData::Table(table)) = (schema, value) {
        assert!(matches!(
            rows,
            mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(1))
        ));
        let index = columns
            .iter()
            .position(|field| field.name == "positions")
            .expect("public N-body point-set positions");
        let SequenceView::Values(values) = table
            .column(index)
            .expect("scene point-set positions column")
        else {
            panic!("scene point-set positions must retain canonical values")
        };
        let [value] = values else {
            panic!("scene point-set table must contain exactly one row")
        };
        return product_scene_matrix_values(&columns[index].schema, value);
    }
    let (schema, value) = match (schema, value) {
        (SchemaBody::Record(fields), ValueData::Record(record)) => (fields.as_ref(), record),
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) if values.len() == 1 => {
            let SchemaBody::Record(fields) = &elements[0] else {
                panic!("scene point-set tuple must contain a record")
            };
            let ValueData::Record(record) = &values[0] else {
                panic!("scene point-set tuple must contain record data")
            };
            (fields.as_ref(), record)
        }
        other => panic!("scene point-sets must contain one record, got {other:?}"),
    };
    let index = schema
        .iter()
        .position(|field| field.name == "positions")
        .expect("public N-body point-set positions");
    product_scene_matrix_values(&schema[index].schema, &value.fields()[index])
}

fn product_scene_matrix_values(schema: &SchemaBody, value: &ValueData) -> Vec<f64> {
    let row_major: Vec<f64> = match value {
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(values) => values.iter().map(|value| value.to_f64()).collect(),
            SequenceView::Values(values) => values
                .iter()
                .map(|value| match value {
                    ValueData::F64(value) => value.to_f64(),
                    other => panic!("scene points must contain f64 values, got {other:?}"),
                })
                .collect(),
            other => panic!("scene points must be an f64 matrix, got {other:?}"),
        },
        other => panic!("scene points must be an f64 matrix, got {other:?}"),
    };
    let SchemaBody::Matrix { dimensions, .. } = schema else {
        panic!("scene points must retain a matrix schema, got {schema:?}")
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        panic!("scene point dimensions must be closed, got {dimensions:?}")
    };
    let (rows, columns) = (*rows as usize, *columns as usize);
    assert_eq!(row_major.len(), rows * columns);
    let mut column_major = Vec::with_capacity(row_major.len());
    for column in 0..columns {
        for row in 0..rows {
            column_major.push(row_major[row * columns + column]);
        }
    }
    column_major
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

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        assert_eq!(request.path, "tick");
        ValueCell::from_exact(0.0_f64)?.snapshot()
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        assert_eq!(request.path, "tick");
        ValueCell::from_exact(0.0_f64)?.snapshot()
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

fn independent_external_runtime_with_source(
    source: &str,
) -> (crate::MechRuntime, Arc<AtomicUsize>) {
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
        .load_source_program(source, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    (runtime, reads)
}

fn independent_canonical_external_runtime_with_source(
    source: &str,
) -> (crate::MechRuntime, Arc<AtomicUsize>) {
    let reads = Arc::new(AtomicUsize::new(0));
    let fast_bits = Arc::new(AtomicU64::new(2.0_f64.to_bits()));
    let slow_bits = Arc::new(AtomicU64::new(3.0_f64.to_bits()));
    let provider = || IndependentObservationProvider {
        reads: reads.clone(),
        fast_bits: fast_bits.clone(),
        slow_bits: slow_bits.clone(),
    };
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .resource_provider(Box::new(provider()))
        .build_compiler()
        .unwrap();
    let artifact = compiler
        .compile_canonical_source(source)
        .unwrap()
        .into_parts()
        .0;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(catalog)
        .input_driver(ResidentTestInputDriver)
        .build()
        .unwrap();
    runtime
        .register_resource_provider(Box::new(provider()))
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
        .load_compiled_program(artifact, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    (runtime, reads)
}

fn independent_external_runtime() -> (crate::MechRuntime, Arc<AtomicUsize>) {
    independent_external_runtime_with_source(
        r#"
@fast := test://clock/fast{:read(delta-seconds)}
@slow := test://clock/slow{:read(delta-seconds)}
fast := @fast/delta-seconds
slow := @slow/delta-seconds
~state := 0.0
state += fast + slow
output := state
"#,
    )
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
fn production_load_drains_fsm_continuations_before_returning_initial_value() {
    let source = "#Deferred() => <u64>\n  | :Start\n  | :Middle(value<u64>)\n  | :Later(value<u64>)\n  | :Done(value<u64>).\n#Deferred() -> :Start\n  :Start ~> :Middle(40u64)\n  :Middle(value) ~> :Later(value + 1u64)\n  :Later(value) -> :Done(value + 1u64)\n  :Done(value) => value.\n#Deferred()\n";
    let parsed = mech_syntax::document::parse_canonical_document(
        mech_syntax::document::TextSnapshot::new(
            mech_syntax::document::DocumentId(0x874),
            mech_syntax::document::Revision(0),
            source,
        )
        .unwrap(),
        mech_syntax::document::ParseConfig::default(),
    );
    let document = <mech_syntax::document::DocumentSyntax as mech_syntax::document::AstNode>::cast(
        parsed.syntax(),
    )
    .unwrap();
    let artifact = mech_engine::CanonicalSourceFrontend
        .compile_document(&document)
        .unwrap()
        .compile_artifact()
        .unwrap();
    let bytecode = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let mut runtime = runtime();
    let loaded = runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);
    assert_eq!(loaded.initial_value.format_canonical_inline(), "42");
    let ActiveProgramExecution::ResidentPure(execution) = &runtime.active_program else {
        panic!("FSM must use the pure resident route")
    };
    assert_eq!(loaded.info.resident_accepted_turns, 3);
    assert!(!execution.instance.has_ready_continuation());
}

#[test]
fn pure_continuation_drains_keep_input_free_activations_dormant() {
    let source = "#Deferred() => <u64>\n  | :Start\n  | :Done.\n#Deferred() -> :Start\n  :Start ~> :Done\n  :Done => 41u64.\ntrigger := true\n~count := 0u64\n~> trigger { count = count + 1u64 }\n#Deferred()\n";
    let parsed = mech_syntax::document::parse_canonical_document(
        mech_syntax::document::TextSnapshot::new(
            mech_syntax::document::DocumentId(0x876),
            mech_syntax::document::Revision(0),
            source,
        )
        .unwrap(),
        mech_syntax::document::ParseConfig::default(),
    );
    let document = <mech_syntax::document::DocumentSyntax as mech_syntax::document::AstNode>::cast(
        parsed.syntax(),
    )
    .unwrap();
    let artifact = mech_engine::CanonicalSourceFrontend
        .compile_document(&document)
        .unwrap()
        .compile_artifact()
        .unwrap();
    let bytecode = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let mut runtime = runtime();
    let loaded = runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();

    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);
    assert_eq!(loaded.info.resident_accepted_turns, 2);
    let ActiveProgramExecution::ResidentPure(execution) = &runtime.active_program else {
        panic!("continuation fixture must remain resident pure")
    };
    let count_slot = execution
        .artifact
        .slots()
        .iter()
        .find(|slot| slot.role == mech_engine::SlotRole::State)
        .unwrap()
        .slot;
    let mech_engine::__resident::ResidentValueBorrow::Snapshot { values, .. } =
        execution.instance.state_borrow(count_slot).unwrap()
    else {
        panic!("count must use snapshot state storage")
    };
    assert_eq!(
        crate::RuntimeValueSnapshot::from_value(values[0].as_ref().unwrap().clone())
            .unwrap()
            .format_canonical_inline(),
        "0"
    );
}

#[test]
fn failed_initial_continuation_drain_releases_the_program_slot() {
    let deferred = "#Deferred() => <u64>\n  | :Start\n  | :Middle(value<u64>)\n  | :Later(value<u64>)\n  | :Done(value<u64>).\n#Deferred() -> :Start\n  :Start ~> :Middle(40u64)\n  :Middle(value) ~> :Later(value + 1u64)\n  :Later(value) -> :Done(value + 1u64)\n  :Done(value) => value.\n#Deferred()\n";
    let compile = |source: &str| {
        let parsed = mech_syntax::document::parse_canonical_document(
            mech_syntax::document::TextSnapshot::new(
                mech_syntax::document::DocumentId(0x875),
                mech_syntax::document::Revision(0),
                source,
            )
            .unwrap(),
            mech_syntax::document::ParseConfig::default(),
        );
        let document =
            <mech_syntax::document::DocumentSyntax as mech_syntax::document::AstNode>::cast(
                parsed.syntax(),
            )
            .unwrap();
        mech_engine::CanonicalSourceFrontend
            .compile_document(&document)
            .unwrap()
            .compile_artifact()
            .unwrap()
    };
    let mut config = crate::RuntimeConfig::default();
    config.limits.max_steps_per_turn = Some(1);
    let mut runtime = RuntimeBuilder::new()
        .config(config)
        .function_catalog(mech_stdlib::source_catalog())
        .input_driver(ResidentTestInputDriver)
        .build()
        .unwrap();
    let deferred = encode_program_artifact_bytecode_v1(&compile(deferred)).unwrap();
    let error = runtime
        .load_bytecode_program(&deferred, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("resident continuation wakeup limit exhausted"),
        "{error:?}"
    );
    assert!(matches!(
        runtime.active_program,
        ActiveProgramExecution::None
    ));

    let replacement = encode_program_artifact_bytecode_v1(&compile("40u64 + 2u64\n")).unwrap();
    let loaded = runtime
        .load_bytecode_program(&replacement, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(loaded.initial_value.format_canonical_inline(), "42");
}

#[test]
fn pending_continuations_are_drained_before_the_next_host_packet() {
    let source = "@clock := test://clock/tick{:read(delta-seconds)}\ntick := @clock/delta-seconds\n#Deferred(value<f64>) => <f64>\n  | :Start(value<f64>)\n  | :One(value<f64>)\n  | :Two(value<f64>)\n  | :Three(value<f64>)\n  | :Done(value<f64>).\n#Deferred(value) -> :Start(value)\n  :Start(value) ~> :One(value)\n  :One(value) ~> :Two(value)\n  :Two(value) ~> :Three(value)\n  :Three(value) -> :Done(value)\n  :Done(value) => value.\n#Deferred(tick)\n";
    let (mut runtime, _, _, _) = configured_external_runtime();
    runtime
        .load_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    runtime.config.limits.max_steps_per_turn = Some(1);
    let trigger = crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap();

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            trigger.clone(),
            crate::RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap();
    let error = runtime.drain_resident_host_inputs(1).unwrap_err();
    assert!(
        format!("{error:?}").contains("resident continuation wakeup limit exhausted"),
        "{error:?}"
    );
    assert_eq!(runtime.pending_host_input_count().unwrap(), 0);

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            trigger,
            crate::RuntimeHostInputValue::F64(2.0),
        ))
        .unwrap();
    let error = runtime.drain_resident_host_inputs(1).unwrap_err();
    assert!(
        format!("{error:?}").contains("resident continuation wakeup limit exhausted"),
        "{error:?}"
    );
    assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
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
    assert_eq!(canonical_f64(&inline), 42.0);
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
    assert_eq!(loaded.initial_value.to_string(), "120");
    assert_eq!(
        runtime
            .output_value(output_id)
            .unwrap()
            .unwrap()
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
fn static_initialization_returns_detached_row_major_matrix_values() {
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
            values: vec![1.0, 2.0, 3.0, 4.0],
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
            values: vec![1.0, 2.0, 3.0, 4.0],
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
            values: vec![1.0, 2.0, 3.0, 4.0],
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
fn canonical_mixed_document_owns_partitioning_and_typed_initializers() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_source(MIXED_COMPUTE_SOURCE).unwrap();

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
fn canonical_rooted_mixed_compilation_shares_transitive_imports_and_initializers() {
    let root = r#"+> ./dep.mec
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
@compute/input/x <- dep/value * 2f32
@compute/turn <- 1

calculation @compute
-------------------
x := dep/value
result := x + dep/value
result
"#;
    let dependency = "+> ./leaf.mec\nvalue := leaf/value + 1f32\n<+ value\n";
    let leaf = "value := 2f32\n<+ value\n";
    let mut resolver = InMemorySourceResolver::new();
    resolver.insert_canonical_string("main.mec", root).unwrap();
    resolver
        .insert_canonical_string("dep.mec", dependency)
        .unwrap();
    resolver.insert_canonical_string("leaf.mec", leaf).unwrap();
    resolver
        .insert_canonical_string("broken.mec", "+> ./broken.mec\nvalue := 1f32\n<+ value\n")
        .unwrap();
    let resolved = crate::SourceResolver::resolve(&resolver, &SourceRequest::new("main.mec"))
        .unwrap()
        .unwrap();
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let options = ModuleBuildOptions::new("test", "v0.4", "native", &["compute"], &[]);
    let products = [
        compiler
            .compile_canonical_mixed_root(SourceRequest::new("main.mec"), options)
            .unwrap(),
        compiler
            .compile_canonical_mixed_resolved_root(resolved, options)
            .unwrap(),
    ];
    assert_eq!(
        products[0].coordinator.artifact().revision(),
        products[1].coordinator.artifact().revision()
    );
    assert_eq!(
        products[0].compute.artifact.revision(),
        products[1].compute.artifact.revision()
    );
    for mixed in products {
        assert_eq!(
            mixed.source_dependencies,
            BTreeMap::from([
                ("memory:dep.mec".into(), mech_core::hash_str(dependency)),
                ("memory:leaf.mec".into(), mech_core::hash_str(leaf)),
            ])
        );
        assert_eq!(mixed.compute.interface.inputs.len(), 1);
        let port = &mixed.compute.interface.inputs[0];
        assert_eq!(port.name.as_ref(), "x");
        assert_eq!(
            mixed.compute.initializers.get(port.id),
            Some(&mech_compute::ComputeValue::ScalarF32(3.0))
        );
        assert_eq!(
            mixed.activation_inputs["x"],
            mech_compute::ComputeValue::ScalarF32(6.0)
        );
        let mut live = mech_engine::resident::activate(
            mech_core::ReactiveInstanceId::new(828, 0),
            &mixed.compute.artifact,
            &catalog,
            &mech_engine::resident::ActivationFacts::default(),
        )
        .unwrap();
        let input = live.plan.inputs[0].clone();
        for (value, expected) in [(6.0, 9.0), (10.0, 13.0)] {
            let value = RuntimeHostInputValue::F32(value)
                .into_value()
                .unwrap()
                .rebind(input.schema, &input.shape, mixed.compute.artifact.schemas())
                .unwrap();
            let prepared = live
                .prepare_turn_values(&[mech_engine::__resident::CapturedValueInput {
                    slot: input.slot,
                    value: &value,
                }])
                .unwrap();
            assert!(
                matches!(prepared.copied_output(0).unwrap().data(), ValueData::F32(value) if value.to_f32() == expected)
            );
            prepared.publish().unwrap();
        }
    }
    let error = compiler
        .compile_canonical_mixed_root(SourceRequest::new("broken.mec"), options)
        .unwrap_err();
    assert!(
        error.kind_message().contains("dependency cycle"),
        "{error:?}"
    );
    assert!(
        compiler
            .compile_canonical_mixed_root(SourceRequest::new("main.mec"), options)
            .is_ok()
    );
}

#[cfg(feature = "compute")]
#[test]
fn canonical_mixed_document_retains_batched_activation_values() {
    let source = r#"
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
lanes := [1f32 2f32 3f32 4f32]
@compute/input/x <- lanes * 0.001<f32>
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
x := 0f32
result := x + 1f32
result
"#;
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_source(source).unwrap();

    assert_eq!(
        mixed.activation_inputs["x"],
        mech_compute::ComputeValue::TensorF32 {
            dimensions: vec![4].into_boxed_slice(),
            layout: mech_compute::TensorLayout::RowMajor,
            values: Arc::from([0.001, 0.002, 0.003, 0.004]),
        }
    );
}

#[cfg(feature = "compute")]
#[test]
fn mixed_tree_retains_only_explicit_sample_read_capabilities() {
    let tree = mech_syntax::parse(
        r#"
@compute := compute://worker/kernel{:read(sample/result), :write(input/x), :write(turn)}
@compute/input/x <- 1f32
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
x := 1f32
result := x + 2f32
unused := x + 3f32
(result, unused)
"#,
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();

    let mixed = compiler.compile_mixed_tree(&tree).unwrap();

    assert_eq!(
        mixed.retained_outputs,
        BTreeSet::from(["result".to_owned()])
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
    assert!(
        rendered.contains("compute boundary planning failed"),
        "{rendered}"
    );
    assert!(rendered.contains("ElementCountMismatch"), "{rendered}");
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
fn mixed_root_inlines_user_function_graphs_into_compute_artifacts() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string(
            "main.mec",
            r#"
@compute := compute://worker/kernel{:write(input/x), :write(turn)}
@compute/input/x <- [3f32; 4f32]
@compute/turn <- 1

calculation @compute
-------------------------------------------------------------------------------
twice(value<[f32]:2,1>) = result<[f32]:2,1> :=
  result := value * 2f32.

x := [1f32; 2f32]
result := twice(x)
result
"#,
        )
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

    assert!(mixed.compute.interface.input_named("x").is_some());
    assert!(mixed.compute.artifact.nodes().iter().any(|node| {
        node.as_operation()
            .expect("ordinary fixture")
            .operation
            .module_path
            .as_ref()
            == ["math"]
            && node
                .as_operation()
                .expect("ordinary fixture")
                .operation
                .operation_name
                == "mul"
    }));
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
fn repeated_user_function_locals_do_not_enter_the_root_symbol_namespace() {
    const SOURCE: &str = r#"
increment(x<f32>) = y<f32> :=
  local := x + 1f32
  y := local.

first := increment(1f32)
second := increment(2f32)
second
"#;

    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler.compile_source(SOURCE).unwrap();
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();

    assert!(parsed.symbols.contains_key(&hash_str("first")));
    assert!(parsed.symbols.contains_key(&hash_str("second")));
    assert!(!parsed.symbols.contains_key(&hash_str("local")));
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
    assert_eq!(canonical_f64(loaded.initial_value.value()), 42.0);
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
    assert_eq!(canonical_f64(loaded.initial_value.value()), 42.0);
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
    assert_eq!(canonical_f64(loaded.initial_value.value()), 42.0);
}

#[test]
fn compiled_conversion_executes_after_bytecode_round_trip() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    for source_text in [
        "value := 3.9\nanswer := value<i32>\nanswer",
        "value := 3<i32>\nanswer := value<f64>\nanswer",
        "value := true\nanswer := value<string>\nanswer",
        "value := 42<u64>\nanswer := value<string>\nanswer",
        "value := [3.9 4.1]\nanswer := value<[i32]>\nanswer",
        "value<[i32]> := [3<i32> 4<i32>]\nanswer := value<[f64]>\nanswer",
    ] {
        let product = compiler
            .compile_source(source_text)
            .unwrap_or_else(|error| {
                panic!("compiled conversion failed for {source_text}: {error:?}")
            });
        assert!(
            product.artifact().nodes().iter().any(|node| {
                node.as_operation()
                    .expect("ordinary fixture")
                    .operation
                    .module_path
                    .as_ref()
                    == ["convert"]
                    && node
                        .as_operation()
                        .expect("ordinary fixture")
                        .operation
                        .operation_name
                        == "kind"
            }),
            "conversion instruction was not retained for {source_text}: {:?}",
            product.artifact().nodes(),
        );

        let mut source_runtime = runtime();
        let source = source_runtime
            .load_source_program(source_text, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap_or_else(|error| {
                panic!("source conversion failed for {source_text}: {error:?}")
            });
        let mut bytecode_runtime = runtime();
        let bytecode = bytecode_runtime
            .load_bytecode_program(
                product.bytecode(),
                crate::ResidentDurabilityPolicy::Volatile,
            )
            .unwrap_or_else(|error| {
                panic!("bytecode conversion failed for {source_text}: {error:?}")
            });
        assert_eq!(
            source.initial_value, bytecode.initial_value,
            "source and bytecode conversions diverged for {source_text}",
        );
    }
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
        loaded.initial_value.value().data(),
        ValueData::Bool(true)
    ));

    let mut matrix = runtime();
    let loaded = matrix
        .load_source_program(
            "answer := [1.0 2.0; 3.0 4.0]\nanswer",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(canonical_matrix_shape(loaded.initial_value.value()), (2, 2));
    assert_eq!(
        canonical_f64_matrix(loaded.initial_value.value()),
        [1.0, 2.0, 3.0, 4.0]
    );
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
        assert_eq!(canonical_matrix_shape(loaded.initial_value.value()), (3, 2));
        assert_eq!(
            canonical_f64_matrix(loaded.initial_value.value()),
            [1.0, 2.0, 1.0, 2.0, 7.0, 8.0]
        );
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
        assert_eq!(canonical_matrix_shape(loaded.initial_value.value()), (0, 0));
        assert!(matches!(
            loaded.initial_value.value().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::F64(values) if values.is_empty())
        ));
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

    assert_eq!(
        canonical_f64_matrix(runtime.root_symbol_value("values").unwrap().value()),
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
fn snapshot_backed_output_is_absent_until_its_first_external_publication() {
    const SOURCE: &str = r#"
@typed := test://typed/value{:read(data)}
sample := @typed/data
sample
"#;

    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(TypedObservationProvider {
            planned: ValueCell::from_exact(0.0_f32).unwrap().snapshot().unwrap(),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_027),
            subject,
            "test://typed/value/data",
            ["read"],
        )))
        .unwrap();

    let loaded = runtime
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal);
    assert!(loaded.initial_value.is_empty());
    assert!(runtime.program_output_value().unwrap().is_none());

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://typed/value", "data").unwrap(),
            crate::RuntimeHostInputValue::F32(3.0),
        ))
        .unwrap();
    assert!(matches!(
        runtime.drain_resident_host_inputs(1).unwrap().turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));

    let output = runtime
        .program_output_value()
        .unwrap()
        .expect("the accepted turn publishes the snapshot-backed matrix");
    let ValueData::F32(value) = output.value().data() else {
        panic!("expected the published f32 value, got {output:?}")
    };
    assert_eq!(value.to_f32().to_bits(), 3.0_f32.to_bits());
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
        node.as_operation()
            .expect("ordinary fixture")
            .operation
            .module_path
            .as_ref()
            == ["access"]
            && node
                .as_operation()
                .expect("ordinary fixture")
                .operation
                .operation_name
                == "index"
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
    planned: crate::RuntimeHostInputValue,
    source_packet: crate::RuntimeHostInputValue,
    bytecode_packet: crate::RuntimeHostInputValue,
) {
    const SOURCE: &str = r#"
@typed := test://typed/value{:read(data)}
values := [10.0 20.0 30.0]
selected := values[1,@typed/data]
selected
"#;

    let configured_runtime = |planned: Value| {
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

    let planned = planned.into_value().unwrap();
    let planned_for_bytecode = planned.clone();
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
                crate::RuntimeHostInputValue::$value($planned),
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
                planned: ValueCell::from_exact(planned).unwrap().snapshot().unwrap(),
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

fn assert_typed_observation_round_trip(
    planned: crate::RuntimeHostInputValue,
    packet: crate::RuntimeHostInputValue,
) {
    const SOURCE: &str = r#"
@typed := test://typed/value{:read(data)}
@typed/data
"#;

    let planned = planned.into_value().unwrap();
    let planned_for_bytecode = planned.clone();
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
        crate::RuntimeHostInputValue::Bool(false),
        crate::RuntimeHostInputValue::Bool(true),
    );
    assert_typed_observation_round_trip(
        crate::RuntimeHostInputValue::Index(1),
        crate::RuntimeHostInputValue::Index(7),
    );
    assert_typed_observation_round_trip(
        crate::RuntimeHostInputValue::F64(0.0),
        crate::RuntimeHostInputValue::F64(7.5),
    );
    assert_typed_observation_round_trip(
        crate::RuntimeHostInputValue::BoolMatrix {
            rows: 2,
            columns: 2,
            values: vec![false; 4],
        },
        crate::RuntimeHostInputValue::BoolMatrix {
            rows: 2,
            columns: 2,
            values: vec![true, false, false, true],
        },
    );
    assert_typed_observation_round_trip(
        crate::RuntimeHostInputValue::IndexMatrix {
            rows: 2,
            columns: 2,
            values: vec![1; 4],
        },
        crate::RuntimeHostInputValue::IndexMatrix {
            rows: 2,
            columns: 2,
            values: vec![1, 2, 3, 4],
        },
    );
    assert_typed_observation_round_trip(
        crate::RuntimeHostInputValue::F64Matrix {
            rows: 2,
            columns: 2,
            values: vec![0.0; 4],
        },
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
    for canonical in [false, true] {
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

        let compile_roots = if canonical {
            ProgramCompiler::compile_canonical_roots
        } else {
            ProgramCompiler::compile_roots
        };
        let product = compile_roots(
            &mut compiler,
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
}

#[test]
fn explicit_dependency_root_plans_provider_reads_exactly_once() {
    for canonical in [false, true] {
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

        let compile_roots = if canonical {
            ProgramCompiler::compile_canonical_roots
        } else {
            ProgramCompiler::compile_roots
        };
        let product = compile_roots(
            &mut compiler,
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
fn production_tuple_access_loads_through_resident_route() {
    let mut runtime = runtime();
    let outcome = runtime
        .load_source_program(
            "tuple := (1, 2); tuple.2",
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap();
    assert_eq!(outcome.route, RuntimeProgramRoute::ResidentPure);
    assert!(matches!(
        outcome.initial_value.value().data(),
        ValueData::F64(value) if value.to_f64() == 2.0
    ));
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::ResidentPure);
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
        outcome.initial_value.value().data(),
        ValueData::String(_)
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
fn resident_source_and_bytecode_enforce_configured_aggregate_memory_limits() {
    const SOURCE: &str = "x := [1.0 2.0; 3.0 4.0]\nx + x";
    let configured_runtime = |limit| {
        let mut config = crate::RuntimeConfig::default();
        config.limits.max_memory_bytes = limit;
        RuntimeBuilder::new()
            .config(config)
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap()
    };

    // Measure retained storage, not source/bytecode length or a per-call output
    // quota. The loader also needs temporary headroom for its owning initial
    // snapshot; retained bytes are not a measurement of that construction peak.
    // Exact physical admission boundaries are covered by the engine/core tests.
    let mut measured = configured_runtime(Some(u64::MAX));
    let expected = measured
        .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap()
        .initial_value;
    let retained = measured
        .resident_memory_budget
        .as_ref()
        .unwrap()
        .used_bytes();
    assert!(retained > 1);
    let ActiveProgramExecution::ResidentPure(execution) = &measured.active_program else {
        panic!("numeric source must activate through Resident")
    };
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();

    for from_bytecode in [false, true] {
        let load = |runtime: &mut crate::MechRuntime| {
            if from_bytecode {
                runtime.load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
            } else {
                runtime.load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
            }
        };
        let mut admitted = configured_runtime(Some(1024 * 1024));
        let result = load(&mut admitted).unwrap();
        assert_eq!(result.initial_value, expected);
        assert_eq!(
            admitted
                .resident_memory_budget
                .as_ref()
                .unwrap()
                .used_bytes(),
            retained
        );
        drop(result);
        admitted.unload_active_program().unwrap();
        assert_eq!(
            admitted
                .resident_memory_budget
                .as_ref()
                .unwrap()
                .used_bytes(),
            0
        );
        assert_eq!(load(&mut admitted).unwrap().initial_value, expected);

        let mut short = configured_runtime(Some(1));
        for _ in 0..2 {
            let error = load(&mut short).unwrap_err();
            assert!(error.kind_message().contains("BudgetExceeded"), "{error:?}");
            assert_eq!(short.program_route(), RuntimeProgramRoute::None);
            assert_eq!(
                short.resident_memory_budget.as_ref().unwrap().used_bytes(),
                0
            );
            assert_eq!(
                short.program_execution_info(),
                RuntimeProgramExecutionInfo::default()
            );
        }
    }

    let mut unconfigured = configured_runtime(None);
    assert!(unconfigured.resident_memory_budget.is_none());
    assert_eq!(
        unconfigured
            .load_source_program(SOURCE, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap()
            .initial_value,
        expected,
    );
}

#[test]
fn resident_string_outputs_obey_one_aggregate_runtime_memory_limit() {
    const LIMIT: u64 = 1024 * 1024;
    let seed = "x".repeat(16 * 1024);
    let singleton = format!("seed := {seed:?}\nseed + seed");
    let mut source = format!("seed := {seed:?}\n");
    for index in 0..40 {
        source.push_str(&format!("result{index} := seed + seed + \"{index}\"\n"));
    }
    let configured = |limit| {
        let mut config = crate::RuntimeConfig::default();
        config.limits.max_memory_bytes = limit;
        RuntimeBuilder::new()
            .config(config)
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap()
    };
    let mut one = configured(Some(LIMIT));
    let result = one
        .load_source_program(&singleton, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert!(
        matches!(result.initial_value.value().data(), ValueData::String(value) if value.len() == 32 * 1024)
    );

    // Each concat is individually legal. Interactive publication retains all
    // forty distinct outputs; their payload alone exceeds the aggregate cap.
    let mut unconfigured = configured(None);
    unconfigured
        .load_interactive_source_program(&source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let ActiveProgramExecution::ResidentPure(execution) = &unconfigured.active_program else {
        unreachable!()
    };
    assert!(execution.artifact.outputs().len() >= 40);
    let bytecode = encode_program_artifact_bytecode_v1(&execution.artifact).unwrap();
    for encoded in [false, true] {
        let mut limited = configured(Some(LIMIT));
        let error = if encoded {
            limited.load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Volatile)
        } else {
            limited
                .load_interactive_source_program(&source, crate::ResidentDurabilityPolicy::Volatile)
        }
        .unwrap_err();
        assert!(error.kind_message().contains("BudgetExceeded"), "{error:?}");
        assert_eq!(limited.program_route(), RuntimeProgramRoute::None);
        assert_eq!(
            limited
                .resident_memory_budget
                .as_ref()
                .unwrap()
                .used_bytes(),
            0
        );
        limited
            .load_source_program(&singleton, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap();
    }
}

#[test]
fn resident_string_and_canonical_exports_retain_their_memory_charge_after_unload() {
    for source in [r#"seed := "retained"; seed + seed"#, r#"("left", "right")"#] {
        let mut config = crate::RuntimeConfig::default();
        config.limits.max_memory_bytes = Some(1024 * 1024);
        let mut runtime = RuntimeBuilder::new()
            .config(config)
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        let exported = runtime
            .load_source_program(source, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap()
            .initial_value;
        let budget = runtime.resident_memory_budget.clone().unwrap();
        let before_clone = budget.used_bytes();
        let shared = exported.clone();
        assert!(exported.value().shares_frozen_storage(shared.value()));
        assert_eq!(budget.used_bytes(), before_clone);
        runtime.unload_active_program().unwrap();
        drop(runtime);
        let retained = budget.used_bytes();
        assert!(
            retained > 0,
            "a live exported payload lost its allocation charge"
        );
        assert_eq!(exported, shared);
        drop(exported);
        assert_eq!(budget.used_bytes(), retained);
        assert!(!shared.is_empty());
        drop(shared);
        assert_eq!(budget.used_bytes(), 0);
    }
}

#[test]
fn resident_string_growth_rejection_preserves_publication_and_recovers_on_same_consumer() {
    use mech_core::*;
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentActivationOptions, activate_with_options,
    };
    use mech_engine::{
        ArtifactBuildContext, OperationReference, SourceInput, SourceNode, SourceNodeOutput,
        SourceOutput, SourceProgram, SourceValue, compile_source_program_with_contracts,
    };

    let mut schemas = SchemaTableBuilder::new();
    let string = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::String,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = schemas.finish().unwrap();
    let string = build.resolve(string).unwrap();
    let (schemas, _) = build.into_parts();
    let (constants, _) = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts();
    let graph = SourceProgram {
        inputs: ["left", "right"]
            .map(|name| SourceInput {
                name: name.to_owned(),
                schema: string,
            })
            .into(),
        nodes: vec![SourceNode {
            body: mech_engine::SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: vec!["string".to_owned()].into(),
                    operation_name: "concat".to_owned(),
                },
                requirement: None,
            },
            inputs: vec![SourceValue::Input(0), SourceValue::Input(1)].into(),
            outputs: vec![SourceNodeOutput::Derived { schema: string }].into(),
        }]
        .into(),
        outputs: vec![SourceOutput {
            name: "result".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: string,
        }]
        .into(),
        ..SourceProgram::default()
    };
    let catalog = mech_stdlib::source_catalog();
    let operation = catalog
        .operation_specializer(OperationId::from_name("string/concat"))
        .unwrap()
        .resolved_operation(
            2,
            &[ResolvedType::from_schema_body(&SchemaBody::String, &[]).unwrap()],
        )
        .unwrap();
    let artifact = compile_source_program_with_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &[&operation.contract],
    )
    .unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    const LIMIT: u64 = 128 * 1024;
    let budget = ManagedMemoryBudget::new(LIMIT);
    let mut instance = activate_with_options(
        ReactiveInstanceId::new(91, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions {
            memory_budget: Some(budget.clone()),
            ..ResidentActivationOptions::default()
        },
    )
    .unwrap();
    let turn = |instance: &mut mech_engine::resident::ReactiveInstance, left: &str, right: &str| {
        let left = [left.to_owned()];
        let right = [right.to_owned()];
        let inputs = [
            CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::String(&left),
            },
            CapturedSignalInput {
                slot: instance.plan.inputs[1].slot,
                value: ResidentValueRef::String(&right),
            },
        ];
        instance.turn_without_summary(&inputs)
    };
    turn(&mut instance, "left", "right").unwrap();
    turn(&mut instance, &"x".repeat(1024), &"y".repeat(1024)).unwrap();
    turn(&mut instance, "small", "again").unwrap();
    let epoch = instance.published_epoch();
    let hash = instance.published_state_hash();
    let error = turn(&mut instance, &"x".repeat(LIMIT as usize + 1), "too large").unwrap_err();
    assert!(format!("{error:?}").contains("BudgetExceeded"), "{error:?}");
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(instance.published_state_hash(), hash);
    let Some(ResidentValueBorrow::String { values, .. }) = instance.output_borrow(0) else {
        panic!("concat must retain its scalar String output")
    };
    assert_eq!(values, &["smallagain"]);
    assert!(budget.used_bytes() <= LIMIT);
    turn(&mut instance, "recover", "ed").unwrap();
    let Some(ResidentValueBorrow::String { values, .. }) = instance.output_borrow(0) else {
        unreachable!()
    };
    assert_eq!(values, &["recovered"]);
    drop(instance);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn resident_canonical_import_allocation_failure_preserves_publication_and_retries() {
    use mech_core::*;
    use mech_engine::resident::{
        ActivationFacts, CapturedValueInput, ResidentActivationOptions, activate_with_options,
    };
    use mech_engine::{
        ArtifactBuildContext, OperationReference, SourceInput, SourceNode, SourceNodeOutput,
        SourceOutput, SourceProgram, SourceValue, compile_source_program_with_contracts,
    };

    let mut schemas = SchemaTableBuilder::new();
    let integer = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = schemas.finish().unwrap();
    let integer = build.resolve(integer).unwrap();
    let (schemas, _) = build.into_parts();
    let (constants, _) = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts();
    let graph = SourceProgram {
        inputs: ["left", "right"]
            .map(|name| SourceInput {
                name: name.to_owned(),
                schema: integer,
            })
            .into(),
        nodes: vec![SourceNode {
            body: mech_engine::SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: vec!["math".to_owned()].into(),
                    operation_name: "add".to_owned(),
                },
                requirement: None,
            },
            inputs: vec![SourceValue::Input(0), SourceValue::Input(1)].into(),
            outputs: vec![SourceNodeOutput::Derived { schema: integer }].into(),
        }]
        .into(),
        outputs: vec![SourceOutput {
            name: "result".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: integer,
        }]
        .into(),
        ..SourceProgram::default()
    };
    let catalog = mech_stdlib::source_catalog();
    let operation =
        catalog
            .operation_specializer(OperationId::from_name("math/add"))
            .unwrap()
            .resolved_operation(
                2,
                &[ResolvedType::from_schema_body(
                    &SchemaBody::UnsignedInteger(IntegerWidth::W64),
                    &[],
                )
                .unwrap()],
            )
            .unwrap();
    let artifact = compile_source_program_with_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &[&operation.contract],
    )
    .unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let context = mech_core::snapshot::SnapshotValidationContext::new(artifact.schemas());
    let value = |number| {
        ValueDraft {
            schema: integer,
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(number),
        }
        .finalize(&context)
        .unwrap()
    };
    let turn =
        |instance: &mut mech_engine::resident::ReactiveInstance, left: &Value, right: &Value| {
            let inputs = [
                CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: left,
                },
                CapturedValueInput {
                    slot: instance.plan.inputs[1].slot,
                    value: right,
                },
            ];
            instance.prepare_turn_values(&inputs)?.publish()
        };
    let output = |instance: &mech_engine::resident::ReactiveInstance| {
        let Some(ResidentValueBorrow::Snapshot { values, .. }) = instance.output_borrow(0) else {
            panic!("U64 addition must execute through the canonical resident lane")
        };
        let ValueData::U64(number) = values[0].as_ref().unwrap().data() else {
            panic!("U64 addition changed its semantic output type")
        };
        *number
    };

    // Exercise both fallible allocations of the returned canonical owner.
    // Caller-owned roots remain alive throughout rejection, retry and teardown:
    // importing them may retain a wrapper, but must not mutate their ownership.
    for successful_allocations in [0, 1] {
        let budget = ManagedMemoryBudget::new(1024 * 1024);
        let mut instance = activate_with_options(
            ReactiveInstanceId::new(92 + successful_allocations, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
            ResidentActivationOptions {
                memory_budget: Some(budget.clone()),
                ..ResidentActivationOptions::default()
            },
        )
        .unwrap();
        let initial = [value(2), value(3)];
        let replacement = [value(11), value(13)];
        turn(&mut instance, &initial[0], &initial[1]).unwrap();
        assert_eq!(output(&instance), 5);
        let epoch = instance.published_epoch();
        let hash = instance.published_state_hash();
        let retained = budget.used_bytes();

        budget.inject_snapshot_import_failure_after(successful_allocations);
        let error = turn(&mut instance, &replacement[0], &replacement[1]).unwrap_err();
        assert!(
            format!("{error:?}").contains("AllocationFailed"),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(instance.published_state_hash(), hash);
        assert_eq!(output(&instance), 5);
        // A failed import can release an obsolete workspace value, but cannot
        // strand any additional registration or returned-owner reservation.
        assert!(budget.used_bytes() <= retained);
        for caller in initial.iter().chain(&replacement) {
            assert_eq!(caller.memory_budget_retained_bytes(&budget), None);
        }

        turn(&mut instance, &replacement[0], &replacement[1]).unwrap();
        assert_eq!(output(&instance), 24);
        assert_ne!(instance.published_epoch(), epoch);
        drop(instance);
        assert_eq!(budget.used_bytes(), 0);
        assert!(matches!(replacement[0].data(), ValueData::U64(11)));
        assert!(matches!(replacement[1].data(), ValueData::U64(13)));
    }
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
    assert_eq!(runtime.program_execution_info().resident_accepted_turns, 2);
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
fn resident_host_packet_groups_preserve_activation_boundaries() {
    let (mut runtime, _, _, _) = external_runtime(crate::ResidentDurabilityPolicy::Retained);
    let trigger = crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap();
    let ingress = runtime.ingress();
    for (group, value) in [(1, 7.0), (1, 8.0), (2, 9.0), (2, 10.0)] {
        ingress
            .submit(
                crate::RuntimeHostInput::single(
                    trigger.clone(),
                    crate::RuntimeHostInputValue::F64(value),
                )
                .with_coalescing_group(group),
            )
            .unwrap();
    }

    let first = runtime.drain_resident_host_inputs(64).unwrap();
    assert_eq!(first.dequeued_packets, 2);
    assert_eq!(first.coalesced_packets, 1);
    assert_eq!(runtime.pending_host_input_count().unwrap(), 2);

    let second = runtime.drain_resident_host_inputs(64).unwrap();
    assert_eq!(second.dequeued_packets, 2);
    assert_eq!(second.coalesced_packets, 1);
    assert_eq!(runtime.pending_host_input_count().unwrap(), 0);
}

#[test]
fn activation_capture_packets_sample_without_running_until_the_trigger_arrives() {
    let (mut runtime, _) = independent_canonical_external_runtime_with_source(
        r#"
@fast := test://clock/fast{:read(delta-seconds)}
@slow := test://clock/slow{:read(delta-seconds)}
event := @fast/delta-seconds
~selected := 0.0
~> event { selected = event + @slow/delta-seconds }
selected
"#,
    );
    let fast = crate::RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap();
    let slow = crate::RuntimeHostInputSource::new("test://clock/slow", "delta-seconds").unwrap();
    let selected = |runtime: &crate::MechRuntime| {
        let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
            panic!("activation input fixture must remain resident external")
        };
        canonical_f64(&execution.coordinator.instance().copied_output(0).unwrap())
    };
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            slow,
            crate::RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();
    let sampled = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(sampled.turn.is_none());
    assert_eq!(selected(&runtime), 0.0);

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            fast,
            crate::RuntimeHostInputValue::F64(4.0),
        ))
        .unwrap();
    let triggered = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        triggered.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(selected(&runtime), 13.0);
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("activation input fixture must remain resident external")
    };
    let batch = execution.coordinator.input_facts().next().unwrap().1;
    assert_eq!(batch.facts.iter().filter(|fact| fact.trigger).count(), 1);
    let evidence = runtime.drain_resident_evidence().unwrap();
    assert_eq!(evidence.input_batches.len(), 1);
    assert_eq!(evidence.receipts.len(), 1);
    runtime.unload_active_program().unwrap();
    assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
}

#[test]
fn mixed_initial_publication_runs_ordinary_roots_and_defers_input_free_activation() {
    let source = r#"
trigger := true
~count := 0
~> trigger { count = count + 1 }
answer := 40 + 2
answer
"#;
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let artifact = compiler
        .compile_canonical_source(source)
        .unwrap()
        .into_parts()
        .0;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(catalog)
        .build()
        .unwrap();
    runtime
        .load_compiled_program(artifact, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();

    let output = |runtime: &crate::MechRuntime| {
        let ActiveProgramExecution::ResidentPure(execution) = &runtime.active_program else {
            panic!("mixed activation fixture must remain resident pure")
        };
        canonical_f64(&execution.instance.copied_output(0).unwrap())
    };
    assert_eq!(output(&runtime), 42.0);

    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let artifact = compiler
        .compile_canonical_source(
            "trigger := true\n~count := 0\n~> trigger { count = count + 1 }\ncount\n",
        )
        .unwrap()
        .into_parts()
        .0;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(catalog)
        .build()
        .unwrap();
    runtime
        .load_compiled_program(artifact, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    assert_eq!(output(&runtime), 0.0);

    let ActiveProgramExecution::ResidentPure(execution) = &mut runtime.active_program else {
        panic!("mixed activation fixture must remain resident pure")
    };
    execution
        .instance
        .prepare_turn_values_with_activation_triggers(&[], &[])
        .unwrap()
        .publish()
        .unwrap();
    assert_eq!(output(&runtime), 1.0);
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

    let state = runtime.root_symbol_value("state").unwrap();
    assert_eq!(canonical_f64(state.value()), 2.0);
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
fn driverless_observation_gets_a_trigger_turn_after_dormant_publication() {
    let reads = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    runtime
        .register_resource_provider(Box::new(DriverlessObservationProvider {
            reads: reads.clone(),
        }))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(9_023),
            subject,
            "snapshot://clock/tick/value",
            ["read"],
        )))
        .unwrap();

    runtime
        .load_source_program(
            r#"
@clock := snapshot://clock/tick{:read(value)}
trigger := @clock/value
~count := 0u64
~> trigger { count = 41u64 }
count
"#,
            crate::ResidentDurabilityPolicy::Retained,
        )
        .unwrap();

    assert_eq!(runtime.program_execution_info().resident_accepted_turns, 2);
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    let count = runtime.root_symbol_value("count").unwrap();
    assert_eq!(
        count.value().canonical_data_draft().unwrap(),
        ValueDataDraft::U64(41)
    );
}

#[test]
fn driverless_observation_gets_a_provider_turn_alongside_driven_triggers() {
    let driverless_reads = Arc::new(AtomicUsize::new(0));
    let driven_reads = Arc::new(AtomicUsize::new(0));
    let driven_plans = Arc::new(AtomicUsize::new(0));
    let driven_value = Arc::new(AtomicU64::new(1.0_f64.to_bits()));
    let driverless_provider = || DriverlessObservationProvider {
        reads: driverless_reads.clone(),
    };
    let driven_provider = || PlanningObservationProvider {
        plans: driven_plans.clone(),
        reads: driven_reads.clone(),
        value_bits: driven_value.clone(),
    };
    let source = r#"
@snapshot := snapshot://clock/tick{:read(value)}
@clock := test://clock/tick{:read(delta-seconds)}
snapshot-trigger := @snapshot/value
driven-trigger := @clock/delta-seconds
~snapshot-count := 0u64
~driven-count := 0u64
~> snapshot-trigger { snapshot-count = snapshot-count + 1u64 }
~> driven-trigger { driven-count = driven-count + 1u64 }
snapshot-count
"#;
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .resource_provider(Box::new(driverless_provider()))
        .resource_provider(Box::new(driven_provider()))
        .build_compiler()
        .unwrap();
    let artifact = compiler
        .compile_canonical_source(source)
        .unwrap()
        .into_parts()
        .0;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(catalog)
        .input_driver(ResidentTestInputDriver)
        .build()
        .unwrap();
    runtime
        .register_resource_provider(Box::new(driverless_provider()))
        .unwrap();
    runtime
        .register_resource_provider(Box::new(driven_provider()))
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    for (id, resource) in [
        (CapabilityId(9_024), "snapshot://clock/tick/value"),
        (CapabilityId(9_025), "test://clock/tick/delta-seconds"),
    ] {
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                id,
                subject.clone(),
                resource,
                ["read"],
            )))
            .unwrap();
    }

    runtime
        .load_compiled_program(artifact, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();

    assert_eq!(runtime.program_execution_info().resident_accepted_turns, 2);
    assert_eq!(driverless_reads.load(Ordering::SeqCst), 2);
    assert_eq!(driven_reads.load(Ordering::SeqCst), 2);
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("mixed provider fixture must remain resident external")
    };
    assert_eq!(
        execution
            .coordinator
            .instance()
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::U64(1)
    );
    let provider_batch = execution.coordinator.input_facts().last().unwrap().1;
    assert!(provider_batch.facts[0].trigger);
    assert!(!provider_batch.facts[1].trigger);

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/tick", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(2.0),
        ))
        .unwrap();
    let outcome = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("mixed provider fixture must remain resident external")
    };
    assert_eq!(
        execution
            .coordinator
            .instance()
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::U64(1)
    );
    let host_batch = execution.coordinator.input_facts().last().unwrap().1;
    assert!(!host_batch.facts[0].trigger);
    assert!(host_batch.facts[1].trigger);
}

#[test]
fn independent_observations_seed_then_retain_the_accepted_host_snapshot() {
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

    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/slow", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(8.0),
        ))
        .unwrap();
    let outcome = runtime.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(
        reads.load(Ordering::SeqCst),
        1,
        "an unrelated provider snapshot must not overwrite accepted host state"
    );
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    let mut values = execution
        .coordinator
        .input_facts()
        .last()
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
    assert_eq!(values, vec![7.0, 8.0]);

    let outcome = {
        let ActiveProgramExecution::ResidentExternal(execution) = &mut runtime.active_program
        else {
            unreachable!()
        };
        execution.coordinator.execute_turn().unwrap()
    };
    assert!(matches!(
        outcome,
        crate::ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(
        reads.load(Ordering::SeqCst),
        3,
        "provider-driven turns must still refresh every live observation"
    );
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        unreachable!()
    };
    let mut values = execution
        .coordinator
        .input_facts()
        .last()
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
    assert_eq!(values, vec![2.0, 3.0]);
}

#[test]
fn compatible_replacement_migrates_the_accepted_host_snapshot() {
    let (mut previous, previous_reads) = independent_external_runtime();
    previous
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(7.0),
        ))
        .unwrap();
    previous.drain_resident_host_inputs(1).unwrap();
    assert_eq!(previous_reads.load(Ordering::SeqCst), 1);

    let (mut candidate, candidate_reads) = independent_external_runtime();
    candidate
        .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
        .unwrap();
    candidate
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/slow", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(8.0),
        ))
        .unwrap();
    let outcome = candidate.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(
        candidate_reads.load(Ordering::SeqCst),
        0,
        "a compatible replacement must not synthesize absent fields from providers"
    );
    let ActiveProgramExecution::ResidentExternal(execution) = &candidate.active_program else {
        unreachable!()
    };
    let mut values = execution
        .coordinator
        .input_facts()
        .last()
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
    assert_eq!(values, vec![7.0, 8.0]);
}

#[test]
fn compatible_replacement_expands_one_snapshot_to_new_duplicate_consumers() {
    let (mut previous, previous_reads) = independent_external_runtime();
    previous
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(7.0),
        ))
        .unwrap();
    previous.drain_resident_host_inputs(1).unwrap();
    assert_eq!(previous_reads.load(Ordering::SeqCst), 1);

    let (mut candidate, candidate_reads) = independent_external_runtime_with_source(
        r#"
@fast := test://clock/fast{:read(delta-seconds)}
@fast-copy := test://clock/fast{:read(delta-seconds)}
@slow := test://clock/slow{:read(delta-seconds)}
fast := @fast/delta-seconds
fast-copy := @fast-copy/delta-seconds
slow := @slow/delta-seconds
~state := 0.0
state += fast + fast-copy + slow
output := state
"#,
    );
    candidate
        .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
        .unwrap();
    candidate
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            crate::RuntimeHostInputSource::new("test://clock/slow", "delta-seconds").unwrap(),
            crate::RuntimeHostInputValue::F64(8.0),
        ))
        .unwrap();
    let outcome = candidate.drain_resident_host_inputs(1).unwrap();
    assert!(matches!(
        outcome.turn,
        Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
    ));
    assert_eq!(
        candidate_reads.load(Ordering::SeqCst),
        0,
        "new duplicate consumers must inherit the accepted source snapshot"
    );
    let ActiveProgramExecution::ResidentExternal(execution) = &candidate.active_program else {
        unreachable!()
    };
    let mut values = execution
        .coordinator
        .input_facts()
        .last()
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
    assert_eq!(values, vec![7.0, 7.0, 8.0]);
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
    assert!(
        matches!(
            outcome.turn,
            Some(crate::ResidentExternalTurnOutcome::Accepted { .. })
        ),
        "n-body turn was not accepted: {:?}",
        outcome.turn,
    );
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

    let published_energy = |runtime: &crate::MechRuntime| {
        let value = runtime
            .program_output_value()
            .unwrap()
            .expect("N-body publishes total energy");
        assert_eq!(canonical_matrix_shape(value.value()), (1, 1));
        canonical_f64_matrix(value.value())[0]
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
fn effect_only_resident_program_executes_during_initial_publication_with_dormant_activation() {
    let (mut runtime, scene) = product_nbody_runtime();
    let loaded = runtime
        .load_source_program(
            r#"
@scene := scene://orbit/frame{:write(points)}
trigger := true
~count := 0
~> trigger { count = count + 1 }
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
fn initial_publication_replays_with_activations_dormant() {
    let (mut runtime, scene) = product_nbody_runtime();
    runtime
        .load_source_program(
            "@scene := scene://orbit/frame{:write(points)}\ntrigger := true\n~count := 0\n~> trigger { count = count + 1 }\npoints := [1.0 2.0]\n@scene/points <- points\n",
            crate::ResidentDurabilityPolicy::Retained,
        )
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("initial publication must use the external resident route")
    };
    let artifact = Arc::clone(&execution.artifact);
    let id = execution.coordinator.instance().id;
    let record = execution.coordinator.receipts().next().unwrap().1.clone();
    assert!(record.body.initial_publication);

    let catalog = mech_stdlib::source_catalog();
    let instance = mech_engine::__resident::activate_external(
        id,
        &artifact,
        &catalog,
        &mech_engine::__resident::ActivationFacts::default(),
        mech_engine::__resident::ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut replay = external::ResidentExternalCoordinator::new_replay(
        instance,
        artifact,
        crate::ResidentDurabilityPolicy::Retained,
        external::ResidentExternalLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        replay.execute_replay_batch(None, &record).unwrap(),
        crate::ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(replay.receipts().next().unwrap().1, &record);
    assert_eq!(scene.lock().unwrap().deliveries, 1);
}

#[test]
fn continuation_drain_replay_keeps_input_free_activations_dormant() {
    let (mut runtime, scene) = product_nbody_runtime();
    let source = "@scene := scene://orbit/frame{:write(points)}\n#Deferred() => <u64>\n  | :Start\n  | :Done.\n#Deferred() -> :Start\n  :Start ~> :Done\n  :Done => 41u64.\ntrigger := true\n~count := 0u64\n~> trigger { count = count + 1u64 }\npoints := [1.0 2.0]\n@scene/points <- points\n#Deferred()\n";
    let document = crate::SourceDocument::parse_resolved(
        "test://continuation-replay",
        mech_syntax::document::Revision(0),
        source,
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap();
    let artifact = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .resource_provider(Box::new(ProductSceneProvider {
            trace: scene,
            contract: ProductSceneContract::AtMostOnce,
            prepare_delay: Duration::ZERO,
        }))
        .build_compiler()
        .unwrap()
        .compile_document_artifact(&document)
        .unwrap()
        .into_artifact();
    let bytecode = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    runtime
        .load_bytecode_program(&bytecode, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    let ActiveProgramExecution::ResidentExternal(execution) = &runtime.active_program else {
        panic!("continuation fixture must use the external resident route")
    };
    let artifact = Arc::clone(&execution.artifact);
    let id = execution.coordinator.instance().id;
    let records = execution
        .coordinator
        .receipts()
        .map(|(_, record)| record.clone())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert!(records[0].body.initial_publication);
    assert!(!records[0].body.continuation_drain);
    assert!(!records[1].body.initial_publication);
    assert!(records[1].body.continuation_drain);

    let catalog = mech_stdlib::source_catalog();
    let instance = mech_engine::__resident::activate_external(
        id,
        &artifact,
        &catalog,
        &mech_engine::__resident::ActivationFacts::default(),
        mech_engine::__resident::ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut replay = external::ResidentExternalCoordinator::new_replay(
        instance,
        artifact,
        crate::ResidentDurabilityPolicy::Retained,
        external::ResidentExternalLimits::default(),
    )
    .unwrap();
    for record in &records {
        assert!(matches!(
            replay.execute_replay_batch(None, record).unwrap(),
            crate::ResidentExternalTurnOutcome::Accepted { .. }
        ));
    }
    assert_eq!(
        replay
            .receipts()
            .map(|(_, record)| record.clone())
            .collect::<Vec<_>>(),
        records
    );
}

#[test]
fn initial_external_export_rejection_precedes_effect_and_epoch_publication() {
    let source = format!(
        r#"
@scene := scene://orbit/frame{{:write(points)}}
seed := {:?}
result := seed + seed
@scene/points <- [1.0 2.0]
result
"#,
        "x".repeat(16 * 1024),
    );
    let configured = |limit| {
        let (mut runtime, trace) = product_nbody_runtime();
        runtime.config.limits.max_memory_bytes = Some(limit);
        runtime.resident_memory_budget = Some(mech_core::ManagedMemoryBudget::new(limit));
        (runtime, trace)
    };

    let (mut probe, _) = configured(u64::MAX);
    let exported = probe
        .load_source_program(&source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap();
    let budget = probe.resident_memory_budget.clone().unwrap();
    let with_export = budget.used_bytes();
    drop(exported);
    let retained = budget.used_bytes();
    assert!(with_export > retained);
    probe.unload_active_program().unwrap();
    assert_eq!(budget.used_bytes(), 0);

    let (mut rejected, trace) = configured(with_export - 1);
    let error = rejected
        .load_source_program(&source, crate::ResidentDurabilityPolicy::Volatile)
        .unwrap_err();
    assert!(
        error
            .kind_message()
            .contains("candidate output snapshot failed"),
        "{error:?}"
    );
    assert_eq!(trace.lock().unwrap().deliveries, 0);
    assert_eq!(rejected.program_route(), RuntimeProgramRoute::None);
    assert_eq!(
        rejected
            .resident_memory_budget
            .as_ref()
            .unwrap()
            .used_bytes(),
        0
    );
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
fn product_nbody_source_and_bytecode_match_reference_for_4096_accepted_turns() {
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
        platform => panic!("unsupported n-body trajectory platform {platform:?}"),
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

#[test]
fn canonical_resource_planning_excludes_inactive_document_owners() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    for inactive in [
        "```mech:worker\nvalue := @missing/input\n@missing/output <- value\n```\n",
        "```mech:disabled\nvalue := @missing/input\n@missing/output <- value\n```\n",
        "╭◉╮⸢value := @missing/input\n@missing/output <- value\n⸥\n",
    ] {
        let product = compiler
            .compile_canonical_source(&format!("answer := 42\n{inactive}"))
            .unwrap();
        assert!(product.artifact().inputs().is_empty());
    }
    assert!(
        compiler
            .compile_canonical_source("answer := @missing/input\n")
            .is_err()
    );
    assert!(
        compiler
            .compile_canonical_source("```mech\nanswer := @missing/input\n```\n")
            .is_err()
    );
}

#[test]
fn canonical_native_sidecars_cover_every_encoded_instruction() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("answer := 40 + 2\n")
        .unwrap();
    let (artifact, bytecode, bindings, requirements, memory) = product.into_native_parts();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    assert!(!artifact.nodes().is_empty());
    assert!(parsed.instructions.is_empty());
    assert_eq!(bindings.len(), parsed.instructions.len());
    assert_eq!(requirements.len(), parsed.instructions.len());
    assert_eq!(memory.len(), parsed.instructions.len());
}

#[test]
fn canonical_trailing_resource_send_preserves_implicit_result() {
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .resource_provider(Box::new(ProductSceneProvider {
            trace: Arc::new(Mutex::new(ProductSceneTrace::default())),
            contract: ProductSceneContract::AtMostOnce,
            prepare_delay: Duration::ZERO,
        }))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source(
            "@scene := scene://orbit/frame{:write(points)}\nanswer := 42\n@scene/points <- [1 2]\n",
        )
        .unwrap();
    let mut instance = mech_engine::__resident::activate_external(
        mech_core::ReactiveInstanceId::new(801, 0),
        product.artifact(),
        &catalog,
        &mech_engine::resident::ActivationFacts::default(),
        mech_engine::resident::ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let prepared = instance.prepare_turn(&[]).unwrap();
    assert_eq!(prepared.effect_intents().count(), 1);
    let value =
        crate::RuntimeValueSnapshot::from_value(prepared.copied_output(0).unwrap()).unwrap();
    assert_eq!(value.format_canonical_inline(), "42");
    prepared.abort();
}

fn canonical_planning_test_document(source: &str) -> crate::SourceDocument {
    crate::SourceDocument::parse_resolved(
        "test:canonical-planning",
        mech_syntax::document::Revision(0),
        source,
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap()
}

#[test]
fn canonical_planning_values_remain_constants_and_live_defaults_are_detached() {
    let document =
        canonical_planning_test_document("port := supplied\nnext := port + 2f32\nnext\n");
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let supplied = BTreeMap::from([("supplied".to_owned(), RuntimeHostInputValue::F32(40.0))]);
    let external = BTreeSet::from(["port".to_owned()]);
    let (product, initializers) = compiler
        .compile_document_artifact_with_input_initializers(&document, &supplied, &external)
        .unwrap();
    assert_eq!(
        initializers,
        BTreeMap::from([("port".to_owned(), RuntimeHostInputValue::F32(40.0))])
    );
    assert_eq!(product.artifact().inputs().len(), 1);
    assert_eq!(
        product.artifact().inputs()[0].name,
        mech_engine::encode_source_input_name("port")
    );
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut live = mech_engine::resident::activate(
        mech_core::ReactiveInstanceId::new(809, 0),
        product.artifact(),
        &catalog,
        &mech_engine::resident::ActivationFacts::default(),
    )
    .unwrap();
    let input = live.plan.inputs[0].clone();
    let changed = RuntimeHostInputValue::F32(50.0)
        .into_value()
        .unwrap()
        .rebind(input.schema, &input.shape, product.artifact().schemas())
        .unwrap();
    let prepared = live
        .prepare_turn_values(&[mech_engine::__resident::CapturedValueInput {
            slot: input.slot,
            value: &changed,
        }])
        .unwrap();
    assert!(
        matches!(prepared.copied_output(0).unwrap().data(), ValueData::F32(value) if value.to_f32() == 52.0)
    );
    prepared.abort();
    let constants = compiler
        .compile_document_artifact_with_inputs(&document, &supplied, &BTreeSet::new())
        .unwrap();
    assert!(constants.artifact().inputs().is_empty());
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut instance = mech_engine::resident::activate(
        mech_core::ReactiveInstanceId::new(808, 0),
        constants.artifact(),
        &catalog,
        &mech_engine::resident::ActivationFacts::default(),
    )
    .unwrap();
    let prepared = instance.prepare_turn(&[]).unwrap();
    assert!(
        matches!(prepared.copied_output(0).unwrap().data(), ValueData::F32(value) if value.to_f32() == 42.0)
    );
    prepared.abort();
    assert!(
        compiler
            .compile_document_artifact_with_inputs(
                &document,
                &supplied,
                &BTreeSet::from(["absent".to_owned()]),
            )
            .is_err()
    );
}

#[test]
fn canonical_static_symbols_filter_and_detach_matrix_values() {
    let document = canonical_planning_test_document(
        "matrix := [1f32 2f32; 3f32 4f32]\nanswer := supplied + 2f32\n",
    );
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let supplied = BTreeMap::from([("supplied".to_owned(), RuntimeHostInputValue::F32(40.0))]);
    let first = compiler
        .evaluate_static_document_symbols_with_inputs(&document, &supplied, &["matrix"])
        .unwrap();
    assert_eq!(
        first,
        BTreeMap::from([(
            "matrix".to_owned(),
            RuntimeHostInputValue::F32Matrix {
                rows: 2,
                columns: 2,
                values: vec![1.0, 2.0, 3.0, 4.0],
            }
        )])
    );
    assert!(
        compiler
            .evaluate_static_document_symbols_with_inputs(&document, &supplied, &["missing"])
            .is_err()
    );
    let second = compiler
        .evaluate_static_document_symbols_with_inputs(&document, &supplied, &["answer"])
        .unwrap();
    assert_eq!(
        second,
        BTreeMap::from([("answer".to_owned(), RuntimeHostInputValue::F32(42.0))])
    );
    let literal = canonical_planning_test_document("matrix := [1f32 2f32; 3f32 4f32]\n");
    assert_eq!(
        compiler
            .evaluate_static_document_symbols(&literal, &["matrix"])
            .unwrap(),
        first
    );
    let (product, defaults) = compiler
        .compile_document_artifact_with_input_initializers(
            &literal,
            &BTreeMap::new(),
            &BTreeSet::from(["matrix".to_owned()]),
        )
        .unwrap();
    assert_eq!(defaults, first);
    assert_eq!(product.artifact().inputs().len(), 1);
}

#[test]
fn canonical_document_functions_inline_typed_named_calls_without_leaking_bindings() {
    let document = canonical_planning_test_document(
        "twice(value<f32>) = result<f32> :=\n  result := value * 2f32.\n\nplus(value<f32>, offset<f32>) = result<f32> :=\n  result := twice(value) + offset.\n\nvalue := 7f32\nanswer := plus(offset: 2f32, value: 20f32)\n",
    );
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    assert_eq!(
        compiler
            .evaluate_static_document_symbols(&document, &["answer", "value"])
            .unwrap(),
        BTreeMap::from([
            ("answer".to_owned(), RuntimeHostInputValue::F32(42.0)),
            ("value".to_owned(), RuntimeHostInputValue::F32(7.0)),
        ])
    );
    for expression in [
        "twice()",
        "twice(value: 1f32, value: 2f32)",
        "twice(wrong: 1f32)",
        "twice(true)",
    ] {
        let document = canonical_planning_test_document(&format!(
            "twice(value<f32>) = result<f32> :=\n  result := value * 2f32.\n\nanswer := {expression}\n",
        ));
        assert!(
            compiler.compile_document(&document).is_err(),
            "{expression}"
        );
    }
    let recursive = canonical_planning_test_document(
        "loop(value<f32>) = result<f32> :=\n  result := loop(value).\n\nanswer := loop(1f32)\n",
    );
    assert!(compiler.compile_document(&recursive).is_err());
    let missing_output = canonical_planning_test_document(
        "result := 42f32\nmissing(value<f32>) = result<f32> :=\n  local := value.\n\nanswer := missing(1f32)\n",
    );
    assert!(compiler.compile_document(&missing_output).is_err());
}

#[cfg(feature = "compute")]
#[test]
fn canonical_mixed_source_inlines_user_function_graphs() {
    let source = "@compute := compute://worker/kernel{:write(input/x), :write(turn)}\n@compute/input/x <- [3f32; 4f32]\n@compute/turn <- 1\n\ncalculation @compute\n-------------------------------------------------------------------------------\ntwice(value<[f32]:2,1>) = result<[f32]:2,1> :=\n  result := value * 2f32.\n\nx := [1f32; 2f32]\nresult := twice(x)\nresult\n";
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let mixed = compiler.compile_mixed_source(source).unwrap();
    assert!(mixed.compute.interface.input_named("x").is_some());
    assert!(mixed.compute.artifact.nodes().iter().any(|node| {
        node.as_operation().is_some_and(|node| {
            node.operation.module_path.as_ref() == ["math"]
                && node.operation.operation_name == "mul"
        })
    }));
}

#[test]
fn canonical_static_symbol_result_does_not_alias_the_implicit_result() {
    let document = canonical_planning_test_document("result := 40f32\nanswer := 42f32\n");
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    assert_eq!(
        compiler
            .evaluate_static_document_symbols(&document, &["result"])
            .unwrap(),
        BTreeMap::from([("result".to_owned(), RuntimeHostInputValue::F32(40.0))])
    );
    let supplied = BTreeMap::from([("unused".to_owned(), RuntimeHostInputValue::F32(7.0))]);
    assert_eq!(
        compiler
            .evaluate_static_document_symbols_with_inputs(&document, &supplied, &["unused"])
            .unwrap(),
        supplied
    );
}

#[test]
fn canonical_functions_do_not_capture_caller_symbols_or_existing_inputs() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for prelude in ["hidden := 42f32", "before := hidden<f32>"] {
        let document = canonical_planning_test_document(&format!(
            "{prelude}\nwrong(value<f32>) = result<f32> :=\n  result := hidden.\n\nanswer := wrong(1f32)\n",
        ));
        let error = compiler
            .compile_document(&document)
            .unwrap_err()
            .display_message();
        assert!(error.contains("undeclared local hidden"), "{error}");
    }
    let document = canonical_planning_test_document(
        "hidden := [42f32]\nwrong(value<f32>) = result<f32> :=\n  result := hidden[1].\n\nanswer := wrong(1f32)\n",
    );
    let error = compiler
        .compile_document(&document)
        .unwrap_err()
        .display_message();
    assert!(error.contains("undeclared local hidden"), "{error}");
}

#[test]
fn canonical_functions_admit_configured_resource_inputs_on_first_use() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(ProductTimerProvider))
        .build_compiler()
        .unwrap();
    let document = canonical_planning_test_document(
        "@clock := timer://clock/tick{:read(tick)}\nsample() = result<f64> :=\n  result := (@clock/tick).\n\nanswer := sample()\n",
    );
    let product = compiler.compile_document(&document).unwrap();
    assert!(product.artifact().requirements().iter().any(|(_, requirement)| matches!(requirement,
        mech_core::ApplicationRequirement::Resource(request) if request.base_uri == "timer://clock/tick")));
    assert_eq!(
        compiler
            .evaluate_static_document_symbols(&document, &["answer"])
            .unwrap(),
        BTreeMap::from([("answer".to_owned(), RuntimeHostInputValue::F64(0.0))])
    );
}

#[test]
fn canonical_resource_defaults_do_not_create_unselected_live_observations() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(ProductTimerProvider))
        .build_compiler()
        .unwrap();
    for tail in ["answer := port + 1.0", "answer := port + @clock/tick"] {
        let document = canonical_planning_test_document(&format!(
            "@clock := timer://clock/tick{{:read(tick)}}\nport := @clock/tick + 2.0\n{tail}\n",
        ));
        let (product, defaults) = compiler
            .compile_document_artifact_with_input_initializers(
                &document,
                &BTreeMap::new(),
                &BTreeSet::from(["port".to_owned()]),
            )
            .unwrap();
        assert_eq!(defaults["port"], RuntimeHostInputValue::F64(2.0));
        assert_eq!(product.artifact().inputs().len(), 1);
        let has_clock = product.artifact().requirements().iter().any(|(_, requirement)| matches!(requirement,
            mech_core::ApplicationRequirement::Resource(request) if request.base_uri == "timer://clock/tick"));
        assert_eq!(has_clock, tail.contains('@'));
    }
}

#[test]
fn canonical_tuple_destructure_consumes_local_function_outputs() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let document = canonical_planning_test_document(
        "pair(value<f32>) = (left<f32>, right<f32>) :=\n  left := value; right := value + 2f32.\n\n(a, b) := pair(20f32)\nanswer := a + b\n",
    );
    assert_eq!(
        compiler
            .evaluate_static_document_symbols(&document, &["answer"])
            .unwrap(),
        BTreeMap::from([("answer".to_owned(), RuntimeHostInputValue::F32(42.0))])
    );
    for source in [
        "(a, b) := 1f32\n",
        "(a, b, c) := (1f32, 2f32)\n",
        "(a, a) := (1f32, 2f32)\n",
        "a := 1f32\n(a, b) := (2f32, 3f32)\n",
    ] {
        assert!(
            compiler
                .compile_document(&canonical_planning_test_document(source))
                .is_err(),
            "{source}"
        );
    }
}

#[cfg(feature = "compute")]
#[test]
fn canonical_mixed_source_retains_tuple_destructured_results() {
    let source = "@compute := compute://worker/kernel{:write(input/x), :write(turn)}\n@compute/input/x <- 2f32\n@compute/turn <- 1\n\ncalculation @compute\n-------------------------------------------------------------------------------\npair(value<f32>) = (left<f32>, right<f32>) :=\n  left := value; right := value + 2f32.\n\nx := 20f32\n(a, b) := pair(x)\nresult := a + b\nresult\n";
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let mixed = compiler.compile_mixed_source(source).unwrap();
    assert!(mixed.compute.interface.input_named("x").is_some());
    assert!(
        mixed
            .compute
            .artifact
            .nodes()
            .iter()
            .all(|node| node.as_operation().is_some())
    );
}

#[cfg(feature = "compute")]
#[test]
fn canonical_mixed_shipped_ekf_region_compiles() {
    let shipped = include_str!("../../../../../examples/ekf/localization.mec");
    let start = shipped.find("5. ekf-batch @compute\n").unwrap();
    let end = shipped.find("6. Live Tracking Field\n").unwrap();
    let source = format!(
        "+> math/*\n@filters := compute://filters/kernel{{:write(input/control), :write(input/camera), :write(input/measurement), :write(turn)}}\n\
         @filters/input/control <- [0.05<f32>; 1f32; 0f32]\n\
         @filters/input/camera <- [1f32; 1f32]\n\
         @filters/input/measurement <- [1f32; 0f32; 0f32]\n\
         @filters/turn <- 1\n\n{}",
        &shipped[start..end]
    );
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let document = canonical_planning_test_document(&source);
    assert!(
        document.is_strictly_clean(),
        "{:#?}",
        document.snapshot().diagnostics
    );
    let mixed = compiler.compile_mixed_document(&document).unwrap();
    for input in ["control", "camera", "measurement"] {
        assert!(
            mixed.compute.interface.input_named(input).is_some(),
            "{input}"
        );
    }
    assert!(!mixed.compute.artifact.nodes().is_empty());
}

#[test]
fn canonical_document_function_imports_use_catalog_exports() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (import, call) in [
        ("+> math/*", "cos"),
        ("+> math/cos", "cos"),
        ("+> math/{sin, cos}", "cos"),
        ("+> wave := math/cos", "wave"),
    ] {
        let document =
            canonical_planning_test_document(&format!("{import}\nanswer := {call}(0f32)\n"));
        assert_eq!(
            compiler
                .evaluate_static_document_symbols(&document, &["answer"])
                .unwrap(),
            BTreeMap::from([("answer".to_owned(), RuntimeHostInputValue::F32(1.0))]),
            "{import}"
        );
    }
    for source in [
        "+> wave := math/cos\n+> wave := math/sin\nanswer := wave(0f32)\n",
        "+> math/missing-function\nanswer := 1f32\n",
    ] {
        assert!(
            compiler
                .compile_document(&canonical_planning_test_document(source))
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn canonical_constant_range_shapes_follow_the_declared_cardinality() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (range, count, last) in [("2..=3", 2, 3.0), ("1..2..=8", 4, 7.0), ("1..8", 7, 7.0)] {
        let source = format!(
            "last(values<[f64]:1,{count}>) = result<f64> :=\n  result := values[{count}].\n\nanswer := last({range})\n"
        );
        let document = canonical_planning_test_document(&source);
        assert_eq!(
            compiler
                .evaluate_static_document_symbols(&document, &["answer"])
                .unwrap(),
            BTreeMap::from([("answer".to_owned(), RuntimeHostInputValue::F64(last))]),
            "{range}"
        );
    }
}

#[test]
fn canonical_dimensionless_matrix_annotations_preserve_inferred_shapes() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (values, expected) in [
        ("[1.0 2.0; 3.0 4.0]", 5.0),
        ("[1.0 2.0 3.0 4.0]", 5.0),
        ("[1.0; 2.0; 3.0; 7.0]", 8.0),
        ("[1f32 2f32 3f32 4f32]", 5.0),
    ] {
        for source in [
            format!(
                "values<[f64]> := {values}\nshifted := values + 1.0\nresult := shifted[4]\nresult\n"
            ),
            format!(
                "last(values<[f64]>) = result<f64> := result := values[4] + 1.0.\n\nresult := last({values})\n"
            ),
        ] {
            let document = canonical_planning_test_document(&source);
            assert_eq!(
                compiler
                    .evaluate_static_document_symbols(&document, &["result"])
                    .unwrap(),
                BTreeMap::from([("result".to_owned(), RuntimeHostInputValue::F64(expected))]),
                "{source}"
            );
        }
    }
    let document = canonical_planning_test_document(
        "pair<([f64],[f64])> := ([1.0 2.0], [3.0;4.0;5.0])\n(left, right) := pair\nresult := left[2] + right[3]\n",
    );
    assert_eq!(
        compiler
            .evaluate_static_document_symbols(&document, &["result"])
            .unwrap(),
        BTreeMap::from([("result".to_owned(), RuntimeHostInputValue::F64(7.0))])
    );
    let document =
        canonical_planning_test_document("matrix := values<[f64]>\nresult := matrix[4]\n");
    for (rows, columns) in [(1, 4), (2, 2), (4, 1)] {
        let supplied = BTreeMap::from([(
            "values".to_owned(),
            RuntimeHostInputValue::F64Matrix {
                rows,
                columns,
                values: vec![1.0, 2.0, 3.0, 8.0],
            },
        )]);
        assert_eq!(
            compiler
                .evaluate_static_document_symbols_with_inputs(&document, &supplied, &["result"])
                .unwrap(),
            BTreeMap::from([("result".to_owned(), RuntimeHostInputValue::F64(8.0))])
        );
    }
    for input in [
        RuntimeHostInputValue::F64(8.0),
        RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 2,
            values: vec![1.0, 2.0, 3.0, 8.0],
        },
    ] {
        let supplied = BTreeMap::from([("values".to_owned(), input)]);
        let error = compiler
            .evaluate_static_document_symbols_with_inputs(&document, &supplied, &["result"])
            .unwrap_err();
        let error = format!("{error:?}");
        assert!(
            error.contains("source-semantics/incompatible-annotation-shape")
                || error.contains("source-semantics/conflicting-input-kind"),
            "{error}"
        );
    }
}

#[test]
fn canonical_resource_planning_closes_provider_matrix_shapes() {
    for (rows, columns) in [(1, 2), (2, 1), (2, 3)] {
        let values = (1..=rows * columns)
            .map(|value| value as f32)
            .collect::<Vec<_>>();
        let supplied = RuntimeHostInputValue::F32Matrix {
            rows,
            columns,
            values,
        };
        let planned = supplied.clone().into_value().unwrap();
        assert!(!planned.shape().parameter_values().is_empty());
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .resource_provider(Box::new(TypedObservationProvider { planned }))
            .build_compiler()
            .unwrap();
        let document = canonical_planning_test_document(
            "@provider := test://typed/value{:read(matrix)}\nanswer := @provider/matrix\n",
        );
        compiler.compile_document(&document).unwrap();
        assert_eq!(
            compiler
                .evaluate_static_document_symbols(&document, &["answer"])
                .unwrap(),
            BTreeMap::from([("answer".to_owned(), supplied)]),
        );
    }
}

#[cfg(feature = "compute")]
#[test]
fn canonical_mixed_shipped_particle_region_initializes() {
    let shipped = include_str!("../../../../../examples/gpu-particles/particles.mec");
    let start = shipped.find("particle-field @compute\n").unwrap();
    for count in [5, 257, 16384, 1_000_000] {
        let source = format!(
            "+> math\n@particles := compute://particles/kernel{{:write(input/force-point), :write(input/force-strength), :write(input/dt), :write(turn)}}\n@particles/input/force-point <- [0f32; 0f32]\n@particles/input/force-strength <- 0f32\n@particles/input/dt <- 0.016666667<f32>\n@particles/turn <- 1\n\n{}",
            shipped[start..].replace(
                "particle-count := 1000000f32",
                &format!("particle-count := {count}f32")
            ),
        );
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .build_compiler()
            .unwrap();
        let document = canonical_planning_test_document(&source);
        let mixed = compiler
            .compile_mixed_document(&document)
            .unwrap_or_else(|error| panic!("{count} particles: {error:?}"));
        assert!(mixed.compute.interface.input_named("force-point").is_some());
    }
}

#[test]
fn canonical_static_projection_preserves_independent_integrity_constraints() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for (limit, valid) in [(50, true), (20, false)] {
        let document = canonical_planning_test_document(&format!(
            "answer := 40 + 2\nunrelated := 10 + 20\nsafe! := unrelated <= {limit}\n",
        ));
        let result = compiler.evaluate_static_document_symbols(&document, &["answer"]);
        assert_eq!(result.is_ok(), valid, "limit {limit}: {result:?}");
    }
}

#[test]
fn canonical_static_projection_drops_unrelated_unbound_inputs() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    for expression in ["42.0", "used<f64>", "used<f64> + 2.0"] {
        let document = canonical_planning_test_document(&format!(
            "other := unused<f64> + 1.0\nanswer := {expression}\n"
        ));
        let inputs = if expression == "42.0" {
            BTreeMap::new()
        } else {
            BTreeMap::from([("used".to_owned(), RuntimeHostInputValue::F64(40.0))])
        };
        let result = compiler
            .evaluate_static_document_symbols_with_inputs(&document, &inputs, &["answer"])
            .unwrap();
        let expected = if expression == "used<f64>" {
            40.0
        } else {
            42.0
        };
        assert_eq!(
            result,
            BTreeMap::from([("answer".to_owned(), RuntimeHostInputValue::F64(expected))])
        );
    }
    let document =
        canonical_planning_test_document("answer := 42.0\nsafe! := checked<f64> > 0.0\n");
    assert!(
        compiler
            .evaluate_static_document_symbols(&document, &["answer"])
            .is_err()
    );
}

#[test]
fn canonical_uncalled_functions_do_not_bind_resource_inputs() {
    for (tail, observed) in [("answer := 42.0", false), ("answer := sample()", true)] {
        let source = format!(
            "@clock := timer://clock/tick{{:read(tick)}}\nsample() = result<f64> :=\n  result := (@clock/tick).\n\n{tail}\n"
        );
        let document = canonical_planning_test_document(&source);
        let mut resolver = InMemorySourceResolver::new();
        resolver.insert_string("main.mec", source).unwrap();
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .resource_provider(Box::new(ProductTimerProvider))
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        for product in [
            compiler.compile_document(&document).unwrap(),
            compiler
                .compile_canonical_root(SourceRequest::new("main.mec"))
                .unwrap(),
        ] {
            let has_resource = product.artifact().requirements().iter().any(|(_, requirement)| matches!(requirement,
                mech_core::ApplicationRequirement::Resource(request) if request.base_uri == "timer://clock/tick"));
            assert_eq!(has_resource, observed);
        }
    }
}

#[test]
fn canonical_resolved_and_rooted_interactive_compilation_preserve_revision_and_symbols() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_canonical_string("dep.mec", "value := 41.0\n<+ value\n")
        .unwrap();
    resolver
        .insert_canonical_string(
            "main.mec",
            "+> ./dep.mec\nanswer := dep/value + 1.0\nanswer\n",
        )
        .unwrap();
    let resolved = crate::SourceResolver::resolve(&resolver, &SourceRequest::new("main.mec"))
        .unwrap()
        .unwrap();
    // Resolving the supplied root again would compile this newer revision.
    resolver
        .insert_canonical_string(
            "main.mec",
            "+> ./dep.mec\nanswer := dep/value + 59.0\nanswer\n",
        )
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let options = ModuleBuildOptions::new(
        "qualified-compiler",
        "v0.4",
        "browser",
        &["source"],
        &["resource://contract"],
    );
    let products = [
        (
            compiler
                .compile_canonical_resolved_root(resolved.clone())
                .unwrap(),
            42.0,
            false,
        ),
        (
            compiler
                .compile_canonical_interactive_resolved_root(resolved.clone())
                .unwrap(),
            42.0,
            true,
        ),
        (
            compiler
                .compile_canonical_root(SourceRequest::new("main.mec"))
                .unwrap(),
            100.0,
            false,
        ),
        (
            compiler
                .compile_canonical_interactive_root(SourceRequest::new("main.mec"))
                .unwrap(),
            100.0,
            true,
        ),
        (
            compiler
                .compile_canonical_resolved_root_with_options(resolved.clone(), options)
                .unwrap(),
            42.0,
            false,
        ),
        (
            compiler
                .compile_canonical_interactive_resolved_root_with_options(resolved, options)
                .unwrap(),
            42.0,
            true,
        ),
        (
            compiler
                .compile_canonical_root_with_options(SourceRequest::new("main.mec"), options)
                .unwrap(),
            100.0,
            false,
        ),
        (
            compiler
                .compile_canonical_interactive_root_with_options(
                    SourceRequest::new("main.mec"),
                    options,
                )
                .unwrap(),
            100.0,
            true,
        ),
    ];
    for (product, expected, interactive) in products {
        assert_eq!(
            product.source_dependencies(),
            &BTreeMap::from([(
                "memory:dep.mec".into(),
                mech_core::hash_str("value := 41.0\n<+ value\n")
            ),])
        );
        assert_eq!(
            product.artifact().outputs().iter().any(|output| {
                mech_engine::decode_interactive_symbol_output_name(&output.name).as_deref()
                    == Some("answer")
            }),
            interactive
        );
        let mut accepted = runtime();
        accepted
            .load_bytecode_program(
                product.bytecode(),
                crate::ResidentDurabilityPolicy::Volatile,
            )
            .unwrap();
        let result = accepted
            .output_value(mech_core::OutputId::new(0))
            .unwrap()
            .unwrap();
        assert_eq!(canonical_f64(result.value()), expected);
        if interactive {
            let id = accepted.root_symbol_output_id("answer").unwrap();
            assert_eq!(
                canonical_f64(accepted.output_value(id).unwrap().unwrap().value()),
                expected
            );
        }
    }
}

#[test]
fn canonical_interactive_root_keeps_resource_authority_and_dependency_errors() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_canonical_string(
            "main.mec",
            "@clock := timer://clock/tick{:read(tick)}\nanswer := @clock/tick\n",
        )
        .unwrap();
    resolver
        .insert_canonical_string("broken.mec", "+> ./absent.mec\nanswer := absent/value\n")
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(ProductTimerProvider))
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_interactive_root(SourceRequest::new("main.mec"))
        .unwrap();
    assert!(product.artifact().requirements().iter().any(|(_, requirement)| matches!(requirement,
        mech_core::ApplicationRequirement::Resource(request) if request.base_uri == "timer://clock/tick")));
    assert!(product.artifact().outputs().iter().any(|output| {
        mech_engine::decode_interactive_symbol_output_name(&output.name).as_deref()
            == Some("answer")
    }));
    let error = compiler
        .compile_canonical_interactive_root(SourceRequest::new("broken.mec"))
        .unwrap_err();
    let missing = error
        .kind_as::<crate::RuntimeModuleDependencyMissingError>()
        .expect("canonical imports retain the public typed missing-dependency error");
    assert_eq!(missing.module, "memory:broken.mec");
    assert_eq!(missing.specifier, "./absent.mec");
    assert_eq!(missing.referrer.as_deref(), Some("memory:broken.mec"));
    // A rejected source graph must not poison the reusable compiler.
    assert!(
        compiler
            .compile_canonical_interactive_root(SourceRequest::new("main.mec"))
            .is_ok()
    );
}

#[test]
fn canonical_interactive_uses_configured_resource_planning() {
    let document = canonical_planning_test_document(
        "@clock := timer://clock/tick{:read(tick)}\nanswer := @clock/tick\n",
    );
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(ProductTimerProvider))
        .build_compiler()
        .unwrap();
    for product in [
        compiler.compile_document(&document).unwrap(),
        compiler.compile_interactive_document(&document).unwrap(),
    ] {
        assert!(product.artifact().requirements().iter().any(|(_, requirement)| matches!(requirement,
            mech_core::ApplicationRequirement::Resource(request) if request.base_uri == "timer://clock/tick")));
    }
    let interactive = compiler.compile_interactive_document(&document).unwrap();
    assert!(interactive.artifact().outputs().iter().any(|output| {
        mech_engine::decode_interactive_symbol_output_name(&output.name).as_deref()
            == Some("answer")
    }));
    // Admission happens in a fresh candidate runtime; a denied candidate must
    // leave the accepted interactive runtime and its state intact.
    struct NoTimerGrantFactory;
    impl crate::ResidentReplRuntimeFactory for NoTimerGrantFactory {
        fn build(&self, _: crate::MechEventBuffer) -> MResult<crate::MechRuntime> {
            let mut runtime = runtime();
            runtime.register_resource_provider(Box::new(ProductTimerProvider))?;
            Ok(runtime)
        }
        fn activate_document(
            &self,
            events: crate::MechEventBuffer,
            document: &crate::SourceDocument,
        ) -> MResult<(crate::MechRuntime, crate::RuntimeProgramLoadOutcome)> {
            let mut runtime = self.build(events)?;
            let mut compiler = RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_native_plan_catalog())
                .resource_provider(Box::new(ProductTimerProvider))
                .build_compiler()?;
            let product = compiler.compile_interactive_document(document)?;
            let outcome = runtime.load_bytecode_program(
                product.bytecode(),
                crate::ResidentDurabilityPolicy::Volatile,
            )?;
            Ok((runtime, outcome))
        }
    }
    let accepted = canonical_planning_test_document("~counter := 0\ncounter += 1\ncounter\n");
    let mut session =
        crate::ResidentReplSession::from_document(NoTimerGrantFactory, accepted).unwrap();
    session.step(2).unwrap();
    let source = session.source().to_owned();
    let value = session.symbol("counter").unwrap();
    let error = session.replace_document(document).unwrap_err();
    assert!(
        error.kind_message().starts_with("AuthorizationDenied:"),
        "{error:?}"
    );
    assert_eq!(session.source(), source);
    assert_eq!(session.symbol("counter").unwrap(), value);
}

#[test]
fn canonical_product_closed_control_lifecycle_survives_rejection_and_reset() {
    struct ConstantDocumentFactory {
        supplied: BTreeMap<String, RuntimeHostInputValue>,
    }

    impl crate::ResidentReplRuntimeFactory for ConstantDocumentFactory {
        fn build(&self, _: crate::MechEventBuffer) -> MResult<crate::MechRuntime> {
            Ok(runtime())
        }

        fn activate_document(
            &self,
            events: crate::MechEventBuffer,
            document: &crate::SourceDocument,
        ) -> MResult<(crate::MechRuntime, crate::RuntimeProgramLoadOutcome)> {
            let mut runtime = self.build(events)?;
            let mut compiler = RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_native_plan_catalog())
                .build_compiler()?;
            let product = compiler.compile_document_artifact_with_inputs(
                document,
                &self.supplied,
                &BTreeSet::new(),
            )?;
            let outcome = runtime.load_compiled_program(
                product.into_artifact(),
                crate::ResidentDurabilityPolicy::Volatile,
            )?;
            Ok((runtime, outcome))
        }
    }

    let supplied = BTreeMap::from([(
        "seed".to_owned(),
        RuntimeHostInputValue::F64Matrix {
            rows: 1,
            columns: 3,
            values: vec![1.0, 2.0, 3.0],
        },
    )]);
    let accepted = canonical_planning_test_document(
        "values := [x | x <- seed]\n~state := values\nstate += [1 1 1]\n<+ state\nstate\n",
    );
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_document_artifact_with_inputs(&accepted, &supplied, &BTreeSet::new())
        .unwrap();
    assert!(
        product.artifact().inputs().is_empty(),
        "the detached host seed must be compiled as a constant"
    );
    assert!(
        product
            .artifact()
            .nodes()
            .iter()
            .any(|node| { matches!(node.body, mech_engine::ExecutableNodeBody::Comprehension(_)) })
    );

    let mut session = crate::ResidentReplSession::from_document(
        ConstantDocumentFactory {
            supplied: supplied.clone(),
        },
        accepted,
    )
    .unwrap();
    let state = |session: &crate::ResidentReplSession<ConstantDocumentFactory>| {
        session.symbol("state").unwrap().unwrap()
    };
    let first = state(&session);
    assert_eq!(canonical_matrix_shape(first.value()), (1, 3));
    assert_eq!(canonical_f64_matrix(first.value()), [2.0, 3.0, 4.0]);
    session.step(1).unwrap();
    assert_eq!(
        canonical_f64_matrix(state(&session).value()),
        [3.0, 4.0, 5.0]
    );

    let rejected = canonical_planning_test_document(
        "values := [x | x <- seed]\nbad := values + [1 1]\n~state := bad\n<+ state\nstate\n",
    );
    assert!(session.replace_document(rejected).is_err());
    assert_eq!(
        canonical_f64_matrix(state(&session).value()),
        [3.0, 4.0, 5.0]
    );
    session.step(1).unwrap();
    assert_eq!(
        canonical_f64_matrix(state(&session).value()),
        [4.0, 5.0, 6.0]
    );

    session.reset().unwrap();
    let reset = state(&session);
    assert_eq!(canonical_matrix_shape(reset.value()), (1, 3));
    assert_eq!(canonical_f64_matrix(reset.value()), [2.0, 3.0, 4.0]);
    session.step(1).unwrap();
    assert_eq!(
        canonical_f64_matrix(state(&session).value()),
        [3.0, 4.0, 5.0]
    );
}

#[test]
fn canonical_interactive_resource_planning_does_not_execute_host_effects() {
    let plans = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let trace = Arc::new(Mutex::new(ProductSceneTrace::default()));
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .resource_provider(Box::new(PlanningObservationProvider {
            plans: plans.clone(),
            reads: reads.clone(),
            value_bits: Arc::new(AtomicU64::new(0.25_f64.to_bits())),
        }))
        .resource_provider(Box::new(ProductSceneProvider {
            trace: trace.clone(),
            contract: ProductSceneContract::AtMostOnce,
            prepare_delay: Duration::ZERO,
        }))
        .build_compiler()
        .unwrap();
    let source = "@clock := test://clock/tick{:read(delta-seconds)}\n@scene := scene://orbit/frame{:write(points)}\nanswer := @clock/delta-seconds\n@scene/points <- [answer; answer]\n";
    let document = canonical_planning_test_document(source);
    for product in [
        compiler.compile_document(&document).unwrap(),
        compiler.compile_interactive_document(&document).unwrap(),
    ] {
        let requests = product
            .artifact()
            .requirements()
            .iter()
            .filter_map(|(_, requirement)| match requirement {
                mech_core::ApplicationRequirement::Resource(request) => Some(request),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            requests
                .iter()
                .any(|request| request.base_uri == "test://clock/tick")
        );
        assert!(
            requests
                .iter()
                .any(|request| request.base_uri == "scene://orbit/frame")
        );
    }
    let product = compiler.compile_interactive_document(&document).unwrap();
    let (mut denied, _, _, _) = configured_external_runtime();
    denied
        .register_resource_provider(Box::new(ProductSceneProvider {
            trace: trace.clone(),
            contract: ProductSceneContract::AtMostOnce,
            prepare_delay: Duration::ZERO,
        }))
        .unwrap();
    // Clock read is granted, scene writes are not. No effect may be prepared.
    let error = denied
        .load_bytecode_program(
            product.bytecode(),
            crate::ResidentDurabilityPolicy::Volatile,
        )
        .unwrap_err();
    assert!(
        error.kind_message().starts_with("AuthorizationDenied:"),
        "{error:?}"
    );
    assert!(plans.load(Ordering::SeqCst) > 0);
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    let trace = trace.lock().unwrap();
    assert_eq!(trace.preparations, 0);
    assert_eq!(trace.deliveries, 0);
}

#[test]
fn canonical_planned_selectors_enforce_portable_index_width_before_activation() {
    let source = "@typed := test://typed/value{:read(data)}\nvalues := [10.0 20.0 30.0]\nselected := values[1,@typed/data]\nselected\n";
    for (planned, accepted) in [(1u64, true), (u32::MAX as u64 + 1, false)] {
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .resource_provider(Box::new(TypedObservationProvider {
                planned: ValueCell::from_exact(planned).unwrap().snapshot().unwrap(),
            }))
            .build_compiler()
            .unwrap();
        assert_eq!(compiler.compile_canonical_source(source).is_ok(), accepted);
    }
}

#[test]
fn canonical_missing_provider_keeps_the_public_route_failure_class() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let error = compiler.compile_canonical_source(
        "@clock := missing://clock/tick{:read(delta-seconds)}\ndelta := @clock/delta-seconds\ndelta\n",
    ).unwrap_err();
    assert_eq!(
        error.kind_as::<ResidentRouteFailure>().unwrap().class,
        ResidentRouteFailureClass::ProviderUnavailable
    );
}

#[test]
fn canonical_ordered_roots_share_prior_definitions_and_reject_invalid_edges() {
    let catalog = mech_stdlib::source_catalog();
    let mut resolver = InMemorySourceResolver::new();
    resolver.insert_string("first.mec", "seed := 41\n").unwrap();
    resolver
        .insert_string("second.mec", "answer := seed + 1\n")
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_roots(
            &[
                SourceRequest::new("first.mec"),
                SourceRequest::new("second.mec"),
            ],
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();
    assert_eq!(
        product
            .artifact()
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        ["seed", "answer"]
    );
    let decoded = decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
    for artifact in [product.artifact(), &decoded] {
        let mut instance = mech_engine::resident::activate(
            mech_core::ReactiveInstanceId::new(0x830, 0),
            artifact,
            &catalog,
            &mech_engine::resident::ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert_eq!(canonical_f64(&instance.copied_output(0).unwrap()), 41.0);
        assert_eq!(canonical_f64(&instance.copied_output(1).unwrap()), 42.0);
    }
    for (first, second) in [
        (
            "+> ./second.mec\na := second/missing\na\n",
            "value := 1\n<+ value\nvalue\n",
        ),
        (
            "+> ./second.mec\na := 1\n<+ a\na\n",
            "+> ./first.mec\nb := 2\n<+ b\nb\n",
        ),
    ] {
        let mut resolver = InMemorySourceResolver::new();
        resolver.insert_string("first.mec", first).unwrap();
        resolver.insert_string("second.mec", second).unwrap();
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(Arc::clone(&catalog))
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        assert!(
            compiler
                .compile_canonical_roots(
                    &[
                        SourceRequest::new("first.mec"),
                        SourceRequest::new("second.mec")
                    ],
                    ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
                )
                .is_err()
        );
    }
}

#[test]
fn canonical_fizzbuzz_preserves_constraints_and_presentation_through_bytecode() {
    let source = include_str!("../../../../../examples/working/fizzbuzz.mec");
    let document = canonical_planning_test_document(source);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    for interactive in [false, true] {
        let product = if interactive {
            compiler.compile_interactive_document(&document)
        } else {
            compiler.compile_document(&document)
        }
        .unwrap();
        assert_eq!(product.artifact().constraints().len(), 4);
        let bytes = product.bytecode().to_vec();
        let mut from_source = runtime();
        let source_loaded = from_source
            .load_compiled_program(
                product.artifact().clone(),
                crate::ResidentDurabilityPolicy::Volatile,
            )
            .unwrap();
        let mut from_bytecode = runtime();
        let bytecode_loaded = from_bytecode
            .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Volatile)
            .unwrap();
        assert_eq!(source_loaded.route, RuntimeProgramRoute::ResidentPure);
        assert_eq!(
            source_loaded.info.program_revision,
            bytecode_loaded.info.program_revision
        );
        for runtime in [&from_source, &from_bytecode] {
            let output = runtime.program_output_id().unwrap();
            assert_eq!(runtime.output_name(output).as_deref(), Some("result"));
        }
        let actual = source_loaded.initial_value.format_canonical_inline();
        assert!(actual.contains("✨🐝"), "{actual}");
        assert_eq!(
            actual,
            bytecode_loaded.initial_value.format_canonical_inline()
        );
    }
}
