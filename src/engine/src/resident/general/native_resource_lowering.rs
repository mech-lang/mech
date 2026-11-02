//! Exact original-artifact resource boundaries for native SSA transport.
//! No host is bound, no grant is issued, and no resource value is observed here.
use super::{Error, unsupported};
use crate::{
    ArtifactSource, BindingDeclaration, ExecutableNodeBody, NodeDeclaration, ProducerReference,
    ProgramArtifact, SlotRole,
};
use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, ApplicationRequirementId, CellSlotId,
    ChangeDetectionPolicy, DeliveryMode, EffectContract, EffectDeliveryPolicy,
    ExecutionResourceRequest, ExternalInteraction, IdempotencyRequirement, ObservationContract,
    ObservationReplayPolicy, OutputConstruction, ResolvedOperationContract, ResourceDelivery,
    ResourceIntent, SchemaBody, SchemaId, ShapeRule,
};

pub(super) fn requirements(
    artifact: &ProgramArtifact,
) -> Result<Box<[ExecutionResourceRequest]>, Error> {
    artifact
        .requirements()
        .iter()
        .enumerate()
        .map(|(ordinal, (id, requirement))| {
            if id.get() as usize != ordinal {
                return Err(Error::InvalidBinding { node: None });
            }
            let ApplicationRequirement::Resource(request) = requirement else {
                return Err(Error::UnsupportedProgram {
                    feature: "native host-function requirement",
                });
            };
            // Live is the ORIGINAL canonical read identity. CaptureAsInputFact is
            // checked at its site below; driverless-provider and capability checks
            // belong to the authorized runtime boundary, never this compiler view.
            let supported = match request.intent {
                ResourceIntent::Read => {
                    request.delivery == ResourceDelivery::Live && request.operation == "read"
                }
                ResourceIntent::Send => {
                    request.delivery == ResourceDelivery::Snapshot && request.operation == "write"
                }
                ResourceIntent::Assign => false,
            };
            if !supported || request.context_name.is_empty() {
                return Err(Error::UnsupportedProgram {
                    feature: "native resources require captured Live reads or Snapshot sends",
                });
            }
            Ok(request.clone())
        })
        .collect()
}

fn string_schema(artifact: &ProgramArtifact, schema: SchemaId) -> Result<(), Error> {
    if artifact.schemas().get(schema).is_some_and(|schema| {
        schema.body() == &SchemaBody::String && schema.dimension_parameters().is_empty()
    }) {
        Ok(())
    } else {
        Err(Error::UnsupportedSchema { schema })
    }
}

fn source_schema(artifact: &ProgramArtifact, source: ArtifactSource) -> Result<SchemaId, Error> {
    match source {
        ArtifactSource::Slot(slot) => artifact
            .slots()
            .get(slot.get() as usize)
            .filter(|declaration| declaration.slot == slot)
            .map(|declaration| declaration.schema),
        ArtifactSource::Constant(constant) => artifact
            .constants()
            .get(constant)
            .map(|value| value.schema()),
    }
    .ok_or(Error::InvalidBinding { node: None })
}

fn site<'a>(
    artifact: &'a ProgramArtifact,
    node: &'a NodeDeclaration,
    requirements: &'a [ExecutionResourceRequest],
) -> Result<
    (
        ApplicationRequirementId,
        &'a ExecutionResourceRequest,
        &'a mech_core::DeclaredOperationContract,
    ),
    Error,
> {
    let ExecutableNodeBody::Operation(operation) = &node.body else {
        return Err(unsupported(
            node.node,
            "resource boundary must be an original root operation",
        ));
    };
    let requirement = operation.requirement.ok_or(Error::InvalidBinding {
        node: Some(node.node),
    })?;
    let request = requirements
        .get(requirement.get() as usize)
        .ok_or(Error::InvalidBinding {
            node: Some(node.node),
        })?;
    if artifact.requirements().get(requirement)
        != Some(&ApplicationRequirement::Resource(request.clone()))
    {
        return Err(Error::InvalidBinding {
            node: Some(node.node),
        });
    }
    let Some(ResolvedOperationContract::Declared(contract)) =
        artifact.contracts().get(operation.contract)
    else {
        return Err(unsupported(
            node.node,
            "resource boundary requires its declared contract",
        ));
    };
    Ok((requirement, request, contract))
}

