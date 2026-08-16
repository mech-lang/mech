use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, DeclaredOperationContract,
    EffectDeliveryPolicy, ExternalInteraction, IdempotencyRequirement, MResult, MechError,
    MechErrorKind, NodeId, OperationContractDeclaration, ResolvedOperationContract, ResourceIntent,
};
use mech_engine::{
    ProgramArtifact,
    resident::{ActivatedInput, ActivatedInputSource, ActivatedPlan, ActivatedTurnStep},
};

use crate::{
    RuntimeResourceRegistry, RuntimeResourceWriteIntent, resource::RuntimeResidentProviderBinding,
};

use super::ResidentExternalAuthority;

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy, Debug)]
pub struct ResidentExternalContractResolver<'a> {
    providers: &'a RuntimeResourceRegistry,
}

#[cfg(feature = "semantic-compiler")]
impl<'a> ResidentExternalContractResolver<'a> {
    pub const fn new(providers: &'a RuntimeResourceRegistry) -> Self {
        Self { providers }
    }
}

#[cfg(feature = "semantic-compiler")]
impl mech_engine::ExternalRequirementContractResolver for ResidentExternalContractResolver<'_> {
    fn resolve_external_contract(
        &self,
        requirement: &ApplicationRequirement,
    ) -> MResult<Option<&'static OperationContractDeclaration>> {
        let ApplicationRequirement::Resource(request) = requirement else {
            return Ok(None);
        };
        let binding = self
            .providers
            .resident_provider_binding(&request.base_uri)?;
        match request.intent {
            ResourceIntent::Read => binding.semantic_read_contract(),
            ResourceIntent::Assign | ResourceIntent::Send => binding
                .semantic_write_contract(write_intent(request).expect("write intent was matched")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BoundResidentObservation {
    pub input: ActivatedInput,
    pub node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub request: mech_core::ExecutionResourceRequest,
    pub(crate) provider_binding: Option<RuntimeResidentProviderBinding>,
}

#[derive(Clone, Debug)]
pub struct BoundResidentEffect {
    pub node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub request: mech_core::ExecutionResourceRequest,
    pub interaction: ExternalInteraction,
    pub ordinal: u32,
    pub(crate) provider_binding: Option<RuntimeResidentProviderBinding>,
}

#[derive(Clone, Debug)]
pub struct BoundResidentExternalPlan {
    observations: Box<[BoundResidentObservation]>,
    effects: Box<[BoundResidentEffect]>,
}

impl BoundResidentExternalPlan {
    pub fn observations(&self) -> &[BoundResidentObservation] {
        &self.observations
    }

    pub fn effects(&self) -> &[BoundResidentEffect] {
        &self.effects
    }

    pub fn ordinary_effect_count(&self) -> usize {
        self.effects
            .iter()
            .filter(|effect| matches!(effect.interaction, ExternalInteraction::Effect(_)))
            .count()
    }
}

pub fn bind_external_requirements(
    plan: &ActivatedPlan,
    artifact: &ProgramArtifact,
    providers: &RuntimeResourceRegistry,
    authority: &dyn ResidentExternalAuthority,
) -> MResult<BoundResidentExternalPlan> {
    let mut bound = bind_replay_requirements(plan, artifact)?;
    for observation in &mut bound.observations {
        authorize_requirement(
            artifact,
            observation.requirement,
            observation.node,
            authority,
        )?;
        let request = &observation.request;
        let provider_binding = providers.resident_provider_binding(&request.base_uri)?;
        let provider_contract = provider_binding.semantic_read_contract()?.ok_or_else(|| {
            contract_mismatch(observation.node, "provider has no semantic read contract")
        })?;
        compare_contract(artifact, observation.node, provider_contract)?;
        observation.provider_binding = Some(provider_binding);
    }
    for effect in &mut bound.effects {
        authorize_requirement(artifact, effect.requirement, effect.node, authority)?;
        let request = &effect.request;
        let intent =
            write_intent(request).expect("replay binding validated the resident write intent");
        let provider_binding = providers.resident_provider_binding(&request.base_uri)?;
        let provider_contract = provider_binding
            .semantic_write_contract(intent)?
            .ok_or_else(|| {
                contract_mismatch(effect.node, "provider has no semantic write contract")
            })?;
        compare_contract(artifact, effect.node, provider_contract)?;
        if requires_provider_idempotency(&effect.interaction)
            && !provider_binding.supports_idempotency(intent)?
        {
            return invalid_binding(
                effect.node,
                "provider does not support the effect's idempotency requirement",
            );
        }
        effect.provider_binding = Some(provider_binding);
    }
    Ok(bound)
}

/// Binds only immutable artifact identities for deterministic replay.
///
/// This path deliberately performs no provider lookup or live authorization:
/// replay consumes canonical captured facts and suppresses every external
/// interaction. Live execution upgrades this plan with provider bindings in
/// [`bind_external_requirements`].
pub fn bind_replay_requirements(
    plan: &ActivatedPlan,
    artifact: &ProgramArtifact,
) -> MResult<BoundResidentExternalPlan> {
    let mut observations = Vec::new();
    for input in &plan.inputs {
        let ActivatedInputSource::Observation { node, requirement } = input.source else {
            continue;
        };
        let request = resource_requirement(artifact, requirement, node)?;
        if request.intent != ResourceIntent::Read {
            return invalid_binding(node, "observation requirement is not a read");
        }
        observations.push(BoundResidentObservation {
            input: input.clone(),
            node,
            requirement,
            request: request.clone(),
            provider_binding: None,
        });
    }

    let mut effects = Vec::new();
    for step in &plan.steps {
        let ActivatedTurnStep::External(external) = step else {
            continue;
        };
        let request = resource_requirement(artifact, external.requirement, external.artifact_node)?;
        if write_intent(request).is_none() {
            return invalid_binding(
                external.artifact_node,
                "effect requirement is not assign/send",
            );
        }
        validate_effect_admission(external.artifact_node, &external.interaction)?;
        effects.push(BoundResidentEffect {
            node: external.artifact_node,
            requirement: external.requirement,
            request: request.clone(),
            interaction: external.interaction.clone(),
            ordinal: external.effect_ordinal,
            provider_binding: None,
        });
    }
    effects.sort_unstable_by_key(|effect| effect.ordinal);
    Ok(BoundResidentExternalPlan {
        observations: observations.into_boxed_slice(),
        effects: effects.into_boxed_slice(),
    })
}

fn resource_requirement(
    artifact: &ProgramArtifact,
    id: ApplicationRequirementId,
    node: NodeId,
) -> MResult<&mech_core::ExecutionResourceRequest> {
    let requirement = artifact.requirements().get(id).ok_or_else(|| {
        invalid_binding_error(node, "activated requirement is absent from artifact")
    })?;
    match requirement {
        ApplicationRequirement::Resource(request) => Ok(request),
        ApplicationRequirement::HostFunction(_) => Err(MechError::new(
            UnsupportedResidentExternalRequirement {
                reason: "host-function requirements are not supported".to_owned(),
            },
            None,
        )),
    }
}

fn authorize_requirement(
    artifact: &ProgramArtifact,
    id: ApplicationRequirementId,
    node: NodeId,
    authority: &dyn ResidentExternalAuthority,
) -> MResult<()> {
    let requirement = artifact.requirements().get(id).ok_or_else(|| {
        invalid_binding_error(node, "activated requirement is absent from artifact")
    })?;
    authority.authorize(requirement)
}

fn validate_effect_admission(node: NodeId, interaction: &ExternalInteraction) -> MResult<()> {
    let ExternalInteraction::Effect(contract) = interaction else {
        return Ok(());
    };
    match contract.delivery {
        EffectDeliveryPolicy::ProviderDefined => {
            invalid_binding(node, "ProviderDefined delivery is not resident-admissible")
        }
        EffectDeliveryPolicy::AtMostOnce
        | EffectDeliveryPolicy::AtLeastOnce
        | EffectDeliveryPolicy::IdempotentRetry => Ok(()),
    }
}

fn requires_provider_idempotency(interaction: &ExternalInteraction) -> bool {
    matches!(
        interaction,
        ExternalInteraction::Effect(mech_core::EffectContract {
            delivery: EffectDeliveryPolicy::IdempotentRetry,
            ..
        }) | ExternalInteraction::Effect(mech_core::EffectContract {
            idempotency: IdempotencyRequirement::Required,
            ..
        })
    )
}

fn write_intent(
    request: &mech_core::ExecutionResourceRequest,
) -> Option<RuntimeResourceWriteIntent> {
    match request.intent {
        ResourceIntent::Assign => Some(RuntimeResourceWriteIntent::Assign),
        ResourceIntent::Send => Some(RuntimeResourceWriteIntent::Send),
        ResourceIntent::Read => None,
    }
}

fn compare_contract(
    artifact: &ProgramArtifact,
    node: NodeId,
    provider: &OperationContractDeclaration,
) -> MResult<()> {
    let declaration = artifact
        .nodes()
        .get(node.get() as usize)
        .ok_or_else(|| contract_mismatch(node, "artifact node is missing"))?;
    let Some(ResolvedOperationContract::Declared(artifact_contract)) =
        artifact.contracts().get(declaration.contract)
    else {
        return Err(contract_mismatch(node, "artifact contract is not declared"));
    };
    if !contract_matches(artifact_contract, provider) {
        return Err(contract_mismatch(
            node,
            "provider semantic contract differs from artifact",
        ));
    }
    Ok(())
}

fn contract_matches(
    artifact: &DeclaredOperationContract,
    provider: &OperationContractDeclaration,
) -> bool {
    let Ok(inputs) = provider.inputs.resolve(artifact.inputs.len()) else {
        return false;
    };
    inputs
        .iter()
        .zip(&artifact.inputs)
        .all(|(left, right)| left.access == right.access && left.delivery == right.delivery)
        && provider.outputs.len() == artifact.outputs.len()
        && provider
            .outputs
            .iter()
            .zip(&artifact.outputs)
            .all(|(left, right)| {
                left.access == right.access
                    && left.delivery == right.delivery
                    && left.construction == right.construction
                    && left.alias == right.alias
                    && left.change_detection == right.change_detection
            })
        && provider.interaction == artifact.interaction
}

fn contract_mismatch(node: NodeId, reason: &'static str) -> MechError {
    MechError::new(
        ResidentProviderContractMismatch {
            node,
            reason: reason.to_owned(),
        },
        None,
    )
}

fn invalid_binding<T>(node: NodeId, reason: &'static str) -> MResult<T> {
    Err(invalid_binding_error(node, reason))
}

fn invalid_binding_error(node: NodeId, reason: &'static str) -> MechError {
    MechError::new(
        ResidentExternalBindingInvalid {
            node,
            reason: reason.to_owned(),
        },
        None,
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentProviderContractMismatch {
    pub node: NodeId,
    pub reason: String,
}

impl MechErrorKind for ResidentProviderContractMismatch {
    fn name(&self) -> &str {
        "ResidentProviderContractMismatch"
    }
    fn message(&self) -> String {
        format!(
            "resident provider contract mismatch at node {}: {}",
            self.node.get(),
            self.reason
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentExternalBindingInvalid {
    pub node: NodeId,
    pub reason: String,
}

impl MechErrorKind for ResidentExternalBindingInvalid {
    fn name(&self) -> &str {
        "ResidentExternalBindingInvalid"
    }
    fn message(&self) -> String {
        format!(
            "resident external binding at node {} is invalid: {}",
            self.node.get(),
            self.reason
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedResidentExternalRequirement {
    pub reason: String,
}

impl MechErrorKind for UnsupportedResidentExternalRequirement {
    fn name(&self) -> &str {
        "UnsupportedResidentExternalRequirement"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}

#[cfg(test)]
mod tests {
    use mech_core::{EffectContract, EffectDeliveryPolicy, IdempotencyRequirement};

    use super::{
        ExternalInteraction, NodeId, requires_provider_idempotency, validate_effect_admission,
    };

    #[test]
    fn resident_retry_policy_and_idempotency_claims_are_coherent() {
        let node = NodeId::new(7);
        let effect = |delivery, idempotency| {
            ExternalInteraction::Effect(EffectContract {
                delivery,
                idempotency,
            })
        };
        assert!(
            validate_effect_admission(
                node,
                &effect(
                    EffectDeliveryPolicy::IdempotentRetry,
                    IdempotencyRequirement::Required,
                ),
            )
            .is_ok()
        );
        let optional_retry = effect(
            EffectDeliveryPolicy::IdempotentRetry,
            IdempotencyRequirement::Optional,
        );
        assert!(validate_effect_admission(node, &optional_retry).is_ok());
        assert!(requires_provider_idempotency(&optional_retry));

        let declared_required = effect(
            EffectDeliveryPolicy::AtLeastOnce,
            IdempotencyRequirement::Required,
        );
        assert!(validate_effect_admission(node, &declared_required).is_ok());
        assert!(requires_provider_idempotency(&declared_required));

        let no_deduplication = effect(
            EffectDeliveryPolicy::AtLeastOnce,
            IdempotencyRequirement::Optional,
        );
        assert!(!requires_provider_idempotency(&no_deduplication));
    }
}
