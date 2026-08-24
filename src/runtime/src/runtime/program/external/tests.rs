use std::sync::{Arc, LazyLock, Mutex};

use mech_core::{
    AccessMode, ApplicationRequirement, ApplicationRequirementId, BindingId, CellSlotId,
    ChangeDetectionPolicy, DeclaredOperationContract, DeliveryMode, EffectContract,
    EffectDeliveryPolicy, ExecutionHostFunctionRequest, ExecutionResourceRequest,
    ExternalInteraction, IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue,
    MResult, MechError, MechExecutionServices, NodeId, ObservationContract,
    ObservationReplayPolicy, OperationContractDeclaration, OperationContractTableBuilder,
    OutputConstruction, OutputPortPolicy, ParsedProgram, ReactiveInstanceId, ResolvedInputPort,
    ResolvedOperationContract, ResourceDelivery, ResourceIntent, ShapeRule, ToMatrix,
    TransactionalEffectProtocol, TransactionalExternalContract, ValRef,
    snapshot::{SequenceView, ValueData},
};
use mech_engine::{
    __resident::{
        ActivationFacts, FrozenEkfCompilationServices, ReactiveInstance, ResidentIntegrityMode,
        ResidentValueBorrow, activate_external, compile_frozen_ekf_source,
        frozen_ekf_compiler_catalog,
    },
    ApplicationRequirementTable, ArtifactSource, BindingDeclaration, NodeDeclaration,
    OperationReference, ProgramArtifact, ProgramArtifactDraft, decode_program_artifact_sections,
};

use crate::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeBuilder, RuntimeCompensatableEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeResidentResourceWriteRequest, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceRegistry, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
    RuntimeTransactionalEffect, TransactionId, config::ResidentDurabilityPolicy,
};

use super::*;

const SOURCE: &str =
    include_str!("../../../../../../tests/architecture/resident-activation/ekf-source-v1.mec");

static EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
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

static AT_MOST_ONCE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    external_write_contract(ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::AtMostOnce,
        idempotency: IdempotencyRequirement::NotRequired,
    }))
});

static AT_LEAST_ONCE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    external_write_contract(ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::AtLeastOnce,
        idempotency: IdempotencyRequirement::Optional,
    }))
});

static OBSERVATION_CONTRACT: LazyLock<OperationContractDeclaration> =
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

static TRANSACTIONAL_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    external_write_contract(ExternalInteraction::TransactionalExternal(
        TransactionalExternalContract {
            protocol: TransactionalEffectProtocol::PrepareCommit,
        },
    ))
});

static COMPENSATABLE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    external_write_contract(ExternalInteraction::TransactionalExternal(
        TransactionalExternalContract {
            protocol: TransactionalEffectProtocol::PrepareCommitCompensate,
        },
    ))
});

fn external_write_contract(interaction: ExternalInteraction) -> OperationContractDeclaration {
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

#[derive(Clone, Debug, Default)]
struct ProviderTrace {
    reads: usize,
    prepared: Vec<(crate::RuntimeEffectId, String)>,
    prepared_f64: Vec<f64>,
    delivered: usize,
    fail_read: bool,
    fail_read_at: Option<usize>,
    read_failure_message: Option<String>,
    observation_shape: Option<(usize, usize)>,
    fail_prepare: bool,
    fail_prepare_at: Option<usize>,
    delivery_failures_remaining: usize,
    transactional_prepare_fail: bool,
    transactional_commit_fail: bool,
    transactional_abort_fail: bool,
    compensatable_apply_fail: bool,
    compensatable_apply_fail_at: Option<usize>,
    compensatable_apply_count: usize,
    compensatable_compensate_fail: bool,
    lifecycle: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProviderProtocol {
    AfterCommit,
    AfterCommitNoIdempotency,
    AfterCommitAtMostOnce,
    AfterCommitAtMostOnceDistinct,
    AfterCommitAtMostOnceThenWrong,
    AfterCommitAtLeastOnce,
    WrongTransactional,
    Transactional,
    Compensatable,
    CompensatableDistinct,
}

#[derive(Debug)]
struct ObservationProvider {
    trace: Arc<Mutex<ProviderTrace>>,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct SourceInputProvider;

#[cfg(feature = "semantic-compiler")]
impl RuntimeResourceProvider for SourceInputProvider {
    fn scheme(&self) -> &str {
        "gate-d3"
    }
    fn base_uris(&self) -> Vec<String> {
        vec!["gate-d3://input/value".to_owned()]
    }
    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&OBSERVATION_CONTRACT)
    }
    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(mech_core::Ref::new(0.25)))
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.read(request)
    }
}

impl RuntimeResourceProvider for ObservationProvider {
    fn scheme(&self) -> &str {
        "gate-d"
    }
    fn base_uris(&self) -> Vec<String> {
        vec!["gate-d://ekf/frame".to_owned()]
    }
    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&OBSERVATION_CONTRACT)
    }
    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        let mut trace = self.trace.lock().unwrap();
        trace.reads += 1;
        if trace.fail_read || trace.fail_read_at == Some(trace.reads) {
            return Err(test_error(
                trace
                    .read_failure_message
                    .as_deref()
                    .unwrap_or("injected observation failure"),
            ));
        }
        let shape = trace.observation_shape.unwrap_or((4, 1));
        drop(trace);
        let values = vec![0.2, 0.01, 5.0, -2.0];
        Ok(LegacyValue::MatrixF64(ToMatrix::to_matrix(
            values, shape.0, shape.1,
        )))
    }
}

#[derive(Debug)]
struct EffectProvider {
    trace: Arc<Mutex<ProviderTrace>>,
    protocol: ProviderProtocol,
}

