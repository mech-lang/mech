//! Portable semantic declarations selected explicitly by resource providers.
//!
//! These declarations describe artifact-time behavior only. The existing
//! `PreparedRuntimeEffect` variants remain authoritative for execution.

use std::sync::LazyLock;

use mech_core::{
    AccessMode, AliasPolicy, ChangeDetectionPolicy, DeliveryMode, EffectContract,
    EffectDeliveryPolicy, ExternalInteraction, IdempotencyRequirement, InputPortLayout,
    InputPortPolicy, ObservationContract, ObservationReplayPolicy, OperationContractDeclaration,
    OutputConstruction, OutputPortPolicy, ShapeRule, TransactionalEffectProtocol,
    TransactionalExternalContract,
};

static RESOURCE_OBSERVATION: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([])),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    });

static PROVIDER_DEFINED_EFFECT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    resource_write_contract(ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::ProviderDefined,
        idempotency: IdempotencyRequirement::Optional,
    }))
});

static COMPUTE_EFFECT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    resource_write_contract(ExternalInteraction::Effect(EffectContract {
        delivery: EffectDeliveryPolicy::AtMostOnce,
        idempotency: IdempotencyRequirement::NotRequired,
    }))
});

static PREPARE_COMMIT_COMPENSATE: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    resource_write_contract(ExternalInteraction::TransactionalExternal(
        TransactionalExternalContract {
            protocol: TransactionalEffectProtocol::PrepareCommitCompensate,
        },
    ))
});

fn resource_write_contract(interaction: ExternalInteraction) -> OperationContractDeclaration {
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

pub fn resource_observation_contract() -> &'static OperationContractDeclaration {
    &RESOURCE_OBSERVATION
}

pub fn provider_defined_effect_contract() -> &'static OperationContractDeclaration {
    &PROVIDER_DEFINED_EFFECT
}

/// Contract for compute input updates and turn dispatches.
///
/// Compute effects are delivered only after the coordinator transaction
/// commits and are never retried by the runtime.
pub fn compute_effect_contract() -> &'static OperationContractDeclaration {
    &COMPUTE_EFFECT
}

pub fn prepare_commit_compensate_contract() -> &'static OperationContractDeclaration {
    &PREPARE_COMMIT_COMPENSATE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_contracts_declare_observation_effect_and_transaction_boundaries() {
        let observation = resource_observation_contract();
        assert!(observation.inputs.resolve(0).is_ok());
        assert_eq!(observation.outputs.len(), 1);
        assert!(matches!(
            observation.interaction,
            ExternalInteraction::Observation(ObservationContract {
                replay: ObservationReplayPolicy::CaptureAsInputFact
            })
        ));

        let effect = provider_defined_effect_contract();
        assert!(effect.inputs.resolve(1).is_ok());
        assert!(effect.outputs.is_empty());
        assert!(matches!(
            effect.interaction,
            ExternalInteraction::Effect(EffectContract {
                delivery: EffectDeliveryPolicy::ProviderDefined,
                idempotency: IdempotencyRequirement::Optional,
            })
        ));

        let transactional = prepare_commit_compensate_contract();
        assert!(transactional.inputs.resolve(1).is_ok());
        assert!(transactional.outputs.is_empty());
        assert!(matches!(
            transactional.interaction,
            ExternalInteraction::TransactionalExternal(TransactionalExternalContract {
                protocol: TransactionalEffectProtocol::PrepareCommitCompensate,
            })
        ));

        let compute = compute_effect_contract();
        assert!(compute.inputs.resolve(1).is_ok());
        assert!(compute.outputs.is_empty());
        assert!(matches!(
            compute.interaction,
            ExternalInteraction::Effect(EffectContract {
                delivery: EffectDeliveryPolicy::AtMostOnce,
                idempotency: IdempotencyRequirement::NotRequired,
            })
        ));
    }
}
