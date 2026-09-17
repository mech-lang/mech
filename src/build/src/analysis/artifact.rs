use std::collections::BTreeMap;

use mech_core::{
    ApplicationRequirement, BindingId, FunctionCatalog, MResult, MechError, ReactiveInstanceId,
    ResourceIntent,
};
use mech_engine::{
    __resident::{ActivationFacts, CapturedValueInput, ResidentIntegrityMode, activate_external},
    BindingDeclaration, ProgramArtifact,
};

use crate::error::{NativeBuildErrorKind, native_build_error};

use super::requirements::NativeBytecodeContractResolver;

/// Plans the external contracts carried by a canonical retained artifact.
///
/// Artifact bytecode intentionally has no shadow instruction stream. Native
/// planning therefore evaluates the artifact itself, supplies provider-owned
/// observation values at its resident input boundary, and validates each
/// materialized effect payload with the same trusted planning providers used
/// by instruction bytecode.
pub(crate) fn plan_artifact_external_contracts(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    resolver: &mut NativeBytecodeContractResolver<'_>,
) -> MResult<()> {
    let mut observation_values = BTreeMap::new();
    let mut facts = ActivationFacts::default();
    let mut requires_effect_turn = false;

    for node in artifact.nodes() {
        let Some(operation) = node.as_operation() else {
            continue;
        };
        let Some(requirement_id) = operation.requirement else {
            continue;
        };
        let Some(ApplicationRequirement::Resource(request)) =
            artifact.requirements().get(requirement_id)
        else {
            requires_effect_turn = true;
            continue;
        };
        if request.intent != ResourceIntent::Read {
            requires_effect_turn = true;
            continue;
        }

        let value = resolver.plan_artifact_resource_read(node.node, request)?;
        let output_slot = node
            .output_bindings
            .clone()
            .map(BindingId::new)
            .find_map(
                |binding| match artifact.bindings().get(binding.get() as usize) {
                    Some(BindingDeclaration::Output {
                        node: owner,
                        target,
                        ..
                    }) if *owner == node.node => Some(*target),
                    _ => None,
                },
            )
            .ok_or_else(|| {
                artifact_error(format!(
                    "resource observation node {} has no output binding",
                    node.node.get()
                ))
            })?;
        facts.slot_shapes.insert(output_slot, value.shape().clone());
        if observation_values.insert(node.node, value).is_some() {
            return Err(artifact_error(format!(
                "resource observation node {} is duplicated",
                node.node.get()
            )));
        }
    }

    let mut instance = activate_external(
        ReactiveInstanceId::new(0x4e41_5449, 0),
        artifact,
        catalog,
        &facts,
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| artifact_error(format!("resident activation failed: {error:?}")))?;

    if !requires_effect_turn {
        return Ok(());
    }

    let captured = instance
        .plan
        .inputs
        .iter()
        .filter_map(|input| match input.source {
            mech_engine::__resident::ActivatedInputSource::Observation { node, .. } => {
                Some((input.slot, node))
            }
            mech_engine::__resident::ActivatedInputSource::DeclaredInput { .. } => None,
        })
        .map(|(slot, node)| {
            observation_values
                .get(&node)
                .map(|value| (slot, value))
                .ok_or_else(|| {
                    artifact_error(format!(
                        "resource observation node {} has no planned provider value",
                        node.get()
                    ))
                })
        })
        .collect::<MResult<Vec<_>>>()?;
    let inputs = captured
        .iter()
        .map(|(slot, value)| CapturedValueInput {
            slot: *slot,
            value: *value,
        })
        .collect::<Vec<_>>();
    let prepared = instance
        .prepare_turn_values(&inputs)
        .map_err(|error| artifact_error(format!("resident planning turn failed: {error:?}")))?;

    let effects = prepared
        .effect_intents()
        .map(|effect| (effect.artifact_node, effect.requirement, effect.ordinal))
        .collect::<Vec<_>>();
    for (node, requirement_id, ordinal) in effects {
        let Some(ApplicationRequirement::Resource(request)) =
            artifact.requirements().get(requirement_id)
        else {
            return Err(artifact_error(format!(
                "external node {} has a non-resource requirement",
                node.get()
            )));
        };
        let payload = prepared
            .materialize_effect_payload(ordinal)
            .map_err(|error| {
                artifact_error(format!(
                    "external node {} payload materialization failed: {}",
                    node.get(),
                    error.display_message()
                ))
            })?;
        resolver.plan_artifact_resource_write(node, request, &payload)?;
    }
    prepared.abort();
    Ok(())
}

fn artifact_error(reason: String) -> MechError {
    native_build_error(
        NativeBuildErrorKind::NativeProgramArtifactInvalid { reason },
        None,
    )
}