impl RuntimeResourceProvider for EffectProvider {
    fn scheme(&self) -> &str {
        "gate-d3"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![
            match self.protocol {
                ProviderProtocol::AfterCommit
                | ProviderProtocol::AfterCommitNoIdempotency
                | ProviderProtocol::AfterCommitAtMostOnce
                | ProviderProtocol::AfterCommitAtLeastOnce
                | ProviderProtocol::WrongTransactional => "gate-d3://scene/output",
                ProviderProtocol::AfterCommitAtMostOnceDistinct
                | ProviderProtocol::AfterCommitAtMostOnceThenWrong => "gate-d3://zz-once/output",
                ProviderProtocol::Transactional | ProviderProtocol::Compensatable => {
                    "gate-d3://transactional/state"
                }
                ProviderProtocol::CompensatableDistinct => match self.protocol {
                    ProviderProtocol::CompensatableDistinct => "gate-d3://compensatable/state",
                    _ => unreachable!(),
                },
            }
            .to_owned(),
        ]
    }
    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        None
    }
    fn semantic_write_contract(
        &self,
        _intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        match self.protocol {
            ProviderProtocol::AfterCommit | ProviderProtocol::WrongTransactional => {
                Some(&EFFECT_CONTRACT)
            }
            ProviderProtocol::AfterCommitNoIdempotency => Some(&EFFECT_CONTRACT),
            ProviderProtocol::AfterCommitAtMostOnce
            | ProviderProtocol::AfterCommitAtMostOnceDistinct
            | ProviderProtocol::AfterCommitAtMostOnceThenWrong => Some(&AT_MOST_ONCE_CONTRACT),
            ProviderProtocol::AfterCommitAtLeastOnce => Some(&AT_LEAST_ONCE_CONTRACT),
            ProviderProtocol::Transactional => Some(&TRANSACTIONAL_CONTRACT),
            ProviderProtocol::Compensatable | ProviderProtocol::CompensatableDistinct => {
                Some(&COMPENSATABLE_CONTRACT)
            }
        }
    }
    fn supports_resident_idempotency(&self, _intent: RuntimeResourceWriteIntent) -> bool {
        self.protocol != ProviderProtocol::AfterCommitNoIdempotency
    }
    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(MechError::new(
            mech_core::GenericError {
                msg: "write-only".to_owned(),
            },
            None,
        ))
    }
    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        if !self.base_uris().contains(&request.base_uri) || request.path.is_empty() {
            return Err(test_error(
                "D3 fixture write is outside its declared target",
            ));
        }
        Ok(())
    }
    fn prepare_resident_write(
        &self,
        request: RuntimeResidentResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        let mut trace = self.trace.lock().unwrap();
        if let LegacyValue::F64(value) = &request.value {
            trace.prepared_f64.push(*value.borrow());
        }
        trace
            .prepared
            .push((request.effect_id, request.idempotency_key));
        if trace.fail_prepare || trace.fail_prepare_at == Some(trace.prepared.len()) {
            return Err(test_error("injected provider preparation failure"));
        }
        let preparation_count = trace.prepared.len();
        drop(trace);
        Ok(match self.protocol {
            ProviderProtocol::AfterCommit
            | ProviderProtocol::AfterCommitNoIdempotency
            | ProviderProtocol::AfterCommitAtMostOnce
            | ProviderProtocol::AfterCommitAtMostOnceDistinct
            | ProviderProtocol::AfterCommitAtLeastOnce => {
                PreparedRuntimeEffect::AfterCommit(Box::new(SceneDelivery {
                    trace: self.trace.clone(),
                }))
            }
            ProviderProtocol::AfterCommitAtMostOnceThenWrong => {
                if preparation_count == 4 {
                    PreparedRuntimeEffect::Transactional(Box::new(TestTransactional {
                        trace: self.trace.clone(),
                    }))
                } else {
                    PreparedRuntimeEffect::AfterCommit(Box::new(SceneDelivery {
                        trace: self.trace.clone(),
                    }))
                }
            }
            ProviderProtocol::WrongTransactional | ProviderProtocol::Transactional => {
                PreparedRuntimeEffect::Transactional(Box::new(TestTransactional {
                    trace: self.trace.clone(),
                }))
            }
            ProviderProtocol::Compensatable | ProviderProtocol::CompensatableDistinct => {
                PreparedRuntimeEffect::Compensatable(Box::new(TestCompensatable {
                    trace: self.trace.clone(),
                }))
            }
        })
    }
}

#[derive(Debug)]
struct SceneDelivery {
    trace: Arc<Mutex<ProviderTrace>>,
}

