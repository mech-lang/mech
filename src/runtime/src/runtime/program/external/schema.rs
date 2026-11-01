//! Resource structure can be admitted without evaluating an observation.

use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, CellSlotId, DeclaredOperationContract,
    ExecutionResourceRequest, ExternalInteraction, MResult, MechError, NodeId,
    OperationContractDeclaration, ResolvedOperationContract, ResourceIntent, Schema, SchemaId,
    Value,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ComprehensionStep, ControlOperationBody,
    ExecutableNodeBody, MatchDeclaration, ProgramArtifact, SlotRole,
};

use crate::ResourceSchemaPreflight;

/// One typed resource operation, including effects dormant on the initial turn.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactResourceSchemaSite<'a> {
    pub node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub request: &'a ExecutionResourceRequest,
    pub contract: &'a DeclaredOperationContract,
    pub schema: &'a Schema,
    pub schema_id: SchemaId,
    pub payload_source: Option<ArtifactSource>,
    pub observation_output: Option<CellSlotId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactResourceSchemaPreflight {
    pub node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub intent: ResourceIntent,
    pub admission: ResourceSchemaPreflight,
}

/// A deferred entry records uncertainty, never a successful payload-type proof.
/// Actual provider preparation remains mandatory for both dispositions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactResourceSchemaPreflightReport {
    pub resources: Vec<ArtifactResourceSchemaPreflight>,
}

fn invalid(node: NodeId, reason: impl Into<String>) -> MechError {
    MechError::new(
        super::ResidentProviderContractMismatch {
            node,
            reason: reason.into(),
        },
        None,
    )
}