pub(super) struct Observation {
    pub(super) requirement: ApplicationRequirementId,
    pub(super) request: ExecutionResourceRequest,
    pub(super) slot: CellSlotId,
    pub(super) schema: SchemaId,
}
pub(super) fn observation(
    artifact: &ProgramArtifact,
    node: &NodeDeclaration,
    requirements: &[ExecutionResourceRequest],
) -> Result<Observation, Error> {
    let (requirement, request, contract) = site(artifact, node, requirements)?;
    let bad = || {
        unsupported(
            node.node,
            "unsupported native observation policy or root bindings",
        )
    };
    let ExecutableNodeBody::Operation(operation) = &node.body else {
        return Err(bad());
    };
    if request.intent != ResourceIntent::Read
        || request.delivery != ResourceDelivery::Live
        || operation.operation.module_path.as_ref() != ["resource", "read"]
        || operation.operation.operation_name != "read"
        || contract.interaction
            != ExternalInteraction::Observation(ObservationContract {
                replay: ObservationReplayPolicy::CaptureAsInputFact,
            })
        || !contract.inputs.is_empty()
        || !node.input_bindings.is_empty()
    {
        return Err(bad());
    }
    let [output] = contract.outputs.as_ref() else {
        return Err(bad());
    };
    let [
        BindingDeclaration::Output {
            node: owner,
            port_ordinal: 0,
            target,
            ..
        },
    ] = &artifact.bindings()
        [node.output_bindings.start as usize..node.output_bindings.end as usize]
    else {
        return Err(bad());
    };
    let declaration = artifact
        .slots()
        .get(target.get() as usize)
        .ok_or_else(bad)?;
    if *owner != node.node
        || declaration.slot != *target
        || declaration.schema != output.schema
        || declaration.role != SlotRole::Derived
        || declaration.producer
            != (ProducerReference::NodeOutput {
                node: node.node,
                output_ordinal: 0,
            })
        || declaration.initializer.is_some()
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::AlwaysChanged
    {
        return Err(bad());
    }
    string_schema(artifact, output.schema)?;
    Ok(Observation {
        requirement,
        request: request.clone(),
        slot: *target,
        schema: output.schema,
    })
}

pub(super) struct Effect {
    pub(super) requirement: ApplicationRequirementId,
    pub(super) request: ExecutionResourceRequest,
    pub(super) payload: ArtifactSource,
    pub(super) schema: SchemaId,
}
pub(super) fn effect(
    artifact: &ProgramArtifact,
    node: &NodeDeclaration,
    requirements: &[ExecutionResourceRequest],
) -> Result<Effect, Error> {
    let (requirement, request, contract) = site(artifact, node, requirements)?;
    let bad = || unsupported(node.node, "unsupported native Send policy or root bindings");
    let ExecutableNodeBody::Operation(operation) = &node.body else {
        return Err(bad());
    };
    let operation_matches = (operation.operation.module_path.as_ref() == ["resource"]
        && operation.operation.operation_name == "send")
        || (operation.operation.module_path.as_ref() == ["resource", "send"]
            && operation.operation.operation_name == request.operation);
    if request.intent != ResourceIntent::Send
        || request.delivery != ResourceDelivery::Snapshot
        || !operation_matches
        || contract.interaction
            != ExternalInteraction::Effect(EffectContract {
                delivery: EffectDeliveryPolicy::AtMostOnce,
                idempotency: IdempotencyRequirement::NotRequired,
            })
        || !contract.outputs.is_empty()
        || !node.output_bindings.is_empty()
    {
        return Err(bad());
    }
    let [input] = contract.inputs.as_ref() else {
        return Err(bad());
    };
    let [
        BindingDeclaration::Input {
            node: owner,
            port_ordinal: 0,
            source,
            ..
        },
    ] = &artifact.bindings()[node.input_bindings.start as usize..node.input_bindings.end as usize]
    else {
        return Err(bad());
    };
    if *owner != node.node
        || input.access != AccessMode::Read
        || input.delivery != DeliveryMode::Signal
        || source_schema(artifact, *source)? != input.schema
    {
        return Err(bad());
    }
    string_schema(artifact, input.schema)?;
    Ok(Effect {
        requirement,
        request: request.clone(),
        payload: *source,
        schema: input.schema,
    })
}