impl RuntimeAfterCommitEffect for SceneDelivery {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "gate-d3".to_owned(),
            },
            "write",
        )
        .with_cost(RuntimeEffectCost {
            bytes: 96,
            items: 1,
        })
    }
    fn deliver(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.delivered += 1;
        trace.lifecycle.push("deliver");
        if trace.delivery_failures_remaining > 0 {
            trace.delivery_failures_remaining -= 1;
            return Err(test_error("injected delivery failure"));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TestTransactional {
    trace: Arc<Mutex<ProviderTrace>>,
}

impl RuntimeTransactionalEffect for TestTransactional {
    fn metadata(&self) -> RuntimeEffectMetadata {
        test_metadata()
    }
    fn prepare(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.lifecycle.push("prepare");
        if trace.transactional_prepare_fail {
            return Err(test_error("injected transactional prepare failure"));
        }
        Ok(())
    }
    fn commit(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.lifecycle.push("commit");
        if trace.transactional_commit_fail {
            return Err(test_error("injected transactional commit failure"));
        }
        Ok(())
    }
    fn abort(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.lifecycle.push("abort");
        if trace.transactional_abort_fail {
            return Err(test_error("injected transactional abort failure"));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TestCompensatable {
    trace: Arc<Mutex<ProviderTrace>>,
}

impl RuntimeCompensatableEffect for TestCompensatable {
    fn metadata(&self) -> RuntimeEffectMetadata {
        test_metadata()
    }
    fn apply(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.lifecycle.push("apply");
        trace.compensatable_apply_count += 1;
        if trace.compensatable_apply_fail
            || trace.compensatable_apply_fail_at == Some(trace.compensatable_apply_count)
        {
            return Err(test_error("injected compensatable apply failure"));
        }
        Ok(())
    }
    fn compensate(&mut self) -> MResult<()> {
        let mut trace = self.trace.lock().unwrap();
        trace.lifecycle.push("compensate");
        if trace.compensatable_compensate_fail {
            return Err(test_error("injected compensation failure"));
        }
        Ok(())
    }
    fn abort(&mut self) -> MResult<()> {
        self.trace.lock().unwrap().lifecycle.push("abort");
        Ok(())
    }
}

fn test_metadata() -> RuntimeEffectMetadata {
    RuntimeEffectMetadata::new(
        RuntimeEffectSource::ResourceProvider {
            scheme: "gate-d3".to_owned(),
        },
        "write",
    )
}

fn test_error(message: &str) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.to_owned(),
        },
        None,
    )
}

fn artifact_with_effect(artifact: &ProgramArtifact, protocol: ProviderProtocol) -> ProgramArtifact {
    let requirement = ApplicationRequirementId::new(artifact.requirements().len() as u32);
    let mut builder = OperationContractTableBuilder::new();
    let handles = artifact
        .contracts()
        .iter()
        .cloned()
        .map(|contract| builder.insert(contract).unwrap())
        .collect::<Vec<_>>();
    let source = artifact.outputs()[0].source;
    let schema = artifact.slots()[source.get() as usize].schema;
    let interaction = match protocol {
        ProviderProtocol::AfterCommit | ProviderProtocol::WrongTransactional => {
            EFFECT_CONTRACT.interaction.clone()
        }
        ProviderProtocol::AfterCommitNoIdempotency => EFFECT_CONTRACT.interaction.clone(),
        ProviderProtocol::AfterCommitAtMostOnce
        | ProviderProtocol::AfterCommitAtMostOnceDistinct
        | ProviderProtocol::AfterCommitAtMostOnceThenWrong => {
            AT_MOST_ONCE_CONTRACT.interaction.clone()
        }
        ProviderProtocol::AfterCommitAtLeastOnce => AT_LEAST_ONCE_CONTRACT.interaction.clone(),
        ProviderProtocol::Transactional => TRANSACTIONAL_CONTRACT.interaction.clone(),
        ProviderProtocol::Compensatable | ProviderProtocol::CompensatableDistinct => {
            COMPENSATABLE_CONTRACT.interaction.clone()
        }
    };
    let effect = builder
        .insert(ResolvedOperationContract::Declared(
            DeclaredOperationContract {
                inputs: vec![ResolvedInputPort {
                    schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
                outputs: Box::new([]),
                interaction,
            },
        ))
        .unwrap();
    let contracts = builder.finish().unwrap();
    let mut nodes = artifact.nodes().to_vec();
    for node in &mut nodes {
        node.contract = contracts
            .resolve(handles[node.contract.get() as usize])
            .unwrap();
    }
    let mut constraints = artifact.constraints().to_vec();
    for constraint in &mut constraints {
        constraint.contract = contracts
            .resolve(handles[constraint.contract.get() as usize])
            .unwrap();
    }
    let node = NodeId::new(nodes.len() as u32);
    let mut bindings = artifact.bindings().to_vec();
    let binding = bindings.len() as u32;
    bindings.push(BindingDeclaration::Input {
        id: BindingId::new(binding),
        node,
        port_ordinal: 0,
        source: ArtifactSource::Slot(source),
    });
    nodes.push(NodeDeclaration {
        node,
        operation: OperationReference {
            module_path: vec!["resource".to_owned()].into_boxed_slice(),
            operation_name: "write".to_owned(),
        },
        contract: contracts.resolve(effect).unwrap(),
        requirement: Some(requirement),
        input_bindings: binding..binding + 1,
        output_bindings: binding + 1..binding + 1,
    });
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: contracts.table,
        requirements: ApplicationRequirementTable::from_canonical_entries(
            artifact
                .requirements()
                .iter()
                .map(|(_, requirement)| requirement.clone())
                .chain([ApplicationRequirement::Resource(ExecutionResourceRequest {
                    base_uri: match protocol {
                        ProviderProtocol::AfterCommit
                        | ProviderProtocol::AfterCommitNoIdempotency
                        | ProviderProtocol::AfterCommitAtMostOnce
                        | ProviderProtocol::AfterCommitAtLeastOnce
                        | ProviderProtocol::WrongTransactional => "gate-d3://scene/output",
                        ProviderProtocol::AfterCommitAtMostOnceDistinct
                        | ProviderProtocol::AfterCommitAtMostOnceThenWrong => {
                            "gate-d3://zz-once/output"
                        }
                        ProviderProtocol::Transactional | ProviderProtocol::Compensatable => {
                            "gate-d3://transactional/state"
                        }
                        ProviderProtocol::CompensatableDistinct => match protocol {
                            ProviderProtocol::CompensatableDistinct => {
                                "gate-d3://compensatable/state"
                            }
                            _ => unreachable!(),
                        },
                    }
                    .to_owned(),
                    path: match protocol {
                        ProviderProtocol::AfterCommit
                        | ProviderProtocol::AfterCommitNoIdempotency
                        | ProviderProtocol::AfterCommitAtMostOnce
                        | ProviderProtocol::AfterCommitAtMostOnceDistinct
                        | ProviderProtocol::AfterCommitAtMostOnceThenWrong
                        | ProviderProtocol::AfterCommitAtLeastOnce
                        | ProviderProtocol::WrongTransactional => "frame",
                        ProviderProtocol::Transactional
                        | ProviderProtocol::Compensatable
                        | ProviderProtocol::CompensatableDistinct => "value",
                    }
                    .to_owned(),
                    context_name: match protocol {
                        ProviderProtocol::AfterCommit
                        | ProviderProtocol::AfterCommitNoIdempotency
                        | ProviderProtocol::AfterCommitAtMostOnce
                        | ProviderProtocol::AfterCommitAtMostOnceDistinct
                        | ProviderProtocol::AfterCommitAtMostOnceThenWrong
                        | ProviderProtocol::AfterCommitAtLeastOnce
                        | ProviderProtocol::WrongTransactional => "output",
                        ProviderProtocol::Transactional
                        | ProviderProtocol::Compensatable
                        | ProviderProtocol::CompensatableDistinct => "state",
                    }
                    .to_owned(),
                    operation: "write".to_owned(),
                    intent: ResourceIntent::Send,
                    delivery: ResourceDelivery::Snapshot,
                })])
                .collect(),
        )
        .unwrap(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: constraints.into_boxed_slice(),
        compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .unwrap()
}

fn artifact_with_duplicate_observation(artifact: &ProgramArtifact) -> ProgramArtifact {
    let requirement = artifact
        .requirements()
        .iter()
        .find_map(|(id, requirement)| match requirement {
            ApplicationRequirement::Resource(request) if request.intent == ResourceIntent::Read => {
                Some(id)
            }
            _ => None,
        })
        .expect("observation requirement");
    let original = artifact
        .nodes()
        .iter()
        .find(|node| node.requirement == Some(requirement))
        .expect("observation node");
    let original_slot = artifact.bindings()[original.output_bindings.start as usize].clone();
    let BindingDeclaration::Output {
        target: original_target,
        ..
    } = original_slot
    else {
        panic!("observation output binding")
    };

    let node = NodeId::new(artifact.nodes().len() as u32);
    let slot = CellSlotId::new(artifact.slots().len() as u32);
    let binding = BindingId::new(artifact.bindings().len() as u32);
    let mut nodes = artifact.nodes().to_vec();
    nodes.push(NodeDeclaration {
        node,
        operation: original.operation.clone(),
        contract: original.contract,
        requirement: original.requirement,
        input_bindings: binding.get()..binding.get(),
        output_bindings: binding.get()..binding.get() + 1,
    });
    let mut slots = artifact.slots().to_vec();
    let mut cloned_slot = slots[original_target.get() as usize].clone();
    cloned_slot.slot = slot;
    cloned_slot.producer = mech_engine::ProducerReference::NodeOutput {
        node,
        output_ordinal: 0,
    };
    slots.push(cloned_slot);
    let mut bindings = artifact.bindings().to_vec();
    bindings.push(BindingDeclaration::Output {
        id: binding,
        node,
        port_ordinal: 0,
        target: slot,
    });
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
        compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .expect("duplicate observation artifact")
}

fn artifact_with_duplicate_effect(
    artifact: &ProgramArtifact,
    requirement: ApplicationRequirementId,
) -> ProgramArtifact {
    let original = artifact
        .nodes()
        .iter()
        .find(|node| node.requirement == Some(requirement) && node.output_bindings.is_empty())
        .expect("effect node");
    let original_binding = artifact.bindings()[original.input_bindings.start as usize].clone();
    let BindingDeclaration::Input { source, .. } = original_binding else {
        panic!("effect input binding")
    };
    let node = NodeId::new(artifact.nodes().len() as u32);
    let binding = BindingId::new(artifact.bindings().len() as u32);
    let mut nodes = artifact.nodes().to_vec();
    nodes.push(NodeDeclaration {
        node,
        operation: original.operation.clone(),
        contract: original.contract,
        requirement: original.requirement,
        input_bindings: binding.get()..binding.get() + 1,
        output_bindings: binding.get() + 1..binding.get() + 1,
    });
    let mut bindings = artifact.bindings().to_vec();
    bindings.push(BindingDeclaration::Input {
        id: binding,
        node,
        port_ordinal: 0,
        source,
    });
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
        compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .expect("duplicate effect artifact")
}

fn duplicate_effect_fixture(
    trace: Arc<Mutex<ProviderTrace>>,
    protocol: ProviderProtocol,
    instance_id: ReactiveInstanceId,
) -> MResult<(ProgramArtifact, ReactiveInstance, RuntimeResourceRegistry)> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let artifact = artifact_with_effect(&compilation.source_artifact, protocol);
    let requirement = ApplicationRequirementId::new(artifact.requirements().len() as u32 - 1);
    let artifact = artifact_with_duplicate_effect(&artifact, requirement);
    let catalog = frozen_ekf_compiler_catalog()?;
    let activation = activate_external(
        instance_id,
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider { trace, protocol }))?;
    Ok((artifact, activation, providers))
}

fn fixture(
    trace: Arc<Mutex<ProviderTrace>>,
    protocol: ProviderProtocol,
) -> MResult<(ProgramArtifact, ReactiveInstance, RuntimeResourceRegistry)> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let artifact = artifact_with_effect(&compilation.source_artifact, protocol);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        ReactiveInstanceId::new(700, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider { trace, protocol }))?;
    Ok((artifact, instance, providers))
}

#[test]
fn retained_turn_captures_publishes_receipts_delivers_and_replays() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let authority = ExactRequirementAuthority::new(
        artifact
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone()),
    )?;
    let mut coordinator = ResidentExternalCoordinator::new_live(
        instance,
        Arc::new(artifact.clone()),
        &providers,
        &authority,
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    let outcome = coordinator.execute_turn()?;
    assert!(
        matches!(outcome, ResidentExternalTurnOutcome::Accepted { .. }),
        "unexpected resident outcome: {outcome:?}; receipt: {:?}",
        coordinator
            .receipts()
            .next()
            .map(|(_, receipt)| &receipt.header)
    );
    assert_eq!(coordinator.instance().published_epoch().get(), 1);
    assert_eq!(coordinator.input_facts().count(), 1);
    assert_eq!(coordinator.receipts().count(), 1);
    assert_eq!(coordinator.outbox().count(), 0);
    let batch = coordinator.input_facts().next().unwrap().1.clone();
    let original_record = coordinator.receipts().next().unwrap().1.clone();
    let original_receipt = original_record.body.clone();
    let original_effect_hash = original_receipt.effect_batch_hash;
    assert_eq!(trace.lock().unwrap().reads, 1);
    assert_eq!(trace.lock().unwrap().delivered, 1);

    let replay_trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (replay_artifact, replay_instance, replay_providers) =
        fixture(replay_trace.clone(), ProviderProtocol::AfterCommit)?;
    drop(replay_providers);
    let mut replay = ResidentExternalCoordinator::new_replay(
        replay_instance,
        Arc::new(replay_artifact),
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        replay.execute_replay_batch(Some(&batch), &original_record)?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(replay_trace.lock().unwrap().reads, 0);
    assert_eq!(
        replay.receipts().next().unwrap().1.body.state_hash,
        original_receipt.state_hash
    );
    assert_eq!(
        replay.receipts().next().unwrap().1.body.effect_batch_hash,
        original_effect_hash
    );
    assert!(replay_trace.lock().unwrap().prepared.is_empty());
    assert_eq!(replay_trace.lock().unwrap().delivered, 0);
    assert!(replay.execute_turn().is_err());
    Ok(())
}

#[test]
fn default_authority_denies_before_any_provider_read() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    assert!(
        ResidentExternalCoordinator::new_live(
            instance,
            Arc::new(artifact),
            &providers,
            &DenyAllResidentExternalAuthority,
            ResidentDurabilityPolicy::Retained,
            ResidentExternalLimits::default(),
        )
        .is_err()
    );
    assert_eq!(trace.lock().unwrap().reads, 0);
    Ok(())
}