/// Visit the validated artifact's resource ports in node order. No graph is
/// activated, no payload is reconstructed, and no provider is observed here.
pub fn preflight_artifact_resource_schemas(
    artifact: &ProgramArtifact,
    mut preflight: impl FnMut(ArtifactResourceSchemaSite<'_>) -> MResult<ResourceSchemaPreflight>,
) -> MResult<ArtifactResourceSchemaPreflightReport> {
    let mut report = ArtifactResourceSchemaPreflightReport::default();
    for node in artifact.nodes() {
        let Some(operation) = node.as_operation() else {
            continue;
        };
        let Some(requirement) = operation.requirement else {
            continue;
        };
        let Some(ApplicationRequirement::Resource(request)) =
            artifact.requirements().get(requirement)
        else {
            return Err(invalid(
                node.node,
                "resource operation has no resource requirement",
            ));
        };
        let Some(ResolvedOperationContract::Declared(contract)) =
            artifact.contracts().get(operation.contract)
        else {
            return Err(invalid(
                node.node,
                "resource operation contract is not declared",
            ));
        };
        let (schema_id, payload_source, observation_output) = match request.intent {
            ResourceIntent::Read => {
                let [output] = contract.outputs.as_ref() else {
                    return Err(invalid(node.node, "resource read requires one output"));
                };
                if !contract.inputs.is_empty() {
                    return Err(invalid(
                        node.node,
                        "resource read cannot have payload inputs",
                    ));
                }
                let [BindingDeclaration::Output { target, .. }] = &artifact.bindings()
                    [node.output_bindings.start as usize..node.output_bindings.end as usize]
                else {
                    return Err(invalid(node.node, "resource read output binding is absent"));
                };
                (output.schema, None, Some(*target))
            }
            ResourceIntent::Assign | ResourceIntent::Send => {
                let [input] = contract.inputs.as_ref() else {
                    return Err(invalid(
                        node.node,
                        "resource write requires one payload input",
                    ));
                };
                if !contract.outputs.is_empty() {
                    return Err(invalid(node.node, "resource write cannot have outputs"));
                }
                let [BindingDeclaration::Input { source, .. }] = &artifact.bindings()
                    [node.input_bindings.start as usize..node.input_bindings.end as usize]
                else {
                    return Err(invalid(
                        node.node,
                        "resource write payload binding is absent",
                    ));
                };
                (input.schema, Some(*source), None)
            }
        };
        // ProgramArtifact construction already verifies that these contract
        // schema IDs equal the source Constant/Slot and output Slot schemas.
        let schema = artifact
            .schemas()
            .get(schema_id)
            .ok_or_else(|| invalid(node.node, "resource port schema is absent"))?;
        let admission = preflight(ArtifactResourceSchemaSite {
            node: node.node,
            requirement,
            request,
            contract,
            schema,
            schema_id,
            payload_source,
            observation_output,
        })?;
        report.resources.push(ArtifactResourceSchemaPreflight {
            node: node.node,
            requirement,
            intent: request.intent,
            admission,
        });
    }
    Ok(report)
}

/// Check the same provider policies used at actual resident binding time.
pub fn validate_artifact_resource_schema_contract(
    site: &ArtifactResourceSchemaSite<'_>,
    provider: Option<&OperationContractDeclaration>,
) -> MResult<()> {
    if provider.is_some_and(|provider| super::provider::contract_matches(site.contract, provider)) {
        Ok(())
    } else {
        Err(invalid(
            site.node,
            "provider semantic contract differs from artifact",
        ))
    }
}

/// Validate a non-consuming read representative against the retained port.
/// Its data is never bound as a constant or supplied to an executable turn.
pub fn validate_artifact_resource_read_schema(
    artifact: &ProgramArtifact,
    site: &ArtifactResourceSchemaSite<'_>,
    representative: &Value,
) -> MResult<()> {
    if site.request.intent != ResourceIntent::Read {
        return Err(invalid(
            site.node,
            "read schema check requires a read resource",
        ));
    }
    representative
        .rebind(site.schema_id, representative.shape(), artifact.schemas())
        .map(|_| ())
        .map_err(|error| {
            invalid(
                site.node,
                format!(
                    "provider read representation differs from artifact schema or shape: {error:?}"
                ),
            )
        })
}

/// Value-domain checks may additionally evaluate a closed, pure application.
/// An input or observation representative is never evidence of a constant.
pub fn artifact_has_closed_resource_payloads(artifact: &ProgramArtifact) -> bool {
    if !artifact.inputs().is_empty()
        || artifact
            .slots()
            .iter()
            .any(|slot| slot.role == SlotRole::State)
    {
        return false;
    }
    artifact.nodes().iter().all(|node| match &node.body {
        ExecutableNodeBody::Operation(operation) => {
            let Some(ResolvedOperationContract::Declared(contract)) =
                artifact.contracts().get(operation.contract)
            else {
                return false;
            };
            match operation.requirement {
                None => contract.interaction == ExternalInteraction::Pure,
                Some(requirement) => matches!(artifact.requirements().get(requirement),
                    Some(ApplicationRequirement::Resource(request))
                        if request.intent != ResourceIntent::Read
                            && matches!(contract.interaction,
                                ExternalInteraction::Effect(_)
                                    | ExternalInteraction::TransactionalExternal(_))),
            }
        }
        ExecutableNodeBody::Match(control) => pure_match(artifact, control),
        ExecutableNodeBody::Comprehension(control) => control.steps.iter().all(|step| match step {
            ComprehensionStep::Operation(operation) => pure_control(artifact, &operation.body),
            _ => true,
        }),
        ExecutableNodeBody::Iteration(control) => control
            .body
            .operations
            .iter()
            .all(|operation| pure_control(artifact, &operation.body)),
        ExecutableNodeBody::Activation(_) | ExecutableNodeBody::Fsm(_) => false,
    })
}

fn pure_match(artifact: &ProgramArtifact, control: &MatchDeclaration) -> bool {
    control.arms.iter().all(|arm| {
        arm.guard
            .iter()
            .chain(core::iter::once(&arm.body))
            .all(|block| {
                block
                    .operations
                    .iter()
                    .all(|operation| pure_control(artifact, &operation.body))
            })
    })
}

fn pure_control(artifact: &ProgramArtifact, body: &ControlOperationBody) -> bool {
    match body {
        ControlOperationBody::Operation { contract, .. } => matches!(
            artifact.contracts().get(*contract),
            Some(ResolvedOperationContract::Declared(contract))
                if contract.interaction == ExternalInteraction::Pure
        ),
        ControlOperationBody::Match(control) => pure_match(artifact, control),
        ControlOperationBody::Comprehension(control) => {
            control.steps.iter().all(|step| match step {
                ComprehensionStep::Operation(operation) => pure_control(artifact, &operation.body),
                _ => true,
            })
        }
        ControlOperationBody::Iteration(control) => control
            .body
            .operations
            .iter()
            .all(|operation| pure_control(artifact, &operation.body)),
        ControlOperationBody::Recur(_) => true,
        ControlOperationBody::Suspend | ControlOperationBody::Publish => false,
    }
}