#[cfg(all(test, feature = "semantic-compiler", feature = "artifact-codec"))]
mod tests {
    use super::*;
    use crate::native_artifact::{
        NativeIterationAction, NativeIterationValueScope, NativeMatchValue,
        lower_native_program_artifact, lower_scalar_iteration_artifact,
    };
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    use std::{
        collections::BTreeMap,
        sync::{Arc, LazyLock},
    };

    static EFFECT: LazyLock<mech_core::OperationContractDeclaration> = LazyLock::new(|| {
        let mut contract = crate::function::external::RESOURCE_EFFECT_CONTRACT.clone();
        contract.interaction = ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        });
        contract
    });
    struct Resolver;
    impl crate::ExternalRequirementContractResolver for Resolver {
        fn resolve_external_contract(
            &self,
            requirement: &ApplicationRequirement,
        ) -> mech_core::MResult<Option<&'static mech_core::OperationContractDeclaration>> {
            Ok(match requirement {
                ApplicationRequirement::Resource(request)
                    if request.intent == ResourceIntent::Read =>
                {
                    Some(&*crate::function::external::RESOURCE_OBSERVATION_CONTRACT)
                }
                ApplicationRequirement::Resource(_) => Some(&*EFFECT),
                ApplicationRequirement::HostFunction(_) => None,
            })
        }
    }
    fn request(intent: ResourceIntent, context: &str) -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: format!("test://{context}"),
            path: "text".into(),
            context_name: context.into(),
            operation: if intent == ResourceIntent::Read {
                "read"
            } else {
                "write"
            }
            .into(),
            intent,
            delivery: if intent == ResourceIntent::Read {
                ResourceDelivery::Live
            } else {
                ResourceDelivery::Snapshot
            },
        }
    }
    fn catalog() -> Arc<mech_core::FunctionCatalog> {
        Arc::new(mech_core::FunctionCatalogBuilder::new().build().unwrap())
    }
    fn fixture(kind: &str) -> ProgramArtifact {
        let source = format!(
            "sample := input<{kind}>\nmarker := seed<u64>\n@out/text <- sample\nsample\nmarker\n"
        );
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5245534e), Revision(1), source.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        crate::CanonicalSourceFrontend
            .compile_document_with_catalog_and_resources(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog(),
                BTreeMap::new(),
                BTreeMap::from([("input".into(), request(ResourceIntent::Read, "stdin"))]),
                BTreeMap::from([("@out/text".into(), request(ResourceIntent::Send, "stdout"))]),
            )
            .unwrap()
            .compile_artifact_with_external_contracts(&Resolver)
            .unwrap()
    }
    fn roundtrips(artifact: ProgramArtifact) -> [ProgramArtifact; 2] {
        let decoded = crate::decode_program_artifact_bytecode_v1(
            &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded.revision(), artifact.revision());
        [artifact, decoded]
    }
    fn with_unused(
        artifact: &ProgramArtifact,
        requirement: ApplicationRequirement,
    ) -> ProgramArtifact {
        let mut entries = artifact
            .requirements()
            .iter()
            .map(|(_, r)| r.clone())
            .collect::<Vec<_>>();
        entries.push(requirement);
        entries.sort_by(mech_core::compare_application_requirements);
        entries.dedup();
        let requirements =
            crate::ApplicationRequirementTable::from_canonical_entries(entries).unwrap();
        let mut nodes = artifact.nodes().to_vec();
        for node in &mut nodes {
            let ExecutableNodeBody::Operation(operation) = &mut node.body else {
                continue;
            };
            if let Some(old) = operation.requirement {
                let original = artifact.requirements().get(old).unwrap();
                operation.requirement = Some(
                    requirements
                        .iter()
                        .find_map(|(id, request)| (request == original).then_some(id))
                        .unwrap(),
                );
            }
        }
        crate::ProgramArtifactDraft {
            schemas: artifact.schemas().clone(),
            constants: artifact.constants().clone(),
            contracts: artifact.contracts().clone(),
            requirements,
            inputs: artifact.inputs().into(),
            slots: artifact.slots().into(),
            nodes: nodes.into(),
            bindings: artifact.bindings().into(),
            outputs: artifact.outputs().into(),
            constraints: artifact.constraints().into(),
            compute_regions: artifact.compute_regions().into(),
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn original_canonical_resources_become_transport_without_fake_application_ports_or_calls() {
        for artifact in roundtrips(fixture("string")) {
            let plan = lower_native_program_artifact(&artifact, &catalog()).unwrap();
            plan.validate_current_visits().unwrap();
            assert_eq!(plan.program_revision(), artifact.revision());
            assert_eq!(plan.observations().len(), 1);
            assert_eq!(plan.effects().len(), 1);
            assert_eq!(
                plan.resource_requirements().len(),
                artifact.requirements().len()
            );
            for ((id, original), selected) in artifact
                .requirements()
                .iter()
                .zip(plan.resource_requirements())
            {
                assert_eq!(
                    original,
                    &ApplicationRequirement::Resource(selected.clone())
                );
                assert_eq!(artifact.requirements().get(id), Some(original));
            }
            assert_eq!(plan.inputs().len(), artifact.inputs().len());
            for (selected, original) in plan.inputs().iter().zip(artifact.inputs()) {
                assert_eq!(selected.input(), original.input);
                assert_eq!(selected.slot(), original.slot);
            }
            assert_eq!(plan.outputs().len(), artifact.outputs().len());
            for (selected, original) in plan.outputs().iter().zip(artifact.outputs()) {
                assert_eq!(selected.output(), original.output);
                assert_eq!(selected.slot(), original.source);
            }
            assert_eq!(plan.transport_input_count(), artifact.inputs().len() + 1);
            assert_eq!(plan.transport_output_count(), artifact.outputs().len() + 1);
            for (index, input) in plan.inputs().iter().enumerate() {
                assert_eq!(plan.transport_input_value(index), input.value());
                assert_eq!(plan.transport_input_type(index), input.ty());
            }
            let observation = &plan.observations()[0];
            assert_eq!(observation.ordinal(), 0);
            assert_eq!(observation.request().delivery, ResourceDelivery::Live);
            assert_eq!(
                plan.transport_input_value(plan.inputs().len()),
                observation.value()
            );
            assert_eq!(
                plan.transport_input_type(plan.inputs().len()),
                observation.ty()
            );
            let effect = &plan.effects()[0];
            assert_eq!(effect.ordinal(), 0);
            assert_eq!(effect.request().intent, ResourceIntent::Send);
            assert_eq!(
                plan.transport_output_value(plan.outputs().len()),
                effect.value()
            );
            assert_eq!(
                plan.transport_output_type(plan.outputs().len()),
                effect.ty()
            );
            assert!(
                artifact.nodes()[effect.node().get() as usize]
                    .output_bindings
                    .is_empty()
            );
            assert!(
                plan.blocks().iter().all(|block| !matches!(
                    block.action(),
                    NativeIterationAction::LibraryCall { .. }
                ))
            );
            assert!(matches!(
                lower_scalar_iteration_artifact(&artifact, &catalog()),
                Err(Error::UnsupportedProgram {
                    feature: "resource ports in a scalar plan"
                })
            ));
        }
    }

    #[test]
    fn all_unused_requirements_remain_obligations_and_unsupported_authority_stays_rejected() {
        let original = fixture("string");
        let extra = request(ResourceIntent::Send, "unused");
        for artifact in roundtrips(with_unused(
            &original,
            ApplicationRequirement::Resource(extra.clone()),
        )) {
            let plan = lower_native_program_artifact(&artifact, &catalog()).unwrap();
            assert_eq!(plan.resource_requirements().len(), 3);
            assert!(plan.resource_requirements().contains(&extra));
            assert_eq!((plan.observations().len(), plan.effects().len()), (1, 1));
        }
        let mut snapshot = request(ResourceIntent::Read, "snapshot");
        snapshot.delivery = ResourceDelivery::Snapshot;
        for unsupported in [
            ApplicationRequirement::Resource(request(ResourceIntent::Assign, "assignment")),
            ApplicationRequirement::Resource(snapshot),
            ApplicationRequirement::HostFunction(mech_core::ExecutionHostFunctionRequest {
                name: "unbound".into(),
            }),
        ] {
            let artifact = with_unused(&original, unsupported);
            assert!(lower_native_program_artifact(&artifact, &catalog()).is_err());
        }
        assert!(matches!(
            lower_native_program_artifact(&fixture("u64"), &catalog()),
            Err(Error::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn current_visit_proof_initializes_observations_and_requires_effects_at_complete() {
        let plan = lower_native_program_artifact(&fixture("string"), &catalog()).unwrap();
        let mut duplicate = plan.clone();
        duplicate.observations[0].value = duplicate.inputs[0].value;
        assert!(duplicate.validate_current_visits().is_err());
        let mut missing = plan.clone();
        let undefined = NativeMatchValue(missing.values.len() as u32);
        let mut values = missing.values.into_vec();
        values.push(missing.effects[0].ty.clone());
        missing.values = values.into();
        let mut scopes = missing.value_scopes.into_vec();
        scopes.push(NativeIterationValueScope::Immutable { iteration: None });
        missing.value_scopes = scopes.into();
        missing.effects[0].value = undefined;
        assert!(missing.validate_current_visits().is_err());
        let mut wrong_request = plan;
        wrong_request.observations[0]
            .request
            .context_name
            .push_str("-forged");
        assert!(wrong_request.validate_current_visits().is_err());
    }

    #[test]
    fn multiple_effects_keep_artifact_ordinals_but_use_the_canonical_dependency_schedule() {
        use crate::{
            ArtifactBuildContext, ControlBlock, ControlBlockId, ControlMatchArm, ControlParameter,
            ControlParameterSource, ControlValue, MatchDeclaration, MatchPattern,
            OperationReference, SourceInput, SourceNode, SourceNodeBody, SourceNodeOutput,
            SourceProgram, SourceValue,
        };
        let mut builder = mech_core::SchemaTableBuilder::new();
        let handle = builder
            .insert(
                mech_core::SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let schema = built.resolve(handle).unwrap();
        let (schemas, _) = built.into_parts();
        let constants = mech_core::ConstantStoreBuilder::new(&schemas)
            .finish()
            .unwrap()
            .store;
        let send = || SourceNodeBody::Operation {
            operation: OperationReference {
                module_path: Box::new(["resource".into()]),
                operation_name: "send".into(),
            },
            requirement: Some(ApplicationRequirementId::new(0)),
        };
        let graph = SourceProgram {
            requirements: crate::ApplicationRequirementTable::from_canonical_entries(vec![
                ApplicationRequirement::Resource(request(ResourceIntent::Send, "stdout")),
            ])
            .unwrap(),
            inputs: Box::new([SourceInput {
                name: "input".into(),
                schema,
            }]),
            nodes: Box::new([
                SourceNode {
                    body: send(),
                    inputs: Box::new([SourceValue::NodeOutput {
                        node: 2,
                        output_ordinal: 0,
                    }]),
                    outputs: Box::new([]),
                },
                SourceNode {
                    body: send(),
                    inputs: Box::new([SourceValue::Input(0)]),
                    outputs: Box::new([]),
                },
                SourceNode {
                    body: SourceNodeBody::Match(MatchDeclaration {
                        scrutinee: 0,
                        partial: false,
                        captures: Box::new([]),
                        arms: Box::new([ControlMatchArm {
                            pattern: MatchPattern::Bind,
                            guard: None,
                            body: ControlBlock {
                                id: ControlBlockId(0),
                                parameters: Box::new([ControlParameter {
                                    source: ControlParameterSource::Scrutinee,
                                    schema,
                                }]),
                                operations: Box::new([]),
                                yield_value: ControlValue::Parameter {
                                    block: ControlBlockId(0),
                                    ordinal: 0,
                                },
                            },
                        }]),
                    }),
                    inputs: Box::new([SourceValue::Input(0)]),
                    outputs: Box::new([SourceNodeOutput::Derived { schema }]),
                },
            ]),
            ..SourceProgram::default()
        };
        let original = crate::compile_source_program_with_control_contracts(
            &graph,
            &mut ArtifactBuildContext::new(&schemas, &constants),
            &[Some(&*EFFECT), Some(&*EFFECT), None],
        )
        .unwrap();
        for artifact in roundtrips(original) {
            let order = crate::resident::canonical_resource_effect_order(&artifact).unwrap();
            assert_eq!(
                order.as_ref(),
                &[mech_core::NodeId::new(1), mech_core::NodeId::new(0)]
            );
            let plan = lower_native_program_artifact(&artifact, &catalog()).unwrap();
            plan.validate_current_visits().unwrap();
            assert!(
                plan.outputs().is_empty(),
                "no synthetic output for either Send"
            );
            assert_eq!(plan.transport_output_count(), 2);
            assert_eq!(
                plan.effects()
                    .iter()
                    .map(|effect| (effect.node(), effect.ordinal()))
                    .collect::<Vec<_>>(),
                [
                    (mech_core::NodeId::new(1), 1),
                    (mech_core::NodeId::new(0), 0)
                ]
            );
        }
    }
}