#[test]
fn idempotent_retry_requires_provider_deduplication_support() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) =
        fixture(trace, ProviderProtocol::AfterCommitNoIdempotency)?;
    assert!(
        coordinator(
            instance,
            &artifact,
            &providers,
            ResidentExternalLimits::default(),
        )
        .is_err()
    );
    Ok(())
}

fn coordinator(
    instance: ReactiveInstance,
    artifact: &ProgramArtifact,
    providers: &RuntimeResourceRegistry,
    limits: ResidentExternalLimits,
) -> MResult<ResidentExternalCoordinator> {
    coordinator_with_durability(
        instance,
        artifact,
        providers,
        ResidentDurabilityPolicy::Retained,
        limits,
    )
}

fn coordinator_with_durability(
    instance: ReactiveInstance,
    artifact: &ProgramArtifact,
    providers: &RuntimeResourceRegistry,
    durability: ResidentDurabilityPolicy,
    limits: ResidentExternalLimits,
) -> MResult<ResidentExternalCoordinator> {
    let authority = ExactRequirementAuthority::new(
        artifact
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone()),
    )?;
    ResidentExternalCoordinator::new_live(
        instance,
        Arc::new(artifact.clone()),
        providers,
        &authority,
        durability,
        limits,
    )
}

#[test]
fn observation_failure_is_rejected_without_publication_or_delivery() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        fail_read: true,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    let outcome = coordinator.execute_turn()?;
    assert!(matches!(
        outcome,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::InputInstallation,
            ..
        }
    ));
    assert_eq!(
        coordinator.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    assert_eq!(trace.lock().unwrap().reads, 1);
    assert_eq!(trace.lock().unwrap().delivered, 0);
    assert_eq!(coordinator.receipts().count(), 1);
    assert_eq!(
        coordinator.receipts().next().unwrap().1.body.state_hash,
        coordinator.instance().published_state_hash()
    );
    Ok(())
}

#[test]
fn provider_prepare_failure_and_wrong_protocol_reject_before_publication() -> MResult<()> {
    for (protocol, fail_prepare) in [
        (ProviderProtocol::AfterCommit, true),
        (ProviderProtocol::WrongTransactional, false),
    ] {
        let trace = Arc::new(Mutex::new(ProviderTrace {
            fail_prepare,
            ..ProviderTrace::default()
        }));
        let (artifact, instance, providers) = fixture(trace.clone(), protocol)?;
        let mut coordinator = coordinator(
            instance,
            &artifact,
            &providers,
            ResidentExternalLimits::default(),
        )?;
        assert!(matches!(
            coordinator.execute_turn()?,
            ResidentExternalTurnOutcome::Rejected {
                phase: TurnFailurePhase::ExternalPrepare,
                ..
            }
        ));
        assert_eq!(
            coordinator.instance().published_epoch(),
            mech_core::InstanceEpoch::ZERO
        );
        assert_eq!(trace.lock().unwrap().delivered, 0);
        let (_, batch) = coordinator
            .input_facts()
            .next()
            .expect("retained input batch");
        let receipt = coordinator.receipts().next().unwrap().1;
        assert_eq!(receipt.body.input_batch_hash, batch.batch_hash);
        assert_eq!(receipt.body.effect_count, 1);
        assert_ne!(receipt.body.effect_batch_hash, [0; 32]);
    }
    Ok(())
}

#[test]
fn later_provider_preparation_failure_aborts_every_earlier_effect() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        fail_prepare_at: Some(2),
        ..ProviderTrace::default()
    }));
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let artifact = artifact_with_effect(
        &compilation.source_artifact,
        ProviderProtocol::Compensatable,
    );
    let requirement = ApplicationRequirementId::new(artifact.requirements().len() as u32 - 1);
    let artifact = artifact_with_duplicate_effect(&artifact, requirement);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        ReactiveInstanceId::new(703, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::Compensatable,
    }))?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::ExternalPrepare,
            ..
        }
    ));
    assert_eq!(trace.lock().unwrap().prepared.len(), 2);
    assert_eq!(trace.lock().unwrap().lifecycle, ["abort"]);
    Ok(())
}

#[test]
fn idempotent_delivery_failure_retains_identity_and_key_for_retry() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        delivery_failures_remaining: 1,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    let ResidentExternalTurnOutcome::Accepted {
        delivery_failures, ..
    } = coordinator.execute_turn()?
    else {
        panic!("delivery failure occurs after accepted resident publication")
    };
    assert_eq!(delivery_failures.len(), 1);
    assert_eq!(coordinator.instance().published_epoch().get(), 1);
    assert_eq!(coordinator.outbox().count(), 1);
    let original = trace.lock().unwrap().prepared[0].clone();
    assert!(coordinator.retry_outbox()?.is_empty());
    assert_eq!(coordinator.outbox().count(), 0);
    let retried = trace.lock().unwrap().prepared[1].clone();
    assert_eq!(retried, original);
    assert_eq!(trace.lock().unwrap().delivered, 2);
    Ok(())
}

#[test]
fn ordinary_delivery_policies_have_executable_failure_lifecycles() -> MResult<()> {
    for (protocol, retained_after_failure) in [
        (ProviderProtocol::AfterCommitAtMostOnce, false),
        (ProviderProtocol::AfterCommitAtLeastOnce, true),
    ] {
        let trace = Arc::new(Mutex::new(ProviderTrace {
            delivery_failures_remaining: 1,
            ..ProviderTrace::default()
        }));
        let (artifact, instance, providers) = fixture(trace.clone(), protocol)?;
        let mut coordinator = coordinator(
            instance,
            &artifact,
            &providers,
            ResidentExternalLimits::default(),
        )?;
        let ResidentExternalTurnOutcome::Accepted {
            delivery_failures, ..
        } = coordinator.execute_turn()?
        else {
            panic!("ordinary delivery failure occurs after resident publication")
        };
        assert_eq!(delivery_failures.len(), 1);
        assert_eq!(
            coordinator.outbox().count(),
            usize::from(retained_after_failure)
        );
        assert!(coordinator.retry_outbox()?.is_empty());
        assert_eq!(coordinator.outbox().count(), 0);
        assert_eq!(
            trace.lock().unwrap().delivered,
            if retained_after_failure { 2 } else { 1 }
        );
    }
    Ok(())
}

#[test]
fn volatile_retryable_effects_survive_failed_delivery() -> MResult<()> {
    for protocol in [
        ProviderProtocol::AfterCommit,
        ProviderProtocol::AfterCommitAtLeastOnce,
    ] {
        let trace = Arc::new(Mutex::new(ProviderTrace {
            delivery_failures_remaining: 1,
            ..ProviderTrace::default()
        }));
        let (artifact, instance, providers) = fixture(trace.clone(), protocol)?;
        let mut coordinator = coordinator_with_durability(
            instance,
            &artifact,
            &providers,
            ResidentDurabilityPolicy::Volatile,
            ResidentExternalLimits::default(),
        )?;
        let ResidentExternalTurnOutcome::Accepted {
            delivery_failures, ..
        } = coordinator.execute_turn()?
        else {
            panic!("retryable delivery failure occurs after publication")
        };
        assert_eq!(delivery_failures.len(), 1);
        assert_eq!(coordinator.input_facts().count(), 0);
        assert_eq!(coordinator.receipts().count(), 0);
        assert_eq!(coordinator.outbox().count(), 1);
        let original = trace.lock().unwrap().prepared[0].clone();

        assert!(coordinator.retry_outbox()?.is_empty());
        assert_eq!(coordinator.outbox().count(), 0);
        assert_eq!(trace.lock().unwrap().prepared[1], original);
        assert_eq!(trace.lock().unwrap().delivered, 2);
    }
    Ok(())
}

#[test]
fn at_most_once_preparation_failure_remains_pending_until_delivery_can_begin() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        fail_prepare_at: Some(4),
        delivery_failures_remaining: 1,
        ..ProviderTrace::default()
    }));
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let retryable = artifact_with_effect(
        &compilation.source_artifact,
        ProviderProtocol::AfterCommitAtLeastOnce,
    );
    let artifact =
        artifact_with_effect(&retryable, ProviderProtocol::AfterCommitAtMostOnceDistinct);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        ReactiveInstanceId::new(704, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::AfterCommitAtLeastOnce,
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::AfterCommitAtMostOnceDistinct,
    }))?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    let ResidentExternalTurnOutcome::Accepted {
        delivery_failures, ..
    } = coordinator.execute_turn()?
    else {
        panic!("first AtMostOnce delivery attempt must be post-publication")
    };
    assert_eq!(delivery_failures.len(), 1);
    assert_eq!(coordinator.outbox().count(), 2);

    let preparation_failures = coordinator.retry_outbox()?;
    assert_eq!(preparation_failures.len(), 1);
    assert_eq!(
        preparation_failures[0].phase,
        crate::RuntimeEffectFailurePhase::Prepare
    );
    assert_eq!(coordinator.outbox().count(), 1);
    trace.lock().unwrap().fail_prepare_at = None;

    assert!(coordinator.retry_outbox()?.is_empty());
    assert_eq!(coordinator.outbox().count(), 0);
    assert_eq!(trace.lock().unwrap().delivered, 3);
    Ok(())
}

fn retained_protocol_mismatch_fixture(
    trace: Arc<Mutex<ProviderTrace>>,
    instance_id: ReactiveInstanceId,
) -> MResult<(ProgramArtifact, ReactiveInstance, RuntimeResourceRegistry)> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let retryable = artifact_with_effect(
        &compilation.source_artifact,
        ProviderProtocol::AfterCommitAtLeastOnce,
    );
    let artifact =
        artifact_with_effect(&retryable, ProviderProtocol::AfterCommitAtMostOnceThenWrong);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        instance_id,
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::AfterCommitAtLeastOnce,
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace,
        protocol: ProviderProtocol::AfterCommitAtMostOnceThenWrong,
    }))?;
    Ok((artifact, instance, providers))
}

#[test]
fn at_most_once_protocol_mismatch_is_preparation_and_aborts_the_handle() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        delivery_failures_remaining: 1,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) =
        retained_protocol_mismatch_fixture(trace.clone(), ReactiveInstanceId::new(705, 0))?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    let ResidentExternalTurnOutcome::Accepted {
        delivery_failures, ..
    } = coordinator.execute_turn()?
    else {
        panic!("front retryable failure occurs after publication")
    };
    assert_eq!(delivery_failures.len(), 1);
    assert_eq!(coordinator.outbox().count(), 2);

    let failures = coordinator.retry_outbox()?;
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].phase, crate::RuntimeEffectFailurePhase::Prepare);
    assert_eq!(coordinator.outbox().count(), 1);
    assert_eq!(trace.lock().unwrap().delivered, 2);
    assert_eq!(
        trace
            .lock()
            .unwrap()
            .lifecycle
            .iter()
            .filter(|event| **event == "abort")
            .count(),
        1
    );
    assert!(coordinator.retry_outbox()?.is_empty());
    assert_eq!(coordinator.outbox().count(), 0);
    assert_eq!(trace.lock().unwrap().delivered, 3);
    Ok(())
}

#[test]
fn retained_protocol_mismatch_cleanup_failure_poisons_the_coordinator() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        delivery_failures_remaining: 1,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) =
        retained_protocol_mismatch_fixture(trace.clone(), ReactiveInstanceId::new(715, 0))?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    trace.lock().unwrap().transactional_abort_fail = true;

    assert!(coordinator.retry_outbox().is_err());
    assert_eq!(coordinator.outbox().count(), 1);
    assert_eq!(trace.lock().unwrap().delivered, 2);
    assert_eq!(
        trace
            .lock()
            .unwrap()
            .lifecycle
            .iter()
            .filter(|event| **event == "abort")
            .count(),
        1
    );
    assert!(matches!(
        coordinator.health(),
        ResidentExternalHealth::PoisonedRetainedEffectCleanup { .. }
    ));
    assert!(coordinator.retry_outbox().is_err());
    Ok(())
}

#[test]
fn transactional_prepare_failure_rejects_and_commit_failure_is_published_indeterminate()
-> MResult<()> {
    let prepare_trace = Arc::new(Mutex::new(ProviderTrace {
        transactional_prepare_fail: true,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) =
        fixture(prepare_trace.clone(), ProviderProtocol::Transactional)?;
    let mut rejected = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        rejected.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::ExternalPrepare,
            ..
        }
    ));
    assert_eq!(
        rejected.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    assert_eq!(
        prepare_trace.lock().unwrap().lifecycle,
        ["prepare", "abort"]
    );

    let commit_trace = Arc::new(Mutex::new(ProviderTrace {
        transactional_commit_fail: true,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) =
        fixture(commit_trace.clone(), ProviderProtocol::Transactional)?;
    let mut indeterminate = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        indeterminate.execute_turn()?,
        ResidentExternalTurnOutcome::PublishedIndeterminate { .. }
    ));
    assert_eq!(indeterminate.instance().published_epoch().get(), 1);
    assert!(matches!(
        indeterminate.health(),
        ResidentExternalHealth::PoisonedPostpublicationCommit { .. }
    ));
    assert_eq!(
        commit_trace.lock().unwrap().lifecycle,
        ["prepare", "commit"]
    );
    assert!(indeterminate.execute_turn().is_err());
    Ok(())
}

#[test]
fn volatile_commit_indeterminate_retains_undelivered_ordinary_effects() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        transactional_commit_fail: true,
        ..ProviderTrace::default()
    }));
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let ordinary =
        artifact_with_effect(&compilation.source_artifact, ProviderProtocol::AfterCommit);
    let artifact = artifact_with_effect(&ordinary, ProviderProtocol::Transactional);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        ReactiveInstanceId::new(705, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::AfterCommit,
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::Transactional,
    }))?;
    let mut coordinator = coordinator_with_durability(
        instance,
        &artifact,
        &providers,
        ResidentDurabilityPolicy::Volatile,
        ResidentExternalLimits::default(),
    )?;

    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::PublishedIndeterminate { .. }
    ));
    assert_eq!(coordinator.instance().published_epoch().get(), 1);
    assert_eq!(coordinator.input_facts().count(), 0);
    assert_eq!(coordinator.receipts().count(), 0);
    assert_eq!(coordinator.outbox().count(), 1);
    assert_eq!(trace.lock().unwrap().delivered, 0);
    assert!(matches!(
        coordinator.health(),
        ResidentExternalHealth::PoisonedPostpublicationCommit { .. }
    ));
    Ok(())
}

#[test]
fn compensatable_apply_failure_rejects_without_publication() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        compensatable_apply_fail: true,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::Compensatable)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::ExternalApply,
            ..
        }
    ));
    assert_eq!(
        coordinator.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    assert_eq!(trace.lock().unwrap().lifecycle, ["apply", "abort"]);
    Ok(())
}

#[test]
fn failed_compensation_is_never_invoked_twice_by_cleanup() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        compensatable_apply_fail_at: Some(2),
        compensatable_compensate_fail: true,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = duplicate_effect_fixture(
        trace.clone(),
        ProviderProtocol::Compensatable,
        ReactiveInstanceId::new(706, 0),
    )?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;

    assert!(coordinator.execute_turn().is_err());
    assert!(matches!(
        coordinator.health(),
        ResidentExternalHealth::PoisonedPrepublicationCleanup { .. }
    ));
    let lifecycle = trace.lock().unwrap().lifecycle.clone();
    assert_eq!(
        lifecycle,
        ["apply", "apply", "compensate", "abort"],
        "an indeterminate compensation must not be retried by abort_all"
    );
    assert_eq!(
        lifecycle
            .iter()
            .filter(|event| **event == "compensate")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn mixed_effect_cleanup_compensates_before_transactional_abort() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        compensatable_apply_fail_at: Some(2),
        ..ProviderTrace::default()
    }));
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let compensatable = artifact_with_effect(
        &compilation.source_artifact,
        ProviderProtocol::CompensatableDistinct,
    );
    let compensatable_requirement =
        ApplicationRequirementId::new(compensatable.requirements().len() as u32 - 1);
    let duplicate = artifact_with_duplicate_effect(&compensatable, compensatable_requirement);
    let artifact = artifact_with_effect(&duplicate, ProviderProtocol::Transactional);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        ReactiveInstanceId::new(702, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ObservationProvider {
        trace: trace.clone(),
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::CompensatableDistinct,
    }))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::Transactional,
    }))?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::ExternalApply,
            ..
        }
    ));
    assert_eq!(
        trace.lock().unwrap().lifecycle,
        [
            "prepare",
            "apply",
            "apply",
            "compensate",
            "abort",
            "abort",
            "abort"
        ]
    );
    Ok(())
}

#[test]
fn capacity_is_reserved_before_observation_capture() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits {
            input_bytes: 1,
            ..ResidentExternalLimits::default()
        },
    )?;
    assert!(coordinator.execute_turn().is_err());
    assert_eq!(trace.lock().unwrap().reads, 0);
    assert_eq!(
        coordinator.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    Ok(())
}

#[test]
fn coordinator_rejects_an_artifact_other_than_the_instance_artifact() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let other = artifact_with_effect(
        &compilation.source_artifact,
        ProviderProtocol::Transactional,
    );
    assert_ne!(artifact.revision(), other.revision());
    let authority = ExactRequirementAuthority::new(
        other
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone()),
    )?;
    assert!(
        ResidentExternalCoordinator::new_live(
            instance,
            Arc::new(other),
            &providers,
            &authority,
            ResidentDurabilityPolicy::Retained,
            ResidentExternalLimits::default(),
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn safe_engine_api_cannot_publish_an_external_candidate() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, mut instance, _providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let input = instance.plan.inputs[0].clone();
    let value = captured_value_from_legacy(
        &LegacyValue::MatrixF64(ToMatrix::to_matrix(vec![0.2, 0.01, 5.0, -2.0], 4, 1)),
        input.schema,
        &input.shape,
        artifact.schemas(),
    )?;
    let captured = [mech_engine::__resident::CapturedValueInput {
        slot: input.slot,
        value: &value,
    }];

    assert_eq!(
        instance.prepare_turn_values(&captured).unwrap().publish(),
        Err(mech_engine::__resident::ResidentExecutionError::ExternalSummaryRequired)
    );
    let signal = [0.2, 0.01, 5.0, -2.0];
    let ordinary = [mech_engine::__resident::CapturedSignalInput {
        slot: input.slot,
        value: mech_core::ResidentValueRef::F64(&signal),
    }];
    assert_eq!(
        instance.turn(&ordinary),
        Err(mech_engine::__resident::ResidentExecutionError::ExternalSummaryRequired)
    );
    assert_eq!(
        instance.turn_without_summary(&ordinary),
        Err(mech_engine::__resident::ResidentExecutionError::ExternalSummaryRequired)
    );
    assert_eq!(instance.published_epoch(), mech_core::InstanceEpoch::ZERO);
    Ok(())
}

#[test]
fn replay_reconstructs_and_rejects_forged_batch_identity() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut source = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        source.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let mut forged = source.input_facts().next().unwrap().1.clone();
    let record = source.receipts().next().unwrap().1.clone();
    forged.batch_hash[0] ^= 0xff;

    let replay_trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (replay_artifact, replay_instance, replay_providers) =
        fixture(replay_trace, ProviderProtocol::AfterCommit)?;
    drop(replay_providers);
    let mut replay = ResidentExternalCoordinator::new_replay(
        replay_instance,
        Arc::new(replay_artifact),
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    assert!(replay.execute_replay_batch(Some(&forged), &record).is_err());
    Ok(())
}

#[test]
fn accepted_replay_receipt_mismatch_does_not_consume_replay_identity() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut source = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        source.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert!(matches!(
        source.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let batches = source
        .input_facts()
        .map(|(_, batch)| batch.clone())
        .collect::<Vec<_>>();
    let records = source
        .receipts()
        .map(|(_, record)| record.clone())
        .collect::<Vec<_>>();
    let mut mismatched = records[0].clone();
    mismatched.body.effect_batch_hash[0] ^= 0xff;

    let replay_trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (replay_artifact, replay_instance, replay_providers) =
        fixture(replay_trace, ProviderProtocol::AfterCommit)?;
    drop(replay_providers);
    let mut replay = ResidentExternalCoordinator::new_replay(
        replay_instance,
        Arc::new(replay_artifact),
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    assert!(
        replay
            .execute_replay_batch(Some(&batches[0]), &mismatched)
            .is_err()
    );
    assert_eq!(replay.input_facts().count(), 0);
    assert_eq!(replay.receipts().count(), 0);
    assert_eq!(
        replay.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    assert!(matches!(
        replay.execute_replay_batch(Some(&batches[0]), &records[0])?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(replay.instance().published_epoch().get(), 1);
    assert!(matches!(
        replay.execute_replay_batch(Some(&batches[1]), &records[1])?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(replay.instance().published_epoch().get(), 2);
    assert_eq!(replay.input_facts().count(), 2);
    assert_eq!(replay.receipts().count(), 2);
    Ok(())
}

#[test]
fn replay_preserves_a_recorded_full_input_rejection_before_later_acceptance() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        fail_prepare: true,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let mut live = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        live.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::ExternalPrepare,
            ..
        }
    ));
    trace.lock().unwrap().fail_prepare = false;
    assert!(matches!(
        live.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let batches = live
        .input_facts()
        .map(|(_, batch)| batch.clone())
        .collect::<Vec<_>>();
    let records = live
        .receipts()
        .map(|(_, record)| record.clone())
        .collect::<Vec<_>>();
    assert_eq!(batches.len(), 2);
    assert_eq!(records.len(), 2);
    drop(live);
    drop(providers);

    let catalog = frozen_ekf_compiler_catalog()?;
    let replay_instance = activate_external(
        ReactiveInstanceId::new(700, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut replay = ResidentExternalCoordinator::new_replay(
        replay_instance,
        Arc::new(artifact.clone()),
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        replay.execute_replay_batch(Some(&batches[0]), &records[0])?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::ExternalPrepare,
            ..
        }
    ));
    assert_eq!(
        replay.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    assert!(matches!(
        replay.execute_replay_batch(Some(&batches[1]), &records[1])?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(replay.instance().published_epoch().get(), 1);
    assert_eq!(replay.receipts().last().unwrap().1, records.last().unwrap());
    Ok(())
}

#[test]
fn retained_turn_batches_allow_distinct_nodes_to_share_a_requirement() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let batch = coordinator
        .input_facts()
        .next()
        .expect("accepted live turn retains its captured input")
        .1
        .clone();
    let first = &batch.facts[0];
    let second = CapturedInputFact::new(
        InputSequence::new(first.sequence.get() + 1).unwrap(),
        first.requirement,
        NodeId::new(first.node.get() + 1),
        CellSlotId::new(first.slot.get() + 1),
        first.schema_key,
        first.shape.clone(),
        first.value.clone(),
        artifact.schemas(),
    )?;
    assert!(CapturedInputBatch::new(vec![first.clone(), second]).is_ok());
    Ok(())
}

#[test]
fn shared_observations_capture_one_authoritative_provider_snapshot() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        fail_read_at: Some(2),
        ..ProviderTrace::default()
    }));
    let (artifact, _instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let artifact = artifact_with_duplicate_observation(&artifact);
    let catalog = frozen_ekf_compiler_catalog()?;
    let instance = activate_external(
        ReactiveInstanceId::new(701, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(trace.lock().unwrap().reads, 1);
    let batch = coordinator.input_facts().next().unwrap().1.clone();
    let record = coordinator.receipts().next().unwrap().1.clone();
    assert_eq!(batch.facts.len(), 2);
    assert_eq!(
        batch.facts[0].value.resident_token(),
        batch.facts[1].value.resident_token()
    );
    let second = &batch.facts[1];
    let divergent_value = captured_value_from_legacy(
        &LegacyValue::MatrixF64(ToMatrix::to_matrix(vec![9.0, 0.01, 5.0, -2.0], 4, 1)),
        second.value.schema(),
        &second.shape,
        artifact.schemas(),
    )?;
    let divergent_second = CapturedInputFact::new(
        second.sequence,
        second.requirement,
        second.node,
        second.slot,
        second.schema_key,
        second.shape.clone(),
        divergent_value,
        artifact.schemas(),
    )?;
    let inconsistent_batch =
        CapturedInputBatch::new(vec![batch.facts[0].clone(), divergent_second])?;
    drop(coordinator);
    drop(providers);

    let replay_instance = activate_external(
        ReactiveInstanceId::new(701, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut replay = ResidentExternalCoordinator::new_replay(
        replay_instance,
        Arc::new(artifact.clone()),
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    let error = replay
        .execute_replay_batch(Some(&inconsistent_batch), &record)
        .unwrap_err();
    assert!(
        error
            .display_message()
            .contains("conflicting snapshots for one source identity")
    );
    assert_eq!(replay.input_facts().count(), 0);
    assert_eq!(replay.receipts().count(), 0);
    assert_eq!(
        replay.instance().published_epoch(),
        mech_core::InstanceEpoch::ZERO
    );
    assert!(matches!(
        replay.execute_replay_batch(Some(&batch), &record)?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(replay.instance().published_epoch().get(), 1);
    Ok(())
}

#[test]
fn idempotency_keys_are_namespaced_by_reactive_instance() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let batch = coordinator
        .input_facts()
        .next()
        .expect("accepted live turn retains its captured input")
        .1
        .clone();
    let requirement = artifact.requirements().iter().last().unwrap().1;
    let left = resident_idempotency_key(
        ReactiveInstanceId::new(1, 0),
        artifact.revision(),
        TurnId::new(1).unwrap(),
        0,
        requirement,
        batch.facts[0].payload_hash,
    )?;
    let right = resident_idempotency_key(
        ReactiveInstanceId::new(2, 0),
        artifact.revision(),
        TurnId::new(1).unwrap(),
        0,
        requirement,
        batch.facts[0].payload_hash,
    )?;
    assert_ne!(left, right);
    let left_id = resident_effect_id(ReactiveInstanceId::new(1, 3), TurnId::new(1).unwrap(), 0);
    let right_id = resident_effect_id(ReactiveInstanceId::new(2, 3), TurnId::new(1).unwrap(), 0);
    assert_ne!(left_id.transaction, right_id.transaction);
    assert_eq!(
        left_id.transaction,
        TransactionId((1_u128 << 96) | (3_u128 << 64) | 1)
    );
    Ok(())
}

#[test]
fn one_receipt_slot_accepts_one_mutually_exclusive_outcome() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits {
            receipts: 1,
            receipt_bytes: 1_024,
            ..ResidentExternalLimits::default()
        },
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(coordinator.receipts().count(), 1);
    Ok(())
}

#[test]
fn receipt_counts_only_effects_materialized_for_the_current_turn() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    let record = coordinator.accepted_record_without_materialized_effects_for_test(
        mech_engine::__resident::ResidentTurnSummary {
            instance: ReactiveInstanceId::new(700, 0),
            program_revision: artifact.revision(),
            before_epoch: mech_core::InstanceEpoch::ZERO,
            after_epoch: mech_core::InstanceEpoch::new(1),
            state_hash: 0,
            touched_slots: 0,
            changed_slots: 0,
            dirty_nodes: 0,
        },
    )?;
    assert_eq!(record.body.effect_count, 0);
    assert_eq!(record.body.outbox_effect_count, 0);
    assert_eq!(record.body.transactional_effect_count, 0);
    Ok(())
}

#[test]
fn volatile_turns_release_all_retained_history() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator_with_durability(
        instance,
        &artifact,
        &providers,
        ResidentDurabilityPolicy::Volatile,
        ResidentExternalLimits {
            input_batches: 1,
            input_bytes: 1_024,
            receipts: 1,
            receipt_bytes: 1_024,
            outbox_effects: 1,
            outbox_bytes: 1_024,
        },
    )?;
    for _ in 0..3 {
        assert!(matches!(
            coordinator.execute_turn()?,
            ResidentExternalTurnOutcome::Accepted { .. }
        ));
        assert_eq!(coordinator.input_facts().count(), 0);
        assert_eq!(coordinator.receipts().count(), 0);
        assert_eq!(coordinator.outbox().count(), 0);
    }
    Ok(())
}

#[test]
fn retained_records_can_be_released_in_fifo_order_and_capacity_reused() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits {
            input_batches: 1,
            input_bytes: 1_024,
            receipts: 1,
            receipt_bytes: 1_024,
            ..ResidentExternalLimits::default()
        },
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let (first_input_sequence, first_batch) = coordinator
        .release_next_input_batch()
        .expect("first retained input batch");
    let (first_receipt_sequence, first_receipt) = coordinator
        .release_next_receipt()
        .expect("first retained receipt");
    assert_eq!(first_receipt.header.input_range, Some(first_batch.range));

    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    let (second_input_sequence, _) = coordinator
        .release_next_input_batch()
        .expect("second retained input batch");
    let (second_receipt_sequence, _) = coordinator
        .release_next_receipt()
        .expect("second retained receipt");
    assert!(second_input_sequence > first_input_sequence);
    assert!(second_receipt_sequence > first_receipt_sequence);
    Ok(())
}

#[test]
fn failed_preflight_does_not_consume_turn_identity() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        delivery_failures_remaining: 1,
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(trace.clone(), ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits {
            outbox_effects: 1,
            outbox_bytes: 1_024,
            ..ResidentExternalLimits::default()
        },
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted {
            turn,
            ref delivery_failures,
            ..
        } if turn == TurnId::new(1).unwrap() && delivery_failures.len() == 1
    ));
    assert_eq!(coordinator.outbox().count(), 1);
    assert!(coordinator.execute_turn().is_err());
    assert_eq!(trace.lock().unwrap().reads, 1);

    assert!(coordinator.retry_outbox()?.is_empty());
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { turn, .. }
            if turn == TurnId::new(2).unwrap()
    ));
    assert_eq!(
        coordinator
            .receipts()
            .map(|(_, receipt)| receipt.header.turn_id)
            .collect::<Vec<_>>(),
        [TurnId::new(1).unwrap(), TurnId::new(2).unwrap()]
    );
    Ok(())
}

#[test]
fn rejected_receipt_bounds_provider_failure_text() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace {
        fail_read: true,
        read_failure_message: Some("x".repeat(32_000)),
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits {
            receipts: 1,
            receipt_bytes: 1_024,
            ..ResidentExternalLimits::default()
        },
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected { .. }
    ));
    let message = &coordinator
        .receipts()
        .next()
        .unwrap()
        .1
        .header
        .failure
        .as_ref()
        .unwrap()
        .message;
    assert!(message.len() <= 256);
    Ok(())
}

#[test]
fn provider_matrix_shape_and_row_major_order_are_preserved() -> MResult<()> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let (artifact, instance, _providers) = fixture(trace, ProviderProtocol::AfterCommit)?;
    let slot = instance
        .plan
        .slots
        .iter()
        .find(|slot| slot.region.shape.rows == 2 && slot.region.shape.columns == 3)
        .expect("EKF 2x3 resident slot");
    let legacy = LegacyValue::MatrixF64(ToMatrix::to_matrix(
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        2,
        3,
    ));
    let canonical =
        captured_value_from_legacy(&legacy, slot.schema, &slot.shape, artifact.schemas())?;
    let ValueData::Matrix(matrix) = canonical.data() else {
        panic!("captured matrix representation")
    };
    let SequenceView::F64(values) = matrix.elements() else {
        panic!("captured f64 matrix representation")
    };
    assert_eq!(
        values
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        [1.0, 3.0, 5.0, 2.0, 4.0, 6.0]
    );

    let mismatch_trace = Arc::new(Mutex::new(ProviderTrace {
        observation_shape: Some((2, 2)),
        ..ProviderTrace::default()
    }));
    let (artifact, instance, providers) = fixture(mismatch_trace, ProviderProtocol::AfterCommit)?;
    let mut coordinator = coordinator(
        instance,
        &artifact,
        &providers,
        ResidentExternalLimits::default(),
    )?;
    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: TurnFailurePhase::InputInstallation,
            ..
        }
    ));
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
fn source_fixture_artifact(
    source: &str,
    protocol: ProviderProtocol,
) -> MResult<(ProgramArtifact, ProgramArtifact)> {
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .resource_provider(Box::new(SourceInputProvider))
        .resource_provider(Box::new(EffectProvider { trace, protocol }))
        .build_compiler()?;
    let product = compiler.compile_source(source)?;
    let parsed = ParsedProgram::from_bytes(product.bytecode())?;
    let decoded = decode_program_artifact_sections(&parsed.artifact).map_err(|error| {
        test_error(&format!("D3 bytecode-v1 artifact decode failed: {error:?}"))
    })?;
    Ok((product.artifact().clone(), decoded))
}

#[cfg(feature = "semantic-compiler")]
#[test]
fn effect_payload_is_captured_before_a_later_state_mutation() -> MResult<()> {
    const SOURCE: &str = r#"
@input := gate-d3://input/value{:read(sample)}
sample := @input/sample

~state := 0.0
state += sample

@scene := gate-d3://scene/output
@scene/frame <- state

state += sample
output := state
"#;
    let (artifact, _) = source_fixture_artifact(SOURCE, ProviderProtocol::AfterCommit)?;
    let catalog = mech_stdlib::source_catalog();
    let instance = activate_external(
        ReactiveInstanceId::new(707, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| test_error(&format!("activate payload timing fixture: {error:?}")))?;
    let trace = Arc::new(Mutex::new(ProviderTrace::default()));
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(SourceInputProvider))?;
    providers.register_provider(Box::new(EffectProvider {
        trace: trace.clone(),
        protocol: ProviderProtocol::AfterCommit,
    }))?;
    let authority = ExactRequirementAuthority::new(
        artifact
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone()),
    )?;
    let mut coordinator = ResidentExternalCoordinator::new_live(
        instance,
        Arc::new(artifact),
        &providers,
        &authority,
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;

    assert!(matches!(
        coordinator.execute_turn()?,
        ResidentExternalTurnOutcome::Accepted { .. }
    ));
    assert_eq!(trace.lock().unwrap().prepared_f64, [0.25]);
    let ResidentValueBorrow::F64 { values, .. } = coordinator.instance().output_borrow(0).unwrap()
    else {
        panic!("payload timing fixture output is f64")
    };
    assert_eq!(values, [0.5]);
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
#[test]
fn ordinary_source_and_bytecode_freeze_equivalent_external_artifacts() -> MResult<()> {
    for (source, protocol, expected_effect) in [
        (
            include_str!("../../../../../../tests/fixtures/resident-external/effect-source.mec"),
            ProviderProtocol::AfterCommit,
            EFFECT_CONTRACT.interaction.clone(),
        ),
        (
            include_str!(
                "../../../../../../tests/fixtures/resident-external/transactional-source.mec"
            ),
            ProviderProtocol::Compensatable,
            COMPENSATABLE_CONTRACT.interaction.clone(),
        ),
    ] {
        let (artifact, decoded) = source_fixture_artifact(source, protocol)?;
        assert_eq!(artifact.revision(), decoded.revision());
        assert_eq!(artifact.requirements(), decoded.requirements());
        assert_eq!(artifact.nodes(), decoded.nodes());
        assert_eq!(artifact.requirements().len(), 2);
        assert_eq!(
            artifact
                .nodes()
                .iter()
                .filter(|node| node.requirement.is_some())
                .count(),
            2
        );
        assert!(artifact.nodes().iter().any(|node| {
            matches!(
                artifact.contracts().get(node.contract),
                Some(ResolvedOperationContract::Declared(contract))
                    if contract.interaction == expected_effect && contract.outputs.is_empty()
            )
        }));
    }
    Ok(())
}
